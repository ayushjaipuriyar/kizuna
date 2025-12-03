pub mod discovery;
pub mod transport;
pub mod browser_support;
pub mod clipboard;
pub mod security;
pub mod file_transfer;
pub mod developer_api;
pub mod streaming;
pub mod cli;
pub mod command_execution;
pub mod platform;

pub use discovery::*;
pub use transport::*;
pub use browser_support::*;
pub use clipboard::*;
pub use security::*;
pub use file_transfer::*;
pub use developer_api::{KizunaAPI, KizunaInstance, KizunaConfig, KizunaError, KizunaEvent};
pub use cli::{CLIConfig, CLIError, CLIResult};

// Command execution exports (avoid glob to prevent ambiguous re-exports)
pub use command_execution::{
    CommandManager, SandboxEngine, AuthorizationManager, ScriptEngine,
    CommandError, CommandRequest, CommandResult as CmdExecutionResult,
    ScriptRequest, ScriptResult, SystemInfo, Notification,
};

// Platform exports
pub use platform::{
    PlatformManager, PlatformAdapter, PlatformInfo, PlatformCapabilities,
    OperatingSystem, Architecture, Feature, PlatformError, PlatformResult,
    DefaultPlatformManager, PlatformConfig,
};

/// Common result type for Kizuna operations
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;