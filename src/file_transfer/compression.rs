// Compression Engine Module
//
// Handles LZ4 compression for file chunks with automatic detection

use crate::file_transfer::{
    error::{FileTransferError, Result},
    types::Chunk,
};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};

/// Compression engine for file transfer chunks
pub struct CompressionEngine {
    /// Minimum file size to enable compression (1MB)
    min_size_for_compression: u64,
    /// Minimum compression ratio to keep compressed data (10% reduction)
    min_compression_ratio: f64,
}

impl CompressionEngine {
    /// Create a new compression engine with default settings
    pub fn new() -> Self {
        Self {
            min_size_for_compression: 1024 * 1024, // 1MB
            min_compression_ratio: 0.90, // Keep if compressed size is 90% or less of original
        }
    }

    /// Create a compression engine with custom settings
    pub fn with_settings(min_size_for_compression: u64, min_compression_ratio: f64) -> Self {
        Self {
            min_size_for_compression,
            min_compression_ratio,
        }
    }

    /// Check if compression should be enabled for a transfer
    /// Compression is enabled for transfers larger than 1MB
    pub fn should_compress_transfer(&self, total_size: u64) -> bool {
        total_size >= self.min_size_for_compression
    }

    /// Compress a chunk using LZ4
    /// Returns the compressed chunk if compression is effective (>10% reduction)
    /// Otherwise returns the original chunk unchanged
    pub fn compress_chunk(&self, mut chunk: Chunk) -> Result<Chunk> {
        // Don't compress if already compressed
        if chunk.compressed {
            return Ok(chunk);
        }

        // Don't compress empty chunks
        if chunk.data.is_empty() {
            return Ok(chunk);
        }

        // Compress the data
        let compressed_data = compress_prepend_size(&chunk.data);

        // Calculate compression ratio
        let original_size = chunk.data.len();
        let compressed_size = compressed_data.len();
        let compression_ratio = compressed_size as f64 / original_size as f64;

        // Check if compression is effective (at least 10% reduction)
        if compression_ratio <= self.min_compression_ratio {
            // Compression is effective, use compressed data
            chunk.data = compressed_data;
            chunk.compressed = true;
            chunk.size = compressed_size;
        }
        // Otherwise keep original data (compression not effective)

        Ok(chunk)
    }

    /// Decompress a chunk using LZ4
    /// Returns the decompressed chunk
    pub fn decompress_chunk(&self, mut chunk: Chunk) -> Result<Chunk> {
        // Don't decompress if not compressed
        if !chunk.compressed {
            return Ok(chunk);
        }

        // Decompress the data
        let decompressed_data = decompress_size_prepended(&chunk.data).map_err(|e| {
            FileTransferError::CompressionError(format!("Failed to decompress chunk: {}", e))
        })?;

        // Update chunk with decompressed data
        chunk.data = decompressed_data;
        chunk.size = chunk.data.len();
        chunk.compressed = false;

        Ok(chunk)
    }

    /// Compress multiple chunks in batch
    /// Returns vector of compressed chunks
    pub fn compress_chunks(&self, chunks: Vec<Chunk>) -> Result<Vec<Chunk>> {
        chunks
            .into_iter()
            .map(|chunk| self.compress_chunk(chunk))
            .collect()
    }

    /// Decompress multiple chunks in batch
    /// Returns vector of decompressed chunks
    pub fn decompress_chunks(&self, chunks: Vec<Chunk>) -> Result<Vec<Chunk>> {
        chunks
            .into_iter()
            .map(|chunk| self.decompress_chunk(chunk))
            .collect()
    }

    /// Calculate compression statistics for a set of chunks
    pub fn calculate_compression_stats(&self, original_chunks: &[Chunk], compressed_chunks: &[Chunk]) -> CompressionStats {
        let original_size: usize = original_chunks.iter().map(|c| c.data.len()).sum();
        let compressed_size: usize = compressed_chunks.iter().map(|c| c.data.len()).sum();
        let compressed_count = compressed_chunks.iter().filter(|c| c.compressed).count();

        let compression_ratio = if original_size > 0 {
            compressed_size as f64 / original_size as f64
        } else {
            1.0
        };

        let space_saved = original_size.saturating_sub(compressed_size);
        let space_saved_percentage = if original_size > 0 {
            (space_saved as f64 / original_size as f64) * 100.0
        } else {
            0.0
        };

        CompressionStats {
            original_size,
            compressed_size,
            compression_ratio,
            space_saved,
            space_saved_percentage,
            chunks_compressed: compressed_count,
            total_chunks: compressed_chunks.len(),
        }
    }
}

impl Default for CompressionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Compression statistics
#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f64,
    pub space_saved: usize,
    pub space_saved_percentage: f64,
    pub chunks_compressed: usize,
    pub total_chunks: usize,
}

impl CompressionStats {
    /// Check if compression was effective overall
    pub fn is_effective(&self) -> bool {
        self.compression_ratio < 0.90 // At least 10% reduction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_chunk(data: Vec<u8>) -> Chunk {
        Chunk {
            chunk_id: 0,
            file_path: PathBuf::from("test.txt"),
            offset: 0,
            size: data.len(),
            data,
            checksum: [0u8; 32],
            compressed: false,
        }
    }

    #[test]
    fn test_should_compress_transfer() {
        let engine = CompressionEngine::new();
        
        // Should not compress small transfers
        assert!(!engine.should_compress_transfer(500 * 1024)); // 500KB
        
        // Should compress large transfers
        assert!(engine.should_compress_transfer(2 * 1024 * 1024)); // 2MB
        assert!(engine.should_compress_transfer(1024 * 1024)); // Exactly 1MB
    }

    #[test]
    fn test_compress_chunk_with_compressible_data() {
        let engine = CompressionEngine::new();
        
        // Create highly compressible data (repeated pattern)
        let data = vec![b'A'; 10000];
        let chunk = create_test_chunk(data.clone());
        
        let compressed = engine.compress_chunk(chunk).unwrap();
        
        // Should be compressed
        assert!(compressed.compressed);
        // Compressed size should be much smaller
        assert!(compressed.data.len() < data.len());
    }

    #[test]
    fn test_compress_chunk_with_incompressible_data() {
        let engine = CompressionEngine::new();
        
        // Create incompressible data (random-like)
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let original_len = data.len();
        let chunk = create_test_chunk(data);
        
        let result = engine.compress_chunk(chunk).unwrap();
        
        // May or may not be compressed depending on effectiveness
        // If compressed, size should not be much larger
        if result.compressed {
            assert!(result.data.len() <= original_len * 2);
        } else {
            assert_eq!(result.data.len(), original_len);
        }
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let engine = CompressionEngine::new();
        
        // Create test data
        let original_data = b"Hello, World! This is a test of compression.".repeat(100);
        let chunk = create_test_chunk(original_data.clone());
        
        // Compress
        let compressed = engine.compress_chunk(chunk).unwrap();
        
        // Decompress
        let decompressed = engine.decompress_chunk(compressed).unwrap();
        
        // Should match original
        assert_eq!(decompressed.data, original_data);
        assert!(!decompressed.compressed);
    }

    #[test]
    fn test_compress_already_compressed_chunk() {
        let engine = CompressionEngine::new();
        
        let data = vec![b'A'; 1000];
        let mut chunk = create_test_chunk(data.clone());
        chunk.compressed = true;
        
        let result = engine.compress_chunk(chunk).unwrap();
        
        // Should remain compressed with same data
        assert!(result.compressed);
        assert_eq!(result.data, data);
    }

    #[test]
    fn test_decompress_uncompressed_chunk() {
        let engine = CompressionEngine::new();
        
        let data = vec![b'A'; 1000];
        let chunk = create_test_chunk(data.clone());
        
        let result = engine.decompress_chunk(chunk).unwrap();
        
        // Should remain uncompressed with same data
        assert!(!result.compressed);
        assert_eq!(result.data, data);
    }

    #[test]
    fn test_compress_empty_chunk() {
        let engine = CompressionEngine::new();
        
        let chunk = create_test_chunk(Vec::new());
        let result = engine.compress_chunk(chunk).unwrap();
        
        // Empty chunk should not be compressed
        assert!(!result.compressed);
        assert!(result.data.is_empty());
    }

    #[test]
    fn test_compress_chunks_batch() {
        let engine = CompressionEngine::new();
        
        let chunks = vec![
            create_test_chunk(vec![b'A'; 1000]),
            create_test_chunk(vec![b'B'; 1000]),
            create_test_chunk(vec![b'C'; 1000]),
        ];
        
        let compressed = engine.compress_chunks(chunks).unwrap();
        
        assert_eq!(compressed.len(), 3);
        // All should be compressed (highly compressible data)
        assert!(compressed.iter().all(|c| c.compressed));
    }

    #[test]
    fn test_calculate_compression_stats() {
        let engine = CompressionEngine::new();
        
        let original_chunks = vec![
            create_test_chunk(vec![b'A'; 1000]),
            create_test_chunk(vec![b'B'; 1000]),
        ];
        
        let compressed_chunks = engine.compress_chunks(original_chunks.clone()).unwrap();
        
        let stats = engine.calculate_compression_stats(&original_chunks, &compressed_chunks);
        
        assert_eq!(stats.original_size, 2000);
        assert!(stats.compressed_size < stats.original_size);
        assert!(stats.compression_ratio < 1.0);
        assert!(stats.space_saved > 0);
        assert!(stats.space_saved_percentage > 0.0);
        assert_eq!(stats.total_chunks, 2);
    }

    #[test]
    fn test_custom_compression_settings() {
        let engine = CompressionEngine::with_settings(
            500 * 1024, // 500KB minimum
            0.95,       // Keep if 95% or less (only 5% reduction needed)
        );
        
        assert!(engine.should_compress_transfer(600 * 1024));
        assert!(!engine.should_compress_transfer(400 * 1024));
    }
}
