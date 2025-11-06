//! WebRTC Browser Support Module
//! 
//! This module provides WebRTC-based browser connectivity, enabling web browsers
//! to connect directly to Kizuna peers for file transfer, clipboard sync, 
//! camera streaming, and command execution.

pub mod webrtc;
pub mod api;
pub mod ui;
pub mod pwa;
pub mod error;
pub mod types;
pub mod discovery;
pub mod communication;
pub mod websocket_fallback;

#[cfg(test)]
mod tests;

pub use error::{BrowserSupportError, BrowserResult};
pub use types::*;
pub use discovery::*;

use crate::Result;
use std::sync::Arc;

/// Main browser support system interface
#[async_trait::async_trait]
pub trait BrowserSupportSystem {
    /// Initialize the browser support system
    async fn initialize(&mut self) -> Result<()>;
    
    /// Start the web server for browser clients
    async fn start_web_server(&self, port: u16) -> Result<()>;
    
    /// Handle incoming WebRTC connection from browser
    async fn handle_browser_connection(&self, connection_info: BrowserConnectionInfo) -> Result<BrowserSession>;
    
    /// Shutdown the browser support system
    async fn shutdown(&mut self) -> Result<()>;
}

/// Browser support system implementation
pub struct BrowserSupport {
    communication_manager: communication::UnifiedCommunicationManager,
    api_server: api::APIServer,
    pwa_controller: pwa::PWAController,
    discovery_manager: Arc<discovery::BrowserDiscovery>,
}

impl BrowserSupport {
    /// Create a new browser support instance
    pub fn new(peer_id: String, device_name: String) -> Self {
        let discovery_manager = Arc::new(discovery::BrowserDiscovery::new(peer_id, device_name));
        
        Self {
            communication_manager: communication::UnifiedCommunicationManager::new(),
            api_server: api::APIServer::new(discovery_manager.clone()),
            pwa_controller: pwa::PWAController::new(),
            discovery_manager,
        }
    }
    
    /// Get the discovery manager
    pub fn discovery(&self) -> &discovery::BrowserDiscovery {
        &self.discovery_manager
    }
    
    /// Get the communication manager
    pub fn communication(&self) -> &communication::UnifiedCommunicationManager {
        &self.communication_manager
    }
}

#[async_trait::async_trait]
impl BrowserSupportSystem for BrowserSupport {
    async fn initialize(&mut self) -> Result<()> {
        self.communication_manager.initialize().await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to initialize communication manager: {:?}", e))) as Box<dyn std::error::Error + Send + Sync>)?;
        self.api_server.initialize().await?;
        self.pwa_controller.initialize().await?;
        Ok(())
    }
    
    async fn start_web_server(&self, port: u16) -> Result<()> {
        // Initialize discovery manager with server address
        let addr = format!("127.0.0.1:{}", port).parse()
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid address: {}", e))) as Box<dyn std::error::Error + Send + Sync>)?;
        
        // Initialize the discovery manager
        self.discovery_manager.initialize(addr).await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to initialize discovery: {:?}", e))) as Box<dyn std::error::Error + Send + Sync>)?;
        
        self.api_server.start(port).await
    }
    
    async fn handle_browser_connection(&self, connection_info: BrowserConnectionInfo) -> Result<BrowserSession> {
        self.communication_manager.establish_connection(connection_info).await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to establish browser connection: {:?}", e))) as Box<dyn std::error::Error + Send + Sync>)
    }
    
    async fn shutdown(&mut self) -> Result<()> {
        self.pwa_controller.shutdown().await?;
        self.api_server.shutdown().await?;
        self.communication_manager.shutdown().await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to shutdown communication manager: {:?}", e))) as Box<dyn std::error::Error + Send + Sync>)?;
        Ok(())
    }
}