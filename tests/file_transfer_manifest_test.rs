// Integration test for file transfer manifest building

use kizuna::file_transfer::{ManifestBuilder, ManifestBuilderImpl};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

#[tokio::test]
async fn test_single_file_manifest() {
    // Create temporary directory and file
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_file.txt");
    
    let mut file = fs::File::create(&file_path).unwrap();
    file.write_all(b"Hello, World!").unwrap();
    drop(file);

    // Build manifest
    let builder = ManifestBuilderImpl::new("test_peer".to_string());
    let manifest = builder.build_file_manifest(file_path.clone()).await.unwrap();

    // Verify manifest
    assert_eq!(manifest.file_count, 1);
    assert_eq!(manifest.files.len(), 1);
    assert_eq!(manifest.files[0].size, 13);
    assert_eq!(manifest.total_size, 13);
    assert_eq!(manifest.directories.len(), 0);
    
    // Verify manifest validation
    let is_valid = builder.verify_manifest(&manifest).await.unwrap();
    assert!(is_valid);
}

#[tokio::test]
async fn test_multi_file_manifest() {
    // Create temporary directory and files
    let temp_dir = TempDir::new().unwrap();
    
    let file1_path = temp_dir.path().join("file1.txt");
    let file2_path = temp_dir.path().join("file2.txt");
    
    fs::write(&file1_path, b"File 1 content").unwrap();
    fs::write(&file2_path, b"File 2 content with more data").unwrap();

    // Build manifest
    let builder = ManifestBuilderImpl::new("test_peer".to_string());
    let manifest = builder
        .build_multi_file_manifest(vec![file1_path, file2_path])
        .await
        .unwrap();

    // Verify manifest
    assert_eq!(manifest.file_count, 2);
    assert_eq!(manifest.files.len(), 2);
    assert_eq!(manifest.total_size, 14 + 30);
    assert_eq!(manifest.directories.len(), 0);
    
    // Verify manifest validation
    let is_valid = builder.verify_manifest(&manifest).await.unwrap();
    assert!(is_valid);
}

#[tokio::test]
async fn test_folder_manifest() {
    // Create temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir).unwrap();
    
    let file1_path = temp_dir.path().join("file1.txt");
    let file2_path = sub_dir.join("file2.txt");
    
    fs::write(&file1_path, b"Root file").unwrap();
    fs::write(&file2_path, b"Subdirectory file").unwrap();

    // Build manifest (recursive)
    let builder = ManifestBuilderImpl::new("test_peer".to_string());
    let manifest = builder
        .build_folder_manifest(temp_dir.path().to_path_buf(), true)
        .await
        .unwrap();

    // Verify manifest
    assert_eq!(manifest.file_count, 2);
    assert_eq!(manifest.files.len(), 2);
    assert_eq!(manifest.total_size, 9 + 17);
    assert_eq!(manifest.directories.len(), 1);
    
    // Verify manifest validation
    let is_valid = builder.verify_manifest(&manifest).await.unwrap();
    assert!(is_valid);
}

#[tokio::test]
async fn test_folder_manifest_non_recursive() {
    // Create temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let sub_dir = temp_dir.path().join("subdir");
    fs::create_dir(&sub_dir).unwrap();
    
    let file1_path = temp_dir.path().join("file1.txt");
    let file2_path = sub_dir.join("file2.txt");
    
    fs::write(&file1_path, b"Root file").unwrap();
    fs::write(&file2_path, b"Subdirectory file").unwrap();

    // Build manifest (non-recursive)
    let builder = ManifestBuilderImpl::new("test_peer".to_string());
    let manifest = builder
        .build_folder_manifest(temp_dir.path().to_path_buf(), false)
        .await
        .unwrap();

    // Verify manifest - should only include root file
    assert_eq!(manifest.file_count, 1);
    assert_eq!(manifest.files.len(), 1);
    assert_eq!(manifest.total_size, 9);
    assert_eq!(manifest.directories.len(), 0);
    
    // Verify manifest validation
    let is_valid = builder.verify_manifest(&manifest).await.unwrap();
    assert!(is_valid);
}
