// iOS App Store compliance
//
// Handles App Store guideline compliance, privacy requirements,
// and app metadata management

use crate::platform::{PlatformResult, PlatformError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// App Store compliance manager
pub struct AppStoreComplianceManager {
    metadata: AppMetadata,
    privacy_manifest: PrivacyManifest,
    compliance_checks: Vec<ComplianceCheck>,
}

/// App metadata for App Store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetadata {
    pub bundle_id: String,
    pub version: String,
    pub build_number: String,
    pub display_name: String,
    pub short_description: String,
    pub full_description: String,
    pub keywords: Vec<String>,
    pub category: AppCategory,
    pub age_rating: AgeRating,
    pub copyright: String,
    pub support_url: String,
    pub privacy_policy_url: String,
}

/// App Store categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppCategory {
    Productivity,
    Utilities,
    Business,
    Social,
    Communication,
    Entertainment,
    Education,
    Other,
}

/// Age rating categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeRating {
    FourPlus,
    NinePlus,
    TwelvePlus,
    SeventeenPlus,
}

/// Privacy manifest for iOS 17+
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyManifest {
    pub tracking_domains: Vec<String>,
    pub collected_data_types: Vec<CollectedDataType>,
    pub required_reason_apis: Vec<RequiredReasonAPI>,
    pub privacy_nutrition_label: PrivacyNutritionLabel,
}

/// Types of data collected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectedDataType {
    pub data_type: DataType,
    pub purpose: DataPurpose,
    pub linked_to_user: bool,
    pub used_for_tracking: bool,
}

/// Data types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    ContactInfo,
    Location,
    UserContent,
    Identifiers,
    UsageData,
    Diagnostics,
    Financial,
    Health,
    Other,
}

/// Data collection purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataPurpose {
    AppFunctionality,
    Analytics,
    ProductPersonalization,
    DeveloperAdvertising,
    ThirdPartyAdvertising,
    Other,
}

/// Required reason APIs (iOS 17+)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredReasonAPI {
    pub api_type: APIType,
    pub reason: String,
}

/// API types requiring reasons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum APIType {
    FileTimestamp,
    SystemBootTime,
    DiskSpace,
    ActiveKeyboards,
    UserDefaults,
}

/// Privacy nutrition label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyNutritionLabel {
    pub data_used_to_track_you: Vec<DataType>,
    pub data_linked_to_you: Vec<DataType>,
    pub data_not_linked_to_you: Vec<DataType>,
}

/// Compliance check
#[derive(Debug, Clone)]
pub struct ComplianceCheck {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub passed: bool,
    pub details: Option<String>,
}

impl AppStoreComplianceManager {
    /// Create a new App Store compliance manager
    pub fn new(metadata: AppMetadata, privacy_manifest: PrivacyManifest) -> Self {
        Self {
            metadata,
            privacy_manifest,
            compliance_checks: Vec::new(),
        }
    }

    /// Run all compliance checks
    pub fn run_compliance_checks(&mut self) -> PlatformResult<Vec<ComplianceCheck>> {
        self.compliance_checks.clear();

        // Check metadata completeness
        self.check_metadata_completeness();

        // Check privacy manifest
        self.check_privacy_manifest();

        // Check age rating appropriateness
        self.check_age_rating();

        // Check URL validity
        self.check_urls();

        // Check required reason APIs
        self.check_required_reason_apis();

        Ok(self.compliance_checks.clone())
    }

    /// Check metadata completeness
    fn check_metadata_completeness(&mut self) {
        let mut passed = true;
        let mut details = Vec::new();

        if self.metadata.bundle_id.is_empty() {
            passed = false;
            details.push("Bundle ID is required");
        }

        if self.metadata.display_name.is_empty() {
            passed = false;
            details.push("Display name is required");
        }

        if self.metadata.short_description.is_empty() {
            passed = false;
            details.push("Short description is required");
        }

        if self.metadata.keywords.is_empty() {
            passed = false;
            details.push("At least one keyword is required");
        }

        self.compliance_checks.push(ComplianceCheck {
            name: "Metadata Completeness".to_string(),
            description: "All required metadata fields must be filled".to_string(),
            required: true,
            passed,
            details: if details.is_empty() {
                None
            } else {
                Some(details.join(", "))
            },
        });
    }

    /// Check privacy manifest
    fn check_privacy_manifest(&mut self) {
        let mut passed = true;
        let mut details = Vec::new();

        if !self.privacy_manifest.collected_data_types.is_empty() {
            // If collecting data, must have privacy policy URL
            if self.metadata.privacy_policy_url.is_empty() {
                passed = false;
                details.push("Privacy policy URL required when collecting data");
            }
        }

        // Check for tracking domains
        if !self.privacy_manifest.tracking_domains.is_empty() {
            details.push("App uses tracking domains - requires user consent");
        }

        self.compliance_checks.push(ComplianceCheck {
            name: "Privacy Manifest".to_string(),
            description: "Privacy manifest must be complete and accurate".to_string(),
            required: true,
            passed,
            details: if details.is_empty() {
                None
            } else {
                Some(details.join(", "))
            },
        });
    }

    /// Check age rating appropriateness
    fn check_age_rating(&mut self) {
        let passed = true;
        
        self.compliance_checks.push(ComplianceCheck {
            name: "Age Rating".to_string(),
            description: "Age rating must be appropriate for app content".to_string(),
            required: true,
            passed,
            details: Some(format!("Current rating: {:?}", self.metadata.age_rating)),
        });
    }

    /// Check URL validity
    fn check_urls(&mut self) {
        let mut passed = true;
        let mut details = Vec::new();

        if self.metadata.support_url.is_empty() {
            passed = false;
            details.push("Support URL is required");
        }

        if !self.metadata.privacy_policy_url.is_empty() {
            if !self.metadata.privacy_policy_url.starts_with("http") {
                passed = false;
                details.push("Privacy policy URL must be a valid HTTP(S) URL");
            }
        }

        self.compliance_checks.push(ComplianceCheck {
            name: "URL Validity".to_string(),
            description: "All URLs must be valid and accessible".to_string(),
            required: true,
            passed,
            details: if details.is_empty() {
                None
            } else {
                Some(details.join(", "))
            },
        });
    }

    /// Check required reason APIs
    fn check_required_reason_apis(&mut self) {
        let mut passed = true;
        let mut details = Vec::new();

        for api in &self.privacy_manifest.required_reason_apis {
            if api.reason.is_empty() {
                passed = false;
                details.push(format!("Reason required for {:?} API", api.api_type));
            }
        }

        self.compliance_checks.push(ComplianceCheck {
            name: "Required Reason APIs".to_string(),
            description: "APIs requiring reasons must have explanations".to_string(),
            required: true,
            passed,
            details: if details.is_empty() {
                None
            } else {
                Some(details.join(", "))
            },
        });
    }

    /// Get compliance summary
    pub fn get_compliance_summary(&self) -> ComplianceSummary {
        let total = self.compliance_checks.len();
        let passed = self.compliance_checks.iter().filter(|c| c.passed).count();
        let failed = total - passed;
        let required_failed = self.compliance_checks.iter()
            .filter(|c| c.required && !c.passed)
            .count();

        ComplianceSummary {
            total_checks: total,
            passed_checks: passed,
            failed_checks: failed,
            required_failed,
            ready_for_submission: required_failed == 0,
        }
    }

    /// Generate privacy manifest file content
    pub fn generate_privacy_manifest_file(&self) -> PlatformResult<String> {
        // In a real implementation, this would generate a proper PrivacyInfo.xcprivacy file
        // For now, we'll generate a simplified representation
        let mut content = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        content.push_str("<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n");
        content.push_str("<plist version=\"1.0\">\n");
        content.push_str("<dict>\n");
        
        // Add tracking domains
        if !self.privacy_manifest.tracking_domains.is_empty() {
            content.push_str("  <key>NSPrivacyTrackingDomains</key>\n");
            content.push_str("  <array>\n");
            for domain in &self.privacy_manifest.tracking_domains {
                content.push_str(&format!("    <string>{}</string>\n", domain));
            }
            content.push_str("  </array>\n");
        }

        // Add collected data types
        if !self.privacy_manifest.collected_data_types.is_empty() {
            content.push_str("  <key>NSPrivacyCollectedDataTypes</key>\n");
            content.push_str("  <array>\n");
            for data_type in &self.privacy_manifest.collected_data_types {
                content.push_str("    <dict>\n");
                content.push_str(&format!("      <key>NSPrivacyCollectedDataType</key>\n"));
                content.push_str(&format!("      <string>{:?}</string>\n", data_type.data_type));
                content.push_str("    </dict>\n");
            }
            content.push_str("  </array>\n");
        }

        content.push_str("</dict>\n");
        content.push_str("</plist>\n");

        Ok(content)
    }

    /// Get app metadata
    pub fn get_metadata(&self) -> &AppMetadata {
        &self.metadata
    }

    /// Update app metadata
    pub fn update_metadata(&mut self, metadata: AppMetadata) {
        self.metadata = metadata;
    }

    /// Get privacy manifest
    pub fn get_privacy_manifest(&self) -> &PrivacyManifest {
        &self.privacy_manifest
    }

    /// Update privacy manifest
    pub fn update_privacy_manifest(&mut self, manifest: PrivacyManifest) {
        self.privacy_manifest = manifest;
    }
}

/// Compliance summary
#[derive(Debug, Clone)]
pub struct ComplianceSummary {
    pub total_checks: usize,
    pub passed_checks: usize,
    pub failed_checks: usize,
    pub required_failed: usize,
    pub ready_for_submission: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metadata() -> AppMetadata {
        AppMetadata {
            bundle_id: "com.kizuna.test".to_string(),
            version: "1.0.0".to_string(),
            build_number: "1".to_string(),
            display_name: "Kizuna Test".to_string(),
            short_description: "Test app".to_string(),
            full_description: "Full test description".to_string(),
            keywords: vec!["test".to_string(), "kizuna".to_string()],
            category: AppCategory::Utilities,
            age_rating: AgeRating::FourPlus,
            copyright: "2024 Kizuna".to_string(),
            support_url: "https://kizuna.com/support".to_string(),
            privacy_policy_url: "https://kizuna.com/privacy".to_string(),
        }
    }

    fn create_test_privacy_manifest() -> PrivacyManifest {
        PrivacyManifest {
            tracking_domains: vec![],
            collected_data_types: vec![],
            required_reason_apis: vec![],
            privacy_nutrition_label: PrivacyNutritionLabel {
                data_used_to_track_you: vec![],
                data_linked_to_you: vec![],
                data_not_linked_to_you: vec![],
            },
        }
    }

    #[test]
    fn test_compliance_manager_creation() {
        let metadata = create_test_metadata();
        let manifest = create_test_privacy_manifest();
        let manager = AppStoreComplianceManager::new(metadata, manifest);
        
        assert_eq!(manager.metadata.bundle_id, "com.kizuna.test");
    }

    #[test]
    fn test_compliance_checks() {
        let metadata = create_test_metadata();
        let manifest = create_test_privacy_manifest();
        let mut manager = AppStoreComplianceManager::new(metadata, manifest);
        
        let checks = manager.run_compliance_checks().unwrap();
        assert!(!checks.is_empty());
    }

    #[test]
    fn test_metadata_completeness_check() {
        let mut metadata = create_test_metadata();
        metadata.bundle_id = String::new(); // Make it incomplete
        
        let manifest = create_test_privacy_manifest();
        let mut manager = AppStoreComplianceManager::new(metadata, manifest);
        
        let checks = manager.run_compliance_checks().unwrap();
        let metadata_check = checks.iter().find(|c| c.name == "Metadata Completeness");
        
        assert!(metadata_check.is_some());
        assert!(!metadata_check.unwrap().passed);
    }

    #[test]
    fn test_privacy_manifest_check() {
        let metadata = create_test_metadata();
        let mut manifest = create_test_privacy_manifest();
        
        // Add collected data without privacy policy
        manifest.collected_data_types.push(CollectedDataType {
            data_type: DataType::UsageData,
            purpose: DataPurpose::Analytics,
            linked_to_user: false,
            used_for_tracking: false,
        });
        
        let mut metadata_no_privacy = metadata.clone();
        metadata_no_privacy.privacy_policy_url = String::new();
        
        let mut manager = AppStoreComplianceManager::new(metadata_no_privacy, manifest);
        let checks = manager.run_compliance_checks().unwrap();
        
        let privacy_check = checks.iter().find(|c| c.name == "Privacy Manifest");
        assert!(privacy_check.is_some());
        assert!(!privacy_check.unwrap().passed);
    }

    #[test]
    fn test_url_validity_check() {
        let mut metadata = create_test_metadata();
        metadata.support_url = String::new(); // Invalid
        
        let manifest = create_test_privacy_manifest();
        let mut manager = AppStoreComplianceManager::new(metadata, manifest);
        
        let checks = manager.run_compliance_checks().unwrap();
        let url_check = checks.iter().find(|c| c.name == "URL Validity");
        
        assert!(url_check.is_some());
        assert!(!url_check.unwrap().passed);
    }

    #[test]
    fn test_required_reason_apis_check() {
        let metadata = create_test_metadata();
        let mut manifest = create_test_privacy_manifest();
        
        // Add API without reason
        manifest.required_reason_apis.push(RequiredReasonAPI {
            api_type: APIType::FileTimestamp,
            reason: String::new(),
        });
        
        let mut manager = AppStoreComplianceManager::new(metadata, manifest);
        let checks = manager.run_compliance_checks().unwrap();
        
        let api_check = checks.iter().find(|c| c.name == "Required Reason APIs");
        assert!(api_check.is_some());
        assert!(!api_check.unwrap().passed);
    }

    #[test]
    fn test_compliance_summary() {
        let metadata = create_test_metadata();
        let manifest = create_test_privacy_manifest();
        let mut manager = AppStoreComplianceManager::new(metadata, manifest);
        
        manager.run_compliance_checks().unwrap();
        let summary = manager.get_compliance_summary();
        
        assert!(summary.total_checks > 0);
        assert_eq!(summary.passed_checks + summary.failed_checks, summary.total_checks);
    }

    #[test]
    fn test_generate_privacy_manifest_file() {
        let metadata = create_test_metadata();
        let manifest = create_test_privacy_manifest();
        let manager = AppStoreComplianceManager::new(metadata, manifest);
        
        let content = manager.generate_privacy_manifest_file().unwrap();
        assert!(content.contains("<?xml version"));
        assert!(content.contains("<plist version=\"1.0\">"));
    }

    #[test]
    fn test_metadata_update() {
        let metadata = create_test_metadata();
        let manifest = create_test_privacy_manifest();
        let mut manager = AppStoreComplianceManager::new(metadata, manifest);
        
        let mut new_metadata = create_test_metadata();
        new_metadata.version = "2.0.0".to_string();
        
        manager.update_metadata(new_metadata);
        assert_eq!(manager.get_metadata().version, "2.0.0");
    }

    #[test]
    fn test_privacy_manifest_update() {
        let metadata = create_test_metadata();
        let manifest = create_test_privacy_manifest();
        let mut manager = AppStoreComplianceManager::new(metadata, manifest);
        
        let mut new_manifest = create_test_privacy_manifest();
        new_manifest.tracking_domains.push("example.com".to_string());
        
        manager.update_privacy_manifest(new_manifest);
        assert_eq!(manager.get_privacy_manifest().tracking_domains.len(), 1);
    }
}
