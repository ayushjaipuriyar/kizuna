// System Information Query Module
//
// This module provides comprehensive system information gathering including
// hardware details, system metrics, software inventory, and network information.

use crate::command_execution::error::{CommandError, CommandResult};
use crate::command_execution::types::*;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use sysinfo::{System, Disks, Networks, Components};

/// System information provider with caching support
pub struct SystemInfoProvider {
    cache: Arc<RwLock<SystemInfoCache>>,
}

/// Cached system information with expiration
struct SystemInfoCache {
    hardware: Option<CachedData<HardwareInfo>>,
    metrics: Option<CachedData<SystemMetrics>>,
    software: Option<CachedData<SoftwareInfo>>,
    network: Option<CachedData<NetworkInfo>>,
}

/// Cached data with timestamp
struct CachedData<T> {
    data: T,
    cached_at: Instant,
    expires_in: Duration,
}

impl<T> CachedData<T> {
    fn new(data: T, expires_in: Duration) -> Self {
        Self {
            data,
            cached_at: Instant::now(),
            expires_in,
        }
    }

    fn is_valid(&self) -> bool {
        self.cached_at.elapsed() < self.expires_in
    }
}

impl SystemInfoProvider {
    /// Create a new system information provider
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(SystemInfoCache {
                hardware: None,
                metrics: None,
                software: None,
                network: None,
            })),
        }
    }

    /// Get complete system information
    pub async fn get_system_info(&self, cache_duration: Option<Duration>) -> CommandResult<SystemInfo> {
        let hardware = self.get_hardware_info(cache_duration).await?;
        let system = self.get_system_metrics(cache_duration).await?;
        let software = self.get_software_info(cache_duration).await?;
        let network = self.get_network_info(cache_duration).await?;

        Ok(SystemInfo {
            hardware,
            system,
            software,
            network,
            collected_at: Utc::now(),
        })
    }

    /// Get hardware information with optional caching
    pub async fn get_hardware_info(&self, cache_duration: Option<Duration>) -> CommandResult<HardwareInfo> {
        // Check cache if duration is specified
        if let Some(duration) = cache_duration {
            let cache = self.cache.read().unwrap();
            if let Some(cached) = &cache.hardware {
                if cached.is_valid() {
                    return Ok(cached.data.clone());
                }
            }
        }

        // Collect fresh hardware information
        let hardware = self.collect_hardware_info().await?;

        // Update cache if duration is specified
        if let Some(duration) = cache_duration {
            let mut cache = self.cache.write().unwrap();
            cache.hardware = Some(CachedData::new(hardware.clone(), duration));
        }

        Ok(hardware)
    }

    /// Collect hardware information from the system
    async fn collect_hardware_info(&self) -> CommandResult<HardwareInfo> {
        let mut sys = System::new_all();
        sys.refresh_all();

        // Collect CPU information
        let cpu = self.collect_cpu_info(&sys)?;

        // Collect memory information
        let memory = self.collect_memory_info(&sys)?;

        // Collect storage information
        let storage = self.collect_storage_info()?;

        // Collect battery information (if available)
        let battery = self.collect_battery_info().await.ok();

        Ok(HardwareInfo {
            cpu,
            memory,
            storage,
            battery,
        })
    }

    /// Collect CPU information
    fn collect_cpu_info(&self, sys: &System) -> CommandResult<CpuInfo> {
        let cpus = sys.cpus();
        
        if cpus.is_empty() {
            return Err(CommandError::SystemInfoError(
                "No CPU information available".to_string()
            ));
        }

        // Get CPU model from first CPU (they're typically all the same)
        let model = cpus[0].brand().to_string();
        let cores = cpus.len();
        
        // Get frequency from first CPU
        let frequency_mhz = cpus[0].frequency();

        Ok(CpuInfo {
            model,
            cores,
            frequency_mhz,
        })
    }

    /// Collect memory information
    fn collect_memory_info(&self, sys: &System) -> CommandResult<MemoryInfo> {
        let total_mb = sys.total_memory() / 1024 / 1024;
        let available_mb = sys.available_memory() / 1024 / 1024;

        Ok(MemoryInfo {
            total_mb,
            available_mb,
        })
    }

    /// Collect storage device information
    fn collect_storage_info(&self) -> CommandResult<Vec<StorageDevice>> {
        let disks = Disks::new_with_refreshed_list();
        let mut storage = Vec::new();

        for disk in disks.list() {
            let name = disk.name().to_string_lossy().to_string();
            let mount_point = disk.mount_point().to_path_buf();
            let total_gb = disk.total_space() / 1024 / 1024 / 1024;
            let available_gb = disk.available_space() / 1024 / 1024 / 1024;

            storage.push(StorageDevice {
                name,
                mount_point,
                total_gb,
                available_gb,
            });
        }

        Ok(storage)
    }

    /// Collect battery information (platform-specific)
    async fn collect_battery_info(&self) -> CommandResult<BatteryInfo> {
        // Try to get battery information using platform-specific methods
        #[cfg(target_os = "linux")]
        {
            self.collect_battery_info_linux().await
        }

        #[cfg(target_os = "macos")]
        {
            self.collect_battery_info_macos().await
        }

        #[cfg(target_os = "windows")]
        {
            self.collect_battery_info_windows().await
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Err(CommandError::SystemInfoError(
                "Battery information not supported on this platform".to_string()
            ))
        }
    }

    #[cfg(target_os = "linux")]
    async fn collect_battery_info_linux(&self) -> CommandResult<BatteryInfo> {
        use std::fs;

        // Try to read battery information from /sys/class/power_supply/
        let battery_path = PathBuf::from("/sys/class/power_supply/BAT0");
        
        if !battery_path.exists() {
            // Try BAT1
            let battery_path = PathBuf::from("/sys/class/power_supply/BAT1");
            if !battery_path.exists() {
                return Err(CommandError::SystemInfoError(
                    "No battery found".to_string()
                ));
            }
        }

        let capacity_path = battery_path.join("capacity");
        let status_path = battery_path.join("status");

        let percentage = fs::read_to_string(&capacity_path)
            .map_err(|e| CommandError::SystemInfoError(format!("Failed to read battery capacity: {}", e)))?
            .trim()
            .parse::<f32>()
            .map_err(|e| CommandError::SystemInfoError(format!("Failed to parse battery capacity: {}", e)))?;

        let status = fs::read_to_string(&status_path)
            .map_err(|e| CommandError::SystemInfoError(format!("Failed to read battery status: {}", e)))?
            .trim()
            .to_string();

        let is_charging = status.to_lowercase().contains("charging");

        Ok(BatteryInfo {
            percentage,
            is_charging,
            time_remaining: None, // Would require more complex calculation
        })
    }

    #[cfg(target_os = "macos")]
    async fn collect_battery_info_macos(&self) -> CommandResult<BatteryInfo> {
        use std::process::Command;

        // Use pmset to get battery information
        let output = Command::new("pmset")
            .arg("-g")
            .arg("batt")
            .output()
            .map_err(|e| CommandError::SystemInfoError(format!("Failed to execute pmset: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        // Parse output to extract battery percentage and charging status
        // Example output: "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1234567)	95%; discharging; 5:30 remaining present: true"
        
        let mut percentage = 0.0;
        let mut is_charging = false;

        for line in output_str.lines() {
            if line.contains("InternalBattery") {
                // Extract percentage
                if let Some(pct_start) = line.find(char::is_numeric) {
                    if let Some(pct_end) = line[pct_start..].find('%') {
                        if let Ok(pct) = line[pct_start..pct_start + pct_end].parse::<f32>() {
                            percentage = pct;
                        }
                    }
                }
                
                // Check charging status
                is_charging = line.contains("charging") && !line.contains("discharging");
            }
        }

        Ok(BatteryInfo {
            percentage,
            is_charging,
            time_remaining: None,
        })
    }

    #[cfg(target_os = "windows")]
    async fn collect_battery_info_windows(&self) -> CommandResult<BatteryInfo> {
        use std::process::Command;

        // Use WMIC to get battery information
        let output = Command::new("WMIC")
            .args(&["Path", "Win32_Battery", "Get", "EstimatedChargeRemaining,BatteryStatus"])
            .output()
            .map_err(|e| CommandError::SystemInfoError(format!("Failed to execute WMIC: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        let mut percentage = 0.0;
        let mut is_charging = false;

        // Parse WMIC output
        for line in output_str.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                // BatteryStatus: 2 = charging, 1 = discharging
                if let Ok(status) = parts[0].parse::<u32>() {
                    is_charging = status == 2;
                }
                
                if let Ok(pct) = parts[1].parse::<f32>() {
                    percentage = pct;
                }
            }
        }

        Ok(BatteryInfo {
            percentage,
            is_charging,
            time_remaining: None,
        })
    }

    /// Get system metrics with optional caching
    pub async fn get_system_metrics(&self, cache_duration: Option<Duration>) -> CommandResult<SystemMetrics> {
        // Check cache if duration is specified
        if let Some(duration) = cache_duration {
            let cache = self.cache.read().unwrap();
            if let Some(cached) = &cache.metrics {
                if cached.is_valid() {
                    return Ok(cached.data.clone());
                }
            }
        }

        // Collect fresh metrics
        let metrics = self.collect_system_metrics().await?;

        // Update cache if duration is specified
        if let Some(duration) = cache_duration {
            let mut cache = self.cache.write().unwrap();
            cache.metrics = Some(CachedData::new(metrics.clone(), duration));
        }

        Ok(metrics)
    }

    /// Collect real-time system metrics
    async fn collect_system_metrics(&self) -> CommandResult<SystemMetrics> {
        let mut sys = System::new_all();
        sys.refresh_all();

        // Calculate CPU usage (average across all CPUs)
        let cpus = sys.cpus();
        let cpu_usage = if !cpus.is_empty() {
            cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32
        } else {
            0.0
        };

        // Calculate memory usage
        let total_memory = sys.total_memory() / 1024 / 1024;
        let used_memory = sys.used_memory() / 1024 / 1024;
        let memory_percentage = if total_memory > 0 {
            (used_memory as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        };

        let memory_usage = MemoryUsage {
            used_mb: used_memory,
            total_mb: total_memory,
            percentage: memory_percentage,
        };

        // Collect disk usage
        let disk_usage = self.collect_disk_usage()?;

        // Get system uptime
        let uptime = Duration::from_secs(System::uptime());

        // Get load average (Unix-like systems only)
        let load_average = System::load_average();
        let load_avg = if load_average.one > 0.0 {
            Some([load_average.one as f32, load_average.five as f32, load_average.fifteen as f32])
        } else {
            None
        };

        Ok(SystemMetrics {
            cpu_usage,
            memory_usage,
            disk_usage,
            uptime,
            load_average: load_avg,
        })
    }

    /// Collect disk usage information
    fn collect_disk_usage(&self) -> CommandResult<Vec<DiskUsage>> {
        let disks = Disks::new_with_refreshed_list();
        let mut disk_usage = Vec::new();

        for disk in disks.list() {
            let mount_point = disk.mount_point().to_path_buf();
            let total_gb = disk.total_space() / 1024 / 1024 / 1024;
            let available_gb = disk.available_space() / 1024 / 1024 / 1024;
            let used_gb = total_gb.saturating_sub(available_gb);
            
            let percentage = if total_gb > 0 {
                (used_gb as f32 / total_gb as f32) * 100.0
            } else {
                0.0
            };

            disk_usage.push(DiskUsage {
                mount_point,
                used_gb,
                total_gb,
                percentage,
            });
        }

        Ok(disk_usage)
    }

    /// Get software information with optional caching
    pub async fn get_software_info(&self, cache_duration: Option<Duration>) -> CommandResult<SoftwareInfo> {
        // Check cache if duration is specified
        if let Some(duration) = cache_duration {
            let cache = self.cache.read().unwrap();
            if let Some(cached) = &cache.software {
                if cached.is_valid() {
                    return Ok(cached.data.clone());
                }
            }
        }

        // Collect fresh software information
        let software = self.collect_software_info().await?;

        // Update cache if duration is specified
        if let Some(duration) = cache_duration {
            let mut cache = self.cache.write().unwrap();
            cache.software = Some(CachedData::new(software.clone(), duration));
        }

        Ok(software)
    }

    /// Collect software and OS information
    async fn collect_software_info(&self) -> CommandResult<SoftwareInfo> {
        let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
        let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
        let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
        let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());

        Ok(SoftwareInfo {
            os_name,
            os_version,
            kernel_version,
            hostname,
        })
    }

    /// Get network information with optional caching
    pub async fn get_network_info(&self, cache_duration: Option<Duration>) -> CommandResult<NetworkInfo> {
        // Check cache if duration is specified
        if let Some(duration) = cache_duration {
            let cache = self.cache.read().unwrap();
            if let Some(cached) = &cache.network {
                if cached.is_valid() {
                    return Ok(cached.data.clone());
                }
            }
        }

        // Collect fresh network information
        let network = self.collect_network_info().await?;

        // Update cache if duration is specified
        if let Some(duration) = cache_duration {
            let mut cache = self.cache.write().unwrap();
            cache.network = Some(CachedData::new(network.clone(), duration));
        }

        Ok(network)
    }

    /// Collect network interface information
    async fn collect_network_info(&self) -> CommandResult<NetworkInfo> {
        let networks = Networks::new_with_refreshed_list();
        let mut interfaces = Vec::new();

        for (interface_name, data) in networks.list() {
            // Get MAC address
            let mac_address = {
                let mac = data.mac_address();
                if mac.0.iter().any(|&b| b != 0) {
                    Some(format!(
                        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                        mac.0[0], mac.0[1], mac.0[2], mac.0[3], mac.0[4], mac.0[5]
                    ))
                } else {
                    None
                }
            };

            // For now, we'll use a simplified approach for IP addresses
            // In a production system, you'd want to use platform-specific APIs
            let ip_addresses = self.get_interface_ip_addresses(interface_name).await;

            interfaces.push(NetworkInterface {
                name: interface_name.clone(),
                ip_addresses,
                mac_address,
                is_up: data.received() > 0 || data.transmitted() > 0,
            });
        }

        // Try to get default gateway
        let default_gateway = self.get_default_gateway().await.ok();

        Ok(NetworkInfo {
            interfaces,
            default_gateway,
        })
    }

    /// Get IP addresses for a network interface
    async fn get_interface_ip_addresses(&self, interface_name: &str) -> Vec<String> {
        // This is a simplified implementation
        // In production, you'd use platform-specific APIs or the `if-addrs` crate
        Vec::new()
    }

    /// Get default gateway
    async fn get_default_gateway(&self) -> CommandResult<String> {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            let output = Command::new("ip")
                .args(&["route", "show", "default"])
                .output()
                .map_err(|e| CommandError::SystemInfoError(format!("Failed to get default gateway: {}", e)))?;

            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains("default via") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        return Ok(parts[2].to_string());
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let output = Command::new("route")
                .args(&["-n", "get", "default"])
                .output()
                .map_err(|e| CommandError::SystemInfoError(format!("Failed to get default gateway: {}", e)))?;

            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.trim().starts_with("gateway:") {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 2 {
                        return Ok(parts[1].trim().to_string());
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            let output = Command::new("route")
                .args(&["print", "0.0.0.0"])
                .output()
                .map_err(|e| CommandError::SystemInfoError(format!("Failed to get default gateway: {}", e)))?;

            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.trim().starts_with("0.0.0.0") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        return Ok(parts[2].to_string());
                    }
                }
            }
        }

        Err(CommandError::SystemInfoError(
            "Could not determine default gateway".to_string()
        ))
    }

    /// Clear all cached data
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.hardware = None;
        cache.metrics = None;
        cache.software = None;
        cache.network = None;
    }

    /// Clear specific cache entry
    pub fn clear_cache_entry(&self, query_type: SystemInfoQueryType) {
        let mut cache = self.cache.write().unwrap();
        match query_type {
            SystemInfoQueryType::Hardware => cache.hardware = None,
            SystemInfoQueryType::SystemMetrics => cache.metrics = None,
            SystemInfoQueryType::Software => cache.software = None,
            SystemInfoQueryType::Network => cache.network = None,
            SystemInfoQueryType::All => {
                cache.hardware = None;
                cache.metrics = None;
                cache.software = None;
                cache.network = None;
            }
        }
    }
}

impl Default for SystemInfoProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hardware_info_collection() {
        let provider = SystemInfoProvider::new();
        let result = provider.get_hardware_info(None).await;
        assert!(result.is_ok());
        
        let hardware = result.unwrap();
        assert!(hardware.cpu.cores > 0);
        assert!(hardware.memory.total_mb > 0);
    }

    #[tokio::test]
    async fn test_system_metrics_collection() {
        let provider = SystemInfoProvider::new();
        let result = provider.get_system_metrics(None).await;
        assert!(result.is_ok());
        
        let metrics = result.unwrap();
        assert!(metrics.cpu_usage >= 0.0);
        assert!(metrics.memory_usage.total_mb > 0);
    }

    #[tokio::test]
    async fn test_software_info_collection() {
        let provider = SystemInfoProvider::new();
        let result = provider.get_software_info(None).await;
        assert!(result.is_ok());
        
        let software = result.unwrap();
        assert!(!software.os_name.is_empty());
        assert!(!software.hostname.is_empty());
    }

    #[tokio::test]
    async fn test_caching() {
        let provider = SystemInfoProvider::new();
        let cache_duration = Duration::from_secs(60);
        
        // First call should populate cache
        let result1 = provider.get_hardware_info(Some(cache_duration)).await;
        assert!(result1.is_ok());
        
        // Second call should use cache
        let result2 = provider.get_hardware_info(Some(cache_duration)).await;
        assert!(result2.is_ok());
        
        // Clear cache
        provider.clear_cache();
        
        // Third call should fetch fresh data
        let result3 = provider.get_hardware_info(Some(cache_duration)).await;
        assert!(result3.is_ok());
    }

    #[tokio::test]
    async fn test_complete_system_info() {
        let provider = SystemInfoProvider::new();
        let result = provider.get_system_info(None).await;
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert!(info.hardware.cpu.cores > 0);
        assert!(info.system.memory_usage.total_mb > 0);
        assert!(!info.software.os_name.is_empty());
    }
}
