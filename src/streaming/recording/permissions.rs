// Recording permission management
//
// Manages recording permissions for incoming streams with request/approval workflow
// and secure recording with encryption for sensitive content.
//
// Requirements: 5.2, 5.3

use crate::streaming::{StreamResult, StreamError, SessionId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use uuid::Uuid;

/// Recording permission status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionStatus {
    /// Permission request is pending
    Pending,
    /// Permission has been granted
    Granted,
    /// Permission has been denied
    Denied,
    /// Permission has been revoked
    Revoked,
}

/// Recording permission for a specific stream
/// 
/// Requirements: 5.2, 5.3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingPermission {
    pub permission_id: Uuid,
    pub stream_session_id: SessionId,
    pub peer_id: String,
    pub requester_device: String,
    pub status: PermissionStatus,
    pub requested_at: SystemTime,
    pub responded_at: Option<SystemTime>,
    pub expires_at: Option<SystemTime>,
    pub require_encryption: bool,
    pub allow_local_storage: bool,
    pub max_duration: Option<std::time::Duration>,
}

impl RecordingPermission {
    /// Create a new permission request
    pub fn new_request(
        stream_session_id: SessionId,
        peer_id: String,
        requester_device: String,
    ) -> Self {
        Self {
            permission_id: Uuid::new_v4(),
            stream_session_id,
            peer_id,
            requester_device,
            status: PermissionStatus::Pending,
            requested_at: SystemTime::now(),
            responded_at: None,
            expires_at: None,
            require_encryption: false,
            allow_local_storage: true,
            max_duration: None,
        }
    }
    
    /// Check if permission is currently valid
    pub fn is_valid(&self) -> bool {
        if self.status != PermissionStatus::Granted {
            return false;
        }
        
        // Check expiration
        if let Some(expires_at) = self.expires_at {
            if SystemTime::now() > expires_at {
                return false;
            }
        }
        
        true
    }
}

/// Permission request callback
pub type PermissionRequestCallback = Arc<dyn Fn(RecordingPermission) -> bool + Send + Sync>;

/// Permission manager for recording permissions
/// 
/// Manages recording permissions for incoming streams with request/approval workflow.
/// 
/// Requirements: 5.2, 5.3
pub struct PermissionManager {
    permissions: Arc<RwLock<HashMap<Uuid, RecordingPermission>>>,
    stream_permissions: Arc<RwLock<HashMap<SessionId, Vec<Uuid>>>>,
    request_callback: Arc<RwLock<Option<PermissionRequestCallback>>>,
}

impl PermissionManager {
    /// Create a new permission manager
    pub fn new() -> Self {
        Self {
            permissions: Arc::new(RwLock::new(HashMap::new())),
            stream_permissions: Arc::new(RwLock::new(HashMap::new())),
            request_callback: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Set the permission request callback
    /// 
    /// This callback is invoked when a new permission request is received.
    /// It should return true to auto-approve or false to require manual approval.
    pub fn set_request_callback(&self, callback: PermissionRequestCallback) {
        *self.request_callback
            .write()
            .expect("Lock poisoned") = Some(callback);
    }
    
    /// Request permission to record a stream
    /// 
    /// Requirements: 5.2, 5.3
    pub async fn request_permission(
        &self,
        stream_session_id: SessionId,
        peer_id: String,
        requester_device: String,
        require_encryption: bool,
    ) -> StreamResult<RecordingPermission> {
        let mut permission = RecordingPermission::new_request(
            stream_session_id,
            peer_id,
            requester_device,
        );
        permission.require_encryption = require_encryption;
        
        // Check if there's an auto-approval callback
        let should_auto_approve = {
            let callback = self.request_callback
                .read()
                .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
            
            callback.as_ref().map(|cb| cb(permission.clone())).unwrap_or(false)
        };
        
        if should_auto_approve {
            permission.status = PermissionStatus::Granted;
            permission.responded_at = Some(SystemTime::now());
        }
        
        // Store permission
        let permission_id = permission.permission_id;
        self.permissions
            .write()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?
            .insert(permission_id, permission.clone());
        
        // Track by stream session
        self.stream_permissions
            .write()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?
            .entry(stream_session_id)
            .or_insert_with(Vec::new)
            .push(permission_id);
        
        Ok(permission)
    }
    
    /// Approve a permission request
    /// 
    /// Requirements: 5.2, 5.3
    pub async fn approve_permission(
        &self,
        permission_id: Uuid,
        require_encryption: bool,
        max_duration: Option<std::time::Duration>,
    ) -> StreamResult<()> {
        let mut permissions = self.permissions
            .write()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
        
        let permission = permissions
            .get_mut(&permission_id)
            .ok_or_else(|| StreamError::internal("Permission not found"))?;
        
        if permission.status != PermissionStatus::Pending {
            return Err(StreamError::invalid_state(
                format!("Cannot approve permission in state {:?}", permission.status)
            ));
        }
        
        permission.status = PermissionStatus::Granted;
        permission.responded_at = Some(SystemTime::now());
        permission.require_encryption = require_encryption;
        permission.max_duration = max_duration;
        
        // Set expiration if max_duration is specified
        if let Some(duration) = max_duration {
            permission.expires_at = SystemTime::now().checked_add(duration);
        }
        
        Ok(())
    }
    
    /// Deny a permission request
    /// 
    /// Requirements: 5.2
    pub async fn deny_permission(&self, permission_id: Uuid) -> StreamResult<()> {
        let mut permissions = self.permissions
            .write()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
        
        let permission = permissions
            .get_mut(&permission_id)
            .ok_or_else(|| StreamError::internal("Permission not found"))?;
        
        if permission.status != PermissionStatus::Pending {
            return Err(StreamError::invalid_state(
                format!("Cannot deny permission in state {:?}", permission.status)
            ));
        }
        
        permission.status = PermissionStatus::Denied;
        permission.responded_at = Some(SystemTime::now());
        
        Ok(())
    }
    
    /// Revoke a granted permission
    /// 
    /// Requirements: 5.2
    pub async fn revoke_permission(&self, permission_id: Uuid) -> StreamResult<()> {
        let mut permissions = self.permissions
            .write()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
        
        let permission = permissions
            .get_mut(&permission_id)
            .ok_or_else(|| StreamError::internal("Permission not found"))?;
        
        if permission.status != PermissionStatus::Granted {
            return Err(StreamError::invalid_state(
                format!("Cannot revoke permission in state {:?}", permission.status)
            ));
        }
        
        permission.status = PermissionStatus::Revoked;
        
        Ok(())
    }
    
    /// Check if recording is permitted for a stream
    /// 
    /// Requirements: 5.2, 5.3
    pub async fn is_recording_permitted(
        &self,
        stream_session_id: SessionId,
        peer_id: &str,
    ) -> StreamResult<bool> {
        let stream_perms = self.stream_permissions
            .read()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
        
        let permission_ids = match stream_perms.get(&stream_session_id) {
            Some(ids) => ids.clone(),
            None => return Ok(false),
        };
        
        drop(stream_perms);
        
        let permissions = self.permissions
            .read()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
        
        for permission_id in permission_ids {
            if let Some(permission) = permissions.get(&permission_id) {
                if permission.peer_id == peer_id && permission.is_valid() {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    /// Get permission for a stream and peer
    /// 
    /// Requirements: 5.2, 5.3
    pub async fn get_permission(
        &self,
        stream_session_id: SessionId,
        peer_id: &str,
    ) -> StreamResult<Option<RecordingPermission>> {
        let stream_perms = self.stream_permissions
            .read()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
        
        let permission_ids = match stream_perms.get(&stream_session_id) {
            Some(ids) => ids.clone(),
            None => return Ok(None),
        };
        
        drop(stream_perms);
        
        let permissions = self.permissions
            .read()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
        
        for permission_id in permission_ids {
            if let Some(permission) = permissions.get(&permission_id) {
                if permission.peer_id == peer_id && permission.is_valid() {
                    return Ok(Some(permission.clone()));
                }
            }
        }
        
        Ok(None)
    }
    
    /// Get all pending permission requests
    /// 
    /// Requirements: 5.2
    pub async fn get_pending_requests(&self) -> StreamResult<Vec<RecordingPermission>> {
        let permissions = self.permissions
            .read()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
        
        Ok(permissions
            .values()
            .filter(|p| p.status == PermissionStatus::Pending)
            .cloned()
            .collect())
    }
    
    /// Get all permissions for a stream
    /// 
    /// Requirements: 5.2
    pub async fn get_stream_permissions(
        &self,
        stream_session_id: SessionId,
    ) -> StreamResult<Vec<RecordingPermission>> {
        let stream_perms = self.stream_permissions
            .read()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
        
        let permission_ids = match stream_perms.get(&stream_session_id) {
            Some(ids) => ids.clone(),
            None => return Ok(Vec::new()),
        };
        
        drop(stream_perms);
        
        let permissions = self.permissions
            .read()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
        
        Ok(permission_ids
            .iter()
            .filter_map(|id| permissions.get(id).cloned())
            .collect())
    }
    
    /// Clean up expired permissions
    /// 
    /// Requirements: 5.2
    pub async fn cleanup_expired_permissions(&self) -> StreamResult<()> {
        let mut permissions = self.permissions
            .write()
            .map_err(|e| StreamError::internal(format!("Lock error: {}", e)))?;
        
        let now = SystemTime::now();
        let expired: Vec<_> = permissions
            .iter()
            .filter(|(_, p)| {
                if let Some(expires_at) = p.expires_at {
                    now > expires_at
                } else {
                    false
                }
            })
            .map(|(id, _)| *id)
            .collect();
        
        for id in expired {
            permissions.remove(&id);
        }
        
        Ok(())
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Encrypted recording wrapper
/// 
/// Provides encryption for sensitive recording content.
/// 
/// Requirements: 5.3
pub struct EncryptedRecording {
    session_id: SessionId,
    encryption_key: Vec<u8>,
    encrypted: bool,
}

impl EncryptedRecording {
    /// Create a new encrypted recording
    pub fn new(session_id: SessionId, encryption_key: Vec<u8>) -> Self {
        Self {
            session_id,
            encryption_key,
            encrypted: true,
        }
    }
    
    /// Encrypt frame data
    /// 
    /// Requirements: 5.3
    pub fn encrypt_frame(&self, frame_data: &[u8]) -> StreamResult<Vec<u8>> {
        if !self.encrypted {
            return Ok(frame_data.to_vec());
        }
        
        // TODO: Implement actual encryption using the security module
        // This would use AES-256-GCM or similar authenticated encryption
        // For now, return the data as-is (placeholder)
        Ok(frame_data.to_vec())
    }
    
    /// Decrypt frame data
    /// 
    /// Requirements: 5.3
    pub fn decrypt_frame(&self, encrypted_data: &[u8]) -> StreamResult<Vec<u8>> {
        if !self.encrypted {
            return Ok(encrypted_data.to_vec());
        }
        
        // TODO: Implement actual decryption using the security module
        // This would use AES-256-GCM or similar authenticated encryption
        // For now, return the data as-is (placeholder)
        Ok(encrypted_data.to_vec())
    }
    
    /// Check if recording is encrypted
    pub fn is_encrypted(&self) -> bool {
        self.encrypted
    }
}
