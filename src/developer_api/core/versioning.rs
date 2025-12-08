/// API stability and versioning system for semantic versioning and compatibility management
use super::Result;
use semver::{Version, VersionReq};
use std::collections::HashMap;
use std::fmt;

/// API version information with semantic versioning
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiVersion {
    /// Semantic version
    pub version: Version,
    
    /// Release date
    pub release_date: chrono::NaiveDate,
    
    /// Is this a stable release?
    pub is_stable: bool,
    
    /// Is this the latest version?
    pub is_latest: bool,
    
    /// Minimum compatible version
    pub min_compatible: Option<Version>,
}

impl ApiVersion {
    /// Creates a new API version
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            version: Version::new(major, minor, patch),
            release_date: chrono::Utc::now().date_naive(),
            is_stable: major >= 1,
            is_latest: false,
            min_compatible: None,
        }
    }
    
    /// Creates from a semver Version
    pub fn from_version(version: Version) -> Self {
        Self {
            is_stable: version.major >= 1,
            release_date: chrono::Utc::now().date_naive(),
            is_latest: false,
            min_compatible: None,
            version,
        }
    }
    
    /// Marks this version as latest
    pub fn mark_as_latest(mut self) -> Self {
        self.is_latest = true;
        self
    }
    
    /// Sets the minimum compatible version
    pub fn with_min_compatible(mut self, min_version: Version) -> Self {
        self.min_compatible = Some(min_version);
        self
    }
    
    /// Sets the release date
    pub fn with_release_date(mut self, date: chrono::NaiveDate) -> Self {
        self.release_date = date;
        self
    }
    
    /// Checks if this version is compatible with another version
    pub fn is_compatible_with(&self, other: &Version) -> bool {
        // Same major version for stable releases (1.x.x)
        if self.version.major >= 1 && other.major >= 1 {
            return self.version.major == other.major;
        }
        
        // For pre-1.0 versions, require exact minor version match
        if self.version.major == 0 && other.major == 0 {
            return self.version.minor == other.minor;
        }
        
        false
    }
    
    /// Gets the version string
    pub fn version_string(&self) -> String {
        self.version.to_string()
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version)
    }
}

/// Compatibility level between versions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityLevel {
    /// Fully compatible - no changes needed
    FullyCompatible,
    
    /// Backward compatible - old code works with new API
    BackwardCompatible,
    
    /// Forward compatible - new code works with old API
    ForwardCompatible,
    
    /// Partially compatible - some features may not work
    PartiallyCompatible,
    
    /// Incompatible - breaking changes
    Incompatible,
}

impl CompatibilityLevel {
    /// Gets a human-readable description
    pub fn description(&self) -> &str {
        match self {
            Self::FullyCompatible => "Fully compatible - no changes needed",
            Self::BackwardCompatible => "Backward compatible - old code works with new API",
            Self::ForwardCompatible => "Forward compatible - new code works with old API",
            Self::PartiallyCompatible => "Partially compatible - some features may not work",
            Self::Incompatible => "Incompatible - breaking changes require migration",
        }
    }
}

/// Compatibility check result
#[derive(Debug, Clone)]
pub struct CompatibilityCheck {
    /// Source version
    pub from_version: Version,
    
    /// Target version
    pub to_version: Version,
    
    /// Compatibility level
    pub level: CompatibilityLevel,
    
    /// Detailed compatibility notes
    pub notes: Vec<String>,
    
    /// Breaking changes (if any)
    pub breaking_changes: Vec<String>,
    
    /// Required migration steps
    pub migration_steps: Vec<String>,
}

impl CompatibilityCheck {
    /// Creates a new compatibility check
    pub fn new(from: Version, to: Version, level: CompatibilityLevel) -> Self {
        Self {
            from_version: from,
            to_version: to,
            level,
            notes: Vec::new(),
            breaking_changes: Vec::new(),
            migration_steps: Vec::new(),
        }
    }
    
    /// Adds a note
    pub fn add_note<S: Into<String>>(mut self, note: S) -> Self {
        self.notes.push(note.into());
        self
    }
    
    /// Adds a breaking change
    pub fn add_breaking_change<S: Into<String>>(mut self, change: S) -> Self {
        self.breaking_changes.push(change.into());
        self
    }
    
    /// Adds a migration step
    pub fn add_migration_step<S: Into<String>>(mut self, step: S) -> Self {
        self.migration_steps.push(step.into());
        self
    }
    
    /// Checks if migration is required
    pub fn requires_migration(&self) -> bool {
        !self.migration_steps.is_empty() || !self.breaking_changes.is_empty()
    }
}

/// Version compatibility manager
pub struct CompatibilityManager {
    /// All registered API versions
    versions: Vec<ApiVersion>,
    
    /// Compatibility matrix
    compatibility_matrix: HashMap<(Version, Version), CompatibilityCheck>,
    
    /// Current API version
    current_version: Version,
}

impl CompatibilityManager {
    /// Creates a new compatibility manager
    pub fn new(current_version: Version) -> Self {
        Self {
            versions: Vec::new(),
            compatibility_matrix: HashMap::new(),
            current_version,
        }
    }
    
    /// Registers a new API version
    pub fn register_version(&mut self, version: ApiVersion) {
        // Mark previous latest as not latest
        for v in &mut self.versions {
            if v.is_latest {
                v.is_latest = false;
            }
        }
        
        self.versions.push(version);
        self.versions.sort_by(|a, b| b.version.cmp(&a.version));
    }
    
    /// Gets the current API version
    pub fn current_version(&self) -> &Version {
        &self.current_version
    }
    
    /// Gets the latest registered version
    pub fn latest_version(&self) -> Option<&ApiVersion> {
        self.versions.iter().find(|v| v.is_latest)
    }
    
    /// Gets all registered versions
    pub fn all_versions(&self) -> &[ApiVersion] {
        &self.versions
    }
    
    /// Checks compatibility between two versions
    pub fn check_compatibility(&self, from: &Version, to: &Version) -> Result<CompatibilityCheck> {
        // Check cache first
        if let Some(cached) = self.compatibility_matrix.get(&(from.clone(), to.clone())) {
            return Ok(cached.clone());
        }
        
        // Determine compatibility level based on semver rules
        let level = if from == to {
            CompatibilityLevel::FullyCompatible
        } else if from.major != to.major {
            CompatibilityLevel::Incompatible
        } else if from.major == 0 {
            // Pre-1.0 versions: minor version changes are breaking
            if from.minor != to.minor {
                CompatibilityLevel::Incompatible
            } else {
                CompatibilityLevel::BackwardCompatible
            }
        } else {
            // Post-1.0 versions: minor and patch changes are backward compatible
            if to > from {
                CompatibilityLevel::BackwardCompatible
            } else {
                CompatibilityLevel::ForwardCompatible
            }
        };
        
        let mut check = CompatibilityCheck::new(from.clone(), to.clone(), level);
        
        // Add notes based on compatibility level
        match level {
            CompatibilityLevel::FullyCompatible => {
                check = check.add_note("Versions are identical");
            }
            CompatibilityLevel::BackwardCompatible => {
                check = check.add_note("Newer version is backward compatible with older version");
            }
            CompatibilityLevel::ForwardCompatible => {
                check = check.add_note("Older version may not support all features of newer version");
            }
            CompatibilityLevel::Incompatible => {
                check = check.add_note("Major version change - breaking changes expected");
                check = check.add_breaking_change("Major version upgrade requires migration");
            }
            CompatibilityLevel::PartiallyCompatible => {
                check = check.add_note("Some features may not be compatible");
            }
        }
        
        Ok(check)
    }
    
    /// Registers a compatibility check result
    pub fn register_compatibility(&mut self, check: CompatibilityCheck) {
        let key = (check.from_version.clone(), check.to_version.clone());
        self.compatibility_matrix.insert(key, check);
    }
    
    /// Validates if a version requirement is satisfied
    pub fn validate_requirement(&self, requirement: &VersionReq) -> Result<bool> {
        Ok(requirement.matches(&self.current_version))
    }
    
    /// Gets all compatible versions for a given version
    pub fn get_compatible_versions(&self, version: &Version) -> Vec<&ApiVersion> {
        self.versions
            .iter()
            .filter(|v| v.is_compatible_with(version))
            .collect()
    }
    
    /// Generates a compatibility report
    pub fn generate_compatibility_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("# API Compatibility Report\n\n");
        report.push_str(&format!("Current API Version: {}\n\n", self.current_version));
        
        report.push_str("## Registered Versions\n\n");
        for version in &self.versions {
            report.push_str(&format!(
                "- {} ({}){}{}\n",
                version.version,
                version.release_date,
                if version.is_latest { " [LATEST]" } else { "" },
                if version.is_stable { " [STABLE]" } else { " [UNSTABLE]" }
            ));
        }
        
        report.push_str("\n## Compatibility Matrix\n\n");
        report.push_str("| From | To | Level | Notes |\n");
        report.push_str("|------|----|----|-------|\n");
        
        for ((from, to), check) in &self.compatibility_matrix {
            report.push_str(&format!(
                "| {} | {} | {:?} | {} |\n",
                from,
                to,
                check.level,
                check.notes.join("; ")
            ));
        }
        
        report
    }
}

impl Default for CompatibilityManager {
    fn default() -> Self {
        // Use the crate version as default
        let version = Version::parse(env!("CARGO_PKG_VERSION"))
            .unwrap_or_else(|_| Version::new(0, 1, 0));
        Self::new(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_version_creation() {
        let version = ApiVersion::new(1, 0, 0);
        assert_eq!(version.version, Version::new(1, 0, 0));
        assert!(version.is_stable);
        assert!(!version.is_latest);
    }
    
    #[test]
    fn test_version_compatibility() {
        let v1_0 = ApiVersion::new(1, 0, 0);
        let v1_1 = Version::new(1, 1, 0);
        let v2_0 = Version::new(2, 0, 0);
        
        assert!(v1_0.is_compatible_with(&v1_1));
        assert!(!v1_0.is_compatible_with(&v2_0));
    }
    
    #[test]
    fn test_pre_1_0_compatibility() {
        let v0_1 = ApiVersion::new(0, 1, 0);
        let v0_1_1 = Version::new(0, 1, 1);
        let v0_2 = Version::new(0, 2, 0);
        
        assert!(v0_1.is_compatible_with(&v0_1_1));
        assert!(!v0_1.is_compatible_with(&v0_2));
    }
    
    #[test]
    fn test_compatibility_manager() {
        let mut manager = CompatibilityManager::new(Version::new(1, 0, 0));
        
        let version = ApiVersion::new(1, 0, 0).mark_as_latest();
        manager.register_version(version);
        
        assert!(manager.latest_version().is_some());
        assert_eq!(manager.all_versions().len(), 1);
    }
    
    #[test]
    fn test_compatibility_check() {
        let manager = CompatibilityManager::new(Version::new(1, 0, 0));
        
        let v1_0 = Version::new(1, 0, 0);
        let v1_1 = Version::new(1, 1, 0);
        
        let check = manager.check_compatibility(&v1_0, &v1_1).unwrap();
        assert_eq!(check.level, CompatibilityLevel::BackwardCompatible);
    }
    
    #[test]
    fn test_major_version_incompatibility() {
        let manager = CompatibilityManager::new(Version::new(1, 0, 0));
        
        let v1_0 = Version::new(1, 0, 0);
        let v2_0 = Version::new(2, 0, 0);
        
        let check = manager.check_compatibility(&v1_0, &v2_0).unwrap();
        assert_eq!(check.level, CompatibilityLevel::Incompatible);
        assert!(!check.breaking_changes.is_empty());
    }
    
    #[test]
    fn test_version_requirement_validation() {
        let manager = CompatibilityManager::new(Version::new(1, 2, 3));
        
        let req = VersionReq::parse("^1.0.0").unwrap();
        assert!(manager.validate_requirement(&req).unwrap());
        
        let req = VersionReq::parse("^2.0.0").unwrap();
        assert!(!manager.validate_requirement(&req).unwrap());
    }
}
