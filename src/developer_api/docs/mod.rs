/// Documentation generation and management system
pub mod generator;
pub mod examples;
pub mod versioning;

pub use generator::{DocGenerator, DocFormat, Documentation, DocMetadata};
pub use examples::{CodeExample, ExampleLanguage, ExampleManager, ExampleCategory};
pub use versioning::{DocVersion, VersionManager, Changelog, ApiChange, ChangeType, CompatibilityEntry};

/// Result type for documentation operations
pub type Result<T> = std::result::Result<T, DocError>;

/// Documentation system errors
#[derive(Debug, thiserror::Error)]
pub enum DocError {
    #[error("Documentation generation failed: {0}")]
    GenerationError(String),
    
    #[error("Example validation failed: {0}")]
    ExampleError(String),
    
    #[error("Version mismatch: {0}")]
    VersionError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Other error: {0}")]
    Other(String),
}
