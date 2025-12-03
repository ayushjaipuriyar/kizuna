// Platform detection implementation

use crate::platform::{
    PlatformResult, PlatformError, PlatformInfo, OperatingSystem, 
    Architecture, PlatformCapabilities,
};

/// Detect the current platform at runtime with comprehensive information
pub fn detect_platform() -> PlatformResult<PlatformInfo> {
    let os = detect_os();
    let architecture = detect_architecture();
    let version = detect_version();
    let variant = detect_variant();
    let capabilities = detect_capabilities(&os, &architecture)?;

    Ok(PlatformInfo {
        os,
        architecture,
        version,
        variant,
        capabilities,
    })
}

/// Detect platform with detailed runtime information
pub fn detect_platform_detailed() -> PlatformResult<DetailedPlatformInfo> {
    let basic_info = detect_platform()?;
    let runtime_info = detect_runtime_info();
    let hardware_info = detect_hardware_info();
    
    Ok(DetailedPlatformInfo {
        basic: basic_info,
        runtime: runtime_info,
        hardware: hardware_info,
    })
}

/// Detailed platform information including runtime and hardware details
#[derive(Debug, Clone)]
pub struct DetailedPlatformInfo {
    pub basic: PlatformInfo,
    pub runtime: RuntimeInfo,
    pub hardware: HardwareInfo,
}

/// Runtime environment information
#[derive(Debug, Clone)]
pub struct RuntimeInfo {
    pub is_containerized: bool,
    pub container_type: Option<String>,
    pub is_virtualized: bool,
    pub virtualization_type: Option<String>,
    pub has_gui: bool,
    pub display_server: Option<String>,
}

/// Hardware information
#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub cpu_count: usize,
    pub total_memory_mb: u64,
    pub has_gpu: bool,
    pub has_hardware_crypto: bool,
    pub has_simd: bool,
}

/// Detect runtime environment information
fn detect_runtime_info() -> RuntimeInfo {
    RuntimeInfo {
        is_containerized: is_running_in_container(),
        container_type: detect_container_type(),
        is_virtualized: is_running_in_vm(),
        virtualization_type: detect_virtualization_type(),
        has_gui: has_gui_environment(),
        display_server: detect_display_server(),
    }
}

/// Detect hardware information
fn detect_hardware_info() -> HardwareInfo {
    use sysinfo::System;
    
    let mut sys = System::new_all();
    sys.refresh_all();
    
    HardwareInfo {
        cpu_count: sys.cpus().len(),
        total_memory_mb: sys.total_memory() / 1024 / 1024,
        has_gpu: detect_gpu_availability(),
        has_hardware_crypto: detect_hardware_crypto(),
        has_simd: detect_simd_support(),
    }
}

/// Detect operating system
fn detect_os() -> OperatingSystem {
    #[cfg(target_os = "linux")]
    {
        // Check if running in a container
        if is_running_in_container() {
            return OperatingSystem::Container;
        }
        OperatingSystem::Linux
    }

    #[cfg(target_os = "macos")]
    {
        OperatingSystem::MacOS
    }

    #[cfg(target_os = "windows")]
    {
        OperatingSystem::Windows
    }

    #[cfg(target_os = "android")]
    {
        OperatingSystem::Android
    }

    #[cfg(target_os = "ios")]
    {
        OperatingSystem::iOS
    }

    #[cfg(target_arch = "wasm32")]
    {
        OperatingSystem::WebBrowser
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows",
        target_os = "android",
        target_os = "ios",
        target_arch = "wasm32"
    )))]
    {
        OperatingSystem::Unknown
    }
}

/// Detect CPU architecture
fn detect_architecture() -> Architecture {
    #[cfg(target_arch = "x86_64")]
    {
        Architecture::X86_64
    }

    #[cfg(target_arch = "aarch64")]
    {
        Architecture::ARM64
    }

    #[cfg(target_arch = "arm")]
    {
        Architecture::ARM32
    }

    #[cfg(target_arch = "wasm32")]
    {
        Architecture::WASM32
    }

    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "wasm32"
    )))]
    {
        Architecture::Unknown
    }
}

/// Detect OS version
fn detect_version() -> String {
    #[cfg(target_os = "linux")]
    {
        detect_linux_version()
    }

    #[cfg(target_os = "macos")]
    {
        detect_macos_version()
    }

    #[cfg(target_os = "windows")]
    {
        detect_windows_version()
    }

    #[cfg(target_os = "android")]
    {
        detect_android_version()
    }

    #[cfg(target_os = "ios")]
    {
        detect_ios_version()
    }

    #[cfg(target_arch = "wasm32")]
    {
        "wasm32".to_string()
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "windows",
        target_os = "android",
        target_os = "ios",
        target_arch = "wasm32"
    )))]
    {
        "unknown".to_string()
    }
}

/// Detect platform variant (distribution, etc.)
fn detect_variant() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        detect_linux_distribution()
    }

    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Detect platform capabilities with runtime discovery
pub fn detect_capabilities(os: &OperatingSystem, arch: &Architecture) -> PlatformResult<PlatformCapabilities> {
    let mut capabilities = PlatformCapabilities::default();

    match os {
        OperatingSystem::Linux => {
            capabilities.system_tray = true;
            capabilities.notifications = true;
            capabilities.file_associations = true;
            capabilities.auto_start = true;
            capabilities.gui_framework = Some(crate::platform::GUIFramework::Native);
        }
        OperatingSystem::MacOS => {
            capabilities.system_tray = true;
            capabilities.notifications = true;
            capabilities.file_associations = true;
            capabilities.auto_start = true;
            capabilities.gui_framework = Some(crate::platform::GUIFramework::Native);
            capabilities.security_features.keychain = true;
            capabilities.security_features.secure_enclave = true;
            capabilities.security_features.code_signing = true;
        }
        OperatingSystem::Windows => {
            capabilities.system_tray = true;
            capabilities.notifications = true;
            capabilities.file_associations = true;
            capabilities.auto_start = true;
            capabilities.gui_framework = Some(crate::platform::GUIFramework::Native);
        }
        OperatingSystem::Android => {
            capabilities.notifications = true;
            capabilities.gui_framework = Some(crate::platform::GUIFramework::Native);
            capabilities.network_features.bluetooth = true;
        }
        OperatingSystem::iOS => {
            capabilities.notifications = true;
            capabilities.gui_framework = Some(crate::platform::GUIFramework::Native);
            capabilities.security_features.keychain = true;
            capabilities.security_features.secure_enclave = true;
            capabilities.security_features.sandboxing = true;
        }
        OperatingSystem::WebBrowser => {
            capabilities.gui_framework = Some(crate::platform::GUIFramework::Web);
            capabilities.notifications = true;
            capabilities.network_features.websocket = true;
            capabilities.network_features.webrtc = true;
            capabilities.network_features.tcp = false;
            capabilities.network_features.udp = false;
            capabilities.network_features.mdns = false;
        }
        OperatingSystem::Container => {
            capabilities.gui_framework = Some(crate::platform::GUIFramework::None);
            capabilities.system_tray = false;
            capabilities.notifications = false;
        }
        OperatingSystem::Unknown => {}
    }

    // Architecture-specific capabilities
    match arch {
        Architecture::X86_64 | Architecture::ARM64 => {
            capabilities.hardware_acceleration.insert(crate::platform::HardwareFeature::SIMD);
        }
        _ => {}
    }

    // Runtime capability discovery
    discover_runtime_capabilities(&mut capabilities, os);

    Ok(capabilities)
}

/// Discover capabilities at runtime
fn discover_runtime_capabilities(capabilities: &mut PlatformCapabilities, os: &OperatingSystem) {
    // Check for GPU availability
    if detect_gpu_availability() {
        capabilities.hardware_acceleration.insert(crate::platform::HardwareFeature::GPU);
    }
    
    // Check for hardware crypto
    if detect_hardware_crypto() {
        capabilities.hardware_acceleration.insert(crate::platform::HardwareFeature::Crypto);
        capabilities.security_features.hardware_crypto = true;
    }
    
    // Check for video codec support
    if detect_video_codec_support() {
        capabilities.hardware_acceleration.insert(crate::platform::HardwareFeature::VideoCodec);
    }
    
    // Platform-specific runtime checks
    match os {
        OperatingSystem::Linux => {
            // Check for Wayland vs X11
            if std::env::var("WAYLAND_DISPLAY").is_ok() {
                capabilities.security_features.sandboxing = true;
            }
        }
        OperatingSystem::Container => {
            // Containers have limited capabilities
            capabilities.hardware_acceleration.clear();
            capabilities.security_features.sandboxing = true;
        }
        _ => {}
    }
}

// Platform-specific detection helpers

#[cfg(target_os = "linux")]
fn detect_linux_version() -> String {
    use std::fs;
    
    // Try to read /etc/os-release
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("VERSION_ID=") {
                return line.trim_start_matches("VERSION_ID=")
                    .trim_matches('"')
                    .to_string();
            }
        }
    }
    
    "unknown".to_string()
}

#[cfg(target_os = "linux")]
fn detect_linux_distribution() -> Option<String> {
    use std::fs;
    
    // Try to read /etc/os-release
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if line.starts_with("ID=") {
                return Some(line.trim_start_matches("ID=")
                    .trim_matches('"')
                    .to_string());
            }
        }
    }
    
    None
}

#[cfg(target_os = "linux")]
fn is_running_in_container() -> bool {
    use std::fs;
    
    // Check for Docker
    if fs::metadata("/.dockerenv").is_ok() {
        return true;
    }
    
    // Check for container in cgroup
    if let Ok(content) = fs::read_to_string("/proc/1/cgroup") {
        if content.contains("docker") || content.contains("lxc") || content.contains("kubepods") {
            return true;
        }
    }
    
    // Check for container environment variables
    if std::env::var("container").is_ok() {
        return true;
    }
    
    false
}

#[cfg(not(target_os = "linux"))]
fn is_running_in_container() -> bool {
    false
}

/// Detect container type
fn detect_container_type() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        
        if fs::metadata("/.dockerenv").is_ok() {
            return Some("docker".to_string());
        }
        
        if let Ok(content) = fs::read_to_string("/proc/1/cgroup") {
            if content.contains("kubepods") {
                return Some("kubernetes".to_string());
            }
            if content.contains("lxc") {
                return Some("lxc".to_string());
            }
            if content.contains("docker") {
                return Some("docker".to_string());
            }
        }
        
        if let Ok(container_type) = std::env::var("container") {
            return Some(container_type);
        }
    }
    
    None
}

/// Check if running in a virtual machine
fn is_running_in_vm() -> bool {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        
        // Check for common VM indicators
        if let Ok(content) = fs::read_to_string("/sys/class/dmi/id/product_name") {
            let content_lower = content.to_lowercase();
            if content_lower.contains("virtualbox") 
                || content_lower.contains("vmware")
                || content_lower.contains("qemu")
                || content_lower.contains("kvm") {
                return true;
            }
        }
        
        if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
            if content.contains("hypervisor") {
                return true;
            }
        }
    }
    
    false
}

/// Detect virtualization type
fn detect_virtualization_type() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        
        if let Ok(content) = fs::read_to_string("/sys/class/dmi/id/product_name") {
            let content_lower = content.to_lowercase();
            if content_lower.contains("virtualbox") {
                return Some("virtualbox".to_string());
            }
            if content_lower.contains("vmware") {
                return Some("vmware".to_string());
            }
            if content_lower.contains("qemu") || content_lower.contains("kvm") {
                return Some("kvm".to_string());
            }
        }
    }
    
    None
}

/// Check if GUI environment is available
fn has_gui_environment() -> bool {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        // Check for display environment variables
        if std::env::var("DISPLAY").is_ok() 
            || std::env::var("WAYLAND_DISPLAY").is_ok() {
            return true;
        }
        
        #[cfg(target_os = "windows")]
        {
            // Windows always has GUI in normal mode
            return true;
        }
        
        #[cfg(target_os = "macos")]
        {
            // macOS always has GUI
            return true;
        }
    }
    
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        // Mobile platforms always have GUI
        return true;
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        // Browser always has GUI
        return true;
    }
    
    false
}

/// Detect display server type
fn detect_display_server() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            return Some("wayland".to_string());
        }
        if std::env::var("DISPLAY").is_ok() {
            return Some("x11".to_string());
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        return Some("quartz".to_string());
    }
    
    #[cfg(target_os = "windows")]
    {
        return Some("dwm".to_string());
    }
    
    None
}

/// Get total system memory in MB
fn get_total_memory_mb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        
        if let Ok(content) = fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb / 1024; // Convert KB to MB
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: estimate based on available info
    4096 // Default to 4GB
}

/// Detect GPU availability
fn detect_gpu_availability() -> bool {
    #[cfg(target_os = "linux")]
    {
        use std::path::Path;
        
        // Check for GPU device files
        if Path::new("/dev/dri").exists() {
            return true;
        }
        
        // Check for NVIDIA GPU
        if Path::new("/dev/nvidia0").exists() {
            return true;
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS always has GPU
        return true;
    }
    
    #[cfg(target_os = "windows")]
    {
        // Assume GPU is available on Windows
        return true;
    }
    
    false
}

/// Detect hardware crypto support
fn detect_hardware_crypto() -> bool {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        // Modern x86_64 and ARM64 CPUs have hardware crypto
        return true;
    }
    
    false
}

/// Detect SIMD support
fn detect_simd_support() -> bool {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        return true;
    }
    
    false
}

/// Detect video codec hardware support
fn detect_video_codec_support() -> bool {
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        // Most modern systems have hardware video codec support
        detect_gpu_availability()
    }
    
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

#[cfg(target_os = "macos")]
fn detect_macos_version() -> String {
    use std::process::Command;
    
    if let Ok(output) = Command::new("sw_vers").arg("-productVersion").output() {
        if let Ok(version) = String::from_utf8(output.stdout) {
            return version.trim().to_string();
        }
    }
    
    "unknown".to_string()
}

#[cfg(target_os = "windows")]
fn detect_windows_version() -> String {
    use std::process::Command;
    
    if let Ok(output) = Command::new("cmd")
        .args(&["/C", "ver"])
        .output() 
    {
        if let Ok(version) = String::from_utf8(output.stdout) {
            return version.trim().to_string();
        }
    }
    
    "unknown".to_string()
}

#[cfg(target_os = "android")]
fn detect_android_version() -> String {
    // Android version detection would require JNI calls
    // For now, return a placeholder
    "android".to_string()
}

#[cfg(target_os = "ios")]
fn detect_ios_version() -> String {
    // iOS version detection would require Objective-C calls
    // For now, return a placeholder
    "ios".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platform() {
        let result = detect_platform();
        assert!(result.is_ok());
        
        let info = result.unwrap();
        assert_ne!(info.os, OperatingSystem::Unknown);
        assert_ne!(info.architecture, Architecture::Unknown);
    }

    #[test]
    fn test_detect_os() {
        let os = detect_os();
        assert_ne!(os, OperatingSystem::Unknown);
    }

    #[test]
    fn test_detect_architecture() {
        let arch = detect_architecture();
        assert_ne!(arch, Architecture::Unknown);
    }
}
