use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::security::{Security, SecurityResult};
use crate::security::encryption::{EncryptionEngine, SessionId};
use crate::security::identity::PeerId;
use crate::security::policy::{PolicyEngine, ConnectionType, SecurityEvent, SecurityEventType};
use crate::transport::{TransportError, Connection, ConnectionInfo};

/// Secure connection wrapper that automatically encrypts/decrypts data
pub struct SecureConnection {
    /// Underlying transport connection
    inner: Box<dyn Connection>,
    /// Session ID for encryption
    session_id: SessionId,
    /// Reference to encryption engine
    encryption_engine: Arc<dyn EncryptionEngine>,
    /// Connection info
    info: ConnectionInfo,
}

impl SecureConnection {
    /// Create a new secure connection wrapper
    pub fn new(
        inner: Box<dyn Connection>,
        session_id: SessionId,
        encryption_engine: Arc<dyn EncryptionEngine>,
    ) -> Self {
        let info = inner.info();
        Self {
            inner,
            session_id,
            encryption_engine,
            info,
        }
    }
    
    /// Get the session ID
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }
}

impl std::fmt::Debug for SecureConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureConnection")
            .field("session_id", &self.session_id)
            .field("peer_id", &self.info.peer_id)
            .field("protocol", &self.info.protocol)
            .finish()
    }
}

#[async_trait]
impl Connection for SecureConnection {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        // Read encrypted data from underlying connection
        let mut encrypted_buf = vec![0u8; buf.len() + 1024]; // Extra space for encryption overhead
        let n = self.inner.read(&mut encrypted_buf).await?;
        
        if n == 0 {
            return Ok(0);
        }
        
        // Decrypt the data
        let decrypted = self.encryption_engine
            .decrypt_message(&self.session_id, &encrypted_buf[..n])
            .await
            .map_err(|e| TransportError::SecurityError {
                details: format!("Decryption failed: {}", e),
            })?;
        
        // Copy decrypted data to output buffer
        let copy_len = decrypted.len().min(buf.len());
        buf[..copy_len].copy_from_slice(&decrypted[..copy_len]);
        
        Ok(copy_len)
    }
    
    async fn write(&mut self, buf: &[u8]) -> Result<usize, TransportError> {
        // Encrypt the data
        let encrypted = self.encryption_engine
            .encrypt_message(&self.session_id, buf)
            .await
            .map_err(|e| TransportError::SecurityError {
                details: format!("Encryption failed: {}", e),
            })?;
        
        // Write encrypted data to underlying connection
        self.inner.write(&encrypted).await
    }
    
    async fn flush(&mut self) -> Result<(), TransportError> {
        self.inner.flush().await
    }
    
    async fn close(&mut self) -> Result<(), TransportError> {
        self.inner.close().await
    }
    
    fn info(&self) -> ConnectionInfo {
        self.info.clone()
    }
    
    fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }
}

/// Security hooks for transport layer
pub struct TransportSecurityHooks {
    /// Security system reference
    security: Arc<dyn Security>,
    /// Encryption engine reference
    encryption_engine: Arc<dyn EncryptionEngine>,
    /// Policy engine reference
    policy_engine: Arc<dyn PolicyEngine>,
    /// Active secure connections
    secure_connections: Arc<RwLock<std::collections::HashMap<String, SessionId>>>,
}

impl TransportSecurityHooks {
    /// Create new transport security hooks
    pub fn new(
        security: Arc<dyn Security>,
        encryption_engine: Arc<dyn EncryptionEngine>,
        policy_engine: Arc<dyn PolicyEngine>,
    ) -> Self {
        Self {
            security,
            encryption_engine,
            policy_engine,
            secure_connections: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Validate connection attempt before establishing
    pub async fn validate_connection(
        &self,
        peer_id: &PeerId,
        connection_type: ConnectionType,
    ) -> SecurityResult<bool> {
        // Log connection attempt
        let event = SecurityEvent::new(
            SecurityEventType::ConnectionAttempt,
            Some(peer_id.clone()),
            format!("Connection attempt from peer: {}", peer_id),
        );
        self.policy_engine.log_event(event).await?;
        
        // Check rate limiting
        if !self.policy_engine.check_rate_limit(peer_id).await? {
            let event = SecurityEvent::new(
                SecurityEventType::RateLimitExceeded,
                Some(peer_id.clone()),
                format!("Rate limit exceeded for peer: {}", peer_id),
            );
            self.policy_engine.log_event(event).await?;
            return Ok(false);
        }
        
        // Check if connection is allowed by policy
        let allowed = self.policy_engine
            .is_connection_allowed(peer_id, connection_type)
            .await?;
        
        if !allowed {
            let event = SecurityEvent::new(
                SecurityEventType::ConnectionRejected,
                Some(peer_id.clone()),
                format!("Connection rejected by policy for peer: {}", peer_id),
            );
            self.policy_engine.log_event(event).await?;
            return Ok(false);
        }
        
        // Check if peer is trusted
        let is_trusted = self.security.is_trusted(peer_id).await?;
        
        if !is_trusted {
            // Check if pairing is required
            let policy = self.policy_engine.get_policy().await?;
            if policy.require_pairing {
                let event = SecurityEvent::new(
                    SecurityEventType::ConnectionRejected,
                    Some(peer_id.clone()),
                    format!("Untrusted peer requires pairing: {}", peer_id),
                );
                self.policy_engine.log_event(event).await?;
                return Ok(false);
            }
        }
        
        // Log successful validation
        let event = SecurityEvent::new(
            SecurityEventType::ConnectionAccepted,
            Some(peer_id.clone()),
            format!("Connection accepted for peer: {}", peer_id),
        );
        self.policy_engine.log_event(event).await?;
        
        Ok(true)
    }
    
    /// Establish secure session for a connection
    pub async fn establish_secure_session(
        &self,
        peer_id: &PeerId,
        connection_id: String,
    ) -> SecurityResult<SessionId> {
        // Establish encryption session
        let session_id = self.security.establish_session(peer_id).await?;
        
        // Store session mapping
        let mut connections = self.secure_connections.write().await;
        connections.insert(connection_id, session_id.clone());
        
        Ok(session_id)
    }
    
    /// Wrap a connection with encryption
    pub async fn wrap_connection(
        &self,
        connection: Box<dyn Connection>,
        session_id: SessionId,
    ) -> Box<dyn Connection> {
        Box::new(SecureConnection::new(
            connection,
            session_id,
            Arc::clone(&self.encryption_engine),
        ))
    }
    
    /// Remove secure session when connection closes
    pub async fn remove_session(&self, connection_id: &str) -> SecurityResult<()> {
        let mut connections = self.secure_connections.write().await;
        connections.remove(connection_id);
        Ok(())
    }
    
    /// Get session ID for a connection
    pub async fn get_session_id(&self, connection_id: &str) -> Option<SessionId> {
        let connections = self.secure_connections.read().await;
        connections.get(connection_id).cloned()
    }
    
    /// Enforce security policy on connection
    pub async fn enforce_policy(
        &self,
        peer_id: &PeerId,
        connection_type: ConnectionType,
    ) -> SecurityResult<()> {
        let policy = self.policy_engine.get_policy().await?;
        
        // Enforce local-only mode
        if policy.local_only_mode {
            match connection_type {
                ConnectionType::LocalNetwork => {
                    // Allow local connections
                }
                ConnectionType::Relay | ConnectionType::Direct => {
                    let event = SecurityEvent::new(
                        SecurityEventType::PolicyViolation,
                        Some(peer_id.clone()),
                        format!("Local-only mode violation: {:?} connection attempted", connection_type),
                    );
                    self.policy_engine.log_event(event).await?;
                    
                    return Err(crate::security::error::SecurityError::PolicyViolation(
                        "Local-only mode: non-local connections not allowed".to_string()
                    ));
                }
            }
        }
        
        // Enforce private mode
        if policy.private_mode {
            // Check if peer is in allowlist
            let is_trusted = self.security.is_trusted(peer_id).await?;
            if !is_trusted {
                let event = SecurityEvent::new(
                    SecurityEventType::PolicyViolation,
                    Some(peer_id.clone()),
                    "Private mode violation: untrusted peer attempted connection".to_string(),
                );
                self.policy_engine.log_event(event).await?;
                
                return Err(crate::security::error::SecurityError::PolicyViolation(
                    "Private mode: only trusted peers allowed".to_string()
                ));
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::encryption::EncryptionEngineImpl;
    use crate::security::identity::DeviceIdentity;
    use crate::security::policy::{PolicyEngineImpl, SecurityPolicy};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::time::SystemTime;
    
    // Mock connection for testing
    #[derive(Debug)]
    struct MockConnection {
        peer_id: String,
        connected: bool,
        read_data: Vec<u8>,
        write_data: Vec<u8>,
    }
    
    impl MockConnection {
        fn new(peer_id: String) -> Self {
            Self {
                peer_id: peer_id.clone(),
                connected: true,
                read_data: Vec::new(),
                write_data: Vec::new(),
            }
        }
    }
    
    #[async_trait]
    impl Connection for MockConnection {
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
            if self.read_data.is_empty() {
                return Ok(0);
            }
            let len = self.read_data.len().min(buf.len());
            buf[..len].copy_from_slice(&self.read_data[..len]);
            self.read_data.drain(..len);
            Ok(len)
        }
        
        async fn write(&mut self, buf: &[u8]) -> Result<usize, TransportError> {
            self.write_data.extend_from_slice(buf);
            Ok(buf.len())
        }
        
        async fn flush(&mut self) -> Result<(), TransportError> {
            Ok(())
        }
        
        async fn close(&mut self) -> Result<(), TransportError> {
            self.connected = false;
            Ok(())
        }
        
        fn info(&self) -> ConnectionInfo {
            ConnectionInfo::new(
                self.peer_id.clone(),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
                "mock".to_string(),
            )
        }
        
        fn is_connected(&self) -> bool {
            self.connected
        }
    }
    
    #[tokio::test]
    async fn test_secure_connection_encryption() {
        // Create encryption engine
        let encryption_engine = Arc::new(EncryptionEngineImpl::with_defaults());
        
        // Create a test identity and peer ID
        let identity = DeviceIdentity::generate().unwrap();
        let peer_id = identity.derive_peer_id();
        
        // Establish a session
        let session_id = encryption_engine.establish_session(&peer_id).await.unwrap();
        
        // Create mock connection
        let mock_conn = MockConnection::new(peer_id.to_string());
        
        // Wrap with secure connection - cast to trait object
        let encryption_engine_trait: Arc<dyn EncryptionEngine> = encryption_engine.clone();
        let mut secure_conn = SecureConnection::new(
            Box::new(mock_conn),
            session_id.clone(),
            encryption_engine_trait,
        );
        
        // Test write (encryption)
        let test_data = b"Hello, secure world!";
        let written = secure_conn.write(test_data).await.unwrap();
        assert!(written > 0);
        
        // Verify connection is still connected
        assert!(secure_conn.is_connected());
    }
}
