pub mod discovery;
pub mod transport;
pub mod browser_support;
pub mod clipboard;
pub mod security;
pub mod file_transfer;
pub mod developer_api;

pub use discovery::*;
pub use transport::*;
pub use browser_support::*;
pub use clipboard::*;
pub use security::*;
pub use file_transfer::*;
pub use developer_api::{KizunaAPI, KizunaInstance, KizunaConfig, KizunaError, KizunaEvent};

/// Common result type for Kizuna operations
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;