// Core CLI data structures and types

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

/// Type alias for peer identifiers
pub type PeerId = Uuid;

/// Type alias for operation identifiers
pub type OperationId = Uuid;

/// Type alias for timestamps
pub type Timestamp = chrono::DateTime<chrono::Utc>;

/// CLI configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLIConfig {
    pub default_peer: Option<String>,
    pub output_format: OutputFormat,
    pub color_mode: ColorMode,
    pub transfer_settings: TransferSettings,
    pub stream_settings: StreamSettings,
    pub profiles: HashMap<String, ConfigProfile>,
}

impl Default for CLIConfig {
    fn default() -> Self {
        Self {
            default_peer: None,
            output_format: OutputFormat::Table,
            color_mode: ColorMode::Auto,
            transfer_settings: TransferSettings::default(),
            stream_settings: StreamSettings::default(),
            profiles: HashMap::new(),
        }
    }
}

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Table,
    JSON,
    CSV,
    Minimal,
}

/// Color mode options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

/// Transfer settings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSettings {
    pub compression: bool,
    pub encryption: bool,
    pub default_download_path: Option<PathBuf>,
    pub auto_accept_trusted: bool,
}

impl Default for TransferSettings {
    fn default() -> Self {
        Self {
            compression: true,
            encryption: true,
            default_download_path: None,
            auto_accept_trusted: false,
        }
    }
}

/// Stream settings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSettings {
    pub default_quality: String,
    pub auto_record: bool,
    pub recording_path: Option<PathBuf>,
}

impl Default for StreamSettings {
    fn default() -> Self {
        Self {
            default_quality: "medium".to_string(),
            auto_record: false,
            recording_path: None,
        }
    }
}

/// Configuration profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigProfile {
    pub name: String,
    pub description: String,
    pub settings: HashMap<String, serde_json::Value>,
}

/// Parsed command structure
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub command: CommandType,
    pub subcommand: Option<String>,
    pub arguments: Vec<String>,
    pub options: HashMap<String, String>,
    pub flags: HashSet<String>,
}

/// Command types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    Discover,
    Send,
    Receive,
    Stream,
    Exec,
    Peers,
    Status,
    Clipboard,
    TUI,
    Config,
}

/// TUI application state
#[derive(Debug, Clone)]
pub struct TUIState {
    pub current_view: ViewType,
    pub selected_peer: Option<PeerId>,
    pub file_browser_path: PathBuf,
    pub active_operations: Vec<OperationStatus>,
    pub peer_list: Vec<PeerInfo>,
    pub navigation_stack: Vec<ViewType>,
}

/// TUI view types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewType {
    PeerList,
    FileBrowser,
    TransferProgress,
    StreamViewer,
    CommandTerminal,
    Settings,
}

/// Peer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: PeerId,
    pub name: String,
    pub device_type: String,
    pub connection_status: ConnectionStatus,
    pub capabilities: Vec<String>,
    pub trust_status: TrustStatus,
    pub last_seen: Option<Timestamp>,
}

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error,
}

/// Trust status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustStatus {
    Trusted,
    Untrusted,
    Blocked,
}

/// Operation status
#[derive(Debug, Clone)]
pub struct OperationStatus {
    pub operation_id: OperationId,
    pub operation_type: OperationType,
    pub peer_id: PeerId,
    pub status: OperationState,
    pub progress: Option<ProgressInfo>,
    pub started_at: Timestamp,
    pub estimated_completion: Option<Timestamp>,
}

/// Operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    FileTransfer,
    CameraStream,
    CommandExecution,
    ClipboardSync,
}

/// Operation state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationState {
    Starting,
    InProgress,
    Completed,
    Failed(String),
    Cancelled,
}

/// Progress information
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub current: u64,
    pub total: Option<u64>,
    pub rate: Option<f64>,
    pub eta: Option<Duration>,
    pub message: Option<String>,
}

/// Command execution result
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub output: CommandOutput,
    pub execution_time: Duration,
    pub exit_code: i32,
}

/// Command output types
#[derive(Debug, Clone)]
pub enum CommandOutput {
    Text(String),
    Table(TableData),
    JSON(serde_json::Value),
    Progress(ProgressInfo),
    Interactive,
}

/// Table data structure
#[derive(Debug, Clone)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Table styling options
#[derive(Debug, Clone, Copy)]
pub struct TableStyle {
    pub borders: bool,
    pub header_style: TextStyle,
    pub row_style: TextStyle,
}

/// Text styling options
#[derive(Debug, Clone, Copy)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<Color>,
}

/// Color options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Gray,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            color: None,
        }
    }
}

impl Default for TableStyle {
    fn default() -> Self {
        Self {
            borders: true,
            header_style: TextStyle {
                bold: true,
                ..Default::default()
            },
            row_style: TextStyle::default(),
        }
    }
}
