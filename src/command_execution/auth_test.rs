#[cfg(test)]
mod tests {
    use crate::command_execution::auth::*;
    use crate::command_execution::types::*;
    use tokio::sync::mpsc;
    use std::time::Duration;
    use uuid::Uuid;
    use chrono::Utc;
    
    fn create_test_command_request(command: &str) -> CommandRequest {
        CommandRequest {
            request_id: Uuid::new_v4(),
            command: command.to_string(),
            arguments: vec![],
            working_directory: None,
            environment: std::collections::HashMap::new(),
            timeout: Duration::from_secs(30),
            sandbox_config: SandboxConfig::default(),
            requester: "test_peer".to_string(),
            created_at: Utc::now(),
        }
    }
    
    #[tokio::test]
    async fn test_add_and_remove_trusted_command() {
        let (tx, _rx) = mpsc::channel(10);
        let manager = DefaultAuthorizationManager::new(tx);
        
        let pattern = CommandPattern {
            pattern: "^ls.*".to_string(),
            description: "List files".to_string(),
            allowed_peers: vec!["test_peer".to_string()],
        };
        
        // Add trusted command
        let command_id = manager.add_trusted_command(pattern.clone(), "test_peer".to_string())
            .await
            .expect("Failed to add trusted command");
        
        // Verify it's trusted
        let is_trusted = manager.is_trusted_command("ls -la", &"test_peer".to_string())
            .await
            .expect("Failed to check trusted command");
        assert!(is_trusted, "Command should be trusted");
        
        // Remove trusted command
        manager.remove_trusted_command(command_id)
            .await
            .expect("Failed to remove trusted command");
        
        // Verify it's no longer trusted
        let is_trusted = manager.is_trusted_command("ls -la", &"test_peer".to_string())
            .await
            .expect("Failed to check trusted command");
        assert!(!is_trusted, "Command should not be trusted after removal");
    }

    #[tokio::test]
    async fn test_trusted_command_pattern_matching() {
        let (tx, _rx) = mpsc::channel(10);
        let manager = DefaultAuthorizationManager::new(tx);
        
        let pattern = CommandPattern {
            pattern: "^(ls|dir).*".to_string(),
            description: "List directory commands".to_string(),
            allowed_peers: vec!["test_peer".to_string()],
        };
        
        manager.add_trusted_command(pattern, "test_peer".to_string())
            .await
            .expect("Failed to add trusted command");
        
        // Test matching commands
        assert!(manager.is_trusted_command("ls", &"test_peer".to_string()).await.unwrap());
        assert!(manager.is_trusted_command("ls -la", &"test_peer".to_string()).await.unwrap());
        assert!(manager.is_trusted_command("dir", &"test_peer".to_string()).await.unwrap());
        
        // Test non-matching commands
        assert!(!manager.is_trusted_command("rm -rf", &"test_peer".to_string()).await.unwrap());
        assert!(!manager.is_trusted_command("cat file.txt", &"test_peer".to_string()).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_peer_specific_trusted_commands() {
        let (tx, _rx) = mpsc::channel(10);
        let manager = DefaultAuthorizationManager::new(tx);
        
        let pattern = CommandPattern {
            pattern: "^ls.*".to_string(),
            description: "List files".to_string(),
            allowed_peers: vec!["allowed_peer".to_string()],
        };
        
        manager.add_trusted_command(pattern, "allowed_peer".to_string())
            .await
            .expect("Failed to add trusted command");
        
        // Should be trusted for allowed peer
        assert!(manager.is_trusted_command("ls", &"allowed_peer".to_string()).await.unwrap());
        
        // Should not be trusted for other peers
        assert!(!manager.is_trusted_command("ls", &"other_peer".to_string()).await.unwrap());
    }

    #[tokio::test]
    async fn test_risk_assessment_low() {
        let (tx, _rx) = mpsc::channel(10);
        let manager = DefaultAuthorizationManager::new(tx);
        
        let command = create_test_command_request("ls -la");
        let risk = manager.assess_risk_level(&command).await.unwrap();
        assert_eq!(risk, RiskLevel::Low);
        
        let command = create_test_command_request("cat file.txt");
        let risk = manager.assess_risk_level(&command).await.unwrap();
        assert_eq!(risk, RiskLevel::Low);
    }
    
    #[tokio::test]
    async fn test_risk_assessment_medium() {
        let (tx, _rx) = mpsc::channel(10);
        let manager = DefaultAuthorizationManager::new(tx);
        
        let command = create_test_command_request("cp file1.txt file2.txt");
        let risk = manager.assess_risk_level(&command).await.unwrap();
        assert_eq!(risk, RiskLevel::Medium);
        
        let command = create_test_command_request("mkdir newdir");
        let risk = manager.assess_risk_level(&command).await.unwrap();
        assert_eq!(risk, RiskLevel::Medium);
    }
    
    #[tokio::test]
    async fn test_risk_assessment_high() {
        let (tx, _rx) = mpsc::channel(10);
        let manager = DefaultAuthorizationManager::new(tx);
        
        let command = create_test_command_request("rm file.txt");
        let risk = manager.assess_risk_level(&command).await.unwrap();
        assert_eq!(risk, RiskLevel::High);
        
        let command = create_test_command_request("wget http://example.com/file");
        let risk = manager.assess_risk_level(&command).await.unwrap();
        assert_eq!(risk, RiskLevel::High);
    }

    #[tokio::test]
    async fn test_risk_assessment_critical() {
        let (tx, _rx) = mpsc::channel(10);
        let manager = DefaultAuthorizationManager::new(tx);
        
        let command = create_test_command_request("rm -rf /");
        let risk = manager.assess_risk_level(&command).await.unwrap();
        assert_eq!(risk, RiskLevel::Critical);
        
        let command = create_test_command_request("sudo reboot");
        let risk = manager.assess_risk_level(&command).await.unwrap();
        assert_eq!(risk, RiskLevel::Critical);
        
        let command = create_test_command_request("format C:");
        let risk = manager.assess_risk_level(&command).await.unwrap();
        assert_eq!(risk, RiskLevel::Critical);
    }
    
    #[tokio::test]
    async fn test_sandbox_policy_management() {
        let (tx, _rx) = mpsc::channel(10);
        let manager = DefaultAuthorizationManager::new(tx);
        
        // Get default policy
        let policy = manager.get_sandbox_policy(RiskLevel::Low).await.unwrap();
        assert_eq!(policy.max_cpu_percent, 25);
        assert_eq!(policy.max_memory_mb, 256);
        
        // Update policy
        let new_policy = SandboxConfig {
            max_cpu_percent: 30,
            max_memory_mb: 300,
            max_execution_time: Duration::from_secs(45),
            allowed_directories: vec![],
            blocked_directories: vec![],
            network_access: NetworkAccess::None,
            environment_isolation: true,
            temp_directory: None,
        };
        
        manager.update_sandbox_policy(RiskLevel::Low, new_policy.clone()).await.unwrap();
        
        // Verify update
        let updated_policy = manager.get_sandbox_policy(RiskLevel::Low).await.unwrap();
        assert_eq!(updated_policy.max_cpu_percent, 30);
        assert_eq!(updated_policy.max_memory_mb, 300);
    }
}
