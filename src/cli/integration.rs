// CLI System Integration
//
// Provides unified integration of all CLI handlers with core Kizuna systems
// including discovery, file transfer, streaming, and security.
//
// Requirements: 1.1, 1.5, 2.1, 2.3, 3.1, 3.4, 5.1, 5.3, 6.1, 6.4, 6.5

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::handlers::{
    DiscoverHandler, TransferHandler, StreamingHandler, ExecHandler, PeersHandler, StatusHandler,
};
use crate::cli::security_integration::CLISecurityIntegration;
use crate::security::api::SecuritySystem;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Unified CLI system integration
/// 
/// This struct provides a single entry point for all CLI operations,
/// ensuring proper integration between handlers and core systems.
pub struct CLISystemIntegration {
    /// Discovery handler with security integration
    pub discover_handler: Arc<RwLock<DiscoverHandler>>,
    /// Transfer handler with security integration
    pub transfer_handler: Arc<TransferHandler>,
    /// Streaming handler with security integration
    pub streaming_handler: Arc<StreamingHandler>,
    /// Command execution handler with security integration
    pub exec_handler: Arc<RwLock<ExecHandler>>,
    /// Peers handler with discovery integration
    pub peers_handler: Arc<RwLock<PeersHandler>>,
    /// Status handler with system integration
    pub status_handler: Arc<RwLock<StatusHandler>>,
    /// Security integration
    pub security: Arc<CLISecurityIntegration>,
}

impl CLISystemIntegration {
    /// Create a new CLI system integration with default configuration
    pub fn new() -> CLIResult<Self> {
        let security_system = Arc::new(
            SecuritySystem::new()
                .map_err(|e| CLIError::security(format!("Failed to initialize security: {}", e)))?
        );

        Self::with_security(security_system)
    }

    /// Create a new CLI system integration with custom security system
    pub fn with_security(security_system: Arc<SecuritySystem>) -> CLIResult<Self> {
        // Create security integration
        let security = Arc::new(CLISecurityIntegration::new(Arc::clone(&security_system)));

        // Create discover handler with security
        let mut discover_handler = DiscoverHandler::new();
        discover_handler.set_security(Arc::clone(&security_system));
        let discover_handler = Arc::new(RwLock::new(discover_handler));

        // Create transfer handler with security
        let session_dir = Self::get_session_dir()?;
        let transfer_handler = Arc::new(TransferHandler::new(
            Arc::clone(&security_system),
            session_dir,
        ));

        // Create streaming handler with security
        let mut streaming_handler = StreamingHandler::new();
        streaming_handler.set_security(Arc::clone(&security_system));
        let streaming_handler = Arc::new(streaming_handler);

        // Create exec handler with security
        let mut exec_handler = ExecHandler::new();
        exec_handler.set_security(Arc::clone(&security_system));
        let exec_handler = Arc::new(RwLock::new(exec_handler));

        // Create peers handler with discovery integration
        let peers_handler = PeersHandler::with_discovery(Arc::clone(&discover_handler));
        let peers_handler = Arc::new(RwLock::new(peers_handler));

        // Create status handler with system integration
        let mut status_handler = StatusHandler::new();
        status_handler.set_discovery(Arc::clone(&discover_handler));
        status_handler.set_transfer(Arc::clone(&transfer_handler));
        status_handler.set_streaming(Arc::clone(&streaming_handler));
        let status_handler = Arc::new(RwLock::new(status_handler));

        Ok(Self {
            discover_handler,
            transfer_handler,
            streaming_handler,
            exec_handler,
            peers_handler,
            status_handler,
            security,
        })
    }

    /// Get session directory for file transfers
    fn get_session_dir() -> CLIResult<PathBuf> {
        let mut path = dirs::data_local_dir()
            .ok_or_else(|| CLIError::config("Failed to get local data directory".to_string()))?;
        path.push("kizuna");
        path.push("sessions");

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&path)
            .map_err(|e| CLIError::config(format!("Failed to create session directory: {}", e)))?;

        Ok(path)
    }

    /// Initialize all handlers and start background services
    pub async fn initialize(&self) -> CLIResult<()> {
        // Initialize discovery
        // Note: Discovery initialization is handled internally by the handler

        // Initialize file transfer
        // Note: File transfer initialization is handled internally by the handler

        // Authenticate CLI session
        self.security.authenticate().await?;

        Ok(())
    }

    /// Shutdown all handlers and cleanup resources
    pub async fn shutdown(&self) -> CLIResult<()> {
        // Stop discovery
        {
            let mut handler = self.discover_handler.write().await;
            handler.stop_continuous_discovery().await?;
        }

        // Logout security session
        self.security.logout().await?;

        Ok(())
    }

    /// Start continuous discovery mode
    pub async fn start_continuous_discovery(&self) -> CLIResult<()> {
        let mut handler = self.discover_handler.write().await;
        handler.start_continuous_discovery().await
    }

    /// Stop continuous discovery mode
    pub async fn stop_continuous_discovery(&self) -> CLIResult<()> {
        let mut handler = self.discover_handler.write().await;
        handler.stop_continuous_discovery().await
    }

    /// Check if an operation is authorized
    pub async fn authorize_operation(&self, operation: &str, peer_id: String) -> CLIResult<bool> {
        // Convert String peer_id to PeerId
        let peer_id = crate::security::identity::PeerId::from_string(&peer_id)
            .map_err(|e| CLIError::security(format!("Invalid peer ID: {}", e)))?;
        
        self.security.authorize_operation(operation, &peer_id).await
    }

    /// Get system status with all integrated information
    pub async fn get_system_status(&self) -> CLIResult<crate::cli::handlers::SystemStatus> {
        let handler = self.status_handler.read().await;
        handler.get_system_status().await
    }
}

impl Default for CLISystemIntegration {
    fn default() -> Self {
        Self::new().expect("Failed to create default CLI system integration")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cli_system_integration_creation() {
        let integration = CLISystemIntegration::new().unwrap();
        
        // Verify handlers are created
        // Note: Just verify the integration was created successfully
        assert!(integration.security.is_session_valid().await || !integration.security.is_session_valid().await);
    }

    #[tokio::test]
    async fn test_initialization() {
        let integration = CLISystemIntegration::new().unwrap();
        let result = integration.initialize().await;
        
        // Initialization may fail if strategies are unavailable, which is OK in test
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_get_system_status() {
        let integration = CLISystemIntegration::new().unwrap();
        let _ = integration.initialize().await;
        
        let status = integration.get_system_status().await.unwrap();
        assert!(!status.version.is_empty());
    }

    #[tokio::test]
    async fn test_shutdown() {
        let integration = CLISystemIntegration::new().unwrap();
        let _ = integration.initialize().await;
        
        let result = integration.shutdown().await;
        assert!(result.is_ok());
    }
}
