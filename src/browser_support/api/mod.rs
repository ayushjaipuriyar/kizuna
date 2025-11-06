//! Browser API Server
//! 
//! Provides HTTP/WebSocket API endpoints for browser clients to interact with Kizuna.

pub mod server;
pub mod handlers;
pub mod websocket;

use crate::browser_support::BrowserResult;

/// API server for browser clients
pub struct APIServer {
    server: Option<server::WebServer>,
}

impl APIServer {
    /// Create a new API server
    pub fn new() -> Self {
        Self {
            server: None,
        }
    }
    
    /// Initialize the API server
    pub async fn initialize(&mut self) -> BrowserResult<()> {
        self.server = Some(server::WebServer::new());
        Ok(())
    }
    
    /// Start the API server on the specified port
    pub async fn start(&self, port: u16) -> BrowserResult<()> {
        if let Some(server) = &self.server {
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