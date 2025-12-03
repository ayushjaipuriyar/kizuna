// Template Sharing and Synchronization
//
// This module provides secure template sharing between trusted devices
// with versioning and access control.

use crate::command_execution::{
    error::{CommandError, CommandResult as CmdResult},
    template::{CommandTemplate, TemplateId},
    types::*,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Template sharing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateShareRequest {
    pub template_id: TemplateId,
    pub target_peers: Vec<PeerId>,
    pub permissions: TemplatePermissions,
    pub requester: PeerId,
}

/// Template permissions for shared templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplatePermissions {
    pub can_view: bool,
    pub can_execute: bool,
    pub can_modify: bool,
    pub can_share: bool,
}

impl Default for TemplatePermissions {
    fn default() -> Self {
        Self {
            can_view: true,
            can_execute: true,
            can_modify: false,
            can_share: false,
        }
    }
}

/// Shared template metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedTemplate {
    pub template: CommandTemplate,
    pub shared_by: PeerId,
    pub shared_at: Timestamp,
    pub permissions: TemplatePermissions,
    pub sync_status: SyncStatus,
}

/// Template synchronization status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Synced,
    OutOfSync,
    Pending,
    Failed(String),
}

/// Template update notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateUpdate {
    pub template_id: TemplateId,
    pub old_version: u32,
    pub new_version: u32,
    pub updated_template: CommandTemplate,
    pub updated_by: PeerId,
    pub updated_at: Timestamp,
}

/// Template sharing manager
pub struct TemplateSharingManager {
    shared_templates: HashMap<TemplateId, SharedTemplate>,
    template_permissions: HashMap<(TemplateId, PeerId), TemplatePermissions>,
    pending_updates: Vec<TemplateUpdate>,
}

impl TemplateSharingManager {
    /// Create a new template sharing manager
    pub fn new() -> Self {
        Self {
            shared_templates: HashMap::new(),
            template_permissions: HashMap::new(),
            pending_updates: Vec::new(),
        }
    }

    /// Share a template with target peers
    pub fn share_template(
        &mut self,
        template: CommandTemplate,
        target_peers: Vec<PeerId>,
        permissions: TemplatePermissions,
        requester: PeerId,
    ) -> CmdResult<Vec<TemplateShareRequest>> {
        // Verify requester has permission to share
        if let Some(shared) = self.shared_templates.get(&template.template_id) {
            if shared.shared_by != requester {
                let perms = self.get_permissions(&template.template_id, &requester)?;
                if !perms.can_share {
                    return Err(CommandError::PermissionError(
                        "No permission to share this template".to_string(),
                    ));
                }
            }
        }

        // Create share requests for each target peer
        let mut requests = Vec::new();
        for peer in target_peers {
            // Store permissions for this peer
            self.template_permissions.insert(
                (template.template_id, peer.clone()),
                permissions.clone(),
            );

            requests.push(TemplateShareRequest {
                template_id: template.template_id,
                target_peers: vec![peer],
                permissions: permissions.clone(),
                requester: requester.clone(),
            });
        }

        // Store as shared template
        let shared = SharedTemplate {
            template: template.clone(),
            shared_by: requester,
            shared_at: chrono::Utc::now(),
            permissions: permissions.clone(),
            sync_status: SyncStatus::Pending,
        };

        self.shared_templates.insert(template.template_id, shared);

        Ok(requests)
    }

    /// Receive a shared template from another peer
    pub fn receive_shared_template(
        &mut self,
        template: CommandTemplate,
        shared_by: PeerId,
        permissions: TemplatePermissions,
    ) -> CmdResult<()> {
        let shared = SharedTemplate {
            template: template.clone(),
            shared_by: shared_by.clone(),
            shared_at: chrono::Utc::now(),
            permissions,
            sync_status: SyncStatus::Synced,
        };

        self.shared_templates.insert(template.template_id, shared);

        Ok(())
    }

    /// Update a shared template
    pub fn update_shared_template(
        &mut self,
        template_id: &TemplateId,
        updated_template: CommandTemplate,
        updater: PeerId,
    ) -> CmdResult<TemplateUpdate> {
        let shared = self.shared_templates
            .get(template_id)
            .ok_or_else(|| CommandError::TemplateNotFound(template_id.to_string()))?;

        // Check permissions
        if shared.shared_by != updater {
            let perms = self.get_permissions(template_id, &updater)?;
            if !perms.can_modify {
                return Err(CommandError::PermissionError(
                    "No permission to modify this template".to_string(),
                ));
            }
        }

        let old_version = shared.template.version;
        let update = TemplateUpdate {
            template_id: *template_id,
            old_version,
            new_version: updated_template.version,
            updated_template: updated_template.clone(),
            updated_by: updater,
            updated_at: chrono::Utc::now(),
        };

        // Update the shared template
        let mut updated_shared = shared.clone();
        updated_shared.template = updated_template;
        updated_shared.sync_status = SyncStatus::Pending;

        self.shared_templates.insert(*template_id, updated_shared);
        self.pending_updates.push(update.clone());

        Ok(update)
    }

    /// Apply a template update from another peer
    pub fn apply_template_update(&mut self, update: TemplateUpdate) -> CmdResult<()> {
        let shared = self.shared_templates
            .get_mut(&update.template_id)
            .ok_or_else(|| CommandError::TemplateNotFound(update.template_id.to_string()))?;

        // Check version compatibility
        if update.old_version != shared.template.version {
            shared.sync_status = SyncStatus::OutOfSync;
            return Err(CommandError::ValidationError(format!(
                "Version mismatch: expected {}, got {}",
                shared.template.version, update.old_version
            )));
        }

        // Apply the update
        shared.template = update.updated_template;
        shared.sync_status = SyncStatus::Synced;

        Ok(())
    }

    /// Get permissions for a peer on a template
    pub fn get_permissions(
        &self,
        template_id: &TemplateId,
        peer_id: &PeerId,
    ) -> CmdResult<TemplatePermissions> {
        // Check if there are specific permissions for this peer
        if let Some(perms) = self.template_permissions.get(&(*template_id, peer_id.clone())) {
            return Ok(perms.clone());
        }

        // Check if this is a shared template and return default permissions
        if let Some(shared) = self.shared_templates.get(template_id) {
            return Ok(shared.permissions.clone());
        }

        Err(CommandError::TemplateNotFound(template_id.to_string()))
    }

    /// Update permissions for a peer on a template
    pub fn update_permissions(
        &mut self,
        template_id: &TemplateId,
        peer_id: &PeerId,
        permissions: TemplatePermissions,
        requester: PeerId,
    ) -> CmdResult<()> {
        let shared = self.shared_templates
            .get(template_id)
            .ok_or_else(|| CommandError::TemplateNotFound(template_id.to_string()))?;

        // Only the owner can update permissions
        if shared.shared_by != requester {
            return Err(CommandError::PermissionError(
                "Only the template owner can update permissions".to_string(),
            ));
        }

        self.template_permissions.insert(
            (*template_id, peer_id.clone()),
            permissions,
        );

        Ok(())
    }

    /// Revoke access to a shared template
    pub fn revoke_access(
        &mut self,
        template_id: &TemplateId,
        peer_id: &PeerId,
        requester: PeerId,
    ) -> CmdResult<()> {
        let shared = self.shared_templates
            .get(template_id)
            .ok_or_else(|| CommandError::TemplateNotFound(template_id.to_string()))?;

        // Only the owner can revoke access
        if shared.shared_by != requester {
            return Err(CommandError::PermissionError(
                "Only the template owner can revoke access".to_string(),
            ));
        }

        self.template_permissions.remove(&(*template_id, peer_id.clone()));

        Ok(())
    }

    /// Get all shared templates
    pub fn list_shared_templates(&self) -> Vec<&SharedTemplate> {
        self.shared_templates.values().collect()
    }

    /// Get shared templates by peer
    pub fn get_templates_shared_by(&self, peer_id: &PeerId) -> Vec<&SharedTemplate> {
        self.shared_templates
            .values()
            .filter(|t| &t.shared_by == peer_id)
            .collect()
    }

    /// Get pending template updates
    pub fn get_pending_updates(&self) -> &[TemplateUpdate] {
        &self.pending_updates
    }

    /// Clear pending updates
    pub fn clear_pending_updates(&mut self) {
        self.pending_updates.clear();
    }

    /// Get sync status for a template
    pub fn get_sync_status(&self, template_id: &TemplateId) -> CmdResult<SyncStatus> {
        let shared = self.shared_templates
            .get(template_id)
            .ok_or_else(|| CommandError::TemplateNotFound(template_id.to_string()))?;

        Ok(shared.sync_status.clone())
    }

    /// Mark template as synced
    pub fn mark_synced(&mut self, template_id: &TemplateId) -> CmdResult<()> {
        let shared = self.shared_templates
            .get_mut(template_id)
            .ok_or_else(|| CommandError::TemplateNotFound(template_id.to_string()))?;

        shared.sync_status = SyncStatus::Synced;

        Ok(())
    }

    /// Mark template sync as failed
    pub fn mark_sync_failed(&mut self, template_id: &TemplateId, reason: String) -> CmdResult<()> {
        let shared = self.shared_templates
            .get_mut(template_id)
            .ok_or_else(|| CommandError::TemplateNotFound(template_id.to_string()))?;

        shared.sync_status = SyncStatus::Failed(reason);

        Ok(())
    }

    /// Check if a peer has permission to execute a template
    pub fn can_execute(&self, template_id: &TemplateId, peer_id: &PeerId) -> bool {
        if let Ok(perms) = self.get_permissions(template_id, peer_id) {
            perms.can_execute
        } else {
            false
        }
    }

    /// Check if a peer has permission to modify a template
    pub fn can_modify(&self, template_id: &TemplateId, peer_id: &PeerId) -> bool {
        if let Ok(perms) = self.get_permissions(template_id, peer_id) {
            perms.can_modify
        } else {
            false
        }
    }
}

impl Default for TemplateSharingManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_execution::template::TemplateParameter;
    use uuid::Uuid;

    fn create_test_template() -> CommandTemplate {
        CommandTemplate {
            template_id: Uuid::new_v4(),
            name: "Test Template".to_string(),
            description: "A test template".to_string(),
            command: "echo {{message}}".to_string(),
            parameters: vec![TemplateParameter {
                name: "message".to_string(),
                description: "Message to echo".to_string(),
                param_type: crate::command_execution::template::ParameterType::String,
                required: true,
                default_value: None,
                validation_pattern: None,
            }],
            sandbox_config: SandboxConfig::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: 1,
            owner: "owner_peer".to_string(),
            tags: vec![],
        }
    }

    #[test]
    fn test_share_template() {
        let mut manager = TemplateSharingManager::new();
        let template = create_test_template();
        let target_peers = vec!["peer1".to_string(), "peer2".to_string()];

        let result = manager.share_template(
            template.clone(),
            target_peers.clone(),
            TemplatePermissions::default(),
            "owner_peer".to_string(),
        );

        assert!(result.is_ok());
        let requests = result.unwrap();
        assert_eq!(requests.len(), 2);
    }

    #[test]
    fn test_receive_shared_template() {
        let mut manager = TemplateSharingManager::new();
        let template = create_test_template();

        let result = manager.receive_shared_template(
            template.clone(),
            "owner_peer".to_string(),
            TemplatePermissions::default(),
        );

        assert!(result.is_ok());
        assert_eq!(manager.list_shared_templates().len(), 1);
    }

    #[test]
    fn test_permissions() {
        let mut manager = TemplateSharingManager::new();
        let template = create_test_template();

        manager.receive_shared_template(
            template.clone(),
            "owner_peer".to_string(),
            TemplatePermissions {
                can_view: true,
                can_execute: true,
                can_modify: false,
                can_share: false,
            },
        ).unwrap();

        assert!(manager.can_execute(&template.template_id, &"any_peer".to_string()));
        assert!(!manager.can_modify(&template.template_id, &"any_peer".to_string()));
    }

    #[test]
    fn test_update_shared_template() {
        let mut manager = TemplateSharingManager::new();
        let mut template = create_test_template();

        manager.receive_shared_template(
            template.clone(),
            "owner_peer".to_string(),
            TemplatePermissions::default(),
        ).unwrap();

        template.version = 2;
        template.description = "Updated description".to_string();

        let result = manager.update_shared_template(
            &template.template_id,
            template.clone(),
            "owner_peer".to_string(),
        );

        assert!(result.is_ok());
        let update = result.unwrap();
        assert_eq!(update.old_version, 1);
        assert_eq!(update.new_version, 2);
    }

    #[test]
    fn test_permission_denied() {
        let mut manager = TemplateSharingManager::new();
        let template = create_test_template();

        manager.receive_shared_template(
            template.clone(),
            "owner_peer".to_string(),
            TemplatePermissions {
                can_view: true,
                can_execute: true,
                can_modify: false,
                can_share: false,
            },
        ).unwrap();

        let mut updated = template.clone();
        updated.version = 2;

        let result = manager.update_shared_template(
            &template.template_id,
            updated,
            "other_peer".to_string(),
        );

        assert!(result.is_err());
    }
}
