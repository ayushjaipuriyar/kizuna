//! Privacy filtering and sensitive content detection

use async_trait::async_trait;
use regex::Regex;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use crate::clipboard::{ClipboardContent, ClipboardResult, ClipboardError};
use crate::clipboard::content::ValidationResult;

/// Privacy analysis result
#[derive(Debug, Clone)]
pub struct PrivacyAnalysis {
    pub sensitivity_score: f32, // 0.0 to 1.0
    pub detected_patterns: Vec<SensitivePattern>,
    pub recommendation: SyncRecommendation,
    pub user_prompt_required: bool,
}

/// Types of sensitive patterns that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SensitivePattern {
    Password,
    CreditCard,
    SocialSecurity,
    Email,
    PhoneNumber,
    ApiKey,
    Custom(String),
}

/// Recommendation for sync operation
#[derive(Debug, Clone, PartialEq)]
pub enum SyncRecommendation {
    Allow,
    Block,
    Prompt,
}

/// User decision for sensitive content
#[derive(Debug, Clone, PartialEq)]
pub enum UserDecision {
    Allow,
    Block,
    AlwaysAllow,
    AlwaysBlock,
}

/// Privacy rule configuration
#[derive(Debug, Clone)]
pub struct PrivacyRule {
    pub pattern: String,
    pub pattern_type: SensitivePattern,
    pub action: SyncRecommendation,
    pub enabled: bool,
}

/// Privacy filter trait
#[async_trait]
pub trait PrivacyFilter: Send + Sync {
    /// Analyze content for privacy concerns
    async fn analyze_content(&self, content: &ClipboardContent) -> ClipboardResult<PrivacyAnalysis>;
    
    /// Determine if content should be synced
    async fn should_sync_content(&self, content: &ClipboardContent) -> ClipboardResult<SyncRecommendation>;
    
    /// Add a new privacy rule
    async fn add_privacy_rule(&self, rule: PrivacyRule) -> ClipboardResult<()>;
    
    /// Prompt user for sensitive content decision
    async fn prompt_user_for_sensitive_content(&self, content: &ClipboardContent) -> ClipboardResult<UserDecision>;
}

/// Sensitive content detector with pattern matching
pub struct SensitiveContentDetector {
    rules: Arc<RwLock<Vec<PrivacyRule>>>,
    compiled_patterns: Arc<RwLock<HashMap<String, Regex>>>,
    custom_keywords: Arc<RwLock<Vec<String>>>,
}

impl SensitiveContentDetector {
    /// Create new detector with default rules
    pub fn new() -> Self {
        let default_rules = Self::default_rules();
        let mut compiled_patterns = HashMap::new();
        
        // Pre-compile regex patterns for performance
        for rule in &default_rules {
            if let Ok(regex) = Regex::new(&rule.pattern) {
                compiled_patterns.insert(rule.pattern.clone(), regex);
            }
        }
        
        Self {
            rules: Arc::new(RwLock::new(default_rules)),
            compiled_patterns: Arc::new(RwLock::new(compiled_patterns)),
            custom_keywords: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Get default privacy rules
    fn default_rules() -> Vec<PrivacyRule> {
        vec![
            // Password patterns
            PrivacyRule {
                pattern: r"(?i)(password|passwd|pwd)[:\s=]+\S+".to_string(),
                pattern_type: SensitivePattern::Password,
                action: SyncRecommendation::Block,
                enabled: true,
            },
            PrivacyRule {
                pattern: r"(?i)pass\s*:\s*\S+".to_string(),
                pattern_type: SensitivePattern::Password,
                action: SyncRecommendation::Block,
                enabled: true,
            },
            // Credit card patterns (Visa, MasterCard, Amex, Discover)
            PrivacyRule {
                pattern: r"\b4\d{3}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b".to_string(),
                pattern_type: SensitivePattern::CreditCard,
                action: SyncRecommendation::Block,
                enabled: true,
            },
            PrivacyRule {
                pattern: r"\b5[1-5]\d{2}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b".to_string(),
                pattern_type: SensitivePattern::CreditCard,
                action: SyncRecommendation::Block,
                enabled: true,
            },
            PrivacyRule {
                pattern: r"\b3[47]\d{2}[-\s]?\d{6}[-\s]?\d{5}\b".to_string(),
                pattern_type: SensitivePattern::CreditCard,
                action: SyncRecommendation::Block,
                enabled: true,
            },
            PrivacyRule {
                pattern: r"\b6011[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b".to_string(),
                pattern_type: SensitivePattern::CreditCard,
                action: SyncRecommendation::Block,
                enabled: true,
            },
            // Social Security Number
            PrivacyRule {
                pattern: r"\b\d{3}[-\s]?\d{2}[-\s]?\d{4}\b".to_string(),
                pattern_type: SensitivePattern::SocialSecurity,
                action: SyncRecommendation::Block,
                enabled: true,
            },
            // Email addresses
            PrivacyRule {
                pattern: r"\b[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b".to_string(),
                pattern_type: SensitivePattern::Email,
                action: SyncRecommendation::Prompt,
                enabled: true,
            },
            // API keys and tokens
            PrivacyRule {
                pattern: r#"(?i)(api[_-]?key|apikey)[:\s=]+['\"]?[a-zA-Z0-9_\-]{16,}['\"]?"#.to_string(),
                pattern_type: SensitivePattern::ApiKey,
                action: SyncRecommendation::Block,
                enabled: true,
            },
            PrivacyRule {
                pattern: r#"(?i)(access[_-]?token|bearer)[:\s=]+['\"]?[a-zA-Z0-9_\-\.]{20,}['\"]?"#.to_string(),
                pattern_type: SensitivePattern::ApiKey,
                action: SyncRecommendation::Block,
                enabled: true,
            },
            PrivacyRule {
                pattern: r#"(?i)(secret[_-]?key|client[_-]?secret)[:\s=]+['\"]?[a-zA-Z0-9_\-]{16,}['\"]?"#.to_string(),
                pattern_type: SensitivePattern::ApiKey,
                action: SyncRecommendation::Block,
                enabled: true,
            },
            // AWS keys
            PrivacyRule {
                pattern: r"(?i)AKIA[0-9A-Z]{16}".to_string(),
                pattern_type: SensitivePattern::ApiKey,
                action: SyncRecommendation::Block,
                enabled: true,
            },
            // Phone numbers (US format)
            PrivacyRule {
                pattern: r"\b\d{3}[-.\s]?\d{3}[-.\s]?\d{4}\b".to_string(),
                pattern_type: SensitivePattern::PhoneNumber,
                action: SyncRecommendation::Prompt,
                enabled: true,
            },
            PrivacyRule {
                pattern: r"\b\(\d{3}\)\s*\d{3}[-.\s]?\d{4}\b".to_string(),
                pattern_type: SensitivePattern::PhoneNumber,
                action: SyncRecommendation::Prompt,
                enabled: true,
            },
        ]
    }
    
    /// Add a custom privacy rule
    pub fn add_rule(&self, rule: PrivacyRule) -> ClipboardResult<()> {
        // Validate and compile the regex pattern
        let regex = Regex::new(&rule.pattern)
            .map_err(|e| ClipboardError::config("privacy_rule", format!("Invalid regex pattern: {}", e)))?;
        
        // Add to compiled patterns
        {
            let mut patterns = self.compiled_patterns.write()
                .map_err(|_| ClipboardError::internal("Failed to acquire write lock on patterns"))?;
            patterns.insert(rule.pattern.clone(), regex);
        }
        
        // Add to rules
        {
            let mut rules = self.rules.write()
                .map_err(|_| ClipboardError::internal("Failed to acquire write lock on rules"))?;
            rules.push(rule);
        }
        
        Ok(())
    }
    
    /// Add custom keyword for filtering
    pub fn add_custom_keyword(&self, keyword: String) -> ClipboardResult<()> {
        let mut keywords = self.custom_keywords.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on keywords"))?;
        
        if !keywords.contains(&keyword) {
            keywords.push(keyword);
        }
        
        Ok(())
    }
    
    /// Remove custom keyword
    pub fn remove_custom_keyword(&self, keyword: &str) -> ClipboardResult<()> {
        let mut keywords = self.custom_keywords.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on keywords"))?;
        
        keywords.retain(|k| k != keyword);
        Ok(())
    }
    
    /// Get all custom keywords
    pub fn get_custom_keywords(&self) -> ClipboardResult<Vec<String>> {
        let keywords = self.custom_keywords.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on keywords"))?;
        
        Ok(keywords.clone())
    }
    
    /// Enable or disable a rule by pattern type
    pub fn set_rule_enabled(&self, pattern_type: &SensitivePattern, enabled: bool) -> ClipboardResult<()> {
        let mut rules = self.rules.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on rules"))?;
        
        for rule in rules.iter_mut() {
            if &rule.pattern_type == pattern_type {
                rule.enabled = enabled;
            }
        }
        
        Ok(())
    }
    
    /// Analyze text content for sensitive patterns
    pub fn analyze_text(&self, text: &str) -> ClipboardResult<PrivacyAnalysis> {
        let mut detected_patterns = Vec::new();
        let mut max_sensitivity = 0.0f32;
        let mut pattern_matches: HashMap<SensitivePattern, usize> = HashMap::new();
        
        // Check against compiled regex patterns
        {
            let rules = self.rules.read()
                .map_err(|_| ClipboardError::internal("Failed to acquire read lock on rules"))?;
            let patterns = self.compiled_patterns.read()
                .map_err(|_| ClipboardError::internal("Failed to acquire read lock on patterns"))?;
            
            for rule in rules.iter() {
                if !rule.enabled {
                    continue;
                }
                
                if let Some(regex) = patterns.get(&rule.pattern) {
                    if regex.is_match(text) {
                        // Count matches for scoring
                        let match_count = regex.find_iter(text).count();
                        *pattern_matches.entry(rule.pattern_type.clone()).or_insert(0) += match_count;
                        
                        if !detected_patterns.contains(&rule.pattern_type) {
                            detected_patterns.push(rule.pattern_type.clone());
                        }
                        
                        // Calculate sensitivity score
                        let base_sensitivity = self.get_pattern_sensitivity(&rule.pattern_type);
                        let adjusted_sensitivity = base_sensitivity * (1.0 + (match_count as f32 * 0.1).min(0.5));
                        max_sensitivity = max_sensitivity.max(adjusted_sensitivity.min(1.0));
                    }
                }
            }
        }
        
        // Check against custom keywords
        {
            let keywords = self.custom_keywords.read()
                .map_err(|_| ClipboardError::internal("Failed to acquire read lock on keywords"))?;
            
            let text_lower = text.to_lowercase();
            for keyword in keywords.iter() {
                if text_lower.contains(&keyword.to_lowercase()) {
                    let custom_pattern = SensitivePattern::Custom(keyword.clone());
                    if !detected_patterns.contains(&custom_pattern) {
                        detected_patterns.push(custom_pattern);
                    }
                    max_sensitivity = max_sensitivity.max(0.7);
                }
            }
        }
        
        // Determine recommendation based on sensitivity score
        let recommendation = if max_sensitivity >= 0.8 {
            SyncRecommendation::Block
        } else if max_sensitivity >= 0.3 {
            SyncRecommendation::Prompt
        } else {
            SyncRecommendation::Allow
        };
        
        Ok(PrivacyAnalysis {
            sensitivity_score: max_sensitivity,
            detected_patterns,
            recommendation: recommendation.clone(),
            user_prompt_required: matches!(recommendation, SyncRecommendation::Prompt),
        })
    }
    
    /// Get sensitivity score for a pattern type
    fn get_pattern_sensitivity(&self, pattern_type: &SensitivePattern) -> f32 {
        match pattern_type {
            SensitivePattern::Password => 1.0,
            SensitivePattern::CreditCard => 1.0,
            SensitivePattern::SocialSecurity => 1.0,
            SensitivePattern::ApiKey => 0.95,
            SensitivePattern::Email => 0.3,
            SensitivePattern::PhoneNumber => 0.4,
            SensitivePattern::Custom(_) => 0.7,
        }
    }
    
    /// Calculate overall sensitivity score with multiple factors
    pub fn calculate_sensitivity_score(&self, content: &ClipboardContent) -> ClipboardResult<f32> {
        match content {
            ClipboardContent::Text(text_content) => {
                let analysis = self.analyze_text(&text_content.text)?;
                Ok(analysis.sensitivity_score)
            }
            ClipboardContent::Image(_) => {
                // Images are generally safe unless OCR is performed
                Ok(0.0)
            }
            ClipboardContent::Files(files) => {
                // Check file paths for sensitive patterns
                let combined_paths = files.join(" ");
                let analysis = self.analyze_text(&combined_paths)?;
                // Reduce sensitivity for file paths
                Ok(analysis.sensitivity_score * 0.5)
            }
            ClipboardContent::Custom { .. } => {
                // Unknown content types are treated with moderate caution
                Ok(0.5)
            }
        }
    }
}

impl Default for SensitiveContentDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Default privacy filter implementation
pub struct DefaultPrivacyFilter {
    detector: Arc<SensitiveContentDetector>,
    content_blacklist: Arc<RwLock<Vec<String>>>,
}

impl DefaultPrivacyFilter {
    /// Create new privacy filter with default rules
    pub fn new() -> Self {
        Self {
            detector: Arc::new(SensitiveContentDetector::new()),
            content_blacklist: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Create with custom detector
    pub fn with_detector(detector: SensitiveContentDetector) -> Self {
        Self {
            detector: Arc::new(detector),
            content_blacklist: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Get reference to the detector
    pub fn detector(&self) -> &SensitiveContentDetector {
        &self.detector
    }
    
    /// Add content type to blacklist
    pub fn add_to_blacklist(&self, content_type: String) -> ClipboardResult<()> {
        let mut blacklist = self.content_blacklist.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on blacklist"))?;
        
        if !blacklist.contains(&content_type) {
            blacklist.push(content_type);
        }
        
        Ok(())
    }
    
    /// Remove content type from blacklist
    pub fn remove_from_blacklist(&self, content_type: &str) -> ClipboardResult<()> {
        let mut blacklist = self.content_blacklist.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on blacklist"))?;
        
        blacklist.retain(|ct| ct != content_type);
        Ok(())
    }
    
    /// Check if content type is blacklisted
    pub fn is_blacklisted(&self, content_type: &str) -> ClipboardResult<bool> {
        let blacklist = self.content_blacklist.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on blacklist"))?;
        
        Ok(blacklist.iter().any(|ct| ct == content_type))
    }
    
    /// Get all blacklisted content types
    pub fn get_blacklist(&self) -> ClipboardResult<Vec<String>> {
        let blacklist = self.content_blacklist.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on blacklist"))?;
        
        Ok(blacklist.clone())
    }
}

#[async_trait]
impl PrivacyFilter for DefaultPrivacyFilter {
    async fn analyze_content(&self, content: &ClipboardContent) -> ClipboardResult<PrivacyAnalysis> {
        // Check if content type is blacklisted
        let content_type_str = match content {
            ClipboardContent::Text(_) => "text",
            ClipboardContent::Image(_) => "image",
            ClipboardContent::Files(_) => "files",
            ClipboardContent::Custom { mime_type, .. } => mime_type.as_str(),
        };
        
        if self.is_blacklisted(content_type_str)? {
            return Ok(PrivacyAnalysis {
                sensitivity_score: 1.0,
                detected_patterns: vec![],
                recommendation: SyncRecommendation::Block,
                user_prompt_required: false,
            });
        }
        
        // Perform content analysis
        match content {
            ClipboardContent::Text(text_content) => {
                self.detector.analyze_text(&text_content.text)
            }
            ClipboardContent::Image(_) => {
                // Images are generally safe unless they contain text (OCR would be needed)
                Ok(PrivacyAnalysis {
                    sensitivity_score: 0.0,
                    detected_patterns: vec![],
                    recommendation: SyncRecommendation::Allow,
                    user_prompt_required: false,
                })
            }
            ClipboardContent::Files(files) => {
                // Analyze file paths for sensitive information
                let combined_paths = files.join(" ");
                let mut analysis = self.detector.analyze_text(&combined_paths)?;
                
                // Reduce sensitivity for file paths
                analysis.sensitivity_score *= 0.5;
                
                // Always prompt for file transfers
                if analysis.recommendation == SyncRecommendation::Allow {
                    analysis.recommendation = SyncRecommendation::Prompt;
                    analysis.user_prompt_required = true;
                }
                
                Ok(analysis)
            }
            ClipboardContent::Custom { .. } => {
                // Unknown content types are treated with caution
                Ok(PrivacyAnalysis {
                    sensitivity_score: 0.5,
                    detected_patterns: vec![],
                    recommendation: SyncRecommendation::Prompt,
                    user_prompt_required: true,
                })
            }
        }
    }
    
    async fn should_sync_content(&self, content: &ClipboardContent) -> ClipboardResult<SyncRecommendation> {
        let analysis = self.analyze_content(content).await?;
        Ok(analysis.recommendation)
    }
    
    async fn add_privacy_rule(&self, rule: PrivacyRule) -> ClipboardResult<()> {
        self.detector.add_rule(rule)
    }
    
    async fn prompt_user_for_sensitive_content(&self, _content: &ClipboardContent) -> ClipboardResult<UserDecision> {
        // This should be implemented by the UI layer
        // For now, return Block as the safe default
        Ok(UserDecision::Block)
    }
}

impl Default for DefaultPrivacyFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Privacy policy configuration
#[derive(Debug, Clone)]
pub struct PrivacyPolicy {
    /// Enable privacy filtering
    pub enabled: bool,
    /// Automatically block high-sensitivity content
    pub auto_block_sensitive: bool,
    /// Prompt user for medium-sensitivity content
    pub prompt_on_medium_sensitivity: bool,
    /// Minimum sensitivity score to trigger prompt (0.0 to 1.0)
    pub prompt_threshold: f32,
    /// Minimum sensitivity score to auto-block (0.0 to 1.0)
    pub block_threshold: f32,
    /// Remember user decisions for similar content
    pub remember_decisions: bool,
    /// Content types to never sync
    pub blacklisted_types: Vec<String>,
    /// Custom keywords to flag
    pub custom_keywords: Vec<String>,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_block_sensitive: true,
            prompt_on_medium_sensitivity: true,
            prompt_threshold: 0.3,
            block_threshold: 0.8,
            remember_decisions: true,
            blacklisted_types: Vec::new(),
            custom_keywords: Vec::new(),
        }
    }
}

impl PrivacyPolicy {
    /// Create a strict privacy policy
    pub fn strict() -> Self {
        Self {
            enabled: true,
            auto_block_sensitive: true,
            prompt_on_medium_sensitivity: true,
            prompt_threshold: 0.2,
            block_threshold: 0.6,
            remember_decisions: false,
            blacklisted_types: Vec::new(),
            custom_keywords: Vec::new(),
        }
    }
    
    /// Create a permissive privacy policy
    pub fn permissive() -> Self {
        Self {
            enabled: true,
            auto_block_sensitive: false,
            prompt_on_medium_sensitivity: false,
            prompt_threshold: 0.7,
            block_threshold: 0.95,
            remember_decisions: true,
            blacklisted_types: Vec::new(),
            custom_keywords: Vec::new(),
        }
    }
    
    /// Validate policy configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.prompt_threshold < 0.0 || self.prompt_threshold > 1.0 {
            return Err("Prompt threshold must be between 0.0 and 1.0".to_string());
        }
        
        if self.block_threshold < 0.0 || self.block_threshold > 1.0 {
            return Err("Block threshold must be between 0.0 and 1.0".to_string());
        }
        
        if self.prompt_threshold >= self.block_threshold {
            return Err("Prompt threshold must be less than block threshold".to_string());
        }
        
        Ok(())
    }
    
    /// Determine action based on sensitivity score
    pub fn determine_action(&self, sensitivity_score: f32) -> SyncRecommendation {
        if !self.enabled {
            return SyncRecommendation::Allow;
        }
        
        if sensitivity_score >= self.block_threshold && self.auto_block_sensitive {
            SyncRecommendation::Block
        } else if sensitivity_score >= self.prompt_threshold && self.prompt_on_medium_sensitivity {
            SyncRecommendation::Prompt
        } else {
            SyncRecommendation::Allow
        }
    }
}

/// User prompt manager for handling sensitive content decisions
pub struct UserPromptManager {
    /// Callback function for prompting user
    prompt_callback: Option<Arc<dyn Fn(&ClipboardContent, &PrivacyAnalysis) -> UserDecision + Send + Sync>>,
    /// Remembered user decisions (content hash -> decision)
    remembered_decisions: Arc<RwLock<HashMap<String, UserDecision>>>,
    /// Whether to remember decisions
    remember_enabled: bool,
}

impl UserPromptManager {
    /// Create new prompt manager
    pub fn new() -> Self {
        Self {
            prompt_callback: None,
            remembered_decisions: Arc::new(RwLock::new(HashMap::new())),
            remember_enabled: true,
        }
    }
    
    /// Set the prompt callback function
    pub fn set_prompt_callback<F>(&mut self, callback: F)
    where
        F: Fn(&ClipboardContent, &PrivacyAnalysis) -> UserDecision + Send + Sync + 'static,
    {
        self.prompt_callback = Some(Arc::new(callback));
    }
    
    /// Enable or disable remembering decisions
    pub fn set_remember_enabled(&mut self, enabled: bool) {
        self.remember_enabled = enabled;
    }
    
    /// Prompt user for decision on sensitive content
    pub fn prompt_user(
        &self,
        content: &ClipboardContent,
        analysis: &PrivacyAnalysis,
    ) -> ClipboardResult<UserDecision> {
        // Check if we have a remembered decision
        if self.remember_enabled {
            let content_hash = self.hash_content(content);
            let decisions = self.remembered_decisions.read()
                .map_err(|_| ClipboardError::internal("Failed to acquire read lock on decisions"))?;
            
            if let Some(decision) = decisions.get(&content_hash) {
                return Ok(decision.clone());
            }
        }
        
        // Call the prompt callback if available
        let decision = if let Some(callback) = &self.prompt_callback {
            callback(content, analysis)
        } else {
            // Default to blocking if no callback is set
            UserDecision::Block
        };
        
        // Remember the decision if it's an "always" decision
        if self.remember_enabled && matches!(decision, UserDecision::AlwaysAllow | UserDecision::AlwaysBlock) {
            let content_hash = self.hash_content(content);
            let mut decisions = self.remembered_decisions.write()
                .map_err(|_| ClipboardError::internal("Failed to acquire write lock on decisions"))?;
            decisions.insert(content_hash, decision.clone());
        }
        
        Ok(decision)
    }
    
    /// Clear all remembered decisions
    pub fn clear_remembered_decisions(&self) -> ClipboardResult<()> {
        let mut decisions = self.remembered_decisions.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on decisions"))?;
        decisions.clear();
        Ok(())
    }
    
    /// Get count of remembered decisions
    pub fn remembered_decision_count(&self) -> ClipboardResult<usize> {
        let decisions = self.remembered_decisions.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on decisions"))?;
        Ok(decisions.len())
    }
    
    /// Hash content for decision caching
    fn hash_content(&self, content: &ClipboardContent) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        match content {
            ClipboardContent::Text(text) => {
                text.text.hash(&mut hasher);
            }
            ClipboardContent::Image(image) => {
                image.data.hash(&mut hasher);
            }
            ClipboardContent::Files(files) => {
                files.hash(&mut hasher);
            }
            ClipboardContent::Custom { mime_type, data } => {
                mime_type.hash(&mut hasher);
                data.hash(&mut hasher);
            }
        }
        
        format!("{:x}", hasher.finish())
    }
}

impl Default for UserPromptManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Privacy policy manager with configuration and user prompting
pub struct PrivacyPolicyManager {
    policy: Arc<RwLock<PrivacyPolicy>>,
    filter: Arc<DefaultPrivacyFilter>,
    prompt_manager: Arc<RwLock<UserPromptManager>>,
}

impl PrivacyPolicyManager {
    /// Create new policy manager with default policy
    pub fn new() -> Self {
        Self {
            policy: Arc::new(RwLock::new(PrivacyPolicy::default())),
            filter: Arc::new(DefaultPrivacyFilter::new()),
            prompt_manager: Arc::new(RwLock::new(UserPromptManager::new())),
        }
    }
    
    /// Create with custom policy
    pub fn with_policy(policy: PrivacyPolicy) -> ClipboardResult<Self> {
        policy.validate()
            .map_err(|e| ClipboardError::config("privacy_policy", e))?;
        
        Ok(Self {
            policy: Arc::new(RwLock::new(policy)),
            filter: Arc::new(DefaultPrivacyFilter::new()),
            prompt_manager: Arc::new(RwLock::new(UserPromptManager::new())),
        })
    }
    
    /// Get current policy
    pub fn get_policy(&self) -> ClipboardResult<PrivacyPolicy> {
        let policy = self.policy.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on policy"))?;
        Ok(policy.clone())
    }
    
    /// Update policy
    pub fn update_policy(&self, policy: PrivacyPolicy) -> ClipboardResult<()> {
        policy.validate()
            .map_err(|e| ClipboardError::config("privacy_policy", e))?;
        
        let mut current_policy = self.policy.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on policy"))?;
        
        // Update filter blacklist
        for content_type in &policy.blacklisted_types {
            self.filter.add_to_blacklist(content_type.clone())?;
        }
        
        // Update detector keywords
        for keyword in &policy.custom_keywords {
            self.filter.detector().add_custom_keyword(keyword.clone())?;
        }
        
        *current_policy = policy;
        Ok(())
    }
    
    /// Add content type to blacklist
    pub fn add_to_blacklist(&self, content_type: String) -> ClipboardResult<()> {
        let mut policy = self.policy.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on policy"))?;
        
        if !policy.blacklisted_types.contains(&content_type) {
            policy.blacklisted_types.push(content_type.clone());
            self.filter.add_to_blacklist(content_type)?;
        }
        
        Ok(())
    }
    
    /// Remove content type from blacklist
    pub fn remove_from_blacklist(&self, content_type: &str) -> ClipboardResult<()> {
        let mut policy = self.policy.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on policy"))?;
        
        policy.blacklisted_types.retain(|ct| ct != content_type);
        self.filter.remove_from_blacklist(content_type)?;
        
        Ok(())
    }
    
    /// Add custom keyword
    pub fn add_custom_keyword(&self, keyword: String) -> ClipboardResult<()> {
        let mut policy = self.policy.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on policy"))?;
        
        if !policy.custom_keywords.contains(&keyword) {
            policy.custom_keywords.push(keyword.clone());
            self.filter.detector().add_custom_keyword(keyword)?;
        }
        
        Ok(())
    }
    
    /// Remove custom keyword
    pub fn remove_custom_keyword(&self, keyword: &str) -> ClipboardResult<()> {
        let mut policy = self.policy.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on policy"))?;
        
        policy.custom_keywords.retain(|k| k != keyword);
        self.filter.detector().remove_custom_keyword(keyword)?;
        
        Ok(())
    }
    
    /// Set user prompt callback
    pub fn set_prompt_callback<F>(&self, callback: F) -> ClipboardResult<()>
    where
        F: Fn(&ClipboardContent, &PrivacyAnalysis) -> UserDecision + Send + Sync + 'static,
    {
        let mut prompt_manager = self.prompt_manager.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on prompt manager"))?;
        
        prompt_manager.set_prompt_callback(callback);
        Ok(())
    }
    
    /// Analyze content and determine if it should be synced
    pub async fn should_sync_content(&self, content: &ClipboardContent) -> ClipboardResult<SyncDecision> {
        let policy = self.get_policy()?;
        
        if !policy.enabled {
            return Ok(SyncDecision::Allow);
        }
        
        // Analyze content
        let analysis = self.filter.analyze_content(content).await?;
        
        // Determine action based on policy
        let recommendation = policy.determine_action(analysis.sensitivity_score);
        
        match recommendation {
            SyncRecommendation::Allow => Ok(SyncDecision::Allow),
            SyncRecommendation::Block => Ok(SyncDecision::Block {
                reason: format!("Content blocked due to sensitivity score: {:.2}", analysis.sensitivity_score),
                patterns: analysis.detected_patterns,
            }),
            SyncRecommendation::Prompt => {
                // Prompt user for decision
                let prompt_manager = self.prompt_manager.read()
                    .map_err(|_| ClipboardError::internal("Failed to acquire read lock on prompt manager"))?;
                
                let user_decision = prompt_manager.prompt_user(content, &analysis)?;
                
                match user_decision {
                    UserDecision::Allow | UserDecision::AlwaysAllow => Ok(SyncDecision::Allow),
                    UserDecision::Block | UserDecision::AlwaysBlock => Ok(SyncDecision::Block {
                        reason: "User blocked content".to_string(),
                        patterns: analysis.detected_patterns,
                    }),
                }
            }
        }
    }
    
    /// Clear all remembered user decisions
    pub fn clear_remembered_decisions(&self) -> ClipboardResult<()> {
        let prompt_manager = self.prompt_manager.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on prompt manager"))?;
        
        prompt_manager.clear_remembered_decisions()
    }
}

impl Default for PrivacyPolicyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Sync decision result
#[derive(Debug, Clone)]
pub enum SyncDecision {
    /// Allow sync to proceed
    Allow,
    /// Block sync with reason
    Block {
        reason: String,
        patterns: Vec<SensitivePattern>,
    },
}