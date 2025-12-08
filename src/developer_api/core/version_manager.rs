/// Integrated version management system combining versioning, deprecation, and change tracking
use super::Result;
use super::versioning::{ApiVersion, CompatibilityManager, CompatibilityCheck};
use super::deprecation::{DeprecationManager, DeprecationInfo, MigrationGuide};
use super::change_tracking::{ChangeTracker, Changelog, ApiChange, CompatibilityMatrixEntry};
use semver::Version;
use std::path::Path;

/// Integrated version manager for the entire API
pub struct IntegratedVersionManager {
    /// Compatibility manager
    compatibility: CompatibilityManager,
    
    /// Deprecation manager
    deprecation: DeprecationManager,
    
    /// Change tracker
    changes: ChangeTracker,
    
    /// Current API version
    current_version: Version,
}

impl IntegratedVersionManager {
    /// Creates a new integrated version manager
    pub fn new(current_version: Version) -> Self {
        Self {
            compatibility: CompatibilityManager::new(current_version.clone()),
            deprecation: DeprecationManager::new(current_version.clone()),
            changes: ChangeTracker::new(current_version.clone()),
            current_version,
        }
    }
    
    /// Gets the current API version
    pub fn current_version(&self) -> &Version {
        &self.current_version
    }
    
    /// Registers a new API version
    pub fn register_version(&mut self, version: ApiVersion) {
        self.compatibility.register_version(version);
    }
    
    /// Registers a deprecation
    pub fn register_deprecation(&mut self, info: DeprecationInfo) {
        self.deprecation.register_deprecation(info);
    }
    
    /// Registers a migration guide
    pub fn register_migration_guide(&mut self, guide: MigrationGuide) {
        self.deprecation.register_migration_guide(guide);
    }
    
    /// Adds a changelog
    pub fn add_changelog(&mut self, changelog: Changelog) {
        self.changes.add_changelog(changelog);
    }
    
    /// Adds a compatibility matrix entry
    pub fn add_compatibility_entry(&mut self, entry: CompatibilityMatrixEntry) {
        self.changes.add_compatibility_entry(entry);
    }
    
    /// Checks compatibility between two versions
    pub fn check_compatibility(&self, from: &Version, to: &Version) -> Result<CompatibilityCheck> {
        self.compatibility.check_compatibility(from, to)
    }
    
    /// Checks if an API element is deprecated
    pub fn is_deprecated(&self, element: &str) -> bool {
        self.deprecation.is_deprecated(element)
    }
    
    /// Warns about deprecated API usage
    pub fn warn_deprecated(&self, element: &str) -> Result<()> {
        self.deprecation.warn(element)
    }
    
    /// Gets deprecation info
    pub fn get_deprecation_info(&self, element: &str) -> Option<&DeprecationInfo> {
        self.deprecation.get_deprecation_info(element)
    }
    
    /// Gets migration guide between versions
    pub fn get_migration_guide(&self, from: &Version, to: &Version) -> Option<&MigrationGuide> {
        self.deprecation.get_migration_guide(from, to)
    }
    
    /// Gets changelog for a version
    pub fn get_changelog(&self, version: &Version) -> Option<&Changelog> {
        self.changes.get_changelog(version)
    }
    
    /// Gets all changes between two versions
    pub fn get_changes_between(&self, from: &Version, to: &Version) -> Result<Vec<&ApiChange>> {
        self.changes.detect_changes(from, to)
    }
    
    /// Validates the entire version management system
    pub fn validate(&self) -> Result<Vec<String>> {
        let mut warnings = Vec::new();
        
        // Validate compatibility manager
        let compat_warnings = self.compatibility.generate_compatibility_report();
        if compat_warnings.contains("warning") {
            warnings.push("Compatibility issues detected".to_string());
        }
        
        // Validate change tracker
        let change_warnings = self.changes.validate()?;
        warnings.extend(change_warnings);
        
        Ok(warnings)
    }
    
    /// Generates a comprehensive version report
    pub fn generate_version_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("# Kizuna API Version Report\n\n");
        report.push_str(&format!("**Current Version:** {}\n\n", self.current_version));
        
        report.push_str("## Compatibility Status\n\n");
        report.push_str(&self.compatibility.generate_compatibility_report());
        report.push_str("\n");
        
        report.push_str("## Deprecation Status\n\n");
        report.push_str(&self.deprecation.generate_deprecation_report());
        report.push_str("\n");
        
        report.push_str("## Recent Changes\n\n");
        if let Some(latest) = self.changes.get_all_changelogs().first() {
            report.push_str(&latest.to_markdown());
        }
        
        report
    }
    
    /// Generates all documentation files
    pub fn generate_all_docs(&self, output_dir: &Path) -> Result<Vec<std::path::PathBuf>> {
        std::fs::create_dir_all(output_dir)?;
        
        let mut written_files = Vec::new();
        
        // Generate CHANGELOG.md
        let changelog_path = output_dir.join("CHANGELOG.md");
        let changelog_content = self.changes.generate_changelog_document();
        std::fs::write(&changelog_path, changelog_content)?;
        written_files.push(changelog_path);
        
        // Generate COMPATIBILITY.md
        let compat_path = output_dir.join("COMPATIBILITY.md");
        let compat_content = self.changes.generate_compatibility_matrix();
        std::fs::write(&compat_path, compat_content)?;
        written_files.push(compat_path);
        
        // Generate DEPRECATIONS.md
        let deprecation_path = output_dir.join("DEPRECATIONS.md");
        let deprecation_content = self.deprecation.generate_deprecation_report();
        std::fs::write(&deprecation_path, deprecation_content)?;
        written_files.push(deprecation_path);
        
        // Generate MIGRATION.md
        let migration_path = output_dir.join("MIGRATION.md");
        let migration_content = self.deprecation.generate_migration_docs();
        std::fs::write(&migration_path, migration_content)?;
        written_files.push(migration_path);
        
        // Generate VERSION_REPORT.md
        let report_path = output_dir.join("VERSION_REPORT.md");
        let report_content = self.generate_version_report();
        std::fs::write(&report_path, report_content)?;
        written_files.push(report_path);
        
        Ok(written_files)
    }
    
    /// Loads sample version data for testing
    pub fn load_sample_data(&mut self) -> Result<()> {
        // Register versions
        let v1_0_0 = ApiVersion::new(1, 0, 0)
            .mark_as_latest()
            .with_release_date(chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        self.register_version(v1_0_0);
        
        let v0_9_0 = ApiVersion::new(0, 9, 0)
            .with_release_date(chrono::NaiveDate::from_ymd_opt(2023, 12, 1).unwrap());
        self.register_version(v0_9_0);
        
        // Add changelog for 1.0.0
        let mut changelog_1_0 = Changelog::new(Version::new(1, 0, 0))
            .mark_as_latest()
            .with_release_date(chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
            .with_notes("First stable release of Kizuna API with full feature coverage.");
        
        changelog_1_0.add_change(
            ApiChange::new(
                "feature-001",
                super::change_tracking::ChangeType::Feature,
                "Core API",
                "Initial stable release with comprehensive API coverage",
            )
            .with_author("Kizuna Team")
        );
        
        changelog_1_0.add_change(
            ApiChange::new(
                "feature-002",
                super::change_tracking::ChangeType::Feature,
                "Language Bindings",
                "Added Node.js, Python, and Flutter bindings",
            )
            .with_author("Kizuna Team")
        );
        
        self.add_changelog(changelog_1_0);
        
        // Add compatibility matrix entry
        let compat_1_0 = CompatibilityMatrixEntry::new(Version::new(1, 0, 0))
            .add_binding("rust", Version::new(1, 0, 0))
            .add_binding("nodejs", Version::new(1, 0, 0))
            .add_binding("python", Version::new(1, 0, 0))
            .add_binding("flutter", Version::new(1, 0, 0))
            .add_platform("Linux")
            .add_platform("macOS")
            .add_platform("Windows")
            .with_requirements("Rust 1.70+, Node.js 18+, Python 3.8+");
        
        self.add_compatibility_entry(compat_1_0);
        
        // Add a sample deprecation
        let deprecation = DeprecationInfo::new(
            "old_api_function",
            Version::new(0, 9, 0),
            "Replaced by more efficient implementation",
        )
        .with_replacement("new_api_function")
        .with_removal_version(Version::new(2, 0, 0))
        .with_migration_guide("Replace all calls to old_api_function() with new_api_function()");
        
        self.register_deprecation(deprecation);
        
        Ok(())
    }
}

impl Default for IntegratedVersionManager {
    fn default() -> Self {
        let version = Version::parse(env!("CARGO_PKG_VERSION"))
            .unwrap_or_else(|_| Version::new(0, 1, 0));
        Self::new(version)
    }
}

// Include integration tests
#[cfg(test)]
#[path = "versioning_test.rs"]
mod versioning_integration_tests;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_integrated_version_manager() {
        let mut manager = IntegratedVersionManager::new(Version::new(1, 0, 0));
        
        let version = ApiVersion::new(1, 0, 0).mark_as_latest();
        manager.register_version(version);
        
        assert_eq!(manager.current_version(), &Version::new(1, 0, 0));
    }
    
    #[test]
    fn test_deprecation_integration() {
        let mut manager = IntegratedVersionManager::new(Version::new(1, 0, 0));
        
        let deprecation = DeprecationInfo::new(
            "old_function",
            Version::new(1, 0, 0),
            "Use new_function",
        );
        
        manager.register_deprecation(deprecation);
        
        assert!(manager.is_deprecated("old_function"));
        assert!(!manager.is_deprecated("new_function"));
    }
    
    #[test]
    fn test_changelog_integration() {
        let mut manager = IntegratedVersionManager::new(Version::new(1, 0, 0));
        
        let mut changelog = Changelog::new(Version::new(1, 0, 0));
        changelog.add_change(ApiChange::new(
            "change-001",
            super::change_tracking::ChangeType::Feature,
            "Core",
            "New feature",
        ));
        
        manager.add_changelog(changelog);
        
        assert!(manager.get_changelog(&Version::new(1, 0, 0)).is_some());
    }
    
    #[test]
    fn test_compatibility_check() {
        let manager = IntegratedVersionManager::new(Version::new(1, 0, 0));
        
        let v1_0 = Version::new(1, 0, 0);
        let v1_1 = Version::new(1, 1, 0);
        
        let check = manager.check_compatibility(&v1_0, &v1_1).unwrap();
        assert_eq!(check.level, super::versioning::CompatibilityLevel::BackwardCompatible);
    }
    
    #[test]
    fn test_sample_data_loading() {
        let mut manager = IntegratedVersionManager::new(Version::new(1, 0, 0));
        
        manager.load_sample_data().unwrap();
        
        assert!(manager.get_changelog(&Version::new(1, 0, 0)).is_some());
        assert!(manager.is_deprecated("old_api_function"));
    }
}
