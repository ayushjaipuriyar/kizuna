use rusqlite::{Connection, params, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use crate::security::error::{SecurityResult, TrustError};
use crate::security::identity::PeerId;
use super::{TrustEntry, TrustLevel, ServicePermissions};

/// Trust database for managing trusted peers
pub struct TrustDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl TrustDatabase {
    /// Create a new trust database
    pub fn new(db_path: PathBuf) -> SecurityResult<Self> {
        let conn = Connection::open(db_path)
            .map_err(|e| TrustError::DatabaseError(format!("Failed to open database: {}", e)))?;
        
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        
        db.initialize_schema()?;
        Ok(db)
    }
    
    /// Initialize database schema
    fn initialize_schema(&self) -> SecurityResult<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS trust_entries (
                peer_id TEXT PRIMARY KEY,
                nickname TEXT NOT NULL,
                first_seen INTEGER NOT NULL,
                last_seen INTEGER NOT NULL,
                trust_level TEXT NOT NULL,
                clipboard_permission INTEGER NOT NULL DEFAULT 1,
                file_transfer_permission INTEGER NOT NULL DEFAULT 1,
                camera_permission INTEGER NOT NULL DEFAULT 0,
                commands_permission INTEGER NOT NULL DEFAULT 0
            )",
            [],
        ).map_err(|e| TrustError::DatabaseError(format!("Failed to create table: {}", e)))?;
        
        Ok(())
    }
    
    /// Add a trusted peer
    pub fn add_peer(&self, entry: TrustEntry) -> SecurityResult<()> {
        let conn = self.conn.lock().unwrap();
        
        let peer_id_str = entry.peer_id.to_string();
        let trust_level_str = match entry.trust_level {
            TrustLevel::Verified => "Verified",
            TrustLevel::Trusted => "Trusted",
            TrustLevel::Allowlisted => "Allowlisted",
        };
        
        conn.execute(
            "INSERT OR REPLACE INTO trust_entries 
             (peer_id, nickname, first_seen, last_seen, trust_level, 
              clipboard_permission, file_transfer_permission, camera_permission, commands_permission)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                peer_id_str,
                entry.nickname,
                entry.first_seen,
                entry.last_seen,
                trust_level_str,
                entry.permissions.clipboard as i32,
                entry.permissions.file_transfer as i32,
                entry.permissions.camera as i32,
                entry.permissions.commands as i32,
            ],
        ).map_err(|e| TrustError::DatabaseError(format!("Failed to add peer: {}", e)))?;
        
        Ok(())
    }
    
    /// Remove a trusted peer
    pub fn remove_peer(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let conn = self.conn.lock().unwrap();
        
        let peer_id_str = peer_id.to_string();
        conn.execute(
            "DELETE FROM trust_entries WHERE peer_id = ?1",
            params![peer_id_str],
        ).map_err(|e| TrustError::DatabaseError(format!("Failed to remove peer: {}", e)))?;
        
        Ok(())
    }
    
    /// Get a trust entry by peer ID
    pub fn get_peer(&self, peer_id: &PeerId) -> SecurityResult<Option<TrustEntry>> {
        let conn = self.conn.lock().unwrap();
        
        let peer_id_str = peer_id.to_string();
        let result = conn.query_row(
            "SELECT peer_id, nickname, first_seen, last_seen, trust_level,
                    clipboard_permission, file_transfer_permission, camera_permission, commands_permission
             FROM trust_entries WHERE peer_id = ?1",
            params![peer_id_str],
            |row| {
                let trust_level_str: String = row.get(4)?;
                let trust_level = match trust_level_str.as_str() {
                    "Verified" => TrustLevel::Verified,
                    "Trusted" => TrustLevel::Trusted,
                    "Allowlisted" => TrustLevel::Allowlisted,
                    _ => TrustLevel::Allowlisted,
                };
                
                Ok(TrustEntry {
                    peer_id: peer_id.clone(),
                    nickname: row.get(1)?,
                    first_seen: row.get(2)?,
                    last_seen: row.get(3)?,
                    trust_level,
                    permissions: ServicePermissions {
                        clipboard: row.get::<_, i32>(5)? != 0,
                        file_transfer: row.get::<_, i32>(6)? != 0,
                        camera: row.get::<_, i32>(7)? != 0,
                        commands: row.get::<_, i32>(8)? != 0,
                    },
                })
            },
        ).optional()
        .map_err(|e| TrustError::DatabaseError(format!("Failed to get peer: {}", e)))?;
        
        Ok(result)
    }
    
    /// Check if a peer is trusted
    pub fn is_trusted(&self, peer_id: &PeerId) -> SecurityResult<bool> {
        Ok(self.get_peer(peer_id)?.is_some())
    }
    
    /// Get all trusted peers
    pub fn get_all_peers(&self) -> SecurityResult<Vec<TrustEntry>> {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT peer_id, nickname, first_seen, last_seen, trust_level,
                    clipboard_permission, file_transfer_permission, camera_permission, commands_permission
             FROM trust_entries"
        ).map_err(|e| TrustError::DatabaseError(format!("Failed to prepare statement: {}", e)))?;
        
        let entries = stmt.query_map([], |row| {
            let peer_id_str: String = row.get(0)?;
            let peer_id = PeerId::from_string(&peer_id_str)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            
            let trust_level_str: String = row.get(4)?;
            let trust_level = match trust_level_str.as_str() {
                "Verified" => TrustLevel::Verified,
                "Trusted" => TrustLevel::Trusted,
                "Allowlisted" => TrustLevel::Allowlisted,
                _ => TrustLevel::Allowlisted,
            };
            
            Ok(TrustEntry {
                peer_id,
                nickname: row.get(1)?,
                first_seen: row.get(2)?,
                last_seen: row.get(3)?,
                trust_level,
                permissions: ServicePermissions {
                    clipboard: row.get::<_, i32>(5)? != 0,
                    file_transfer: row.get::<_, i32>(6)? != 0,
                    camera: row.get::<_, i32>(7)? != 0,
                    commands: row.get::<_, i32>(8)? != 0,
                },
            })
        }).map_err(|e| TrustError::DatabaseError(format!("Failed to query peers: {}", e)))?;
        
        let mut result = Vec::new();
        for entry in entries {
            result.push(entry.map_err(|e| TrustError::DatabaseError(format!("Failed to parse entry: {}", e)))?);
        }
        
        Ok(result)
    }
    
    /// Update last seen timestamp for a peer
    pub fn update_last_seen(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let conn = self.conn.lock().unwrap();
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let peer_id_str = peer_id.to_string();
        conn.execute(
            "UPDATE trust_entries SET last_seen = ?1 WHERE peer_id = ?2",
            params![now, peer_id_str],
        ).map_err(|e| TrustError::DatabaseError(format!("Failed to update last seen: {}", e)))?;
        
        Ok(())
    }
    
    /// Update permissions for a peer
    pub fn update_permissions(&self, peer_id: &PeerId, permissions: ServicePermissions) -> SecurityResult<()> {
        let conn = self.conn.lock().unwrap();
        
        let peer_id_str = peer_id.to_string();
        conn.execute(
            "UPDATE trust_entries 
             SET clipboard_permission = ?1, file_transfer_permission = ?2, 
                 camera_permission = ?3, commands_permission = ?4
             WHERE peer_id = ?5",
            params![
                permissions.clipboard as i32,
                permissions.file_transfer as i32,
                permissions.camera as i32,
                permissions.commands as i32,
                peer_id_str,
            ],
        ).map_err(|e| TrustError::DatabaseError(format!("Failed to update permissions: {}", e)))?;
        
        Ok(())
    }
    
    /// Update trust level for a peer
    pub fn update_trust_level(&self, peer_id: &PeerId, trust_level: TrustLevel) -> SecurityResult<()> {
        let conn = self.conn.lock().unwrap();
        
        let trust_level_str = match trust_level {
            TrustLevel::Verified => "Verified",
            TrustLevel::Trusted => "Trusted",
            TrustLevel::Allowlisted => "Allowlisted",
        };
        
        let peer_id_str = peer_id.to_string();
        conn.execute(
            "UPDATE trust_entries SET trust_level = ?1 WHERE peer_id = ?2",
            params![trust_level_str, peer_id_str],
        ).map_err(|e| TrustError::DatabaseError(format!("Failed to update trust level: {}", e)))?;
        
        Ok(())
    }
}
