use async_trait::async_trait;
use crate::command_execution::{
    error::{CommandError, CommandResult as CmdResult},
    types::*,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::timeout;

#[cfg(target_os = "linux")]
use nix::sys::resource::{setrlimit, Resource};

/// Sandbox handle for managing isolated execution environments
#[derive(Debug, Clone)]
pub struct Sandbox {
    pub id: uuid::Uuid,
    pub config: SandboxConfig,
    pub temp_dir: Option<PathBuf>,
    pub created_at: Instant,
}

/// Sandbox Engine trait for secure command execution
#[async_trait]
pub trait SandboxEngine: Send + Sync {
    /// Create a new sandbox with the specified configuration
    async fn create_sandbox(&self, config: SandboxConfig) -> CmdResult<Sandbox>;

    /// Execute a command within a sandbox
    async fn execute_in_sandbox(
        &self,
        sandbox: &Sandbox,
        command: &str,
        args: &[String],
    ) -> CmdResult<CommandResult>;

    /// Destroy a sandbox and clean up resources
    async fn destroy_sandbox(&self, sandbox: Sandbox) -> CmdResult<()>;

    /// Get resource usage statistics for a sandbox
    async fn get_sandbox_stats(&self, sandbox: &Sandbox) -> CmdResult<ResourceUsage>;

    /// Update resource limits for an existing sandbox
    async fn update_sandbox_limits(
        &self,
        sandbox: &Sandbox,
        limits: ResourceLimits,
    ) -> CmdResult<()>;
}

/// Resource limits for sandbox execution
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_cpu_percent: Option<u32>,
    pub max_memory_mb: Option<u64>,
    pub max_execution_time: Option<std::time::Duration>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_percent: Some(50),
            max_memory_mb: Some(512),
            max_execution_time: Some(std::time::Duration::from_secs(60)),
        }
    }
}

/// Default sandbox engine implementation
pub struct DefaultSandboxEngine {
    active_sandboxes: Arc<RwLock<HashMap<uuid::Uuid, SandboxState>>>,
    system: Arc<RwLock<sysinfo::System>>,
    monitoring_interval: Duration,
}

/// Internal state tracking for active sandboxes
#[derive(Debug)]
struct SandboxState {
    sandbox: Sandbox,
    process_ids: Vec<u32>,
    resource_usage: ResourceUsage,
    start_time: Instant,
    peak_memory_mb: u64,
    total_cpu_time: Duration,
    violation_count: u32,
}

/// Resource monitoring snapshot
#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    pub timestamp: Instant,
    pub cpu_usage_percent: f32,
    pub memory_mb: u64,
    pub disk_read_mb: u64,
    pub disk_write_mb: u64,
    pub thread_count: usize,
}

/// Resource violation event
#[derive(Debug, Clone)]
pub enum ResourceViolation {
    CpuLimitExceeded { current: f32, limit: u32 },
    MemoryLimitExceeded { current: u64, limit: u64 },
    ExecutionTimeExceeded { elapsed: Duration, limit: Duration },
    TooManyViolations { count: u32 },
}

impl DefaultSandboxEngine {
    /// Create a new sandbox engine
    pub fn new() -> Self {
        Self {
            active_sandboxes: Arc::new(RwLock::new(HashMap::new())),
            system: Arc::new(RwLock::new(sysinfo::System::new_all())),
            monitoring_interval: Duration::from_millis(100),
        }
    }

    /// Create a new sandbox engine with custom monitoring interval
    pub fn with_monitoring_interval(interval: Duration) -> Self {
        Self {
            active_sandboxes: Arc::new(RwLock::new(HashMap::new())),
            system: Arc::new(RwLock::new(sysinfo::System::new_all())),
            monitoring_interval: interval,
        }
    }

    /// Validate sandbox configuration
    fn validate_config(&self, config: &SandboxConfig) -> CmdResult<()> {
        // Validate resource limits
        if config.max_cpu_percent > 100 {
            return Err(CommandError::invalid_request(
                "CPU limit cannot exceed 100%",
            ));
        }

        if config.max_memory_mb == 0 {
            return Err(CommandError::invalid_request(
                "Memory limit must be greater than 0",
            ));
        }

        // Validate directory paths
        for dir in &config.allowed_directories {
            if !dir.is_absolute() {
                return Err(CommandError::invalid_request(format!(
                    "Allowed directory must be absolute path: {:?}",
                    dir
                )));
            }
        }

        for dir in &config.blocked_directories {
            if !dir.is_absolute() {
                return Err(CommandError::invalid_request(format!(
                    "Blocked directory must be absolute path: {:?}",
                    dir
                )));
            }
        }

        Ok(())
    }

    /// Check if a path is allowed by the sandbox configuration
    fn is_path_allowed(&self, path: &Path, config: &SandboxConfig) -> bool {
        // Check if path is in blocked directories
        for blocked in &config.blocked_directories {
            if path.starts_with(blocked) {
                return false;
            }
        }

        // If allowed directories is empty, allow all (except blocked)
        if config.allowed_directories.is_empty() {
            return true;
        }

        // Check if path is in allowed directories
        for allowed in &config.allowed_directories {
            if path.starts_with(allowed) {
                return true;
            }
        }

        false
    }

    /// Create a temporary directory for the sandbox
    async fn create_temp_directory(&self) -> CmdResult<PathBuf> {
        let temp_dir = std::env::temp_dir().join(format!("kizuna_sandbox_{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir)
            .await
            .map_err(|e| CommandError::sandbox_error(format!("Failed to create temp directory: {}", e)))?;
        Ok(temp_dir)
    }

    /// Apply resource limits to a process (platform-specific)
    #[cfg(target_os = "linux")]
    fn apply_resource_limits(&self, config: &SandboxConfig) -> CmdResult<()> {
        // Set memory limit
        let memory_bytes = (config.max_memory_mb * 1024 * 1024) as u64;
        setrlimit(Resource::RLIMIT_AS, memory_bytes, memory_bytes)
            .map_err(|e| CommandError::sandbox_error(format!("Failed to set memory limit: {}", e)))?;

        // Set CPU time limit
        let cpu_seconds = config.max_execution_time.as_secs();
        setrlimit(Resource::RLIMIT_CPU, cpu_seconds, cpu_seconds)
            .map_err(|e| CommandError::sandbox_error(format!("Failed to set CPU limit: {}", e)))?;

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn apply_resource_limits(&self, _config: &SandboxConfig) -> CmdResult<()> {
        // Resource limits are handled differently on Windows and macOS
        // Windows: Use Job Objects (implemented in execute_in_sandbox)
        // macOS: Use launchd or similar mechanisms
        Ok(())
    }

    /// Monitor process resource usage with enforcement
    async fn monitor_process(&self, pid: u32, limits: &SandboxConfig) -> CmdResult<ResourceUsage> {
        let mut system = self.system.write().await;
        system.refresh_process(sysinfo::Pid::from_u32(pid));

        if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
            let memory_mb = process.memory() / 1024 / 1024;
            let cpu_usage = process.cpu_usage();

            // Check if limits are exceeded
            if memory_mb > limits.max_memory_mb {
                return Err(CommandError::ResourceLimitExceeded(format!(
                    "Memory limit exceeded: {} MB > {} MB",
                    memory_mb, limits.max_memory_mb
                )));
            }

            if cpu_usage > limits.max_cpu_percent as f32 {
                return Err(CommandError::ResourceLimitExceeded(format!(
                    "CPU limit exceeded: {}% > {}%",
                    cpu_usage, limits.max_cpu_percent
                )));
            }

            Ok(ResourceUsage {
                cpu_time: Duration::from_secs(process.run_time()),
                memory_peak_mb: memory_mb,
                disk_read_mb: process.disk_usage().read_bytes / 1024 / 1024,
                disk_write_mb: process.disk_usage().written_bytes / 1024 / 1024,
            })
        } else {
            Ok(ResourceUsage::default())
        }
    }

    /// Get detailed resource snapshot for a process
    async fn get_resource_snapshot(&self, pid: u32) -> CmdResult<ResourceSnapshot> {
        let mut system = self.system.write().await;
        system.refresh_process(sysinfo::Pid::from_u32(pid));

        if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
            Ok(ResourceSnapshot {
                timestamp: Instant::now(),
                cpu_usage_percent: process.cpu_usage(),
                memory_mb: process.memory() / 1024 / 1024,
                disk_read_mb: process.disk_usage().read_bytes / 1024 / 1024,
                disk_write_mb: process.disk_usage().written_bytes / 1024 / 1024,
                thread_count: process.tasks().map(|t| t.len()).unwrap_or(0),
            })
        } else {
            Err(CommandError::execution_error("Process not found"))
        }
    }

    /// Monitor process with violation tracking
    async fn monitor_with_enforcement(
        &self,
        pid: u32,
        sandbox_id: uuid::Uuid,
        limits: &SandboxConfig,
    ) -> CmdResult<()> {
        let snapshot = self.get_resource_snapshot(pid).await?;
        
        let mut violations = Vec::new();

        // Check CPU limit
        if snapshot.cpu_usage_percent > limits.max_cpu_percent as f32 {
            violations.push(ResourceViolation::CpuLimitExceeded {
                current: snapshot.cpu_usage_percent,
                limit: limits.max_cpu_percent,
            });
        }

        // Check memory limit
        if snapshot.memory_mb > limits.max_memory_mb {
            violations.push(ResourceViolation::MemoryLimitExceeded {
                current: snapshot.memory_mb,
                limit: limits.max_memory_mb,
            });
        }

        // Update sandbox state with violations
        if !violations.is_empty() {
            let mut sandboxes = self.active_sandboxes.write().await;
            if let Some(state) = sandboxes.get_mut(&sandbox_id) {
                state.violation_count += 1;
                
                // Update peak memory
                if snapshot.memory_mb > state.peak_memory_mb {
                    state.peak_memory_mb = snapshot.memory_mb;
                }

                // If too many violations, terminate
                if state.violation_count > 3 {
                    return Err(CommandError::ResourceLimitExceeded(format!(
                        "Too many resource violations: {}",
                        state.violation_count
                    )));
                }
            }
        }

        Ok(())
    }

    /// Enforce resource limits and terminate if necessary
    async fn enforce_limits(&self, pid: u32, sandbox_id: uuid::Uuid, limits: &SandboxConfig) -> CmdResult<()> {
        match self.monitor_with_enforcement(pid, sandbox_id, limits).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Terminate process on resource violation
                let _ = self.terminate_process(pid).await;
                Err(e)
            }
        }
    }

    /// Get comprehensive resource statistics for a sandbox
    async fn get_comprehensive_stats(&self, sandbox_id: uuid::Uuid) -> CmdResult<SandboxStatistics> {
        let sandboxes = self.active_sandboxes.read().await;
        if let Some(state) = sandboxes.get(&sandbox_id) {
            let elapsed = state.start_time.elapsed();
            
            Ok(SandboxStatistics {
                sandbox_id,
                uptime: elapsed,
                resource_usage: state.resource_usage.clone(),
                peak_memory_mb: state.peak_memory_mb,
                total_cpu_time: state.total_cpu_time,
                violation_count: state.violation_count,
                active_processes: state.process_ids.len(),
            })
        } else {
            Err(CommandError::sandbox_error("Sandbox not found"))
        }
    }

    /// Clean up terminated processes from sandbox state
    async fn cleanup_terminated_processes(&self, sandbox_id: uuid::Uuid) -> CmdResult<()> {
        let mut sandboxes = self.active_sandboxes.write().await;
        if let Some(state) = sandboxes.get_mut(&sandbox_id) {
            let mut system = self.system.write().await;
            system.refresh_processes();
            
            // Remove PIDs that no longer exist
            state.process_ids.retain(|&pid| {
                system.process(sysinfo::Pid::from_u32(pid)).is_some()
            });
        }
        Ok(())
    }

    /// Terminate a process and its children
    async fn terminate_process(&self, pid: u32) -> CmdResult<()> {
        #[cfg(target_os = "linux")]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            
            kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
                .map_err(|e| CommandError::execution_error(format!("Failed to terminate process: {}", e)))?;
        }

        #[cfg(target_os = "windows")]
        {
            use std::process::Command as StdCommand;
            StdCommand::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F", "/T"])
                .output()
                .map_err(|e| CommandError::execution_error(format!("Failed to terminate process: {}", e)))?;
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command as StdCommand;
            StdCommand::new("kill")
                .args(&["-TERM", &pid.to_string()])
                .output()
                .map_err(|e| CommandError::execution_error(format!("Failed to terminate process: {}", e)))?;
        }

        Ok(())
    }

    /// Apply network isolation based on sandbox configuration
    #[cfg(target_os = "linux")]
    fn apply_network_isolation(&self, config: &SandboxConfig) -> CmdResult<()> {
        match &config.network_access {
            NetworkAccess::None => {
                // Block all network access using network namespaces
                // This would typically be done using unshare(CLONE_NEWNET)
                // For now, we'll document this as a limitation
                Ok(())
            }
            NetworkAccess::LocalOnly => {
                // Allow only localhost connections
                // This would require iptables rules or similar
                Ok(())
            }
            NetworkAccess::Limited(allowed) => {
                // Allow only specific domains/IPs
                // This would require iptables rules or similar
                Ok(())
            }
            NetworkAccess::Full => {
                // No restrictions
                Ok(())
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn apply_network_isolation(&self, _config: &SandboxConfig) -> CmdResult<()> {
        // Network isolation on Windows and macOS would use different mechanisms
        // Windows: Windows Firewall API
        // macOS: pf (packet filter) or similar
        Ok(())
    }

    /// Check if network access is allowed for a given destination
    fn is_network_allowed(&self, destination: &str, config: &SandboxConfig) -> bool {
        match &config.network_access {
            NetworkAccess::None => false,
            NetworkAccess::LocalOnly => {
                destination == "localhost" 
                    || destination == "127.0.0.1" 
                    || destination == "::1"
            }
            NetworkAccess::Limited(allowed) => {
                allowed.iter().any(|allowed_dest| {
                    destination.contains(allowed_dest) || allowed_dest.contains(destination)
                })
            }
            NetworkAccess::Full => true,
        }
    }

    /// Validate file system permissions for a path
    fn check_file_permission(&self, path: &Path, permission: FilePermission, config: &SandboxConfig) -> CmdResult<()> {
        // Check if path is blocked
        for blocked in &config.blocked_directories {
            if path.starts_with(blocked) {
                return Err(CommandError::permission_error(format!(
                    "Access denied to blocked directory: {:?}",
                    path
                )));
            }
        }

        // If allowed directories is specified, check if path is allowed
        if !config.allowed_directories.is_empty() {
            let mut allowed = false;
            for allowed_dir in &config.allowed_directories {
                if path.starts_with(allowed_dir) {
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                return Err(CommandError::permission_error(format!(
                    "Access denied: path not in allowed directories: {:?}",
                    path
                )));
            }
        }

        // Additional checks for write operations
        if matches!(permission, FilePermission::Write | FilePermission::Execute) {
            // Ensure we're not writing to system directories
            let system_dirs = get_system_directories();
            for sys_dir in system_dirs {
                if path.starts_with(sys_dir) {
                    return Err(CommandError::permission_error(format!(
                        "Write access denied to system directory: {:?}",
                        path
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get sandbox policy based on trust level
    fn get_policy_for_trust_level(&self, trust_level: TrustLevel) -> SandboxPolicy {
        match trust_level {
            TrustLevel::Untrusted => SandboxPolicy {
                max_cpu_percent: 25,
                max_memory_mb: 256,
                max_execution_time: Duration::from_secs(30),
                network_access: NetworkAccess::None,
                allowed_directories: vec![],
                blocked_directories: get_system_directories(),
                environment_isolation: true,
            },
            TrustLevel::Low => SandboxPolicy {
                max_cpu_percent: 50,
                max_memory_mb: 512,
                max_execution_time: Duration::from_secs(60),
                network_access: NetworkAccess::LocalOnly,
                allowed_directories: vec![std::env::temp_dir()],
                blocked_directories: get_system_directories(),
                environment_isolation: true,
            },
            TrustLevel::Medium => SandboxPolicy {
                max_cpu_percent: 75,
                max_memory_mb: 1024,
                max_execution_time: Duration::from_secs(300),
                network_access: NetworkAccess::Limited(vec![]),
                allowed_directories: vec![
                    std::env::temp_dir(),
                    dirs::home_dir().unwrap_or_default(),
                ],
                blocked_directories: get_critical_system_directories(),
                environment_isolation: false,
            },
            TrustLevel::High => SandboxPolicy {
                max_cpu_percent: 90,
                max_memory_mb: 2048,
                max_execution_time: Duration::from_secs(600),
                network_access: NetworkAccess::Full,
                allowed_directories: vec![],
                blocked_directories: get_critical_system_directories(),
                environment_isolation: false,
            },
        }
    }
}

/// Comprehensive sandbox statistics
#[derive(Debug, Clone)]
pub struct SandboxStatistics {
    pub sandbox_id: uuid::Uuid,
    pub uptime: Duration,
    pub resource_usage: ResourceUsage,
    pub peak_memory_mb: u64,
    pub total_cpu_time: Duration,
    pub violation_count: u32,
    pub active_processes: usize,
}

/// File permission types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePermission {
    Read,
    Write,
    Execute,
}

/// Trust levels for sandbox policies
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustLevel {
    Untrusted,
    Low,
    Medium,
    High,
}

/// Sandbox policy configuration
#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    pub max_cpu_percent: u32,
    pub max_memory_mb: u64,
    pub max_execution_time: Duration,
    pub network_access: NetworkAccess,
    pub allowed_directories: Vec<PathBuf>,
    pub blocked_directories: Vec<PathBuf>,
    pub environment_isolation: bool,
}

impl SandboxPolicy {
    /// Convert policy to SandboxConfig
    pub fn to_config(&self) -> SandboxConfig {
        SandboxConfig {
            max_cpu_percent: self.max_cpu_percent,
            max_memory_mb: self.max_memory_mb,
            max_execution_time: self.max_execution_time,
            allowed_directories: self.allowed_directories.clone(),
            blocked_directories: self.blocked_directories.clone(),
            network_access: self.network_access.clone(),
            environment_isolation: self.environment_isolation,
            temp_directory: None,
        }
    }
}

/// Get system directories that should be protected
fn get_system_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    
    #[cfg(target_os = "linux")]
    {
        dirs.extend(vec![
            PathBuf::from("/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/etc"),
            PathBuf::from("/sys"),
            PathBuf::from("/proc"),
            PathBuf::from("/boot"),
            PathBuf::from("/root"),
        ]);
    }

    #[cfg(target_os = "windows")]
    {
        dirs.extend(vec![
            PathBuf::from("C:\\Windows"),
            PathBuf::from("C:\\Program Files"),
            PathBuf::from("C:\\Program Files (x86)"),
        ]);
    }

    #[cfg(target_os = "macos")]
    {
        dirs.extend(vec![
            PathBuf::from("/System"),
            PathBuf::from("/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/private/etc"),
        ]);
    }

    dirs
}

/// Get critical system directories that should always be blocked
fn get_critical_system_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    
    #[cfg(target_os = "linux")]
    {
        dirs.extend(vec![
            PathBuf::from("/etc/passwd"),
            PathBuf::from("/etc/shadow"),
            PathBuf::from("/etc/sudoers"),
            PathBuf::from("/root"),
        ]);
    }

    #[cfg(target_os = "windows")]
    {
        dirs.extend(vec![
            PathBuf::from("C:\\Windows\\System32\\config"),
        ]);
    }

    #[cfg(target_os = "macos")]
    {
        dirs.extend(vec![
            PathBuf::from("/private/etc/master.passwd"),
            PathBuf::from("/private/etc/sudoers"),
        ]);
    }

    dirs
}

impl Default for DefaultSandboxEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SandboxEngine for DefaultSandboxEngine {
    async fn create_sandbox(&self, config: SandboxConfig) -> CmdResult<Sandbox> {
        // Validate configuration
        self.validate_config(&config)?;

        // Create temporary directory if needed
        let temp_dir = if config.temp_directory.is_none() {
            Some(self.create_temp_directory().await?)
        } else {
            config.temp_directory.clone()
        };

        let sandbox = Sandbox {
            id: uuid::Uuid::new_v4(),
            config,
            temp_dir,
            created_at: Instant::now(),
        };

        // Register sandbox in active sandboxes
        let state = SandboxState {
            sandbox: sandbox.clone(),
            process_ids: Vec::new(),
            resource_usage: ResourceUsage::default(),
            start_time: Instant::now(),
            peak_memory_mb: 0,
            total_cpu_time: Duration::from_secs(0),
            violation_count: 0,
        };

        self.active_sandboxes.write().await.insert(sandbox.id, state);

        Ok(sandbox)
    }

    async fn execute_in_sandbox(
        &self,
        sandbox: &Sandbox,
        command: &str,
        args: &[String],
    ) -> CmdResult<CommandResult> {
        let start_time = Instant::now();
        let request_id = uuid::Uuid::new_v4();

        // Validate command path if it's a file
        let command_path = Path::new(command);
        if command_path.is_absolute() && !self.is_path_allowed(command_path, &sandbox.config) {
            return Err(CommandError::permission_error(format!(
                "Command path not allowed: {:?}",
                command_path
            )));
        }

        // Build command with environment isolation
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Set working directory to temp directory if available
        if let Some(temp_dir) = &sandbox.temp_dir {
            cmd.current_dir(temp_dir);
        }

        // Environment isolation
        if sandbox.config.environment_isolation {
            cmd.env_clear();
            // Add minimal safe environment variables
            cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
            cmd.env("HOME", std::env::var("HOME").unwrap_or_default());
        }

        // Platform-specific sandboxing setup
        #[cfg(target_os = "windows")]
        {
            // On Windows, we'll use Job Objects to limit resources
            use winapi::um::jobapi2::*;
            use winapi::um::winnt::*;
            use std::ptr;

            unsafe {
                let job = CreateJobObjectW(ptr::null_mut(), ptr::null());
                if !job.is_null() {
                    let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_PROCESS_MEMORY;
                    info.ProcessMemoryLimit = (sandbox.config.max_memory_mb * 1024 * 1024) as usize;
                    
                    SetInformationJobObject(
                        job,
                        JobObjectExtendedLimitInformation,
                        &mut info as *mut _ as *mut _,
                        std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                    );
                }
            }
        }

        // Spawn the process
        let mut child = cmd.spawn()
            .map_err(|e| CommandError::execution_error(format!("Failed to spawn process: {}", e)))?;

        let pid = child.id().ok_or_else(|| CommandError::execution_error("Failed to get process ID"))?;

        // Register process with sandbox
        {
            let mut sandboxes = self.active_sandboxes.write().await;
            if let Some(state) = sandboxes.get_mut(&sandbox.id) {
                state.process_ids.push(pid);
            }
        }

        // Capture stdout and stderr
        let stdout = child.stdout.take().ok_or_else(|| CommandError::execution_error("Failed to capture stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| CommandError::execution_error("Failed to capture stderr"))?;

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let mut stdout_lines = stdout_reader.lines();
        let mut stderr_lines = stderr_reader.lines();

        // Execute with timeout and resource monitoring
        let execution_timeout = sandbox.config.max_execution_time;
        let result = timeout(execution_timeout, async {
            // Spawn tasks to read output
            let stdout_task = tokio::spawn(async move {
                let mut output = String::new();
                while let Ok(Some(line)) = stdout_lines.next_line().await {
                    output.push_str(&line);
                    output.push('\n');
                }
                output
            });

            let stderr_task = tokio::spawn(async move {
                let mut output = String::new();
                while let Ok(Some(line)) = stderr_lines.next_line().await {
                    output.push_str(&line);
                    output.push('\n');
                }
                output
            });

            // Monitor resource usage periodically with enforcement
            let monitor_handle: tokio::task::JoinHandle<CmdResult<()>> = {
                let engine = self.clone();
                let sandbox_config = sandbox.config.clone();
                let sandbox_id = sandbox.id;
                let monitoring_interval = engine.monitoring_interval;
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(monitoring_interval);
                    loop {
                        interval.tick().await;
                        if let Err(e) = engine.enforce_limits(pid, sandbox_id, &sandbox_config).await {
                            // Resource limit exceeded, process already terminated
                            return Err(e);
                        }
                    }
                })
            };

            // Wait for process to complete
            let status = child.wait().await
                .map_err(|e| CommandError::execution_error(format!("Process wait failed: {}", e)))?;

            // Cancel monitoring
            monitor_handle.abort();

            // Collect output
            let stdout = stdout_task.await
                .map_err(|e| CommandError::execution_error(format!("Failed to read stdout: {}", e)))?;
            let stderr = stderr_task.await
                .map_err(|e| CommandError::execution_error(format!("Failed to read stderr: {}", e)))?;

            Ok::<_, CommandError>((status, stdout, stderr))
        }).await;

        // Handle timeout
        let (status, stdout_output, stderr_output) = match result {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // Timeout occurred, terminate process
                let _ = self.terminate_process(pid).await;
                return Err(CommandError::Timeout(execution_timeout));
            }
        };

        // Get final resource usage
        let resource_usage = self.monitor_process(pid, &sandbox.config).await.unwrap_or_default();

        // Update sandbox state
        {
            let mut sandboxes = self.active_sandboxes.write().await;
            if let Some(state) = sandboxes.get_mut(&sandbox.id) {
                state.resource_usage = resource_usage.clone();
                state.process_ids.retain(|&p| p != pid);
            }
        }

        Ok(CommandResult {
            request_id,
            exit_code: status.code().unwrap_or(-1),
            stdout: stdout_output,
            stderr: stderr_output,
            execution_time: start_time.elapsed(),
            resource_usage,
            completed_at: chrono::Utc::now(),
        })
    }

    async fn destroy_sandbox(&self, sandbox: Sandbox) -> CmdResult<()> {
        // Clean up any terminated processes first
        let _ = self.cleanup_terminated_processes(sandbox.id).await;

        // Remove from active sandboxes
        let state = self.active_sandboxes.write().await.remove(&sandbox.id);

        // Terminate any remaining processes
        if let Some(state) = state {
            for pid in state.process_ids {
                let _ = self.terminate_process(pid).await;
            }
        }

        // Clean up temporary directory
        if let Some(temp_dir) = &sandbox.temp_dir {
            // Give processes time to terminate before removing directory
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            tokio::fs::remove_dir_all(temp_dir)
                .await
                .map_err(|e| CommandError::sandbox_error(format!("Failed to remove temp directory: {}", e)))?;
        }

        Ok(())
    }

    async fn get_sandbox_stats(&self, sandbox: &Sandbox) -> CmdResult<ResourceUsage> {
        let sandboxes = self.active_sandboxes.read().await;
        if let Some(state) = sandboxes.get(&sandbox.id) {
            Ok(state.resource_usage.clone())
        } else {
            Err(CommandError::sandbox_error("Sandbox not found"))
        }
    }

    async fn update_sandbox_limits(
        &self,
        sandbox: &Sandbox,
        limits: ResourceLimits,
    ) -> CmdResult<()> {
        let mut sandboxes = self.active_sandboxes.write().await;
        if let Some(state) = sandboxes.get_mut(&sandbox.id) {
            // Update limits in the sandbox config
            if let Some(cpu) = limits.max_cpu_percent {
                state.sandbox.config.max_cpu_percent = cpu;
            }
            if let Some(memory) = limits.max_memory_mb {
                state.sandbox.config.max_memory_mb = memory;
            }
            if let Some(time) = limits.max_execution_time {
                state.sandbox.config.max_execution_time = time;
            }
            Ok(())
        } else {
            Err(CommandError::sandbox_error("Sandbox not found"))
        }
    }
}

// Implement Clone for DefaultSandboxEngine to support monitoring tasks
impl Clone for DefaultSandboxEngine {
    fn clone(&self) -> Self {
        Self {
            active_sandboxes: Arc::clone(&self.active_sandboxes),
            system: Arc::clone(&self.system),
            monitoring_interval: self.monitoring_interval,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sandbox_creation() {
        let engine = DefaultSandboxEngine::new();
        let config = SandboxConfig::default();
        
        let sandbox = engine.create_sandbox(config).await;
        assert!(sandbox.is_ok());
        
        let sandbox = sandbox.unwrap();
        assert!(sandbox.temp_dir.is_some());
        
        // Clean up
        let _ = engine.destroy_sandbox(sandbox).await;
    }

    #[tokio::test]
    async fn test_sandbox_config_validation() {
        let engine = DefaultSandboxEngine::new();
        
        // Test invalid CPU limit
        let mut config = SandboxConfig::default();
        config.max_cpu_percent = 150;
        
        let result = engine.create_sandbox(config).await;
        assert!(result.is_err());
        
        // Test invalid memory limit
        let mut config = SandboxConfig::default();
        config.max_memory_mb = 0;
        
        let result = engine.create_sandbox(config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_simple_command_execution() {
        let engine = DefaultSandboxEngine::new();
        let config = SandboxConfig {
            max_cpu_percent: 50,
            max_memory_mb: 512,
            max_execution_time: Duration::from_secs(10),
            allowed_directories: vec![],
            blocked_directories: vec![],
            network_access: NetworkAccess::None,
            environment_isolation: false,
            temp_directory: None,
        };
        
        let sandbox = engine.create_sandbox(config).await.unwrap();
        
        // Execute a simple echo command
        #[cfg(not(target_os = "windows"))]
        let result = engine.execute_in_sandbox(&sandbox, "echo", &["hello".to_string()]).await;
        
        #[cfg(target_os = "windows")]
        let result = engine.execute_in_sandbox(&sandbox, "cmd", &["/C".to_string(), "echo".to_string(), "hello".to_string()]).await;
        
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("hello"));
        
        // Clean up
        let _ = engine.destroy_sandbox(sandbox).await;
    }

    #[tokio::test]
    async fn test_command_timeout() {
        let engine = DefaultSandboxEngine::new();
        let config = SandboxConfig {
            max_cpu_percent: 50,
            max_memory_mb: 512,
            max_execution_time: Duration::from_millis(100),
            allowed_directories: vec![],
            blocked_directories: vec![],
            network_access: NetworkAccess::None,
            environment_isolation: false,
            temp_directory: None,
        };
        
        let sandbox = engine.create_sandbox(config).await.unwrap();
        
        // Execute a command that takes longer than the timeout
        #[cfg(not(target_os = "windows"))]
        let result = engine.execute_in_sandbox(&sandbox, "sleep", &["5".to_string()]).await;
        
        #[cfg(target_os = "windows")]
        let result = engine.execute_in_sandbox(&sandbox, "timeout", &["/T".to_string(), "5".to_string()]).await;
        
        assert!(result.is_err());
        
        // Clean up
        let _ = engine.destroy_sandbox(sandbox).await;
    }

    #[test]
    fn test_network_access_policy() {
        let engine = DefaultSandboxEngine::new();
        
        // Test None policy
        let config = SandboxConfig {
            network_access: NetworkAccess::None,
            ..Default::default()
        };
        assert!(!engine.is_network_allowed("example.com", &config));
        
        // Test LocalOnly policy
        let config = SandboxConfig {
            network_access: NetworkAccess::LocalOnly,
            ..Default::default()
        };
        assert!(engine.is_network_allowed("localhost", &config));
        assert!(engine.is_network_allowed("127.0.0.1", &config));
        assert!(!engine.is_network_allowed("example.com", &config));
        
        // Test Limited policy
        let config = SandboxConfig {
            network_access: NetworkAccess::Limited(vec!["example.com".to_string()]),
            ..Default::default()
        };
        assert!(engine.is_network_allowed("example.com", &config));
        assert!(!engine.is_network_allowed("other.com", &config));
        
        // Test Full policy
        let config = SandboxConfig {
            network_access: NetworkAccess::Full,
            ..Default::default()
        };
        assert!(engine.is_network_allowed("example.com", &config));
    }

    #[test]
    fn test_trust_level_policies() {
        let engine = DefaultSandboxEngine::new();
        
        let untrusted = engine.get_policy_for_trust_level(TrustLevel::Untrusted);
        assert_eq!(untrusted.max_cpu_percent, 25);
        assert_eq!(untrusted.max_memory_mb, 256);
        assert!(matches!(untrusted.network_access, NetworkAccess::None));
        
        let low = engine.get_policy_for_trust_level(TrustLevel::Low);
        assert_eq!(low.max_cpu_percent, 50);
        assert_eq!(low.max_memory_mb, 512);
        assert!(matches!(low.network_access, NetworkAccess::LocalOnly));
        
        let medium = engine.get_policy_for_trust_level(TrustLevel::Medium);
        assert_eq!(medium.max_cpu_percent, 75);
        assert_eq!(medium.max_memory_mb, 1024);
        
        let high = engine.get_policy_for_trust_level(TrustLevel::High);
        assert_eq!(high.max_cpu_percent, 90);
        assert_eq!(high.max_memory_mb, 2048);
        assert!(matches!(high.network_access, NetworkAccess::Full));
    }
}
