// Command handler module

mod batch;
mod clipboard;
mod discover;
#[cfg(feature = "streaming")]
mod streaming;
mod transfer;

pub use batch::{
    BatchOperationArgs, BatchOperationHandler, BatchOperationItem, BatchOperationResult,
    BatchOperationStatus, BatchProgressInfo,
};
pub use clipboard::{ClipboardAction, ClipboardArgs, ClipboardHandler, ClipboardResult};
pub use discover::DiscoverHandler;
#[cfg(feature = "streaming")]
pub use streaming::{
    ExecHandler, NetworkDiagnostics, PeersHandler, StatusHandler, StreamingHandler, SystemStatus,
};
pub use transfer::TransferHandler;

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::types::{CommandResult, OperationStatus, PeerInfo};
use async_trait::async_trait;

/// Command handler trait
#[async_trait]
pub trait CommandHandler {
    /// Handle discover command
    async fn handle_discover(&self, args: DiscoverArgs) -> CLIResult<DiscoverResult>;

    /// Handle send command
    async fn handle_send(&self, args: SendArgs) -> CLIResult<TransferResult>;

    /// Handle receive command
    async fn handle_receive(&self, args: ReceiveArgs) -> CLIResult<ReceiveResult>;

    /// Handle stream command
    async fn handle_stream(&self, args: StreamArgs) -> CLIResult<StreamResult>;

    /// Handle exec command
    async fn handle_exec(&self, args: ExecArgs) -> CLIResult<ExecResult>;
}

/// Discover command arguments
#[derive(Debug, Clone)]
pub struct DiscoverArgs {
    pub filter_type: Option<String>,
    pub filter_name: Option<String>,
    pub timeout: Option<u64>,
    pub continuous: bool,
}

/// Discover command result
#[derive(Debug, Clone)]
pub struct DiscoverResult {
    pub peers: Vec<PeerInfo>,
    pub discovery_time: std::time::Duration,
}

/// Send command arguments
#[derive(Debug, Clone)]
pub struct SendArgs {
    pub files: Vec<std::path::PathBuf>,
    pub peer: String,
    pub compression: Option<bool>,
    pub encryption: Option<bool>,
}

/// Transfer result
#[derive(Debug, Clone)]
pub struct TransferResult {
    pub operation_id: uuid::Uuid,
    pub status: OperationStatus,
}

/// Receive command arguments
#[derive(Debug, Clone)]
pub struct ReceiveArgs {
    pub download_path: Option<std::path::PathBuf>,
    pub auto_accept: bool,
}

/// Receive command result
#[derive(Debug, Clone)]
pub struct ReceiveResult {
    pub operation_id: uuid::Uuid,
    pub status: OperationStatus,
}

/// Stream command arguments
#[derive(Debug, Clone)]
pub struct StreamArgs {
    pub camera_id: Option<String>,
    pub quality: Option<String>,
    pub record: bool,
    pub output_file: Option<std::path::PathBuf>,
}

/// Stream command result
#[derive(Debug, Clone)]
pub struct StreamResult {
    pub operation_id: uuid::Uuid,
    pub status: OperationStatus,
    pub stream_url: Option<String>,
}

/// Exec command arguments
#[derive(Debug, Clone)]
pub struct ExecArgs {
    pub command: String,
    pub peer: String,
    pub timeout: Option<u64>,
}

/// Exec command result
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub output: String,
    pub exit_code: i32,
    pub execution_time: std::time::Duration,
}
