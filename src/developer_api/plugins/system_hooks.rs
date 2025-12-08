/// System-wide plugin hooks for integrating with core Kizuna functionality
/// 
/// This module provides plugin hook interfaces for all major Kizuna systems,
/// allowing plugins to extend and customize behavior throughout the application.
/// 
/// Requirements: 5.2

use async_trait::async_trait;
use std::sync::Arc;
use crate::developer_api::core::KizunaError;

// Import types from core systems
use crate::discovery::ServiceRecord;
use crate::transport::{PeerId, Connection};
use crate::file_transfer::types::{TransferManifest, TransferSession};
#[cfg(feature = "streaming")]
use crate::streaming::{StreamSession, StreamConfig};
use crate::clipboard::ClipboardContent;
use crate::command_execution::{CommandRequest, CommandResult};

/// Discovery plugin hook for custom peer discovery strategies
#[async_trait]
pub trait DiscoveryHook: Send + Sync {
    /// Called when discovery is initiated
    async fn on_discovery_start(&self) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called to perform custom discovery
    /// Returns a list of discovered peers
    async fn discover_peers(&self) -> Result<Vec<ServiceRecord>, KizunaError>;
    
    /// Called when a peer is discovered by any strategy
    async fn on_peer_discovered(&self, peer: &ServiceRecord) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called when discovery is stopped
    async fn on_discovery_stop(&self) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Returns the name of this discovery strategy
    fn strategy_name(&self) -> &str;
    
    /// Returns whether this strategy is available on the current platform
    fn is_available(&self) -> bool {
        true
    }
}

/// Transport plugin hook for custom transport protocols
#[async_trait]
pub trait TransportHook: Send + Sync {
    /// Called before establishing a connection
    async fn on_connection_start(&self, peer_id: &PeerId) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called after a connection is established
    async fn on_connection_established(
        &self,
        peer_id: &PeerId,
        connection: &Arc<dyn Connection>,
    ) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called when a connection is closed
    async fn on_connection_closed(&self, peer_id: &PeerId) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called to transform outgoing data
    async fn transform_outgoing_data(&self, data: Vec<u8>) -> Result<Vec<u8>, KizunaError> {
        Ok(data)
    }
    
    /// Called to transform incoming data
    async fn transform_incoming_data(&self, data: Vec<u8>) -> Result<Vec<u8>, KizunaError> {
        Ok(data)
    }
    
    /// Returns the protocol name
    fn protocol_name(&self) -> &str;
}

/// Security plugin hook for custom security policies
#[async_trait]
pub trait SecurityHook: Send + Sync {
    /// Called to validate a peer before trusting
    async fn validate_peer(&self, peer_id: &PeerId) -> Result<bool, KizunaError> {
        Ok(true)
    }
    
    /// Called before encrypting data
    async fn pre_encrypt(&self, data: &[u8]) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called after decrypting data
    async fn post_decrypt(&self, data: &[u8]) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called when a security event occurs
    async fn on_security_event(&self, event: SecurityEvent) -> Result<(), KizunaError> {
        Ok(())
    }
}

/// Security event types
#[derive(Debug, Clone)]
pub enum SecurityEvent {
    /// Peer trust established
    PeerTrusted { peer_id: PeerId },
    
    /// Peer trust revoked
    PeerUntrusted { peer_id: PeerId },
    
    /// Authentication attempt
    AuthenticationAttempt { peer_id: PeerId, success: bool },
    
    /// Encryption session established
    SessionEstablished { peer_id: PeerId },
    
    /// Security violation detected
    SecurityViolation { peer_id: PeerId, reason: String },
}

/// File transfer plugin hook for custom transfer handling
#[async_trait]
pub trait FileTransferHook: Send + Sync {
    /// Called before starting a file transfer
    async fn on_transfer_start(
        &self,
        manifest: &TransferManifest,
    ) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called during file transfer progress
    async fn on_transfer_progress(
        &self,
        session: &TransferSession,
        bytes_transferred: u64,
    ) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called when a file transfer completes
    async fn on_transfer_complete(
        &self,
        session: &TransferSession,
    ) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called when a file transfer fails
    async fn on_transfer_error(
        &self,
        session: &TransferSession,
        error: &KizunaError,
    ) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called to validate a file before transfer
    async fn validate_file(&self, file_path: &std::path::Path) -> Result<bool, KizunaError> {
        Ok(true)
    }
}

/// Streaming plugin hook for custom streaming behavior
#[cfg(feature = "streaming")]
#[async_trait]
pub trait StreamingHook: Send + Sync {
    /// Called before starting a stream
    async fn on_stream_start(&self, config: &StreamConfig) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called when a stream is active
    async fn on_stream_active(&self, session: &StreamSession) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called when a viewer joins
    async fn on_viewer_join(
        &self,
        session: &StreamSession,
        viewer_id: &str,
    ) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called when a viewer leaves
    async fn on_viewer_leave(
        &self,
        session: &StreamSession,
        viewer_id: &str,
    ) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called when a stream ends
    async fn on_stream_end(&self, session: &StreamSession) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called to process video frames
    async fn process_frame(&self, frame_data: Vec<u8>) -> Result<Vec<u8>, KizunaError> {
        Ok(frame_data)
    }
}

/// Clipboard plugin hook for custom clipboard handling
#[async_trait]
pub trait ClipboardHook: Send + Sync {
    /// Called when clipboard content changes
    async fn on_clipboard_change(&self, content: &ClipboardContent) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called before setting clipboard content
    async fn pre_clipboard_set(&self, content: &ClipboardContent) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called after getting clipboard content
    async fn post_clipboard_get(&self, content: &ClipboardContent) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called to filter clipboard content
    async fn filter_content(&self, content: ClipboardContent) -> Result<ClipboardContent, KizunaError> {
        Ok(content)
    }
}

/// Command execution plugin hook for custom command handling
#[async_trait]
pub trait CommandExecutionHook: Send + Sync {
    /// Called before executing a command
    async fn on_command_start(&self, request: &CommandRequest) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called after a command completes
    async fn on_command_complete(
        &self,
        request: &CommandRequest,
        result: &CommandResult,
    ) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called when a command fails
    async fn on_command_error(
        &self,
        request: &CommandRequest,
        error: &KizunaError,
    ) -> Result<(), KizunaError> {
        Ok(())
    }
    
    /// Called to validate a command before execution
    async fn validate_command(&self, request: &CommandRequest) -> Result<bool, KizunaError> {
        Ok(true)
    }
    
    /// Called to transform command output
    async fn transform_output(&self, output: String) -> Result<String, KizunaError> {
        Ok(output)
    }
}

/// Hook registry for managing all system hooks
pub struct SystemHookRegistry {
    discovery_hooks: Vec<Arc<dyn DiscoveryHook>>,
    transport_hooks: Vec<Arc<dyn TransportHook>>,
    security_hooks: Vec<Arc<dyn SecurityHook>>,
    file_transfer_hooks: Vec<Arc<dyn FileTransferHook>>,
    #[cfg(feature = "streaming")]
    streaming_hooks: Vec<Arc<dyn StreamingHook>>,
    clipboard_hooks: Vec<Arc<dyn ClipboardHook>>,
    command_execution_hooks: Vec<Arc<dyn CommandExecutionHook>>,
}

impl SystemHookRegistry {
    /// Creates a new hook registry
    pub fn new() -> Self {
        Self {
            discovery_hooks: Vec::new(),
            transport_hooks: Vec::new(),
            security_hooks: Vec::new(),
            file_transfer_hooks: Vec::new(),
            #[cfg(feature = "streaming")]
            streaming_hooks: Vec::new(),
            clipboard_hooks: Vec::new(),
            command_execution_hooks: Vec::new(),
        }
    }
    
    /// Registers a discovery hook
    pub fn register_discovery_hook(&mut self, hook: Arc<dyn DiscoveryHook>) {
        self.discovery_hooks.push(hook);
    }
    
    /// Registers a transport hook
    pub fn register_transport_hook(&mut self, hook: Arc<dyn TransportHook>) {
        self.transport_hooks.push(hook);
    }
    
    /// Registers a security hook
    pub fn register_security_hook(&mut self, hook: Arc<dyn SecurityHook>) {
        self.security_hooks.push(hook);
    }
    
    /// Registers a file transfer hook
    pub fn register_file_transfer_hook(&mut self, hook: Arc<dyn FileTransferHook>) {
        self.file_transfer_hooks.push(hook);
    }
    
    /// Registers a streaming hook
    #[cfg(feature = "streaming")]
    pub fn register_streaming_hook(&mut self, hook: Arc<dyn StreamingHook>) {
        self.streaming_hooks.push(hook);
    }
    
    /// Registers a clipboard hook
    pub fn register_clipboard_hook(&mut self, hook: Arc<dyn ClipboardHook>) {
        self.clipboard_hooks.push(hook);
    }
    
    /// Registers a command execution hook
    pub fn register_command_execution_hook(&mut self, hook: Arc<dyn CommandExecutionHook>) {
        self.command_execution_hooks.push(hook);
    }
    
    /// Gets all discovery hooks
    pub fn discovery_hooks(&self) -> &[Arc<dyn DiscoveryHook>] {
        &self.discovery_hooks
    }
    
    /// Gets all transport hooks
    pub fn transport_hooks(&self) -> &[Arc<dyn TransportHook>] {
        &self.transport_hooks
    }
    
    /// Gets all security hooks
    pub fn security_hooks(&self) -> &[Arc<dyn SecurityHook>] {
        &self.security_hooks
    }
    
    /// Gets all file transfer hooks
    pub fn file_transfer_hooks(&self) -> &[Arc<dyn FileTransferHook>] {
        &self.file_transfer_hooks
    }
    
    /// Gets all streaming hooks
    #[cfg(feature = "streaming")]
    pub fn streaming_hooks(&self) -> &[Arc<dyn StreamingHook>] {
        &self.streaming_hooks
    }
    
    /// Gets all clipboard hooks
    pub fn clipboard_hooks(&self) -> &[Arc<dyn ClipboardHook>] {
        &self.clipboard_hooks
    }
    
    /// Gets all command execution hooks
    pub fn command_execution_hooks(&self) -> &[Arc<dyn CommandExecutionHook>] {
        &self.command_execution_hooks
    }
    
    /// Executes all discovery hooks for peer discovery
    pub async fn execute_discovery_hooks(&self) -> Result<Vec<ServiceRecord>, KizunaError> {
        let mut all_peers = Vec::new();
        
        for hook in &self.discovery_hooks {
            match hook.discover_peers().await {
                Ok(peers) => {
                    all_peers.extend(peers);
                }
                Err(e) => {
                    eprintln!("Discovery hook '{}' failed: {}", hook.strategy_name(), e);
                }
            }
        }
        
        Ok(all_peers)
    }
    
    /// Notifies all hooks of a peer discovery
    pub async fn notify_peer_discovered(&self, peer: &ServiceRecord) {
        for hook in &self.discovery_hooks {
            if let Err(e) = hook.on_peer_discovered(peer).await {
                eprintln!("Discovery hook notification failed: {}", e);
            }
        }
    }
    
    /// Notifies all hooks of a connection establishment
    pub async fn notify_connection_established(
        &self,
        peer_id: &PeerId,
        connection: &Arc<dyn Connection>,
    ) {
        for hook in &self.transport_hooks {
            if let Err(e) = hook.on_connection_established(peer_id, connection).await {
                eprintln!("Transport hook notification failed: {}", e);
            }
        }
    }
    
    /// Notifies all hooks of a security event
    pub async fn notify_security_event(&self, event: SecurityEvent) {
        for hook in &self.security_hooks {
            if let Err(e) = hook.on_security_event(event.clone()).await {
                eprintln!("Security hook notification failed: {}", e);
            }
        }
    }
}

impl Default for SystemHookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestDiscoveryHook;
    
    #[async_trait]
    impl DiscoveryHook for TestDiscoveryHook {
        async fn discover_peers(&self) -> Result<Vec<ServiceRecord>, KizunaError> {
            Ok(vec![])
        }
        
        fn strategy_name(&self) -> &str {
            "test"
        }
    }
    
    #[tokio::test]
    async fn test_hook_registry() {
        let mut registry = SystemHookRegistry::new();
        let hook = Arc::new(TestDiscoveryHook);
        
        registry.register_discovery_hook(hook);
        
        assert_eq!(registry.discovery_hooks().len(), 1);
    }
    
    #[tokio::test]
    async fn test_execute_discovery_hooks() {
        let mut registry = SystemHookRegistry::new();
        let hook = Arc::new(TestDiscoveryHook);
        
        registry.register_discovery_hook(hook);
        
        let peers = registry.execute_discovery_hooks().await.unwrap();
        assert_eq!(peers.len(), 0);
    }
}
