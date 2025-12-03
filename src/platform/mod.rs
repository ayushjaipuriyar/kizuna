// Platform abstraction and cross-platform support
//
// This module provides platform detection, capability management,
// and platform-specific implementations for all supported platforms.

pub mod types;
pub mod traits;
pub mod detection;
pub mod capabilities;
pub mod adapter;
pub mod container;
pub mod performance;
pub mod resource_monitor;
pub mod metrics;
pub mod build_system;
pub mod deployment;
pub mod feature_parity;

// Platform-specific implementations
#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "ios")]
pub mod ios;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

// Re-exports
pub use types::*;
pub use traits::*;
pub use detection::*;
pub use capabilities::*;
pub use adapter::{DefaultPlatformManager, GenericAdapter};
pub use performance::*;
pub use resource_monitor::*;
pub use metrics::*;
pub use build_system::*;
pub use deployment::*;
pub use feature_parity::*;

use thiserror::Error;

/// Platform-specific errors
#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("Platform detection failed: {0}")]
    DetectionError(String),

    #[error("Feature unavailable on this platform: {0}")]
    FeatureUnavailable(String),

    #[error("Platform integration error: {0}")]
    IntegrationError(String),

    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("System error: {0}")]
    SystemError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type PlatformResult<T> = Result<T, PlatformError>;
