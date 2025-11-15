// Manifest Building Module
//
// Handles file scanning, metadata extraction, and manifest creation

use crate::file_transfer::{
    error::{FileTransferError, Result},
    types::*,
};
use sha2::{Digest, Sha256};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use tokio::fs as tokio_fs;
use tokio::io::AsyncReadExt;
use walkdir::WalkDir;

/// Scanned file information
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub size: u64,
    pub permissions: FilePermissions,
    pub modified_at: Timestamp,
    pub is_symlink: bool,
    pub symlink_target: Option<PathBuf>,
}

/// Scanned directory information
#[derive(Debug, Clone)]
pub struct ScannedDirectory {
    pub path: PathBuf,
    pub permissions: FilePermissions,
    pub created_at: Timestamp,
}

/// File and directory scanner
pub struct FileScanner;

impl FileScanner {
    /// Scan a single file and extract metadata
    pub fn scan_file(path: &Path) -> Result<ScannedFile> {
        let metadata = fs::metadata(path).map_err(|e| FileTransferError::ScanError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let is_symlink = metadata.file_type().is_symlink();
        let symlink_target = if is_symlink {
            fs::read_link(path).ok()
        } else {
            None
        };

        // Get actual file metadata (follow symlink if needed)
        let actual_metadata = if is_symlink {
            fs::metadata(path).map_err(|e| FileTransferError::ScanError {
                path: path.to_path_buf(),
                source: e,
            })?
        } else {
            metadata
        };

        Ok(ScannedFile {
            path: path.to_path_buf(),
            size: actual_metadata.len(),
            permissions: Self::extract_permissions(&actual_metadata),
            modified_at: Self::extract_modified_time(&actual_metadata),
            is_symlink,
            symlink_target,
        })
    }

    /// Scan a directory recursively
    pub fn scan_directory(path: &Path, recursive: bool) -> Result<(Vec<ScannedFile>, Vec<ScannedDirectory>)> {
        let mut files = Vec::new();
        let mut directories = Vec::new();

        let walker = if recursive {
            WalkDir::new(path).follow_links(false)
        } else {
            WalkDir::new(path).max_depth(1).follow_links(false)
        };

        for entry in walker {
            let entry = entry.map_err(|e| FileTransferError::ScanError {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;

            let entry_path = entry.path();
            
            // Skip the root directory itself
            if entry_path == path {
                continue;
            }

            let metadata = entry.metadata().map_err(|e| FileTransferError::ScanError {
                path: entry_path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::Other, e),
            })?;

            if metadata.is_dir() {
                directories.push(ScannedDirectory {
                    path: entry_path.to_path_buf(),
                    permissions: Self::extract_permissions(&metadata),
                    created_at: Self::extract_created_time(&metadata),
                });
            } else if metadata.is_file() {
                let is_symlink = metadata.file_type().is_symlink();
                let symlink_target = if is_symlink {
                    fs::read_link(entry_path).ok()
                } else {
                    None
                };

                files.push(ScannedFile {
                    path: entry_path.to_path_buf(),
                    size: metadata.len(),
                    permissions: Self::extract_permissions(&metadata),
                    modified_at: Self::extract_modified_time(&metadata),
                    is_symlink,
                    symlink_target,
                });
            }
            // Skip other special file types (devices, sockets, etc.)
        }

        Ok((files, directories))
    }

    /// Extract file permissions from metadata
    fn extract_permissions(metadata: &Metadata) -> FilePermissions {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();
            FilePermissions {
                readonly: metadata.permissions().readonly(),
                executable: (mode & 0o111) != 0,
                mode,
            }
        }

        #[cfg(not(unix))]
        {
            FilePermissions {
                readonly: metadata.permissions().readonly(),
                executable: false,
            }
        }
    }

    /// Extract modified time from metadata
    fn extract_modified_time(metadata: &Metadata) -> Timestamp {
        metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Extract created time from metadata
    fn extract_created_time(metadata: &Metadata) -> Timestamp {
        metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or_else(|| Self::extract_modified_time(metadata))
    }
}

/// Manifest builder implementation
pub struct ManifestBuilderImpl {
    sender_id: PeerId,
}

impl ManifestBuilderImpl {
    pub fn new(sender_id: PeerId) -> Self {
        Self { sender_id }
    }
}

/// Checksum calculator for files and manifests
pub struct ChecksumCalculator;

impl ChecksumCalculator {
    /// Calculate SHA-256 checksum for a file
    pub async fn calculate_file_checksum(path: &Path) -> Result<[u8; 32]> {
        let mut file = tokio_fs::File::open(path)
            .await
            .map_err(|e| FileTransferError::ChecksumError {
                path: path.to_path_buf(),
                source: e,
            })?;

        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

        loop {
            let bytes_read = file
                .read(&mut buffer)
                .await
                .map_err(|e| FileTransferError::ChecksumError {
                    path: path.to_path_buf(),
                    source: e,
                })?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        let mut checksum = [0u8; 32];
        checksum.copy_from_slice(&result);
        Ok(checksum)
    }

    /// Calculate SHA-256 checksum for manifest data
    pub fn calculate_manifest_checksum(manifest: &TransferManifest) -> Result<[u8; 32]> {
        let mut hasher = Sha256::new();

        // Hash transfer ID
        hasher.update(manifest.transfer_id.as_bytes());

        // Hash sender ID
        hasher.update(manifest.sender_id.as_bytes());

        // Hash total size and file count
        hasher.update(&manifest.total_size.to_le_bytes());
        hasher.update(&manifest.file_count.to_le_bytes());

        // Hash all file entries
        for file in &manifest.files {
            hasher.update(file.path.to_string_lossy().as_bytes());
            hasher.update(&file.size.to_le_bytes());
            hasher.update(&file.checksum);
        }

        // Hash all directory entries
        for dir in &manifest.directories {
            hasher.update(dir.path.to_string_lossy().as_bytes());
        }

        let result = hasher.finalize();
        let mut checksum = [0u8; 32];
        checksum.copy_from_slice(&result);
        Ok(checksum)
    }
}

/// Manifest validator
pub struct ManifestValidator;

impl ManifestValidator {
    /// Validate manifest structure and checksums
    pub fn validate(manifest: &TransferManifest) -> Result<bool> {
        // Check that file count matches
        if manifest.files.len() != manifest.file_count {
            return Err(FileTransferError::InvalidManifest {
                reason: format!(
                    "File count mismatch: expected {}, found {}",
                    manifest.file_count,
                    manifest.files.len()
                ),
            });
        }

        // Check that total size matches sum of file sizes
        let calculated_size: u64 = manifest.files.iter().map(|f| f.size).sum();
        if calculated_size != manifest.total_size {
            return Err(FileTransferError::InvalidManifest {
                reason: format!(
                    "Total size mismatch: expected {}, calculated {}",
                    manifest.total_size, calculated_size
                ),
            });
        }

        // Verify manifest checksum
        let calculated_checksum = ChecksumCalculator::calculate_manifest_checksum(manifest)?;
        if calculated_checksum != manifest.checksum {
            return Err(FileTransferError::ManifestVerificationFailed {
                reason: "Manifest checksum mismatch".to_string(),
            });
        }

        Ok(true)
    }

    /// Validate file entry
    pub fn validate_file_entry(entry: &FileEntry) -> Result<bool> {
        // Check that chunk count is correct for file size
        let expected_chunks = (entry.size + Chunk::DEFAULT_SIZE as u64 - 1) / Chunk::DEFAULT_SIZE as u64;
        if entry.chunk_count != expected_chunks as usize {
            return Err(FileTransferError::InvalidManifest {
                reason: format!(
                    "Chunk count mismatch for {}: expected {}, found {}",
                    entry.path.display(),
                    expected_chunks,
                    entry.chunk_count
                ),
            });
        }

        Ok(true)
    }
}

/// Progress callback for manifest creation
pub type ManifestProgressCallback = Box<dyn Fn(usize, usize) + Send + Sync>;

impl ManifestBuilderImpl {
    /// Build manifest for a single file
    pub async fn build_file_manifest(&self, path: PathBuf) -> Result<TransferManifest> {
        self.build_file_manifest_with_progress(path, None).await
    }

    /// Build manifest for a single file with progress tracking
    pub async fn build_file_manifest_with_progress(
        &self,
        path: PathBuf,
        progress_callback: Option<ManifestProgressCallback>,
    ) -> Result<TransferManifest> {
        // Validate path exists and is a file
        if !path.exists() {
            return Err(FileTransferError::InvalidPath {
                path: path.clone(),
            });
        }

        if !path.is_file() {
            return Err(FileTransferError::InvalidManifest {
                reason: format!("{} is not a file", path.display()),
            });
        }

        // Scan the file
        let scanned_file = FileScanner::scan_file(&path)?;

        // Report progress
        if let Some(ref callback) = progress_callback {
            callback(0, 1);
        }

        // Calculate checksum
        let checksum = ChecksumCalculator::calculate_file_checksum(&path).await?;

        // Calculate chunk count
        let chunk_count = ((scanned_file.size + Chunk::DEFAULT_SIZE as u64 - 1) 
            / Chunk::DEFAULT_SIZE as u64) as usize;

        // Create file entry
        let file_entry = FileEntry {
            path: scanned_file.path,
            size: scanned_file.size,
            checksum,
            permissions: scanned_file.permissions,
            modified_at: scanned_file.modified_at,
            chunk_count,
        };

        // Report progress
        if let Some(ref callback) = progress_callback {
            callback(1, 1);
        }

        // Create manifest
        let mut manifest = TransferManifest::new(self.sender_id.clone());
        manifest.files.push(file_entry);
        manifest.file_count = 1;
        manifest.total_size = scanned_file.size;

        // Calculate manifest checksum
        manifest.checksum = ChecksumCalculator::calculate_manifest_checksum(&manifest)?;

        Ok(manifest)
    }

    /// Build manifest for multiple files
    pub async fn build_multi_file_manifest(&self, paths: Vec<PathBuf>) -> Result<TransferManifest> {
        self.build_multi_file_manifest_with_progress(paths, None).await
    }

    /// Build manifest for multiple files with progress tracking
    pub async fn build_multi_file_manifest_with_progress(
        &self,
        paths: Vec<PathBuf>,
        progress_callback: Option<ManifestProgressCallback>,
    ) -> Result<TransferManifest> {
        if paths.is_empty() {
            return Err(FileTransferError::InvalidManifest {
                reason: "No files provided".to_string(),
            });
        }

        let total_files = paths.len();
        let mut manifest = TransferManifest::new(self.sender_id.clone());
        let mut processed = 0;

        for path in paths {
            // Validate path exists and is a file
            if !path.exists() {
                return Err(FileTransferError::InvalidPath {
                    path: path.clone(),
                });
            }

            if !path.is_file() {
                return Err(FileTransferError::InvalidManifest {
                    reason: format!("{} is not a file", path.display()),
                });
            }

            // Scan the file
            let scanned_file = FileScanner::scan_file(&path)?;

            // Calculate checksum
            let checksum = ChecksumCalculator::calculate_file_checksum(&path).await?;

            // Calculate chunk count
            let chunk_count = ((scanned_file.size + Chunk::DEFAULT_SIZE as u64 - 1) 
                / Chunk::DEFAULT_SIZE as u64) as usize;

            // Create file entry
            let file_entry = FileEntry {
                path: scanned_file.path,
                size: scanned_file.size,
                checksum,
                permissions: scanned_file.permissions,
                modified_at: scanned_file.modified_at,
                chunk_count,
            };

            manifest.files.push(file_entry);
            manifest.total_size += scanned_file.size;

            processed += 1;

            // Report progress
            if let Some(ref callback) = progress_callback {
                callback(processed, total_files);
            }
        }

        manifest.file_count = manifest.files.len();

        // Calculate manifest checksum
        manifest.checksum = ChecksumCalculator::calculate_manifest_checksum(&manifest)?;

        Ok(manifest)
    }

    /// Build manifest for a folder (recursive)
    pub async fn build_folder_manifest(
        &self,
        path: PathBuf,
        recursive: bool,
    ) -> Result<TransferManifest> {
        self.build_folder_manifest_with_progress(path, recursive, None).await
    }

    /// Build manifest for a folder with progress tracking
    pub async fn build_folder_manifest_with_progress(
        &self,
        path: PathBuf,
        recursive: bool,
        progress_callback: Option<ManifestProgressCallback>,
    ) -> Result<TransferManifest> {
        // Validate path exists and is a directory
        if !path.exists() {
            return Err(FileTransferError::InvalidPath {
                path: path.clone(),
            });
        }

        if !path.is_dir() {
            return Err(FileTransferError::InvalidManifest {
                reason: format!("{} is not a directory", path.display()),
            });
        }

        // Scan directory
        let (scanned_files, scanned_directories) = FileScanner::scan_directory(&path, recursive)?;

        let total_files = scanned_files.len();
        let mut manifest = TransferManifest::new(self.sender_id.clone());
        let mut processed = 0;

        // Add directory entries
        for scanned_dir in scanned_directories {
            manifest.directories.push(DirectoryEntry {
                path: scanned_dir.path,
                permissions: scanned_dir.permissions,
                created_at: scanned_dir.created_at,
            });
        }

        // Process files
        for scanned_file in scanned_files {
            // Calculate checksum
            let checksum = ChecksumCalculator::calculate_file_checksum(&scanned_file.path).await?;

            // Calculate chunk count
            let chunk_count = ((scanned_file.size + Chunk::DEFAULT_SIZE as u64 - 1) 
                / Chunk::DEFAULT_SIZE as u64) as usize;

            // Create file entry
            let file_entry = FileEntry {
                path: scanned_file.path,
                size: scanned_file.size,
                checksum,
                permissions: scanned_file.permissions,
                modified_at: scanned_file.modified_at,
                chunk_count,
            };

            manifest.files.push(file_entry);
            manifest.total_size += scanned_file.size;

            processed += 1;

            // Report progress
            if let Some(ref callback) = progress_callback {
                callback(processed, total_files);
            }
        }

        manifest.file_count = manifest.files.len();

        // Calculate manifest checksum
        manifest.checksum = ChecksumCalculator::calculate_manifest_checksum(&manifest)?;

        Ok(manifest)
    }

    /// Verify manifest integrity
    pub async fn verify_manifest(&self, manifest: &TransferManifest) -> Result<bool> {
        ManifestValidator::validate(manifest)
    }
}

// Implement the ManifestBuilder trait
use crate::file_transfer::ManifestBuilder;
use async_trait::async_trait;

#[async_trait]
impl ManifestBuilder for ManifestBuilderImpl {
    async fn build_file_manifest(&self, path: PathBuf) -> Result<TransferManifest> {
        self.build_file_manifest(path).await
    }

    async fn build_multi_file_manifest(&self, paths: Vec<PathBuf>) -> Result<TransferManifest> {
        self.build_multi_file_manifest(paths).await
    }

    async fn build_folder_manifest(
        &self,
        path: PathBuf,
        recursive: bool,
    ) -> Result<TransferManifest> {
        self.build_folder_manifest(path, recursive).await
    }

    async fn verify_manifest(&self, manifest: &TransferManifest) -> Result<bool> {
        self.verify_manifest(manifest).await
    }
}
