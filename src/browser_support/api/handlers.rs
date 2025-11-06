//! API Request Handlers
//! 
//! HTTP request handlers for browser API endpoints.

use crate::browser_support::{BrowserResult, BrowserSupportError, discovery::BrowserDiscovery};
use crate::browser_support::types::*;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// API request handlers
pub struct APIHandlers {
    discovery_manager: Arc<BrowserDiscovery>,
}

impl APIHandlers {
    /// Create new API handlers
    pub fn new(discovery_manager: Arc<BrowserDiscovery>) -> Self {
        Self {
            discovery_manager,
        }
    }
    
    /// Handle connection setup creation request
    pub async fn handle_create_connection_setup(&self) -> BrowserResult<Value> {
        let setup = self.discovery_manager.create_connection_setup().await?;
        
        Ok(serde_json::json!({
            "setup_id": setup.setup_id,
            "connection_url": setup.connection_url,
            "qr_code_data": setup.qr_code_data,
            "peer_info": setup.peer_info,
            "expires_at": setup.expires_at.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
            "ice_servers": setup.ice_servers
        }))
    }
    
    /// Handle connection setup retrieval request
    pub async fn handle_get_connection_setup(&self, setup_id: Uuid) -> BrowserResult<Value> {
        let setup = self.discovery_manager.get_connection_setup(setup_id).await?;
        
        Ok(serde_json::json!({
            "setup_id": setup.setup_id,
            "connection_url": setup.connection_url,
            "qr_code_data": setup.qr_code_data,
            "peer_info": setup.peer_info,
            "expires_at": setup.expires_at.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_secs(),
            "ice_servers": setup.ice_servers
        }))
    }
    
    /// Handle QR code generation request
    pub async fn handle_generate_qr_code(&self, setup_id: Uuid) -> BrowserResult<String> {
        let setup = self.discovery_manager.get_connection_setup(setup_id).await?;
        let qr_svg = self.discovery_manager.generate_qr_code_svg(&setup.qr_code_data)?;
        Ok(qr_svg)
    }
    
    /// Handle peer discovery request
    pub async fn handle_discover_peers(&self) -> BrowserResult<Value> {
        let peers = self.discovery_manager.get_discovered_peers().await?;
        let local_peer = self.discovery_manager.get_local_peer_info().await;
        
        Ok(serde_json::json!({
            "local_peer": local_peer,
            "discovered_peers": peers,
            "status": "active"
        }))
    }
    
    /// Handle connection status request
    pub async fn handle_get_connection_status(&self, session_id: Uuid) -> BrowserResult<Value> {
        let status = self.discovery_manager.get_connection_status(session_id).await?;
        
        Ok(serde_json::json!({
            "session_id": status.session_id,
            "peer_id": status.peer_id,
            "connection_state": status.connection_state,
            "ice_connection_state": status.ice_connection_state,
            "data_channels": status.data_channels,
            "connection_quality": status.connection_quality,
            "established_at": status.established_at.map(|t| 
                t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
            ),
            "last_activity": status.last_activity.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_secs()
        }))
    }
    
    /// Handle all connection statuses request
    pub async fn handle_get_all_connection_statuses(&self) -> BrowserResult<Value> {
        let statuses = self.discovery_manager.get_all_connection_statuses().await?;
        
        Ok(serde_json::json!({
            "connections": statuses,
            "count": statuses.len()
        }))
    }
    
    /// Handle browser connection info creation
    pub async fn handle_create_browser_connection_info(
        &self,
        setup_id: Uuid,
        browser_info: BrowserInfo,
    ) -> BrowserResult<Value> {
        let connection_info = self.discovery_manager
            .create_browser_connection_info(setup_id, browser_info)
            .await?;
        
        Ok(serde_json::json!({
            "peer_id": connection_info.peer_id,
            "signaling_info": connection_info.signaling_info,
            "browser_info": connection_info.browser_info
        }))
    }
    
    /// Handle file transfer request
    pub async fn handle_file_transfer(&self, _request: Value) -> BrowserResult<Value> {
        // TODO: Implement file transfer
        Ok(serde_json::json!({
            "status": "not_implemented"
        }))
    }
    
    /// Handle clipboard sync request
    pub async fn handle_clipboard_sync(&self, _request: Value) -> BrowserResult<Value> {
        // TODO: Implement clipboard sync
        Ok(serde_json::json!({
            "status": "not_implemented"
        }))
    }
}