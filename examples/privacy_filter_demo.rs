//! Privacy Filter Demo
//! 
//! Demonstrates the privacy filtering and sensitive content detection system.

use kizuna::clipboard::{
    ClipboardContent, TextContent, TextFormat, TextEncoding,
    privacy::{
        DefaultPrivacyFilter, PrivacyFilter, SensitiveContentDetector,
        PrivacyPolicy, PrivacyPolicyManager, PrivacyRule, SensitivePattern,
        SyncRecommendation, UserDecision,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Privacy Filter Demo ===\n");
    
    // Demo 1: Basic sensitive content detection
    demo_sensitive_detection().await?;
    
    // Demo 2: Custom privacy rules
    demo_custom_rules().await?;
    
    // Demo 3: Privacy policy management
    demo_policy_management().await?;
    
    // Demo 4: Content blacklisting
    demo_blacklisting().await?;
    
    Ok(())
}

async fn demo_sensitive_detection() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 1: Sensitive Content Detection ---\n");
    
    let detector = SensitiveContentDetector::new();
    
    // Test various sensitive patterns
    let test_cases = vec![
        ("password: mySecretPass123", "Password"),
        ("Credit card: 4532-1234-5678-9010", "Credit Card"),
        ("SSN: 123-45-6789", "Social Security Number"),
        ("API_KEY=sk_live_abcdef1234567890", "API Key"),
        ("Contact: john@example.com", "Email"),
        ("Call me at 555-123-4567", "Phone Number"),
        ("Just some normal text", "Normal Text"),
    ];
    
    for (text, label) in test_cases {
        let analysis = detector.analyze_text(text)?;
        
        println!("Text: \"{}\"", text);
        println!("Label: {}", label);
        println!("Sensitivity Score: {:.2}", analysis.sensitivity_score);
        println!("Detected Patterns: {:?}", analysis.detected_patterns);
        println!("Recommendation: {:?}", analysis.recommendation);
        println!("User Prompt Required: {}", analysis.user_prompt_required);
        println!();
    }
    
    Ok(())
}

async fn demo_custom_rules() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 2: Custom Privacy Rules ---\n");
    
    let detector = SensitiveContentDetector::new();
    
    // Add custom keyword
    detector.add_custom_keyword("confidential".to_string())?;
    detector.add_custom_keyword("internal-only".to_string())?;
    
    // Add custom regex rule
    let custom_rule = PrivacyRule {
        pattern: r"(?i)employee[_-]?id[:\s=]+\d+".to_string(),
        pattern_type: SensitivePattern::Custom("Employee ID".to_string()),
        action: SyncRecommendation::Block,
        enabled: true,
    };
    detector.add_rule(custom_rule)?;
    
    // Test custom rules
    let test_texts = vec![
        "This is a CONFIDENTIAL document",
        "Employee_ID: 12345",
        "Regular text without keywords",
    ];
    
    for text in test_texts {
        let analysis = detector.analyze_text(text)?;
        println!("Text: \"{}\"", text);
        println!("Sensitivity Score: {:.2}", analysis.sensitivity_score);
        println!("Detected Patterns: {:?}", analysis.detected_patterns);
        println!("Recommendation: {:?}\n", analysis.recommendation);
    }
    
    Ok(())
}

async fn demo_policy_management() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 3: Privacy Policy Management ---\n");
    
    // Create different policy configurations
    let policies = vec![
        ("Default", PrivacyPolicy::default()),
        ("Strict", PrivacyPolicy::strict()),
        ("Permissive", PrivacyPolicy::permissive()),
    ];
    
    let test_text = "password: secret123";
    let content = ClipboardContent::Text(TextContent {
        text: test_text.to_string(),
        encoding: TextEncoding::Utf8,
        format: TextFormat::Plain,
        size: test_text.len(),
    });
    
    for (name, policy) in policies {
        println!("Policy: {}", name);
        println!("  Enabled: {}", policy.enabled);
        println!("  Auto Block Sensitive: {}", policy.auto_block_sensitive);
        println!("  Prompt Threshold: {:.2}", policy.prompt_threshold);
        println!("  Block Threshold: {:.2}", policy.block_threshold);
        
        let manager = PrivacyPolicyManager::with_policy(policy)?;
        let decision = manager.should_sync_content(&content).await?;
        
        match decision {
            kizuna::clipboard::privacy::SyncDecision::Allow => {
                println!("  Decision: ALLOW\n");
            }
            kizuna::clipboard::privacy::SyncDecision::Block { reason, patterns } => {
                println!("  Decision: BLOCK");
                println!("  Reason: {}", reason);
                println!("  Patterns: {:?}\n", patterns);
            }
        }
    }
    
    Ok(())
}

async fn demo_blacklisting() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 4: Content Blacklisting ---\n");
    
    let manager = PrivacyPolicyManager::new();
    
    // Add content types to blacklist
    manager.add_to_blacklist("text/html".to_string())?;
    manager.add_to_blacklist("application/x-custom".to_string())?;
    
    println!("Added content types to blacklist:");
    println!("  - text/html");
    println!("  - application/x-custom\n");
    
    // Test different content types
    let test_contents = vec![
        (
            "Plain Text",
            ClipboardContent::Text(TextContent::new("Hello world".to_string())),
        ),
        (
            "Custom Type (blacklisted)",
            ClipboardContent::Custom {
                mime_type: "application/x-custom".to_string(),
                data: vec![1, 2, 3, 4],
            },
        ),
    ];
    
    for (label, content) in test_contents {
        println!("Testing: {}", label);
        
        let decision = manager.should_sync_content(&content).await?;
        match decision {
            kizuna::clipboard::privacy::SyncDecision::Allow => {
                println!("  Result: ALLOWED\n");
            }
            kizuna::clipboard::privacy::SyncDecision::Block { reason, .. } => {
                println!("  Result: BLOCKED");
                println!("  Reason: {}\n", reason);
            }
        }
    }
    
    Ok(())
}
