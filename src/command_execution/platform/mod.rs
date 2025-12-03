// Platform-specific command execution implementations
//
// This module provides platform-specific command execution backends for Windows,
// macOS, and Linux systems with cross-platform command translation.

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod unix;

pub mod translator;
pub mod executor;
pub mod manager;

// Re-export platform-specific executor
#[cfg(target_os = "windows")]
pub use windows::WindowsExecutor;

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use unix::UnixExecutor;

pub use translator::{CommandTranslator, Platform};
pub use executor::{PlatformExecutor, ExecutionContext, ExecutionResult};
pub use manager::UnifiedCommandManager;
