/// Documentation versioning and maintenance system
use super::{Result, DocError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Documentation version
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocVersion {
    /// Semantic version
    pub version: semver::Version,
    /// Release date
    pub release_date: chrono::NaiveDate,
    /// Is this the latest version?
    pub is_latest: bool,
}

impl DocVersion {
    /// Creates a new documentation version
    pub fn new(version: semver::Version, release_date: chrono::NaiveDate) -> Self {
        Self {
            version,
            release_date,
            is_latest: false,
        }
    }
    
    /// Marks this version as latest
    pub fn mark_as_latest(mut self) -> Self {
        self.is_latest = true;
        self
    }
    
    /// Gets the version string
    pub fn version_string(&self) -> String {
        self.version.to_string()
    }
}

/// API change type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// Breaking change
    Breaking,
    /// New feature
    Feature,
    /// Bug fix
    Fix,
    /// Deprecation
    Deprecation,
    /// Documentation update
    Documentation,
}

impl ChangeType {
    /// Gets the change type name
    pub fn name(&self) -> &str {
        match self {
            Self::Breaking => "Breaking Change",
            Self::Feature => "New Feature",
            Self::Fix => "Bug Fix",
            Self::Deprecation => "Deprecation",
            Self::Documentation => "Documentation",
        }
    }
    
    /// Gets the emoji for this change type
    pub fn emoji(&self) -> &str {
        match self {
            Self::Breaking => "⚠️",
            Self::Feature => "✨",
            Self::Fix => "🐛",
            Self::Deprecation => "⚰️",
            Self::Documentation => "📝",
        }
    }
}

/// API change entry
#[derive(Debug, Clone)]
pub struct ApiChange {
    /// Change type
    pub change_type: ChangeType,
    /// Component affected
    pub component: String,
    /// Description
    pub description: String,
    /// Migration guide (for breaking changes)
    pub migration_guide: Option<String>,
}

impl ApiChange {
    /// Creates a new API change
    pub fn new(change_type: ChangeType, component: String, description: String) -> Self {
        Self {
            change_type,
            component,
            description,
            migration_guide: None,
        }
    }
    
    /// Adds a migration guide
    pub fn with_migration(mut self, guide: String) -> Self {
        self.migration_guide = Some(guide);
        self
    }
    
    /// Formats the change as markdown
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str(&format!(
            "- {} **{}** ({}): {}\n",
            self.change_type.emoji(),
            self.change_type.name(),
            self.component,
            self.description
        ));
        
        if let Some(guide) = &self.migration_guide {
            md.push_str(&format!("  - Migration: {}\n", guide));
        }
        
        md
    }
}

/// Changelog for a version
#[derive(Debug, Clone)]
pub struct Changelog {
    /// Version
    pub version: DocVersion,
    /// Changes
    pub changes: Vec<ApiChange>,
    /// Additional notes
    pub notes: Option<String>,
}

impl Changelog {
    /// Creates a new changelog
    pub fn new(version: DocVersion) -> Self {
        Self {
            version,
            changes: Vec::new(),
            notes: None,
        }
    }
    
    /// Adds a change
    pub fn add_change(&mut self, change: ApiChange) {
        self.changes.push(change);
    }
    
    /// Sets notes
    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }
    
    /// Generates markdown changelog
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("## Version {} ({})\n\n", 
            self.version.version_string(),
            self.version.release_date.format("%Y-%m-%d")
        ));
        
        if self.version.is_latest {
            md.push_str("**Latest Version**\n\n");
        }
        
        if let Some(notes) = &self.notes {
            md.push_str(&format!("{}\n\n", notes));
        }
        
        // Group changes by type
        let mut breaking = Vec::new();
        let mut features = Vec::new();
        let mut fixes = Vec::new();
        let mut deprecations = Vec::new();
        let mut docs = Vec::new();
        
        for change in &self.changes {
            match change.change_type {
                ChangeType::Breaking => breaking.push(change),
                ChangeType::Feature => features.push(change),
                ChangeType::Fix => fixes.push(change),
                ChangeType::Deprecation => deprecations.push(change),
                ChangeType::Documentation => docs.push(change),
            }
        }
        
        if !breaking.is_empty() {
            md.push_str("### Breaking Changes\n\n");
            for change in breaking {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        if !features.is_empty() {
            md.push_str("### New Features\n\n");
            for change in features {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        if !deprecations.is_empty() {
            md.push_str("### Deprecations\n\n");
            for change in deprecations {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        if !fixes.is_empty() {
            md.push_str("### Bug Fixes\n\n");
            for change in fixes {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        if !docs.is_empty() {
            md.push_str("### Documentation\n\n");
            for change in docs {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        md
    }
}

/// Compatibility matrix entry
#[derive(Debug, Clone)]
pub struct CompatibilityEntry {
    /// API version
    pub api_version: semver::Version,
    /// Language binding versions
    pub binding_versions: HashMap<String, semver::Version>,
    /// Compatibility notes
    pub notes: Option<String>,
}

impl CompatibilityEntry {
    /// Creates a new compatibility entry
    pub fn new(api_version: semver::Version) -> Self {
        Self {
            api_version,
            binding_versions: HashMap::new(),
            notes: None,
        }
    }
    
    /// Adds a binding version
    pub fn add_binding(&mut self, language: String, version: semver::Version) {
        self.binding_versions.insert(language, version);
    }
    
    /// Sets notes
    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }
}

/// Version manager for documentation
pub struct VersionManager {
    /// All versions
    versions: Vec<DocVersion>,
    /// Changelogs by version
    changelogs: HashMap<semver::Version, Changelog>,
    /// Compatibility matrix
    compatibility: Vec<CompatibilityEntry>,
}

impl VersionManager {
    /// Creates a new version manager
    pub fn new() -> Self {
        Self {
            versions: Vec::new(),
            changelogs: HashMap::new(),
            compatibility: Vec::new(),
        }
    }
    
    /// Adds a version
    pub fn add_version(&mut self, version: DocVersion) {
        // Mark previous latest as not latest
        for v in &mut self.versions {
            if v.is_latest {
                v.is_latest = false;
            }
        }
        
        self.versions.push(version);
        self.versions.sort_by(|a, b| b.version.cmp(&a.version));
    }
    
    /// Gets the latest version
    pub fn get_latest(&self) -> Option<&DocVersion> {
        self.versions.iter().find(|v| v.is_latest)
    }
    
    /// Gets all versions
    pub fn get_all_versions(&self) -> &[DocVersion] {
        &self.versions
    }
    
    /// Adds a changelog
    pub fn add_changelog(&mut self, changelog: Changelog) {
        self.changelogs.insert(changelog.version.version.clone(), changelog);
    }
    
    /// Gets a changelog
    pub fn get_changelog(&self, version: &semver::Version) -> Option<&Changelog> {
        self.changelogs.get(version)
    }
    
    /// Adds a compatibility entry
    pub fn add_compatibility(&mut self, entry: CompatibilityEntry) {
        self.compatibility.push(entry);
        self.compatibility.sort_by(|a, b| b.api_version.cmp(&a.api_version));
    }
    
    /// Generates full changelog document
    pub fn generate_changelog_document(&self) -> Result<String> {
        let mut doc = String::new();
        
        doc.push_str("# Kizuna API Changelog\n\n");
        doc.push_str("All notable changes to the Kizuna API will be documented in this file.\n\n");
        
        for version in &self.versions {
            if let Some(changelog) = self.changelogs.get(&version.version) {
                doc.push_str(&changelog.to_markdown());
            }
        }
        
        Ok(doc)
    }
    
    /// Generates compatibility matrix document
    pub fn generate_compatibility_matrix(&self) -> Result<String> {
        let mut doc = String::new();
        
        doc.push_str("# Kizuna API Compatibility Matrix\n\n");
        doc.push_str("This document shows the compatibility between Kizuna core API versions ");
        doc.push_str("and language binding versions.\n\n");
        
        if self.compatibility.is_empty() {
            return Ok(doc);
        }
        
        // Generate table header
        doc.push_str("| Core API | Rust | Node.js | Python | Flutter |\n");
        doc.push_str("|----------|------|---------|--------|----------|\n");
        
        for entry in &self.compatibility {
            doc.push_str(&format!("| {} ", entry.api_version));
            
            for lang in &["rust", "nodejs", "python", "flutter"] {
                if let Some(version) = entry.binding_versions.get(*lang) {
                    doc.push_str(&format!("| {} ", version));
                } else {
                    doc.push_str("| - ");
                }
            }
            
            doc.push_str("|\n");
        }
        
        doc.push_str("\n");
        
        Ok(doc)
    }
    
    /// Validates documentation consistency
    pub fn validate_consistency(&self) -> Result<Vec<String>> {
        let mut warnings = Vec::new();
        
        // Check that all versions have changelogs
        for version in &self.versions {
            if !self.changelogs.contains_key(&version.version) {
                warnings.push(format!(
                    "Version {} is missing a changelog",
                    version.version_string()
                ));
            }
        }
        
        // Check that there's exactly one latest version
        let latest_count = self.versions.iter().filter(|v| v.is_latest).count();
        if latest_count == 0 {
            warnings.push("No version marked as latest".to_string());
        } else if latest_count > 1 {
            warnings.push("Multiple versions marked as latest".to_string());
        }
        
        // Check compatibility entries
        for entry in &self.compatibility {
            if entry.binding_versions.is_empty() {
                warnings.push(format!(
                    "Compatibility entry for {} has no binding versions",
                    entry.api_version
                ));
            }
        }
        
        Ok(warnings)
    }
    
    /// Writes version documentation to files
    pub fn write_version_docs(&self, output_dir: &Path) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(output_dir)?;
        
        let mut written_files = Vec::new();
        
        // Write changelog
        let changelog_path = output_dir.join("CHANGELOG.md");
        let changelog_content = self.generate_changelog_document()?;
        std::fs::write(&changelog_path, changelog_content)?;
        written_files.push(changelog_path);
        
        // Write compatibility matrix
        let compat_path = output_dir.join("COMPATIBILITY.md");
        let compat_content = self.generate_compatibility_matrix()?;
        std::fs::write(&compat_path, compat_content)?;
        written_files.push(compat_path);
        
        Ok(written_files)
    }
    
    /// Loads sample version data
    pub fn load_sample_data(&mut self) -> Result<()> {
        // Add versions
        let v1_0_0 = DocVersion::new(
            semver::Version::new(1, 0, 0),
            chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
        ).mark_as_latest();
        
        let v0_9_0 = DocVersion::new(
            semver::Version::new(0, 9, 0),
            chrono::NaiveDate::from_ymd_opt(2023, 12, 1).unwrap(),
        );
        
        self.add_version(v1_0_0.clone());
        self.add_version(v0_9_0.clone());
        
        // Add changelog for 1.0.0
        let mut changelog_1_0 = Changelog::new(v1_0_0.clone());
        changelog_1_0.add_change(ApiChange::new(
            ChangeType::Feature,
            "Core API".to_string(),
            "Initial stable release with full API coverage".to_string(),
        ));
        changelog_1_0.add_change(ApiChange::new(
            ChangeType::Feature,
            "Language Bindings".to_string(),
            "Added Node.js, Python, and Flutter bindings".to_string(),
        ));
        self.add_changelog(changelog_1_0);
        
        // Add changelog for 0.9.0
        let mut changelog_0_9 = Changelog::new(v0_9_0.clone());
        changelog_0_9.add_change(ApiChange::new(
            ChangeType::Feature,
            "Core API".to_string(),
            "Beta release with core functionality".to_string(),
        ));
        self.add_changelog(changelog_0_9);
        
        // Add compatibility entries
        let mut compat_1_0 = CompatibilityEntry::new(semver::Version::new(1, 0, 0));
        compat_1_0.add_binding("rust".to_string(), semver::Version::new(1, 0, 0));
        compat_1_0.add_binding("nodejs".to_string(), semver::Version::new(1, 0, 0));
        compat_1_0.add_binding("python".to_string(), semver::Version::new(1, 0, 0));
        compat_1_0.add_binding("flutter".to_string(), semver::Version::new(1, 0, 0));
        self.add_compatibility(compat_1_0);
        
        Ok(())
    }
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_doc_version() {
        let version = DocVersion::new(
            semver::Version::new(1, 0, 0),
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        );
        
        assert_eq!(version.version_string(), "1.0.0");
        assert!(!version.is_latest);
    }
    
    #[test]
    fn test_api_change() {
        let change = ApiChange::new(
            ChangeType::Feature,
            "Core".to_string(),
            "New feature".to_string(),
        );
        
        let md = change.to_markdown();
        assert!(md.contains("New Feature"));
        assert!(md.contains("Core"));
    }
    
    #[test]
    fn test_version_manager() {
        let mut manager = VersionManager::new();
        
        let version = DocVersion::new(
            semver::Version::new(1, 0, 0),
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        ).mark_as_latest();
        
        manager.add_version(version);
        
        assert!(manager.get_latest().is_some());
        assert_eq!(manager.get_all_versions().len(), 1);
    }
    
    #[test]
    fn test_changelog_generation() {
        let version = DocVersion::new(
            semver::Version::new(1, 0, 0),
            chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        );
        
        let mut changelog = Changelog::new(version);
        changelog.add_change(ApiChange::new(
            ChangeType::Feature,
            "Core".to_string(),
            "New feature".to_string(),
        ));
        
        let md = changelog.to_markdown();
        assert!(md.contains("Version 1.0.0"));
        assert!(md.contains("New Features"));
    }
}
