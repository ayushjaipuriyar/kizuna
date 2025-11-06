pub mod discovery;
pub mod transport;
pub mod browser_support;

pub use discovery::*;
pub use transport::*;
pub use browser_support::*;

/// Common result type for Kizuna operations
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;