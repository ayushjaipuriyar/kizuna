use kizuna::command_execution::{
    auth::{AuthorizationManager, DefaultAuthorizationManager, PolicyEnforcedAuthorizationManager, SecurityPolicy},
    types::*,
};
use tokio::sync::mpsc;
use std::time::Duration;
use uuid::Uuid;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Command Authorization System Demo ===\n");
    
    // Create a channel for user prompts
    let (tx, mut rx) = mpsc::channel(10);
    
    // Create authorization manager
    let auth_manager = DefaultAuthorizationManager::new(tx.clone());
    
    // Demo 1: Add trusted commands
    println!("1. Adding trusted commands...");
    let ls_pattern = CommandPattern {
        pattern: "^ls.*".to_string(),
        description: "List directory contents".to_string(),
        allowed_peers: vec!["trusted_peer".to_string()],
    };
    
    let command_id = auth_manager
        .add_trusted_command(ls_pattern, "trusted_peer".to_string())
        .await?;
    println!("   Added trusted command with ID: {}", command_id);
    
    // Demo 2: Check if command is trusted
    println!("\n2. Checking trusted commands...");
    let is_trusted = auth_manager
        .is_trusted_command("ls -la", &"trusted_peer".to_string())
        .await?;
    println!("   'ls -la' from trusted_peer is trusted: {}", is_trusted);
    
    let is_trusted = auth_manager
        .is_trusted_command("rm -rf /", &"trusted_peer".to_string())
        .await?;
    println!("   'rm -rf /' from trusted_peer is trusted: {}", is_trusted);
    
    // Demo 3: Risk assessment
    println!("\n3. Assessing command risk levels...");
    let commands = vec![
        ("ls -la", "Low risk - read-only"),
        ("cp file1.txt file2.txt", "Medium risk - file operation"),
        ("rm important.txt", "High risk - deletion"),
        ("sudo rm -rf /", "Critical risk - dangerous operation"),
    ];
    
    for (cmd, expected) in commands {
        let request = CommandRequest {
            request_id: Uuid::new_v4(),
            command: cmd.to_string(),
            arguments: vec![],
            working_directory: None,
            environment: std::collections::HashMap::new(),
            timeout: Duration::from_secs(30),
            sandbox_config: SandboxConfig::default(),
            requester: "test_peer".to_string(),
            created_at: Utc::now(),
        };
        
        let risk = auth_manager.assess_risk_level(&request).await?;
        println!("   '{}' -> {:?} ({})", cmd, risk, expected);
    }
    
    // Demo 4: Sandbox policies
    println!("\n4. Sandbox policies for different risk levels...");
    for risk_level in &[RiskLevel::Low, RiskLevel::Medium, RiskLevel::High, RiskLevel::Critical] {
        let policy = auth_manager.get_sandbox_policy(*risk_level).await?;
        println!("   {:?}:", risk_level);
        println!("     - Max CPU: {}%", policy.max_cpu_percent);
        println!("     - Max Memory: {} MB", policy.max_memory_mb);
        println!("     - Max Time: {:?}", policy.max_execution_time);
        println!("     - Network: {:?}", policy.network_access);
    }
    
    // Demo 5: Policy-enforced authorization
    println!("\n5. Policy-enforced authorization...");
    let policy = SecurityPolicy {
        require_authorization: true,
        allow_trusted_auto_approval: true,
        max_auto_approve_risk: RiskLevel::Low,
        default_timeout: Duration::from_secs(60),
        log_all_decisions: true,
    };
    
    let policy_manager = PolicyEnforcedAuthorizationManager::new(tx, policy);
    
    // This should auto-approve because it's trusted and low risk
    let is_trusted = policy_manager
        .is_trusted_command("ls -la", &"trusted_peer".to_string())
        .await?;
    println!("   Auto-approval enabled for trusted low-risk commands: {}", is_trusted);
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}
