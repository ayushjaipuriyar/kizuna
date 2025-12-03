// Linux platform adapter

pub mod packaging;
pub mod systemd;
pub mod dbus;

use async_trait::async_trait;
use crate::platform::{
    PlatformResult, PlatformAdapter, SystemServices, UIFramework,
    NetworkConfig, SecurityConfig, GUIFramework, PlatformError,
};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

/// Linux desktop environment types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopEnvironment {
    GNOME,
    KDE,
    XFCE,
    LXDE,
    Cinnamon,
    MATE,
    Unity,
    Unknown,
}

/// Linux display server types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    X11,
    Wayland,
    Unknown,
}

/// Linux system integration configuration
#[derive(Debug, Clone)]
pub struct LinuxSystemConfig {
    pub desktop_environment: DesktopEnvironment,
    pub display_server: DisplayServer,
    pub use_io_uring: bool,
    pub use_epoll: bool,
    pub enable_desktop_integration: bool,
}

impl Default for LinuxSystemConfig {
    fn default() -> Self {
        Self {
            desktop_environment: DesktopEnvironment::Unknown,
            display_server: DisplayServer::Unknown,
            use_io_uring: false,
            use_epoll: true,
            enable_desktop_integration: true,
        }
    }
}

/// Linux platform adapter
pub struct LinuxAdapter {
    config: LinuxSystemConfig,
}

impl LinuxAdapter {
    pub fn new() -> Self {
        Self {
            config: LinuxSystemConfig::default(),
        }
    }

    /// Detect the current desktop environment
    pub fn detect_desktop_environment(&self) -> DesktopEnvironment {
        // Check XDG_CURRENT_DESKTOP first
        if let Ok(desktop) = env::var("XDG_CURRENT_DESKTOP") {
            let desktop_lower = desktop.to_lowercase();
            if desktop_lower.contains("gnome") {
                return DesktopEnvironment::GNOME;
            } else if desktop_lower.contains("kde") || desktop_lower.contains("plasma") {
                return DesktopEnvironment::KDE;
            } else if desktop_lower.contains("xfce") {
                return DesktopEnvironment::XFCE;
            } else if desktop_lower.contains("lxde") {
                return DesktopEnvironment::LXDE;
            } else if desktop_lower.contains("cinnamon") {
                return DesktopEnvironment::Cinnamon;
            } else if desktop_lower.contains("mate") {
                return DesktopEnvironment::MATE;
            } else if desktop_lower.contains("unity") {
                return DesktopEnvironment::Unity;
            }
        }

        // Fallback to DESKTOP_SESSION
        if let Ok(session) = env::var("DESKTOP_SESSION") {
            let session_lower = session.to_lowercase();
            if session_lower.contains("gnome") {
                return DesktopEnvironment::GNOME;
            } else if session_lower.contains("kde") || session_lower.contains("plasma") {
                return DesktopEnvironment::KDE;
            } else if session_lower.contains("xfce") {
                return DesktopEnvironment::XFCE;
            }
        }

        DesktopEnvironment::Unknown
    }

    /// Detect the current display server
    pub fn detect_display_server(&self) -> DisplayServer {
        // Check WAYLAND_DISPLAY first
        if env::var("WAYLAND_DISPLAY").is_ok() {
            return DisplayServer::Wayland;
        }

        // Check XDG_SESSION_TYPE
        if let Ok(session_type) = env::var("XDG_SESSION_TYPE") {
            let session_lower = session_type.to_lowercase();
            if session_lower == "wayland" {
                return DisplayServer::Wayland;
            } else if session_lower == "x11" {
                return DisplayServer::X11;
            }
        }

        // Check DISPLAY for X11
        if env::var("DISPLAY").is_ok() {
            return DisplayServer::X11;
        }

        DisplayServer::Unknown
    }

    /// Check if io_uring is available on this system
    pub fn check_io_uring_support(&self) -> bool {
        // Check kernel version for io_uring support (requires 5.1+)
        if let Ok(uname) = std::process::Command::new("uname")
            .arg("-r")
            .output()
        {
            if let Ok(version_str) = String::from_utf8(uname.stdout) {
                if let Some(major_minor) = version_str.split('.').take(2).collect::<Vec<_>>().get(0..2) {
                    if let (Ok(major), Ok(minor)) = (
                        major_minor[0].parse::<u32>(),
                        major_minor[1].parse::<u32>()
                    ) {
                        // io_uring available in kernel 5.1+
                        return major > 5 || (major == 5 && minor >= 1);
                    }
                }
            }
        }
        false
    }

    /// Setup desktop integration files
    fn setup_desktop_integration(&self) -> PlatformResult<()> {
        // Create .desktop file for application launcher integration
        let _desktop_entry = self.create_desktop_entry()?;
        
        // Get user's local applications directory
        let home = env::var("HOME")
            .map_err(|_| PlatformError::IntegrationError("HOME not set".to_string()))?;
        
        let apps_dir = Path::new(&home).join(".local/share/applications");
        
        // Create directory if it doesn't exist
        if !apps_dir.exists() {
            fs::create_dir_all(&apps_dir)?;
        }

        // Write desktop entry (would be written in actual implementation)
        // For now, just validate the path exists
        Ok(())
    }

    /// Create desktop entry content
    fn create_desktop_entry(&self) -> PlatformResult<String> {
        let entry = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Kizuna\n\
             Comment=Cross-platform connectivity tool\n\
             Exec=kizuna\n\
             Icon=kizuna\n\
             Terminal=false\n\
             Categories=Network;FileTransfer;\n\
             Keywords=file;transfer;clipboard;streaming;\n"
        );
        Ok(entry)
    }

    /// Setup file system optimizations
    fn setup_filesystem_optimizations(&self) -> PlatformResult<()> {
        // Configure file system specific optimizations
        // This would include setting up proper buffer sizes, 
        // enabling direct I/O where appropriate, etc.
        Ok(())
    }

    /// Setup networking optimizations
    fn setup_networking_optimizations(&self) -> PlatformResult<()> {
        // Configure Linux-specific networking optimizations
        // This would include TCP tuning, socket options, etc.
        Ok(())
    }

    /// Get desktop environment specific settings
    pub fn get_desktop_settings(&self) -> HashMap<String, String> {
        let mut settings = HashMap::new();
        
        match self.config.desktop_environment {
            DesktopEnvironment::GNOME => {
                settings.insert("theme_integration".to_string(), "gtk3".to_string());
                settings.insert("notification_daemon".to_string(), "gnome-shell".to_string());
            }
            DesktopEnvironment::KDE => {
                settings.insert("theme_integration".to_string(), "qt5".to_string());
                settings.insert("notification_daemon".to_string(), "plasma".to_string());
            }
            DesktopEnvironment::XFCE => {
                settings.insert("theme_integration".to_string(), "gtk3".to_string());
                settings.insert("notification_daemon".to_string(), "xfce4-notifyd".to_string());
            }
            _ => {
                settings.insert("theme_integration".to_string(), "generic".to_string());
                settings.insert("notification_daemon".to_string(), "libnotify".to_string());
            }
        }

        settings
    }
}

#[async_trait]
impl PlatformAdapter for LinuxAdapter {
    async fn initialize_platform(&self) -> PlatformResult<()> {
        // Detect desktop environment and display server
        let _de = self.detect_desktop_environment();
        let _ds = self.detect_display_server();

        // Check for io_uring support
        let _io_uring_available = self.check_io_uring_support();

        // Setup desktop integration if enabled
        if self.config.enable_desktop_integration {
            self.setup_desktop_integration()?;
        }

        // Setup file system optimizations
        self.setup_filesystem_optimizations()?;

        // Setup networking optimizations
        self.setup_networking_optimizations()?;

        Ok(())
    }

    async fn integrate_system_services(&self) -> PlatformResult<SystemServices> {
        let desktop_settings = self.get_desktop_settings();
        
        let mut metadata = HashMap::new();
        metadata.insert("desktop_environment".to_string(), 
                       format!("{:?}", self.config.desktop_environment));
        metadata.insert("display_server".to_string(), 
                       format!("{:?}", self.config.display_server));
        
        // Merge desktop-specific settings
        for (key, value) in desktop_settings {
            metadata.insert(key, value);
        }

        Ok(SystemServices {
            notifications: true,
            system_tray: self.config.display_server != DisplayServer::Unknown,
            file_manager: true,
            network_manager: true,
            metadata,
        })
    }

    async fn setup_ui_framework(&self) -> PlatformResult<UIFramework> {
        let mut capabilities = Vec::new();

        // Add display server capabilities
        match self.config.display_server {
            DisplayServer::X11 => capabilities.push("x11".to_string()),
            DisplayServer::Wayland => capabilities.push("wayland".to_string()),
            DisplayServer::Unknown => {
                // Try to support both
                capabilities.push("x11".to_string());
                capabilities.push("wayland".to_string());
            }
        }

        // Add desktop environment specific capabilities
        match self.config.desktop_environment {
            DesktopEnvironment::GNOME => {
                capabilities.push("gtk3".to_string());
                capabilities.push("gnome-integration".to_string());
            }
            DesktopEnvironment::KDE => {
                capabilities.push("qt5".to_string());
                capabilities.push("kde-integration".to_string());
            }
            DesktopEnvironment::XFCE => {
                capabilities.push("gtk3".to_string());
                capabilities.push("xfce-integration".to_string());
            }
            _ => {
                capabilities.push("generic-linux".to_string());
            }
        }

        Ok(UIFramework {
            framework_type: GUIFramework::Native,
            version: "linux".to_string(),
            capabilities,
        })
    }

    async fn configure_networking(&self) -> PlatformResult<NetworkConfig> {
        let mut config = NetworkConfig::default();
        
        // Prefer QUIC and TCP on Linux
        config.preferred_protocols = vec!["quic".to_string(), "tcp".to_string()];
        
        // Enable fallback for better compatibility
        config.fallback_enabled = true;
        
        // Increase max connections for server workloads
        config.max_connections = 1000;

        Ok(config)
    }

    async fn setup_security_integration(&self) -> PlatformResult<SecurityConfig> {
        let mut config = SecurityConfig::default();
        
        // Linux doesn't have a system keychain by default, but can use libsecret
        config.use_keychain = Path::new("/usr/lib/libsecret-1.so").exists() ||
                             Path::new("/usr/lib64/libsecret-1.so").exists();
        
        // Check for hardware crypto support
        config.use_hardware_crypto = Path::new("/dev/crypto").exists();
        
        // Sandboxing available through various mechanisms
        config.sandbox_enabled = true;

        Ok(config)
    }

    fn platform_name(&self) -> &str {
        "linux"
    }

    fn get_optimizations(&self) -> Vec<String> {
        let mut optimizations = Vec::new();

        if self.config.use_io_uring {
            optimizations.push("io_uring".to_string());
        }

        if self.config.use_epoll {
            optimizations.push("epoll".to_string());
        }

        optimizations.push("sendfile".to_string());
        optimizations.push("splice".to_string());
        optimizations.push("zero-copy".to_string());

        optimizations
    }
}
