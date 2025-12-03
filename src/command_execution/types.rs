use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

/// Unique identifier for a command request
pub type RequestId = Uuid;

/// Unique identifier for a command execution
pub type ExecutionId = Uuid;

/// Unique identifier for a notification
pub type NotificationId = Uuid;

/// Unique identifier for a command history entry
pub type EntryId = Uuid;

/// Unique identifier for a command in the trusted list
pub type CommandId = Uuid;

/// Peer identifier (from security/identity module)
pub type PeerId = String;

/// Timestamp type
pub type Timestamp = chrono::DateTime<chrono::Utc>;

/// Command execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    pub request_id: RequestId,
    pub command: String,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub environment: HashMap<String, String>,
    pub timeout: Duration,
    pub sandbox_config: SandboxConfig,
    pub requester: PeerId,
    pub created_at: Timestamp,
}

/// Result of command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub request_id: RequestId,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time: Duration,
    pub resource_usage: ResourceUsage,
    pub completed_at: Timestamp,
}

/// Script execution request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRequest {
    pub request_id: RequestId,
    pub content: String,
    pub language: ScriptLanguage,
    pub parameters: HashMap<String, String>,
    pub working_directory: Option<PathBuf>,
    pub timeout: Duration,
    pub sandbox_config: SandboxConfig,
    pub requester: PeerId,
}

/// Supported script languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptLanguage {
    Bash,
    PowerShell,
    Python,
    JavaScript,
    Batch,
    Auto, // Auto-detect based on content
}

/// Result of script execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptResult {
    pub request_id: RequestId,
    pub exit_code: i32,
    pub output: String,
    pub errors: Vec<ScriptError>,
    pub execution_time: Duration,
    pub lines_executed: usize,
}

/// Script execution error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptError {
    pub line: Option<usize>,
    pub message: String,
    pub error_type: String,
}

/// System information query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfoQuery {
    pub query_id: Uuid,
    pub query_type: SystemInfoQueryType,
    pub cache_duration: Option<Duration>,
}

/// Types of system information queries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemInfoQueryType {
    Hardware,
    SystemMetrics,
    Software,
    Network,
    All,
}

/// Complete system information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hardware: HardwareInfo,
    pub system: SystemMetrics,
    pub software: SoftwareInfo,
    pub network: NetworkInfo,
    pub collected_at: Timestamp,
}

/// Hardware information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub storage: Vec<StorageDevice>,
    pub battery: Option<BatteryInfo>,
}

/// CPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model: String,
    pub cores: usize,
    pub frequency_mhz: u64,
}

/// Memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_mb: u64,
    pub available_mb: u64,
}

/// Storage device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDevice {
    pub name: String,
    pub mount_point: PathBuf,
    pub total_gb: u64,
    pub available_gb: u64,
}

/// Battery information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub percentage: f32,
    pub is_charging: bool,
    pub time_remaining: Option<Duration>,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub memory_usage: MemoryUsage,
    pub disk_usage: Vec<DiskUsage>,
    pub uptime: Duration,
    pub load_average: Option<[f32; 3]>,
}

/// Memory usage details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub used_mb: u64,
    pub total_mb: u64,
    pub percentage: f32,
}

/// Disk usage details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub mount_point: PathBuf,
    pub used_gb: u64,
    pub total_gb: u64,
    pub percentage: f32,
}

/// Software information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareInfo {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub hostname: String,
}

/// Network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub interfaces: Vec<NetworkInterface>,
    pub default_gateway: Option<String>,
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub ip_addresses: Vec<String>,
    pub mac_address: Option<String>,
    pub is_up: bool,
}

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub max_cpu_percent: u32,
    pub max_memory_mb: u64,
    pub max_execution_time: Duration,
    pub allowed_directories: Vec<PathBuf>,
    pub blocked_directories: Vec<PathBuf>,
    pub network_access: NetworkAccess,
    pub environment_isolation: bool,
    pub temp_directory: Option<PathBuf>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_cpu_percent: 50,
            max_memory_mb: 512,
            max_execution_time: Duration::from_secs(60),
            allowed_directories: vec![],
            blocked_directories: vec![],
            network_access: NetworkAccess::None,
            environment_isolation: true,
            temp_directory: None,
        }
    }
}

/// Network access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkAccess {
    None,
    LocalOnly,
    Limited(Vec<String>), // Allowed domains/IPs
    Full,
}

/// Resource usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time: Duration,
    pub memory_peak_mb: u64,
    pub disk_read_mb: u64,
    pub disk_write_mb: u64,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            cpu_time: Duration::from_secs(0),
            memory_peak_mb: 0,
            disk_read_mb: 0,
            disk_write_mb: 0,
        }
    }
}

/// Authorization request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRequest {
    pub request_id: RequestId,
    pub command_type: CommandType,
    pub command_preview: String,
    pub requester: PeerId,
    pub risk_level: RiskLevel,
    pub requested_permissions: Vec<Permission>,
    pub timeout: Duration,
}

/// Type of command being executed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandType {
    SimpleCommand,
    Script,
    SystemQuery,
    Notification,
}

/// Risk level assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,    // Read-only operations, system info
    Medium, // File operations in safe directories
    High,   // System modifications, network access
    Critical, // Administrative operations
}

/// Permission types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Permission {
    FileRead(PathBuf),
    FileWrite(PathBuf),
    NetworkAccess,
    SystemModification,
    ProcessCreation,
}

/// Authorization decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorizationDecision {
    Approved,
    Denied(String), // Reason for denial
    Modified(CommandRequest), // User-modified command
    Timeout,
}

/// Authorization record for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationRecord {
    pub request_id: RequestId,
    pub decision: AuthorizationDecision,
    pub decided_at: Timestamp,
    pub decided_by: String, // User or system
}

/// Notification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub notification_id: NotificationId,
    pub title: String,
    pub message: String,
    pub notification_type: NotificationType,
    pub priority: NotificationPriority,
    pub duration: Option<Duration>,
    pub actions: Vec<NotificationAction>,
    pub sender: PeerId,
}

/// Notification types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    Info,
    Warning,
    Error,
    Success,
}

/// Notification priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Notification action button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
}

/// Notification delivery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResult {
    pub notification_id: NotificationId,
    pub delivered: bool,
    pub delivery_time: Option<Timestamp>,
    pub error: Option<String>,
}

/// Notification delivery status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed(String),
    Cancelled,
}

/// Command execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Authorized,
    Executing,
    Completed,
    Failed(String),
    Cancelled,
    Timeout,
}

/// Command history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandHistoryEntry {
    pub entry_id: EntryId,
    pub command_request: CommandRequest,
    pub result: Option<CommandResult>,
    pub authorization: AuthorizationRecord,
    pub execution_status: ExecutionStatus,
    pub created_at: Timestamp,
    pub completed_at: Option<Timestamp>,
}

/// Command pattern for trusted commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPattern {
    pub pattern: String,
    pub description: String,
    pub allowed_peers: Vec<PeerId>,
}
