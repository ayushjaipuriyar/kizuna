//! Browser API Server
//! 
//! Provides HTTP/WebSocket API endpoints for browser clients to interact with Kizuna.

pub mod server;
pub mod handlers;
pub mod websocket;

use crate::browser_support::{BrowserResult, discovery::BrowserDiscovery};
use std::sync::Arc;

/// API server for browser clients
pub struct APIServer {
    server: Option<server::WebServer>,
    discovery_manager: Arc<BrowserDiscovery>,
}

impl APIServer {
    /// Create a new API server
    pub fn new(discovery_manager: Arc<BrowserDiscovery>) -> Self {
        Self {
            server: None,
            discovery_manager,
        }
    }
    
    /// Initialize the API server
    pub async fn initialize(&mut self) -> BrowserResult<()> {
        self.server = Some(server::WebServer::new(self.discovery_manager.clone()));
        Ok(())
    }
    
    /// Start the API server on the specified port
    pub async fn start(&mut self, port: u16) -> BrowserResult<()> {
        if let Some(server) = &mut self.server {
            server.start(port).await
        } else {
            Err(crate::browser_support::BrowserSupportError::ConfigurationError {
                parameter: "server".to_string(),
                issue: "Not initialized".to_string(),
            })
        }
    }
    
    /// Shutdown the API server
    pub async fn shutdown(&mut self) -> BrowserResult<()> {
        if let Some(server) = &mut self.server {
            server.shutdown().await?;
        }
        self.server = None;
        Ok(())
    }
}