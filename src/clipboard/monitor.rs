//! Clipboard monitoring and change detection

use async_trait::async_trait;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use uuid::Uuid;
use crate::clipboard::{
    ClipboardEvent, ClipboardResult, ClipboardContent, ClipboardEventType,
    ContentSource, Clipboard, ClipboardError,
};
use crate::clipboard::platform::UnifiedClipboard;

/// Clipboard monitor trait for detecting changes
#[async_trait]
pub trait ClipboardMonitor: Send + Sync {
    /// Start monitoring clipboard changes
    async fn start_monitoring(&self) -> ClipboardResult<()>;
    
    /// Stop monitoring clipboard changes
    async fn stop_monitoring(&self) -> ClipboardResult<()>;
    
    /// Check if monitoring is active
    fn is_monitoring(&self) -> bool;
    
    /// Subscribe to clipboard change events
    fn subscribe_to_changes(&self) -> broadcast::Receiver<ClipboardEvent>;
    
    /// Get current clipboard content
    async fn get_current_content(&self) -> ClipboardResult<Option<ClipboardContent>>;
    
    /// Set clipboard content
    async fn set_content(&self, content: ClipboardContent) -> ClipboardResult<()>;
}

/// Configuration for error handling and retry logic
#[derive(Debug, Clone)]
pub struct ErrorHandlingConfig {
    /// Maximum number of retry attempts for transient errors
    pub max_retries: usize,
    /// Initial backoff duration for retries
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Whether to continue monitoring after permission errors
    pub continue_on_permission_error: bool,
}

impl Default for ErrorHandlingConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            continue_on_permission_error: false,
        }
    }
}

/// Configuration for change filtering and throttling
#[derive(Debug, Clone)]
pub struct ChangeFilterConfig {
    /// Minimum time between change events (throttling)
    pub min_change_interval: Duration,
    /// Time window to ignore changes after programmatic set
    pub programmatic_ignore_window: Duration,
    /// Maximum number of changes per time window
    pub max_changes_per_window: usize,
    /// Time window for rate limiting
    pub rate_limit_window: Duration,
}

impl Default for ChangeFilterConfig {
    fn default() -> Self {
        Self {
            min_change_interval: Duration::from_millis(100),
            programmatic_ignore_window: Duration::from_secs(1),
            max_changes_per_window: 10,
            rate_limit_window: Duration::from_secs(5),
        }
    }
}

/// Tracks change events for rate limiting
#[derive(Debug)]
struct ChangeTracker {
    timestamps: Vec<std::time::Instant>,
    last_event_time: Option<std::time::Instant>,
}

impl ChangeTracker {
    fn new() -> Self {
        Self {
            timestamps: Vec::new(),
            last_event_time: None,
        }
    }
    
    /// Check if a change should be throttled
    fn should_throttle(&mut self, now: std::time::Instant, config: &ChangeFilterConfig) -> bool {
        // Check minimum interval throttling
        if let Some(last_time) = self.last_event_time {
            if now.duration_since(last_time) < config.min_change_interval {
                return true;
            }
        }
        
        // Clean up old timestamps outside the rate limit window
        self.timestamps.retain(|&t| now.duration_since(t) < config.rate_limit_window);
        
        // Check rate limiting
        if self.timestamps.len() >= config.max_changes_per_window {
            return true;
        }
        
        false
    }
    
    /// Record a change event
    fn record_change(&mut self, now: std::time::Instant) {
        self.timestamps.push(now);
        self.last_event_time = Some(now);
    }
}

/// Error tracking for retry logic
#[derive(Debug)]
struct ErrorTracker {
    consecutive_errors: usize,
    last_error_time: Option<std::time::Instant>,
    current_backoff: Duration,
}

impl ErrorTracker {
    fn new(initial_backoff: Duration) -> Self {
        Self {
            consecutive_errors: 0,
            last_error_time: None,
            current_backoff: initial_backoff,
        }
    }
    
    /// Record an error and calculate next backoff
    fn record_error(&mut self, now: std::time::Instant, config: &ErrorHandlingConfig) {
        self.consecutive_errors += 1;
        self.last_error_time = Some(now);
        
        // Calculate exponential backoff
        let new_backoff = Duration::from_secs_f64(
            self.current_backoff.as_secs_f64() * config.backoff_multiplier
        );
        self.current_backoff = new_backoff.min(config.max_backoff);
    }
    
    /// Reset error tracking after successful operation
    fn reset(&mut self, initial_backoff: Duration) {
        self.consecutive_errors = 0;
        self.last_error_time = None;
        self.current_backoff = initial_backoff;
    }
    
    /// Check if we should retry based on error count
    fn should_retry(&self, config: &ErrorHandlingConfig) -> bool {
        self.consecutive_errors < config.max_retries
    }
    
    /// Get current backoff duration
    fn get_backoff(&self) -> Duration {
        self.current_backoff
    }
}

/// Unified clipboard monitor implementation with platform detection
pub struct UnifiedClipboardMonitor {
    clipboard: Arc<UnifiedClipboard>,
    event_sender: broadcast::Sender<ClipboardEvent>,
    monitoring: Arc<AtomicBool>,
    last_content: Arc<RwLock<Option<ClipboardContent>>>,
    monitor_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    programmatic_change: Arc<AtomicBool>,
    /// Tracks the last time we set content programmatically for loop prevention
    last_programmatic_set: Arc<RwLock<Option<std::time::Instant>>>,
    /// Content hash for duplicate detection
    last_content_hash: Arc<RwLock<Option<u64>>>,
    /// Change filter configuration
    filter_config: Arc<RwLock<ChangeFilterConfig>>,
    /// Change tracker for throttling and rate limiting
    change_tracker: Arc<RwLock<ChangeTracker>>,
    /// Error handling configuration
    error_config: Arc<RwLock<ErrorHandlingConfig>>,
    /// Error tracker for retry logic
    error_tracker: Arc<RwLock<ErrorTracker>>,
}

impl UnifiedClipboardMonitor {
    /// Create new unified clipboard monitor
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(100);
        let error_config = ErrorHandlingConfig::default();
        let initial_backoff = error_config.initial_backoff;
        
        Self {
            clipboard: Arc::new(UnifiedClipboard::new()),
            event_sender,
            monitoring: Arc::new(AtomicBool::new(false)),
            last_content: Arc::new(RwLock::new(None)),
            monitor_handle: Arc::new(RwLock::new(None)),
            programmatic_change: Arc::new(AtomicBool::new(false)),
            last_programmatic_set: Arc::new(RwLock::new(None)),
            last_content_hash: Arc::new(RwLock::new(None)),
            filter_config: Arc::new(RwLock::new(ChangeFilterConfig::default())),
            change_tracker: Arc::new(RwLock::new(ChangeTracker::new())),
            error_config: Arc::new(RwLock::new(error_config)),
            error_tracker: Arc::new(RwLock::new(ErrorTracker::new(initial_backoff))),
        }
    }
    
    /// Create new monitor with custom filter configuration
    pub fn with_config(config: ChangeFilterConfig) -> Self {
        let (event_sender, _) = broadcast::channel(100);
        let error_config = ErrorHandlingConfig::default();
        let initial_backoff = error_config.initial_backoff;
        
        Self {
            clipboard: Arc::new(UnifiedClipboard::new()),
            event_sender,
            monitoring: Arc::new(AtomicBool::new(false)),
            last_content: Arc::new(RwLock::new(None)),
            monitor_handle: Arc::new(RwLock::new(None)),
            programmatic_change: Arc::new(AtomicBool::new(false)),
            last_programmatic_set: Arc::new(RwLock::new(None)),
            last_content_hash: Arc::new(RwLock::new(None)),
            filter_config: Arc::new(RwLock::new(config)),
            change_tracker: Arc::new(RwLock::new(ChangeTracker::new())),
            error_config: Arc::new(RwLock::new(error_config)),
            error_tracker: Arc::new(RwLock::new(ErrorTracker::new(initial_backoff))),
        }
    }
    
    /// Update filter configuration
    pub async fn set_filter_config(&self, config: ChangeFilterConfig) {
        let mut filter_config = self.filter_config.write().await;
        *filter_config = config;
    }
    
    /// Get current filter configuration
    pub async fn get_filter_config(&self) -> ChangeFilterConfig {
        self.filter_config.read().await.clone()
    }
    
    /// Update error handling configuration
    pub async fn set_error_config(&self, config: ErrorHandlingConfig) {
        let mut error_config = self.error_config.write().await;
        let initial_backoff = config.initial_backoff;
        *error_config = config;
        
        // Reset error tracker with new initial backoff
        let mut tracker = self.error_tracker.write().await;
        tracker.reset(initial_backoff);
    }
    
    /// Get current error handling configuration
    pub async fn get_error_config(&self) -> ErrorHandlingConfig {
        self.error_config.read().await.clone()
    }
    
    /// Get platform name
    pub fn platform_name(&self) -> &'static str {
        self.clipboard.platform_name()
    }
    
    /// Start the monitoring loop
    async fn start_monitor_loop(&self) -> ClipboardResult<()> {
        let clipboard = self.clipboard.clone();
        let event_sender = self.event_sender.clone();
        let monitoring = self.monitoring.clone();
        let last_content = self.last_content.clone();
        let programmatic_change = self.programmatic_change.clone();
        let last_programmatic_set = self.last_programmatic_set.clone();
        let last_content_hash = self.last_content_hash.clone();
        let filter_config = self.filter_config.clone();
        let change_tracker = self.change_tracker.clone();
        let error_config = self.error_config.clone();
        let error_tracker = self.error_tracker.clone();
        
        // Initialize last content and hash
        if let Ok(Some(content)) = clipboard.get_content().await {
            let hash = Self::hash_content(&content);
            let mut last = last_content.write().await;
            let mut last_hash = last_content_hash.write().await;
            *last = Some(content);
            *last_hash = Some(hash);
        }
        
        let handle = tokio::spawn(async move {
            // Poll every 500ms for clipboard changes (meets 500ms detection latency requirement)
            let mut ticker = interval(Duration::from_millis(500));
            
            while monitoring.load(Ordering::Relaxed) {
                ticker.tick().await;
                
                let now = std::time::Instant::now();
                
                // Skip if this was a programmatic change (loop prevention)
                if programmatic_change.swap(false, Ordering::Relaxed) {
                    continue;
                }
                
                // Check if we should ignore changes due to recent programmatic set
                let config = filter_config.read().await;
                let should_ignore = {
                    let last_set = last_programmatic_set.read().await;
                    if let Some(last_set_time) = *last_set {
                        now.duration_since(last_set_time) < config.programmatic_ignore_window
                    } else {
                        false
                    }
                };
                
                if should_ignore {
                    continue;
                }
                
                // Check for clipboard changes
                match clipboard.get_content().await {
                    Ok(Some(current_content)) => {
                        // Reset error tracker on successful clipboard access
                        {
                            let error_cfg = error_config.read().await;
                            let mut err_tracker = error_tracker.write().await;
                            if err_tracker.consecutive_errors > 0 {
                                err_tracker.reset(error_cfg.initial_backoff);
                            }
                        }
                        
                        // Validate content before processing
                        if !Self::validate_content(&current_content) {
                            continue;
                        }
                        
                        // Calculate hash for duplicate detection
                        let current_hash = Self::hash_content(&current_content);
                        
                        let mut last = last_content.write().await;
                        let mut last_hash = last_content_hash.write().await;
                        
                        // Check if content has changed using hash comparison first (faster)
                        let has_changed = match *last_hash {
                            Some(prev_hash) => prev_hash != current_hash,
                            None => true,
                        };
                        
                        if has_changed {
                            // Double-check with full content comparison to avoid hash collisions
                            let confirmed_change = match &*last {
                                Some(prev) => !Self::content_equals(prev, &current_content),
                                None => true,
                            };
                            
                            if confirmed_change {
                                // Check if this is a user-initiated change
                                let last_set_time = *last_programmatic_set.read().await;
                                let is_user_change = Self::is_user_initiated_change(
                                    last_set_time,
                                    now,
                                    &config,
                                );
                                
                                if !is_user_change {
                                    // Skip programmatic changes
                                    continue;
                                }
                                
                                // Check throttling and rate limiting
                                let mut tracker = change_tracker.write().await;
                                if tracker.should_throttle(now, &config) {
                                    // Skip this change due to throttling
                                    continue;
                                }
                                
                                // Update last content and hash
                                *last = Some(current_content.clone());
                                *last_hash = Some(current_hash);
                                
                                // Record the change event
                                tracker.record_change(now);
                                
                                // Generate and send event with content extraction
                                let event = ClipboardEvent {
                                    event_id: Uuid::new_v4(),
                                    event_type: ClipboardEventType::ContentChanged,
                                    content: Some(current_content),
                                    source: ContentSource::Local,
                                    timestamp: std::time::SystemTime::now(),
                                };
                                
                                // Ignore send errors (no receivers)
                                let _ = event_sender.send(event);
                            }
                        }
                    }
                    Ok(None) => {
                        // Clipboard is empty
                        let mut last = last_content.write().await;
                        let mut last_hash = last_content_hash.write().await;
                        if last.is_some() {
                            *last = None;
                            *last_hash = None;
                        }
                    }
                    Err(err) => {
                        // Handle clipboard access errors with retry logic
                        let error_cfg = error_config.read().await;
                        
                        let mut err_tracker = error_tracker.write().await;
                        
                        // Check if this is a permission error
                        let is_permission_error = matches!(err, ClipboardError::PermissionError { .. });
                        
                        if is_permission_error && !error_cfg.continue_on_permission_error {
                            // Stop monitoring on permission errors if configured
                            monitoring.store(false, Ordering::Relaxed);
                            break;
                        }
                        
                        // Record the error
                        err_tracker.record_error(now, &error_cfg);
                        
                        // Check if we should retry
                        if err_tracker.should_retry(&error_cfg) {
                            // Apply exponential backoff
                            let backoff = err_tracker.get_backoff();
                            tokio::time::sleep(backoff).await;
                        } else {
                            // Max retries exceeded, reset and continue with normal polling
                            err_tracker.reset(error_cfg.initial_backoff);
                        }
                    }
                }
            }
        });
        
        let mut monitor_handle = self.monitor_handle.write().await;
        *monitor_handle = Some(handle);
        
        Ok(())
    }
    
    /// Compare two clipboard contents for equality
    fn content_equals(a: &ClipboardContent, b: &ClipboardContent) -> bool {
        match (a, b) {
            (ClipboardContent::Text(a_text), ClipboardContent::Text(b_text)) => {
                a_text.text == b_text.text
            }
            (ClipboardContent::Image(a_img), ClipboardContent::Image(b_img)) => {
                a_img.data == b_img.data
            }
            (ClipboardContent::Files(a_files), ClipboardContent::Files(b_files)) => {
                a_files == b_files
            }
            (
                ClipboardContent::Custom { mime_type: a_mime, data: a_data },
                ClipboardContent::Custom { mime_type: b_mime, data: b_data }
            ) => {
                a_mime == b_mime && a_data == b_data
            }
            _ => false,
        }
    }
    
    /// Calculate a hash for clipboard content for duplicate detection
    fn hash_content(content: &ClipboardContent) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        match content {
            ClipboardContent::Text(text) => {
                "text".hash(&mut hasher);
                text.text.hash(&mut hasher);
            }
            ClipboardContent::Image(img) => {
                "image".hash(&mut hasher);
                img.data.hash(&mut hasher);
            }
            ClipboardContent::Files(files) => {
                "files".hash(&mut hasher);
                files.hash(&mut hasher);
            }
            ClipboardContent::Custom { mime_type, data } => {
                "custom".hash(&mut hasher);
                mime_type.hash(&mut hasher);
                data.hash(&mut hasher);
            }
        }
        
        hasher.finish()
    }
    
    /// Validate clipboard content for quality and integrity
    fn validate_content(content: &ClipboardContent) -> bool {
        match content {
            ClipboardContent::Text(text) => {
                // Validate text content is not empty and within reasonable size
                !text.text.is_empty() && text.size > 0
            }
            ClipboardContent::Image(img) => {
                // Validate image has data and reasonable dimensions
                !img.data.is_empty() && img.width > 0 && img.height > 0
            }
            ClipboardContent::Files(files) => {
                // Validate file list is not empty
                !files.is_empty()
            }
            ClipboardContent::Custom { data, .. } => {
                // Validate custom content has data
                !data.is_empty()
            }
        }
    }
    
    /// Determine if a change is user-initiated vs programmatic
    /// This is a heuristic based on timing and patterns
    fn is_user_initiated_change(
        last_set_time: Option<std::time::Instant>,
        now: std::time::Instant,
        config: &ChangeFilterConfig,
    ) -> bool {
        // If we recently set content programmatically, this is likely not user-initiated
        if let Some(last_set) = last_set_time {
            if now.duration_since(last_set) < config.programmatic_ignore_window {
                return false;
            }
        }
        
        // Otherwise, assume it's user-initiated
        true
    }
}

#[async_trait]
impl ClipboardMonitor for UnifiedClipboardMonitor {
    async fn start_monitoring(&self) -> ClipboardResult<()> {
        if self.monitoring.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        // Start platform-specific monitoring
        self.clipboard.start_monitoring().await?;
        
        // Set monitoring flag
        self.monitoring.store(true, Ordering::Relaxed);
        
        // Start the monitoring loop
        self.start_monitor_loop().await?;
        
        Ok(())
    }
    
    async fn stop_monitoring(&self) -> ClipboardResult<()> {
        if !self.monitoring.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        // Clear monitoring flag
        self.monitoring.store(false, Ordering::Relaxed);
        
        // Stop the monitoring task
        let mut handle = self.monitor_handle.write().await;
        if let Some(task) = handle.take() {
            task.abort();
        }
        
        // Stop platform-specific monitoring
        self.clipboard.stop_monitoring().await?;
        
        Ok(())
    }
    
    fn is_monitoring(&self) -> bool {
        self.monitoring.load(Ordering::Relaxed)
    }
    
    fn subscribe_to_changes(&self) -> broadcast::Receiver<ClipboardEvent> {
        self.event_sender.subscribe()
    }
    
    async fn get_current_content(&self) -> ClipboardResult<Option<ClipboardContent>> {
        // Attempt to get clipboard content with retry logic
        let error_config = self.error_config.read().await.clone();
        let mut retries = 0;
        let mut backoff = error_config.initial_backoff;
        
        loop {
            match self.clipboard.get_content().await {
                Ok(content) => return Ok(content),
                Err(err) => {
                    // Check if error is recoverable
                    if !err.is_recoverable() {
                        return Err(err);
                    }
                    
                    retries += 1;
                    if retries >= error_config.max_retries {
                        return Err(err);
                    }
                    
                    // Apply exponential backoff
                    tokio::time::sleep(backoff).await;
                    backoff = Duration::from_secs_f64(
                        backoff.as_secs_f64() * error_config.backoff_multiplier
                    ).min(error_config.max_backoff);
                }
            }
        }
    }
    
    async fn set_content(&self, content: ClipboardContent) -> ClipboardResult<()> {
        // Mark this as a programmatic change to prevent loop
        self.programmatic_change.store(true, Ordering::Relaxed);
        
        // Track the time of programmatic set for loop prevention
        let mut last_set = self.last_programmatic_set.write().await;
        *last_set = Some(std::time::Instant::now());
        
        // Calculate and update content hash
        let hash = Self::hash_content(&content);
        let mut last_hash = self.last_content_hash.write().await;
        *last_hash = Some(hash);
        
        // Update last content to prevent false change detection
        let mut last = self.last_content.write().await;
        *last = Some(content.clone());
        
        // Release locks before attempting clipboard operation
        drop(last);
        drop(last_hash);
        drop(last_set);
        
        // Attempt to set clipboard content with retry logic
        let error_config = self.error_config.read().await.clone();
        let mut retries = 0;
        let mut backoff = error_config.initial_backoff;
        
        loop {
            match self.clipboard.set_content(content.clone()).await {
                Ok(()) => return Ok(()),
                Err(err) => {
                    // Check if error is recoverable
                    if !err.is_recoverable() {
                        return Err(err);
                    }
                    
                    retries += 1;
                    if retries >= error_config.max_retries {
                        return Err(err);
                    }
                    
                    // Apply exponential backoff
                    tokio::time::sleep(backoff).await;
                    backoff = Duration::from_secs_f64(
                        backoff.as_secs_f64() * error_config.backoff_multiplier
                    ).min(error_config.max_backoff);
                }
            }
        }
    }
}

impl Default for UnifiedClipboardMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Default clipboard monitor implementation (legacy)
pub struct DefaultClipboardMonitor {
    event_sender: broadcast::Sender<ClipboardEvent>,
    monitoring: AtomicBool,
}

impl DefaultClipboardMonitor {
    /// Create new clipboard monitor
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(100);
        
        Self {
            event_sender,
            monitoring: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl ClipboardMonitor for DefaultClipboardMonitor {
    async fn start_monitoring(&self) -> ClipboardResult<()> {
        self.monitoring.store(true, Ordering::Relaxed);
        Ok(())
    }
    
    async fn stop_monitoring(&self) -> ClipboardResult<()> {
        self.monitoring.store(false, Ordering::Relaxed);
        Ok(())
    }
    
    fn is_monitoring(&self) -> bool {
        self.monitoring.load(Ordering::Relaxed)
    }
    
    fn subscribe_to_changes(&self) -> broadcast::Receiver<ClipboardEvent> {
        self.event_sender.subscribe()
    }
    
    async fn get_current_content(&self) -> ClipboardResult<Option<ClipboardContent>> {
        Err(ClipboardError::internal("Not implemented"))
    }
    
    async fn set_content(&self, _content: ClipboardContent) -> ClipboardResult<()> {
        Err(ClipboardError::internal("Not implemented"))
    }
}