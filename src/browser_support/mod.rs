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

pub use error::{BrowserSupportError, BrowserResult};
pub use types::*;

use crate::Result;

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
    webrtc_manager: webrtc::WebRTCManager,
    api_server: api::APIServer,
    pwa_controller: pwa::PWAController,
}

impl BrowserSupport {
    /// Create a new browser support instance
    pub fn new() -> Self {
        Self {
            webrtc_manager: webrtc::WebRTCManager::new(),
            api_server: api::APIServer::new(),
            pwa_controller: pwa::PWAController::new(),
        }
    }
}

#[async_trait::async_trait]
impl BrowserSupportSystem for BrowserSupport {
    async fn initialize(&mut self) -> Result<()> {
        self.webrtc_manager.initialize().await?;
        self.api_server.initialize().await?;
        self.pwa_controller.initialize().await?;
        Ok(())
    }
    
    async fn start_web_server(&self, port: u16) -> Result<()> {
        self.api_server.start(port).await
    }
    
    async fn handle_browser_connection(&self, connection_info: BrowserConnectionInfo) -> Result<BrowserSession> {
        self.webrtc_manager.establish_connection(connection_info).await
    }
    
    async fn shutdown(&mut self) -> Result<()> {
        self.pwa_controller.shutdown().await?;
        self.api_server.shutdown().await?;
        self.webrtc_manager.shutdown().await?;
        Ok(())
    }
}