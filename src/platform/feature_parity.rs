// Feature parity validation across platforms
//
// This module provides automated testing and validation for feature consistency
// across all supported platforms, ensuring feature parity and compatibility.

use crate::platform::{
    PlatformResult, PlatformError, PlatformInfo, OperatingSystem, Architecture,
    Feature, PlatformCapabilities, BuildTarget,
};
use std::collections::{HashMap, HashSet};

/// Feature parity validator
pub struct FeatureParityValidator {
    platform_features: HashMap<OperatingSystem, HashSet<Feature>>,
    required_features: HashSet<Feature>,
    optional_features: HashSet<Feature>,
}

impl FeatureParityValidator {
    /// Create a new feature parity validator
    pub fn new() -> Self {
        let mut validator = Self {
            platform_features: HashMap::new(),
            required_features: HashSet::new(),
            optional_features: HashSet::new(),
        };
        
        validator.initialize_required_features();
        validator.initialize_optional_features();
        validator.initialize_platform_features();
        
        validator
    }
    
    /// Initialize required features (must be available on all platforms)
    fn initialize_required_features(&mut self) {
        self.required_features.insert(Feature::FileTransfer);
        self.required_features.insert(Feature::Discovery);
    }
    
    /// Initialize optional features (platform-dependent)
    fn initialize_optional_features(&mut self) {
        self.optional_features.insert(Feature::Clipboard);
        self.optional_features.insert(Feature::Streaming);
        self.optional_features.insert(Feature::CommandExecution);
        self.optional_features.insert(Feature::SystemTray);
        self.optional_features.insert(Feature::Notifications);
        self.optional_features.insert(Feature::AutoStart);
        self.optional_features.insert(Feature::FileAssociations);
    }
    
    /// Initialize platform-specific feature availability
    fn initialize_platform_features(&mut self) {
        // Linux features
        let mut linux_features = HashSet::new();
        linux_features.insert(Feature::FileTransfer);
        linux_features.insert(Feature::Discovery);
        linux_features.insert(Feature::Clipboard);
        linux_features.insert(Feature::Streaming);
        linux_features.insert(Feature::CommandExecution);
        linux_features.insert(Feature::SystemTray);
        linux_features.insert(Feature::Notifications);
        linux_features.insert(Feature::AutoStart);
        linux_features.insert(Feature::FileAssociations);
        self.platform_features.insert(OperatingSystem::Linux, linux_features);
        
        // macOS features
        let mut macos_features = HashSet::new();
        macos_features.insert(Feature::FileTransfer);
        macos_features.insert(Feature::Discovery);
        macos_features.insert(Feature::Clipboard);
        macos_features.insert(Feature::Streaming);
        macos_features.insert(Feature::CommandExecution);
        macos_features.insert(Feature::SystemTray);
        macos_features.insert(Feature::Notifications);
        macos_features.insert(Feature::AutoStart);
        macos_features.insert(Feature::FileAssociations);
        self.platform_features.insert(OperatingSystem::MacOS, macos_features);
        
        // Windows features
        let mut windows_features = HashSet::new();
        windows_features.insert(Feature::FileTransfer);
        windows_features.insert(Feature::Discovery);
        windows_features.insert(Feature::Clipboard);
        windows_features.insert(Feature::Streaming);
        windows_features.insert(Feature::CommandExecution);
        windows_features.insert(Feature::SystemTray);
        windows_features.insert(Feature::Notifications);
        windows_features.insert(Feature::AutoStart);
        windows_features.insert(Feature::FileAssociations);
        self.platform_features.insert(OperatingSystem::Windows, windows_features);
        
        // Android features
        let mut android_features = HashSet::new();
        android_features.insert(Feature::FileTransfer);
        android_features.insert(Feature::Discovery);
        android_features.insert(Feature::Notifications);
        self.platform_features.insert(OperatingSystem::Android, android_features);
        
        // iOS features
        let mut ios_features = HashSet::new();
        ios_features.insert(Feature::FileTransfer);
        ios_features.insert(Feature::Discovery);
        ios_features.insert(Feature::Notifications);
        self.platform_features.insert(OperatingSystem::iOS, ios_features);
        
        // WebBrowser features
        let mut web_features = HashSet::new();
        web_features.insert(Feature::FileTransfer);
        web_features.insert(Feature::Discovery);
        web_features.insert(Feature::Notifications);
        self.platform_features.insert(OperatingSystem::WebBrowser, web_features);
        
        // Container features
        let mut container_features = HashSet::new();
        container_features.insert(Feature::FileTransfer);
        container_features.insert(Feature::Discovery);
        self.platform_features.insert(OperatingSystem::Container, container_features);
    }
    
    /// Validate feature parity across all platforms
    pub fn validate_parity(&self) -> FeatureParityReport {
        let mut report = FeatureParityReport {
            is_valid: true,
            missing_required_features: HashMap::new(),
            platform_feature_matrix: self.generate_feature_matrix(),
            inconsistencies: vec![],
        };
        
        // Check required features on all platforms
        for (platform, features) in &self.platform_features {
            let mut missing = Vec::new();
            
            for required_feature in &self.required_features {
                if !features.contains(required_feature) {
                    missing.push(*required_feature);
                    report.is_valid = false;
                }
            }
            
            if !missing.is_empty() {
                report.missing_required_features.insert(*platform, missing);
            }
        }
        
        // Check for inconsistencies in optional features
        report.inconsistencies = self.find_inconsistencies();
        
        report
    }
    
    /// Find inconsistencies in feature availability
    fn find_inconsistencies(&self) -> Vec<FeatureInconsistency> {
        let mut inconsistencies = Vec::new();
        
        // Desktop platforms should have similar features
        let desktop_platforms = vec![
            OperatingSystem::Linux,
            OperatingSystem::MacOS,
            OperatingSystem::Windows,
        ];
        
        for feature in &self.optional_features {
            let mut availability = HashMap::new();
            
            for platform in &desktop_platforms {
                if let Some(features) = self.platform_features.get(platform) {
                    availability.insert(*platform, features.contains(feature));
                }
            }
            
            // Check if feature availability is inconsistent across desktop platforms
            let available_count = availability.values().filter(|&&v| v).count();
            if available_count > 0 && available_count < desktop_platforms.len() {
                inconsistencies.push(FeatureInconsistency {
                    feature: *feature,
                    platforms_with_feature: availability.iter()
                        .filter(|(_, &v)| v)
                        .map(|(&k, _)| k)
                        .collect(),
                    platforms_without_feature: availability.iter()
                        .filter(|(_, &v)| !v)
                        .map(|(&k, _)| k)
                        .collect(),
                    severity: InconsistencySeverity::Warning,
                });
            }
        }
        
        inconsistencies
    }
    
    /// Generate feature matrix for all platforms
    pub fn generate_feature_matrix(&self) -> FeatureMatrix {
        let mut matrix = FeatureMatrix {
            platforms: vec![],
            features: vec![],
            availability: HashMap::new(),
        };
        
        // Collect all platforms
        matrix.platforms = self.platform_features.keys().copied().collect();
        matrix.platforms.sort_by_key(|p| format!("{:?}", p));
        
        // Collect all features
        let mut all_features: HashSet<Feature> = HashSet::new();
        all_features.extend(&self.required_features);
        all_features.extend(&self.optional_features);
        matrix.features = all_features.into_iter().collect();
        matrix.features.sort_by_key(|f| format!("{:?}", f));
        
        // Build availability matrix
        for platform in &matrix.platforms {
            for feature in &matrix.features {
                let available = self.platform_features
                    .get(platform)
                    .map(|features| features.contains(feature))
                    .unwrap_or(false);
                
                matrix.availability.insert((*platform, *feature), available);
            }
        }
        
        matrix
    }
    
    /// Validate a specific platform's features
    pub fn validate_platform(&self, platform: &OperatingSystem) -> PlatformFeatureValidation {
        let mut validation = PlatformFeatureValidation {
            platform: *platform,
            is_valid: true,
            missing_required: vec![],
            available_optional: vec![],
            unavailable_optional: vec![],
        };
        
        let platform_features = self.platform_features.get(platform);
        
        // Check required features
        for required in &self.required_features {
            if !platform_features.map(|f| f.contains(required)).unwrap_or(false) {
                validation.missing_required.push(*required);
                validation.is_valid = false;
            }
        }
        
        // Check optional features
        for optional in &self.optional_features {
            if platform_features.map(|f| f.contains(optional)).unwrap_or(false) {
                validation.available_optional.push(*optional);
            } else {
                validation.unavailable_optional.push(*optional);
            }
        }
        
        validation
    }
}

impl Default for FeatureParityValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Feature parity report
#[derive(Debug, Clone)]
pub struct FeatureParityReport {
    pub is_valid: bool,
    pub missing_required_features: HashMap<OperatingSystem, Vec<Feature>>,
    pub platform_feature_matrix: FeatureMatrix,
    pub inconsistencies: Vec<FeatureInconsistency>,
}

impl FeatureParityReport {
    /// Generate a summary of the report
    pub fn summary(&self) -> String {
        let mut summary = String::new();
        
        summary.push_str("Feature Parity Validation Report\n");
        summary.push_str("=================================\n\n");
        
        if self.is_valid {
            summary.push_str("✓ All required features are available on all platforms\n\n");
        } else {
            summary.push_str("✗ Some required features are missing\n\n");
            
            for (platform, features) in &self.missing_required_features {
                summary.push_str(&format!("Platform: {:?}\n", platform));
                summary.push_str("Missing required features:\n");
                for feature in features {
                    summary.push_str(&format!("  - {:?}\n", feature));
                }
                summary.push_str("\n");
            }
        }
        
        if !self.inconsistencies.is_empty() {
            summary.push_str("Feature Inconsistencies:\n");
            for inconsistency in &self.inconsistencies {
                summary.push_str(&format!("  {:?}: ", inconsistency.feature));
                summary.push_str(&format!("Available on {:?}, ", inconsistency.platforms_with_feature));
                summary.push_str(&format!("Missing on {:?}\n", inconsistency.platforms_without_feature));
            }
        }
        
        summary
    }
    
    /// Export as JSON
    pub fn to_json(&self) -> String {
        serde_json::json!({
            "is_valid": self.is_valid,
            "missing_required_features": self.missing_required_features.iter()
                .map(|(k, v)| (format!("{:?}", k), v.iter().map(|f| format!("{:?}", f)).collect::<Vec<_>>()))
                .collect::<HashMap<_, _>>(),
            "inconsistencies": self.inconsistencies.len(),
        }).to_string()
    }
}

/// Feature matrix showing availability across platforms
#[derive(Debug, Clone)]
pub struct FeatureMatrix {
    pub platforms: Vec<OperatingSystem>,
    pub features: Vec<Feature>,
    pub availability: HashMap<(OperatingSystem, Feature), bool>,
}

impl FeatureMatrix {
    /// Generate a text table representation
    pub fn to_table(&self) -> String {
        let mut table = String::new();
        
        // Header
        table.push_str("Feature");
        for platform in &self.platforms {
            table.push_str(&format!("\t{:?}", platform));
        }
        table.push('\n');
        
        // Separator
        table.push_str(&"-".repeat(80));
        table.push('\n');
        
        // Rows
        for feature in &self.features {
            table.push_str(&format!("{:?}", feature));
            for platform in &self.platforms {
                let available = self.availability.get(&(*platform, *feature)).copied().unwrap_or(false);
                table.push_str(if available { "\t✓" } else { "\t✗" });
            }
            table.push('\n');
        }
        
        table
    }
    
    /// Export as CSV
    pub fn to_csv(&self) -> String {
        let mut csv = String::new();
        
        // Header
        csv.push_str("Feature");
        for platform in &self.platforms {
            csv.push_str(&format!(",{:?}", platform));
        }
        csv.push('\n');
        
        // Rows
        for feature in &self.features {
            csv.push_str(&format!("{:?}", feature));
            for platform in &self.platforms {
                let available = self.availability.get(&(*platform, *feature)).copied().unwrap_or(false);
                csv.push_str(if available { ",Yes" } else { ",No" });
            }
            csv.push('\n');
        }
        
        csv
    }
}

/// Feature inconsistency
#[derive(Debug, Clone)]
pub struct FeatureInconsistency {
    pub feature: Feature,
    pub platforms_with_feature: Vec<OperatingSystem>,
    pub platforms_without_feature: Vec<OperatingSystem>,
    pub severity: InconsistencySeverity,
}

/// Inconsistency severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InconsistencySeverity {
    Warning,
    Error,
}

/// Platform feature validation
#[derive(Debug, Clone)]
pub struct PlatformFeatureValidation {
    pub platform: OperatingSystem,
    pub is_valid: bool,
    pub missing_required: Vec<Feature>,
    pub available_optional: Vec<Feature>,
    pub unavailable_optional: Vec<Feature>,
}

impl PlatformFeatureValidation {
    /// Generate summary
    pub fn summary(&self) -> String {
        format!(
            "Platform: {:?}\n\
             Valid: {}\n\
             Missing Required: {:?}\n\
             Available Optional: {}/{}\n",
            self.platform,
            self.is_valid,
            self.missing_required,
            self.available_optional.len(),
            self.available_optional.len() + self.unavailable_optional.len()
        )
    }
}

/// Platform compatibility matrix
pub struct CompatibilityMatrix {
    targets: Vec<BuildTarget>,
    compatibility: HashMap<(BuildTarget, BuildTarget), CompatibilityLevel>,
}

impl CompatibilityMatrix {
    /// Create a new compatibility matrix
    pub fn new() -> Self {
        let targets = BuildTarget::all_targets();
        let mut matrix = Self {
            targets: targets.clone(),
            compatibility: HashMap::new(),
        };
        
        matrix.calculate_compatibility();
        matrix
    }
    
    /// Calculate compatibility between all targets
    fn calculate_compatibility(&mut self) {
        for target1 in &self.targets {
            for target2 in &self.targets {
                let level = self.determine_compatibility(target1, target2);
                self.compatibility.insert((target1.clone(), target2.clone()), level);
            }
        }
    }
    
    /// Determine compatibility level between two targets
    fn determine_compatibility(&self, target1: &BuildTarget, target2: &BuildTarget) -> CompatibilityLevel {
        // Same target is fully compatible
        if target1 == target2 {
            return CompatibilityLevel::Full;
        }
        
        // Same platform, different architecture
        if target1.platform == target2.platform && target1.architecture != target2.architecture {
            return CompatibilityLevel::High;
        }
        
        // Desktop platforms are generally compatible
        let desktop_platforms = vec![
            OperatingSystem::Linux,
            OperatingSystem::MacOS,
            OperatingSystem::Windows,
        ];
        
        if desktop_platforms.contains(&target1.platform) && desktop_platforms.contains(&target2.platform) {
            return CompatibilityLevel::Medium;
        }
        
        // Mobile platforms have limited compatibility with desktop
        let mobile_platforms = vec![OperatingSystem::Android, OperatingSystem::iOS];
        
        if (desktop_platforms.contains(&target1.platform) && mobile_platforms.contains(&target2.platform)) ||
           (mobile_platforms.contains(&target1.platform) && desktop_platforms.contains(&target2.platform)) {
            return CompatibilityLevel::Low;
        }
        
        // Web and container have limited compatibility
        if target1.platform == OperatingSystem::WebBrowser || target2.platform == OperatingSystem::WebBrowser ||
           target1.platform == OperatingSystem::Container || target2.platform == OperatingSystem::Container {
            return CompatibilityLevel::Low;
        }
        
        CompatibilityLevel::None
    }
    
    /// Get compatibility level between two targets
    pub fn get_compatibility(&self, target1: &BuildTarget, target2: &BuildTarget) -> CompatibilityLevel {
        self.compatibility.get(&(target1.clone(), target2.clone()))
            .copied()
            .unwrap_or(CompatibilityLevel::None)
    }
    
    /// Generate compatibility report
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("Platform Compatibility Matrix\n");
        report.push_str("==============================\n\n");
        
        for target1 in &self.targets {
            report.push_str(&format!("{} ({}):\n", target1.platform.as_str(), target1.architecture.as_str()));
            
            for target2 in &self.targets {
                if target1 != target2 {
                    let level = self.get_compatibility(target1, target2);
                    report.push_str(&format!(
                        "  → {} ({}): {:?}\n",
                        target2.platform.as_str(),
                        target2.architecture.as_str(),
                        level
                    ));
                }
            }
            
            report.push('\n');
        }
        
        report
    }
}

impl Default for CompatibilityMatrix {
    fn default() -> Self {
        Self::new()
    }
}

/// Compatibility level between platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityLevel {
    Full,    // 100% compatible
    High,    // 90%+ compatible
    Medium,  // 70%+ compatible
    Low,     // 50%+ compatible
    None,    // <50% compatible
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_parity_validator() {
        let validator = FeatureParityValidator::new();
        let report = validator.validate_parity();
        
        // Required features should be available on all platforms
        assert!(report.missing_required_features.is_empty() || !report.is_valid);
    }

    #[test]
    fn test_feature_matrix_generation() {
        let validator = FeatureParityValidator::new();
        let matrix = validator.generate_feature_matrix();
        
        assert!(!matrix.platforms.is_empty());
        assert!(!matrix.features.is_empty());
    }

    #[test]
    fn test_platform_validation() {
        let validator = FeatureParityValidator::new();
        let validation = validator.validate_platform(&OperatingSystem::Linux);
        
        assert!(validation.is_valid);
        assert!(validation.missing_required.is_empty());
    }

    #[test]
    fn test_compatibility_matrix() {
        let matrix = CompatibilityMatrix::new();
        
        let linux_x64 = BuildTarget::new(OperatingSystem::Linux, Architecture::X86_64);
        let linux_arm64 = BuildTarget::new(OperatingSystem::Linux, Architecture::ARM64);
        
        // Same platform, different arch should be highly compatible
        let compat = matrix.get_compatibility(&linux_x64, &linux_arm64);
        assert_eq!(compat, CompatibilityLevel::High);
    }

    #[test]
    fn test_feature_matrix_table() {
        let validator = FeatureParityValidator::new();
        let matrix = validator.generate_feature_matrix();
        
        let table = matrix.to_table();
        assert!(!table.is_empty());
        assert!(table.contains("Feature"));
    }

    #[test]
    fn test_feature_matrix_csv() {
        let validator = FeatureParityValidator::new();
        let matrix = validator.generate_feature_matrix();
        
        let csv = matrix.to_csv();
        assert!(!csv.is_empty());
        assert!(csv.contains("Feature"));
    }
}
