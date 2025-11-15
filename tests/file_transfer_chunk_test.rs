// Integration test for file transfer chunk engine

use kizuna::file_transfer::{ChunkEngine, ChunkEngineImpl};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_chunk_creation_and_reassembly() {
    // Create temporary directory and test file
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.txt");
    let output_path = temp_dir.path().join("output.txt");
    
    // Create test data (larger than one chunk to test multiple chunks)
    let test_data = "A".repeat(100_000); // 100KB of data
    fs::write(&input_path, &test_data).unwrap();

    // Create chunk engine
    let engine = ChunkEngineImpl::new();

    // Create chunks from file
    let chunks = engine.create_chunks(input_path.clone()).await.unwrap();

    // Verify chunks were created
    assert!(!chunks.is_empty());
    assert!(chunks.len() > 1, "Should have multiple chunks for 100KB file");

    // Verify chunk properties
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.chunk_id, i as u64);
        assert_eq!(chunk.file_path, input_path);
        assert!(!chunk.data.is_empty());
        
        // Verify chunk integrity
        let is_valid = engine.verify_chunk(chunk).await.unwrap();
        assert!(is_valid, "Chunk {} should be valid", i);
    }

    // Reassemble file from chunks
    engine.reassemble_file(chunks, output_path.clone()).await.unwrap();

    // Verify reassembled file matches original
    let output_data = fs::read_to_string(&output_path).unwrap();
    assert_eq!(output_data, test_data);
}

#[tokio::test]
async fn test_small_file_chunking() {
    // Create temporary directory and small test file
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("small.txt");
    let output_path = temp_dir.path().join("small_output.txt");
    
    // Create small test data (less than one chunk)
    let test_data = "Hello, World!";
    fs::write(&input_path, test_data).unwrap();

    // Create chunk engine
    let engine = ChunkEngineImpl::new();

    // Create chunks from file
    let chunks = engine.create_chunks(input_path.clone()).await.unwrap();

    // Verify single chunk was created
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].chunk_id, 0);
    assert_eq!(chunks[0].size, test_data.len());

    // Reassemble file from chunks
    engine.reassemble_file(chunks, output_path.clone()).await.unwrap();

    // Verify reassembled file matches original
    let output_data = fs::read_to_string(&output_path).unwrap();
    assert_eq!(output_data, test_data);
}
