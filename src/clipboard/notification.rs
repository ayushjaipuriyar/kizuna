//! Clipboard notification system and user feedback

use async_trait::async_trait;
use std::sync::{Arc, RwLock};
use crate::clipboard::{ClipboardEvent, ClipboardResult, ClipboardError, PeerId, DeviceId};

/// Notification preferences
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NotificationPreferences {
    pub enabled: bool,
    pub show_content_preview: bool,
    pub show_source_device: bool,
    pub duration_ms: u32,
    pub sound_enabled: bool,
    pub privacy_safe_preview: bool,
    pub max_preview_length: usize,
    pub show_timestamp: bool,
    pub show_content_type: bool,
}

/// Notification content
#[derive(Debug, Clone)]
pub struct NotificationContent {
    pub title: String,
    pub message: String,
    pub source_device: Option<String>,
    pub content_preview: Option<String>,
    pub notification_type: NotificationType,
    pub timestamp: Option<std::time::SystemTime>,
    pub content_type: Option<String>,
    pub priority: NotificationPriority,
}

/// Notification priority levels
#[derive(Debug, Clone, PartialEq)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Types of notifications
#[derive(Debug, Clone, PartialEq)]
pub enum NotificationType {
    ContentReceived,
    ContentSent,
    SyncStarted,
    SyncCompleted,
    SyncFailed,
    PrivacyWarning,
}

/// Sync status indicator state
#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    Idle,
    Syncing { device_count: usize },
    Success { device_count: usize },
    Failed { error: String },
    PartialSuccess { succeeded: usize, failed: usize },
}

/// Sync status indicator trait
#[async_trait]
pub trait SyncStatusIndicator: Send + Sync {
    /// Update the sync status indicator
    async fn update_status(&self, status: SyncStatus) -> ClipboardResult<()>;
    
    /// Show device-specific sync status
    async fn show_device_status(&self, device_id: &DeviceId, status: &str) -> ClipboardResult<()>;
    
    /// Clear the status indicator
    async fn clear_status(&self) -> ClipboardResult<()>;
    
    /// Check if status indicators are supported
    fn is_supported(&self) -> bool;
}

/// Notification manager trait
#[async_trait]
pub trait NotificationManager: Send + Sync {
    /// Show a notification to the user
    async fn show_notification(&self, content: NotificationContent) -> ClipboardResult<()>;
    
    /// Handle clipboard events and generate appropriate notifications
    async fn handle_clipboard_event(&self, event: ClipboardEvent) -> ClipboardResult<()>;
    
    /// Update notification preferences
    async fn update_preferences(&self, preferences: NotificationPreferences) -> ClipboardResult<()>;
    
    /// Get current notification preferences
    async fn get_preferences(&self) -> ClipboardResult<NotificationPreferences>;
    
    /// Get sync status indicator
    fn get_status_indicator(&self) -> Arc<dyn SyncStatusIndicator>;
}

/// Platform-specific notification backend
#[async_trait]
pub trait NotificationBackend: Send + Sync {
    /// Show a platform-specific notification
    async fn show(&self, content: &NotificationContent, preferences: &NotificationPreferences) -> ClipboardResult<()>;
    
    /// Check if notifications are supported on this platform
    fn is_supported(&self) -> bool;
    
    /// Get platform name
    fn platform_name(&self) -> &'static str;
}

/// Default notification manager implementation
pub struct DefaultNotificationManager {
    preferences: Arc<RwLock<NotificationPreferences>>,
    backend: Arc<dyn NotificationBackend>,
    status_indicator: Arc<dyn SyncStatusIndicator>,
}

impl DefaultNotificationManager {
    /// Create new notification manager with platform detection
    pub fn new() -> Self {
        let backend = Self::create_platform_backend();
        let status_indicator = Self::create_platform_status_indicator();
        Self {
            preferences: Arc::new(RwLock::new(NotificationPreferences::default())),
            backend,
            status_indicator,
        }
    }
    
    /// Create new notification manager with custom backend
    pub fn with_backend(backend: Arc<dyn NotificationBackend>) -> Self {
        let status_indicator = Self::create_platform_status_indicator();
        Self {
            preferences: Arc::new(RwLock::new(NotificationPreferences::default())),
            backend,
            status_indicator,
        }
    }
    
    /// Create new notification manager with custom preferences
    pub fn with_preferences(preferences: NotificationPreferences) -> Self {
        let backend = Self::create_platform_backend();
        let status_indicator = Self::create_platform_status_indicator();
        Self {
            preferences: Arc::new(RwLock::new(preferences)),
            backend,
            status_indicator,
        }
    }
    
    /// Create platform-specific notification backend
    fn create_platform_backend() -> Arc<dyn NotificationBackend> {
        #[cfg(target_os = "windows")]
        {
            Arc::new(WindowsNotificationBackend::new())
        }
        
        #[cfg(target_os = "macos")]
        {
            Arc::new(MacOSNotificationBackend::new())
        }
        
        #[cfg(target_os = "linux")]
        {
            Arc::new(LinuxNotificationBackend::new())
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Arc::new(GenericNotificationBackend::new())
        }
    }
    
    /// Create platform-specific status indicator
    fn create_platform_status_indicator() -> Arc<dyn SyncStatusIndicator> {
        Arc::new(ConsoleSyncStatusIndicator::new())
    }
    
    /// Create notification content from clipboard event
    fn create_notification_from_event(&self, event: &ClipboardEvent) -> Option<NotificationContent> {
        let prefs = self.preferences.read().ok()?;
        
        if !prefs.enabled {
            return None;
        }
        
        match event.event_type {
            crate::clipboard::ClipboardEventType::ContentReceived => {
                let source_device = if prefs.show_source_device {
                    match &event.source {
                        crate::clipboard::ContentSource::Remote(peer_id) => {
                            Some(Self::format_device_name(peer_id))
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                
                let (content_preview, content_type) = if prefs.show_content_preview {
                    event.content.as_ref().map(|content| {
                        let preview = Self::create_content_preview(content, &prefs);
                        let content_type = if prefs.show_content_type {
                            Some(Self::get_content_type_name(content))
                        } else {
                            None
                        };
                        (preview, content_type)
                    }).unwrap_or((None, None))
                } else {
                    (None, None)
                };
                
                let timestamp = if prefs.show_timestamp {
                    Some(event.timestamp)
                } else {
                    None
                };
                
                Some(NotificationContent {
                    title: "Clipboard Updated".to_string(),
                    message: Self::create_received_message(&source_device, &content_type),
                    source_device,
                    content_preview,
                    notification_type: NotificationType::ContentReceived,
                    timestamp,
                    content_type,
                    priority: NotificationPriority::Normal,
                })
            }
            crate::clipboard::ClipboardEventType::SyncStarted => {
                Some(NotificationContent {
                    title: "Clipboard Sync".to_string(),
                    message: "Synchronizing clipboard content...".to_string(),
                    source_device: None,
                    content_preview: None,
                    notification_type: NotificationType::SyncStarted,
                    timestamp: Some(event.timestamp),
                    content_type: None,
                    priority: NotificationPriority::Low,
                })
            }
            crate::clipboard::ClipboardEventType::SyncCompleted => {
                Some(NotificationContent {
                    title: "Clipboard Sync".to_string(),
                    message: "Clipboard synchronized successfully".to_string(),
                    source_device: None,
                    content_preview: None,
                    notification_type: NotificationType::SyncCompleted,
                    timestamp: Some(event.timestamp),
                    content_type: None,
                    priority: NotificationPriority::Low,
                })
            }
            crate::clipboard::ClipboardEventType::SyncFailed => {
                Some(NotificationContent {
                    title: "Clipboard Sync Failed".to_string(),
                    message: "Failed to synchronize clipboard content".to_string(),
                    source_device: None,
                    content_preview: None,
                    notification_type: NotificationType::SyncFailed,
                    timestamp: Some(event.timestamp),
                    content_type: None,
                    priority: NotificationPriority::High,
                })
            }
            _ => None,
        }
    }
    
    /// Create content preview with privacy considerations
    fn create_content_preview(content: &crate::clipboard::ClipboardContent, prefs: &NotificationPreferences) -> Option<String> {
        match content {
            crate::clipboard::ClipboardContent::Text(text) => {
                if prefs.privacy_safe_preview {
                    // Check for potentially sensitive patterns
                    if Self::contains_sensitive_patterns(&text.text) {
                        return Some("[Content hidden for privacy]".to_string());
                    }
                }
                
                let max_len = prefs.max_preview_length;
                let preview = if text.text.len() > max_len {
                    format!("{}...", &text.text[..max_len.saturating_sub(3)])
                } else {
                    text.text.clone()
                };
                Some(preview)
            }
            crate::clipboard::ClipboardContent::Image(img) => {
                Some(format!("Image ({}x{}, {})", img.width, img.height, Self::format_size(img.data.len())))
            }
            crate::clipboard::ClipboardContent::Files(files) => {
                if files.len() == 1 {
                    Some(format!("1 file: {}", files[0]))
                } else {
                    Some(format!("{} files", files.len()))
                }
            }
            crate::clipboard::ClipboardContent::Custom { mime_type, data } => {
                Some(format!("Custom content ({}, {})", mime_type, Self::format_size(data.len())))
            }
        }
    }
    
    /// Check if text contains potentially sensitive patterns
    fn contains_sensitive_patterns(text: &str) -> bool {
        // Simple heuristics for sensitive content
        let sensitive_keywords = [
            "password", "passwd", "secret", "token", "api_key", "apikey",
            "private_key", "privatekey", "credit_card", "ssn", "social_security"
        ];
        
        let lower_text = text.to_lowercase();
        sensitive_keywords.iter().any(|keyword| lower_text.contains(keyword))
    }
    
    /// Format device name for display
    fn format_device_name(peer_id: &str) -> String {
        // Truncate long device IDs for display
        if peer_id.len() > 20 {
            format!("{}...", &peer_id[..17])
        } else {
            peer_id.to_string()
        }
    }
    
    /// Get human-readable content type name
    fn get_content_type_name(content: &crate::clipboard::ClipboardContent) -> String {
        match content {
            crate::clipboard::ClipboardContent::Text(_) => "Text".to_string(),
            crate::clipboard::ClipboardContent::Image(_) => "Image".to_string(),
            crate::clipboard::ClipboardContent::Files(_) => "Files".to_string(),
            crate::clipboard::ClipboardContent::Custom { mime_type, .. } => mime_type.clone(),
        }
    }
    
    /// Create message for received content notification
    fn create_received_message(source_device: &Option<String>, content_type: &Option<String>) -> String {
        match (source_device, content_type) {
            (Some(device), Some(ctype)) => {
                format!("Received {} from {}", ctype, device)
            }
            (Some(device), None) => {
                format!("New content received from {}", device)
            }
            (None, Some(ctype)) => {
                format!("Received {}", ctype)
            }
            (None, None) => {
                "New content received".to_string()
            }
        }
    }
    
    /// Format byte size for display
    fn format_size(bytes: usize) -> String {
        const KB: usize = 1024;
        const MB: usize = KB * 1024;
        
        if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }
    
    /// Show platform-specific notification
    async fn show_platform_notification(&self, content: &NotificationContent) -> ClipboardResult<()> {
        let preferences = self.preferences.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on preferences"))?
            .clone();
        
        if !preferences.enabled {
            return Ok(());
        }
        
        self.backend.show(content, &preferences).await
    }
    
    /// Create sanitized content preview for privacy
    fn sanitize_preview(preview: &str, privacy_safe: bool) -> String {
        if !privacy_safe {
            return "[Content hidden for privacy]".to_string();
        }
        
        // Limit preview length
        if preview.len() > 50 {
            format!("{}...", &preview[..47])
        } else {
            preview.to_string()
        }
    }
}

#[async_trait]
impl NotificationManager for DefaultNotificationManager {
    async fn show_notification(&self, content: NotificationContent) -> ClipboardResult<()> {
        self.show_platform_notification(&content).await
    }
    
    async fn handle_clipboard_event(&self, event: ClipboardEvent) -> ClipboardResult<()> {
        if let Some(notification) = self.create_notification_from_event(&event) {
            self.show_notification(notification).await?;
        }
        Ok(())
    }
    
    async fn update_preferences(&self, preferences: NotificationPreferences) -> ClipboardResult<()> {
        let mut prefs = self.preferences.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on preferences"))?;
        
        *prefs = preferences;
        Ok(())
    }
    
    async fn get_preferences(&self) -> ClipboardResult<NotificationPreferences> {
        let prefs = self.preferences.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on preferences"))?;
        
        Ok(prefs.clone())
    }
    
    fn get_status_indicator(&self) -> Arc<dyn SyncStatusIndicator> {
        self.status_indicator.clone()
    }
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            enabled: true,
            show_content_preview: true,
            show_source_device: true,
            duration_ms: 3000,
            sound_enabled: false,
            privacy_safe_preview: true,
            max_preview_length: 50,
            show_timestamp: false,
            show_content_type: true,
        }
    }
}

impl NotificationContent {
    /// Create a new notification content with default values
    pub fn new(title: String, message: String, notification_type: NotificationType) -> Self {
        Self {
            title,
            message,
            source_device: None,
            content_preview: None,
            notification_type,
            timestamp: Some(std::time::SystemTime::now()),
            content_type: None,
            priority: NotificationPriority::Normal,
        }
    }
    
    /// Set source device
    pub fn with_source_device(mut self, device: String) -> Self {
        self.source_device = Some(device);
        self
    }
    
    /// Set content preview
    pub fn with_content_preview(mut self, preview: String) -> Self {
        self.content_preview = Some(preview);
        self
    }
    
    /// Set priority
    pub fn with_priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set content type
    pub fn with_content_type(mut self, content_type: String) -> Self {
        self.content_type = Some(content_type);
        self
    }
}

impl Default for DefaultNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

// Platform-Specific Notification Backends

/// Windows notification backend using Windows Toast notifications
#[cfg(target_os = "windows")]
pub struct WindowsNotificationBackend;

#[cfg(target_os = "windows")]
impl WindowsNotificationBackend {
    pub fn new() -> Self {
        Self
    }
    
    fn build_notification_body(content: &NotificationContent, preferences: &NotificationPreferences) -> String {
        let mut parts = vec![content.message.clone()];
        
        // Add timestamp if enabled
        if preferences.show_timestamp {
            if let Some(timestamp) = content.timestamp {
                if let Ok(duration) = timestamp.duration_since(std::time::UNIX_EPOCH) {
                    let datetime = chrono::DateTime::<chrono::Local>::from(
                        std::time::UNIX_EPOCH + duration
                    );
                    parts.push(format!("Time: {}", datetime.format("%H:%M:%S")));
                }
            }
        }
        
        // Add source device if enabled
        if preferences.show_source_device {
            if let Some(source) = &content.source_device {
                parts.push(format!("From: {}", source));
            }
        }
        
        // Add content type if enabled
        if preferences.show_content_type {
            if let Some(ctype) = &content.content_type {
                parts.push(format!("Type: {}", ctype));
            }
        }
        
        // Add content preview if enabled
        if preferences.show_content_preview {
            if let Some(preview) = &content.content_preview {
                let sanitized = if preferences.privacy_safe_preview {
                    DefaultNotificationManager::sanitize_preview(preview, true)
                } else {
                    preview.clone()
                };
                parts.push(format!("Content: {}", sanitized));
            }
        }
        
        parts.join("\n")
    }
}

#[cfg(target_os = "windows")]
#[async_trait]
impl NotificationBackend for WindowsNotificationBackend {
    async fn show(&self, content: &NotificationContent, preferences: &NotificationPreferences) -> ClipboardResult<()> {
        // Use notify-rust crate for cross-platform notifications
        use notify_rust::{Notification, Timeout};
        
        let mut notification = Notification::new();
        notification.summary(&content.title);
        
        // Build notification body with all enabled components
        let body = Self::build_notification_body(content, preferences);
        notification.body(&body);
        
        // Set timeout based on preferences
        notification.timeout(Timeout::Milliseconds(preferences.duration_ms));
        
        // Set icon based on notification type
        match content.notification_type {
            NotificationType::ContentReceived => {
                notification.icon("clipboard");
            }
            NotificationType::SyncFailed | NotificationType::PrivacyWarning => {
                notification.icon("dialog-warning");
            }
            _ => {
                notification.icon("clipboard");
            }
        }
        
        // Show notification
        notification.show()
            .map_err(|e| ClipboardError::platform("windows", format!("Failed to show notification: {}", e)))?;
        
        Ok(())
    }
    
    fn is_supported(&self) -> bool {
        true
    }
    
    fn platform_name(&self) -> &'static str {
        "windows"
    }
}

/// macOS notification backend using NSUserNotificationCenter
#[cfg(target_os = "macos")]
pub struct MacOSNotificationBackend;

#[cfg(target_os = "macos")]
impl MacOSNotificationBackend {
    pub fn new() -> Self {
        Self
    }
    
    fn build_notification_body(content: &NotificationContent, preferences: &NotificationPreferences) -> String {
        let mut parts = vec![content.message.clone()];
        
        // Add timestamp if enabled
        if preferences.show_timestamp {
            if let Some(timestamp) = content.timestamp {
                if let Ok(duration) = timestamp.duration_since(std::time::UNIX_EPOCH) {
                    let datetime = chrono::DateTime::<chrono::Local>::from(
                        std::time::UNIX_EPOCH + duration
                    );
                    parts.push(format!("Time: {}", datetime.format("%H:%M:%S")));
                }
            }
        }
        
        // Add source device if enabled
        if preferences.show_source_device {
            if let Some(source) = &content.source_device {
                parts.push(format!("From: {}", source));
            }
        }
        
        // Add content type if enabled
        if preferences.show_content_type {
            if let Some(ctype) = &content.content_type {
                parts.push(format!("Type: {}", ctype));
            }
        }
        
        // Add content preview if enabled
        if preferences.show_content_preview {
            if let Some(preview) = &content.content_preview {
                let sanitized = if preferences.privacy_safe_preview {
                    DefaultNotificationManager::sanitize_preview(preview, true)
                } else {
                    preview.clone()
                };
                parts.push(format!("Content: {}", sanitized));
            }
        }
        
        parts.join("\n")
    }
}

#[cfg(target_os = "macos")]
#[async_trait]
impl NotificationBackend for MacOSNotificationBackend {
    async fn show(&self, content: &NotificationContent, preferences: &NotificationPreferences) -> ClipboardResult<()> {
        use notify_rust::{Notification, Timeout};
        
        let mut notification = Notification::new();
        notification.summary(&content.title);
        
        // Build notification body with all enabled components
        let body = Self::build_notification_body(content, preferences);
        notification.body(&body);
        
        // Set timeout based on preferences
        notification.timeout(Timeout::Milliseconds(preferences.duration_ms));
        
        // Enable sound if configured
        if preferences.sound_enabled {
            notification.sound_name("default");
        }
        
        // Show notification
        notification.show()
            .map_err(|e| ClipboardError::platform("macos", format!("Failed to show notification: {}", e)))?;
        
        Ok(())
    }
    
    fn is_supported(&self) -> bool {
        true
    }
    
    fn platform_name(&self) -> &'static str {
        "macos"
    }
}

/// Linux notification backend using libnotify (D-Bus)
#[cfg(target_os = "linux")]
pub struct LinuxNotificationBackend;

#[cfg(target_os = "linux")]
impl LinuxNotificationBackend {
    pub fn new() -> Self {
        Self
    }
    
    fn build_notification_body(content: &NotificationContent, preferences: &NotificationPreferences) -> String {
        let mut parts = vec![content.message.clone()];
        
        // Add timestamp if enabled
        if preferences.show_timestamp {
            if let Some(timestamp) = content.timestamp {
                if let Ok(duration) = timestamp.duration_since(std::time::UNIX_EPOCH) {
                    let datetime = chrono::DateTime::<chrono::Local>::from(
                        std::time::UNIX_EPOCH + duration
                    );
                    parts.push(format!("Time: {}", datetime.format("%H:%M:%S")));
                }
            }
        }
        
        // Add source device if enabled
        if preferences.show_source_device {
            if let Some(source) = &content.source_device {
                parts.push(format!("From: {}", source));
            }
        }
        
        // Add content type if enabled
        if preferences.show_content_type {
            if let Some(ctype) = &content.content_type {
                parts.push(format!("Type: {}", ctype));
            }
        }
        
        // Add content preview if enabled
        if preferences.show_content_preview {
            if let Some(preview) = &content.content_preview {
                let sanitized = if preferences.privacy_safe_preview {
                    DefaultNotificationManager::sanitize_preview(preview, true)
                } else {
                    preview.clone()
                };
                parts.push(format!("Content: {}", sanitized));
            }
        }
        
        parts.join("\n")
    }
}

#[cfg(target_os = "linux")]
#[async_trait]
impl NotificationBackend for LinuxNotificationBackend {
    async fn show(&self, content: &NotificationContent, preferences: &NotificationPreferences) -> ClipboardResult<()> {
        use notify_rust::{Notification, Timeout, Urgency};
        
        let mut notification = Notification::new();
        notification.summary(&content.title);
        
        // Build notification body with all enabled components
        let body = Self::build_notification_body(content, preferences);
        notification.body(&body);
        
        // Set timeout based on preferences
        notification.timeout(Timeout::Milliseconds(preferences.duration_ms));
        
        // Set urgency based on notification priority
        match content.priority {
            NotificationPriority::Critical => {
                notification.urgency(Urgency::Critical);
            }
            NotificationPriority::High => {
                notification.urgency(Urgency::Critical);
            }
            NotificationPriority::Normal => {
                notification.urgency(Urgency::Normal);
            }
            NotificationPriority::Low => {
                notification.urgency(Urgency::Low);
            }
        }
        
        // Set icon based on notification type
        match content.notification_type {
            NotificationType::ContentReceived => {
                notification.icon("edit-paste");
            }
            NotificationType::SyncFailed | NotificationType::PrivacyWarning => {
                notification.icon("dialog-warning");
            }
            _ => {
                notification.icon("edit-paste");
            }
        }
        
        // Show notification
        notification.show()
            .map_err(|e| ClipboardError::platform("linux", format!("Failed to show notification: {}", e)))?;
        
        Ok(())
    }
    
    fn is_supported(&self) -> bool {
        // Check if D-Bus is available
        true
    }
    
    fn platform_name(&self) -> &'static str {
        "linux"
    }
}

/// Generic notification backend for unsupported platforms (console output)
pub struct GenericNotificationBackend;

impl GenericNotificationBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NotificationBackend for GenericNotificationBackend {
    async fn show(&self, content: &NotificationContent, preferences: &NotificationPreferences) -> ClipboardResult<()> {
        // Fallback to console output for unsupported platforms
        println!("╔═══════════════════════════════════════════════════════════╗");
        println!("║ {} - {}", content.title, content.message);
        
        if preferences.show_source_device {
            if let Some(source) = &content.source_device {
                println!("║ Source: {}", source);
            }
        }
        
        if preferences.show_content_preview {
            if let Some(preview) = &content.content_preview {
                let sanitized = if preferences.privacy_safe_preview {
                    DefaultNotificationManager::sanitize_preview(preview, true)
                } else {
                    preview.clone()
                };
                println!("║ Content: {}", sanitized);
            }
        }
        
        println!("╚═══════════════════════════════════════════════════════════╝");
        
        Ok(())
    }
    
    fn is_supported(&self) -> bool {
        true
    }
    
    fn platform_name(&self) -> &'static str {
        "generic"
    }
}

// Sync Status Indicators

/// Console-based sync status indicator (fallback for all platforms)
pub struct ConsoleSyncStatusIndicator {
    current_status: Arc<RwLock<SyncStatus>>,
}

impl ConsoleSyncStatusIndicator {
    pub fn new() -> Self {
        Self {
            current_status: Arc::new(RwLock::new(SyncStatus::Idle)),
        }
    }
    
    fn format_status(&self, status: &SyncStatus) -> String {
        match status {
            SyncStatus::Idle => "⚪ Clipboard Sync: Idle".to_string(),
            SyncStatus::Syncing { device_count } => {
                format!("🔄 Clipboard Sync: Syncing to {} device(s)...", device_count)
            }
            SyncStatus::Success { device_count } => {
                format!("✅ Clipboard Sync: Successfully synced to {} device(s)", device_count)
            }
            SyncStatus::Failed { error } => {
                format!("❌ Clipboard Sync: Failed - {}", error)
            }
            SyncStatus::PartialSuccess { succeeded, failed } => {
                format!("⚠️  Clipboard Sync: {} succeeded, {} failed", succeeded, failed)
            }
        }
    }
}

#[async_trait]
impl SyncStatusIndicator for ConsoleSyncStatusIndicator {
    async fn update_status(&self, status: SyncStatus) -> ClipboardResult<()> {
        let mut current = self.current_status.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on status"))?;
        
        *current = status.clone();
        
        // Print status to console
        println!("{}", self.format_status(&status));
        
        Ok(())
    }
    
    async fn show_device_status(&self, device_id: &DeviceId, status: &str) -> ClipboardResult<()> {
        println!("📱 Device {}: {}", device_id, status);
        Ok(())
    }
    
    async fn clear_status(&self) -> ClipboardResult<()> {
        let mut current = self.current_status.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on status"))?;
        
        *current = SyncStatus::Idle;
        println!("{}", self.format_status(&SyncStatus::Idle));
        
        Ok(())
    }
    
    fn is_supported(&self) -> bool {
        true
    }
}

impl Default for ConsoleSyncStatusIndicator {
    fn default() -> Self {
        Self::new()
    }
}

/// System tray sync status indicator (platform-specific implementations would go here)
/// This is a placeholder for future system tray integration
pub struct SystemTraySyncStatusIndicator {
    current_status: Arc<RwLock<SyncStatus>>,
}

impl SystemTraySyncStatusIndicator {
    pub fn new() -> Self {
        Self {
            current_status: Arc::new(RwLock::new(SyncStatus::Idle)),
        }
    }
}

#[async_trait]
impl SyncStatusIndicator for SystemTraySyncStatusIndicator {
    async fn update_status(&self, status: SyncStatus) -> ClipboardResult<()> {
        let mut current = self.current_status.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on status"))?;
        
        *current = status.clone();
        
        // TODO: Update system tray icon/tooltip
        // This would require platform-specific system tray integration
        
        Ok(())
    }
    
    async fn show_device_status(&self, device_id: &DeviceId, status: &str) -> ClipboardResult<()> {
        // TODO: Update system tray menu with device-specific status
        Ok(())
    }
    
    async fn clear_status(&self) -> ClipboardResult<()> {
        let mut current = self.current_status.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on status"))?;
        
        *current = SyncStatus::Idle;
        
        // TODO: Clear system tray status
        
        Ok(())
    }
    
    fn is_supported(&self) -> bool {
        // System tray support would need to be checked per platform
        false
    }
}

impl Default for SystemTraySyncStatusIndicator {
    fn default() -> Self {
        Self::new()
    }
}
