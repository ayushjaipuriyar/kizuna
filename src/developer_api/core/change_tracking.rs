/// API change tracking and documentation system
use super::Result;
use semver::Version;
use std::collections::HashMap;
use std::fmt;

/// Type of API change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeType {
    /// Breaking change - requires code changes
    Breaking,
    
    /// New feature added
    Feature,
    
    /// Bug fix
    Fix,
    
    /// Deprecation announcement
    Deprecation,
    
    /// Performance improvement
    Performance,
    
    /// Documentation update
    Documentation,
    
    /// Internal refactoring (no API changes)
    Internal,
}

impl ChangeType {
    /// Gets the change type name
    pub fn name(&self) -> &str {
        match self {
            Self::Breaking => "Breaking Change",
            Self::Feature => "New Feature",
            Self::Fix => "Bug Fix",
            Self::Deprecation => "Deprecation",
            Self::Performance => "Performance",
            Self::Documentation => "Documentation",
            Self::Internal => "Internal",
        }
    }
    
    /// Gets the emoji for this change type
    pub fn emoji(&self) -> &str {
        match self {
            Self::Breaking => "⚠️",
            Self::Feature => "✨",
            Self::Fix => "🐛",
            Self::Deprecation => "⚰️",
            Self::Performance => "⚡",
            Self::Documentation => "📝",
            Self::Internal => "🔧",
        }
    }
    
    /// Checks if this change type affects API compatibility
    pub fn affects_compatibility(&self) -> bool {
        matches!(self, Self::Breaking | Self::Deprecation)
    }
}

impl fmt::Display for ChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// API change entry
#[derive(Debug, Clone)]
pub struct ApiChange {
    /// Unique change ID
    pub id: String,
    
    /// Change type
    pub change_type: ChangeType,
    
    /// Component affected (e.g., "Core API", "Node.js Binding")
    pub component: String,
    
    /// API element affected (e.g., function name, type name)
    pub element: Option<String>,
    
    /// Description of the change
    pub description: String,
    
    /// Migration guide (for breaking changes)
    pub migration_guide: Option<String>,
    
    /// Code example (before)
    pub code_before: Option<String>,
    
    /// Code example (after)
    pub code_after: Option<String>,
    
    /// Related issue/PR numbers
    pub references: Vec<String>,
    
    /// Author of the change
    pub author: Option<String>,
    
    /// Date of the change
    pub date: chrono::NaiveDate,
}

impl ApiChange {
    /// Creates a new API change
    pub fn new<S: Into<String>>(
        id: S,
        change_type: ChangeType,
        component: S,
        description: S,
    ) -> Self {
        Self {
            id: id.into(),
            change_type,
            component: component.into(),
            element: None,
            description: description.into(),
            migration_guide: None,
            code_before: None,
            code_after: None,
            references: Vec::new(),
            author: None,
            date: chrono::Utc::now().date_naive(),
        }
    }
    
    /// Sets the affected element
    pub fn with_element<S: Into<String>>(mut self, element: S) -> Self {
        self.element = Some(element.into());
        self
    }
    
    /// Adds a migration guide
    pub fn with_migration_guide<S: Into<String>>(mut self, guide: S) -> Self {
        self.migration_guide = Some(guide.into());
        self
    }
    
    /// Adds code examples
    pub fn with_code_examples<S: Into<String>>(mut self, before: S, after: S) -> Self {
        self.code_before = Some(before.into());
        self.code_after = Some(after.into());
        self
    }
    
    /// Adds a reference (issue/PR number)
    pub fn add_reference<S: Into<String>>(mut self, reference: S) -> Self {
        self.references.push(reference.into());
        self
    }
    
    /// Sets the author
    pub fn with_author<S: Into<String>>(mut self, author: S) -> Self {
        self.author = Some(author.into());
        self
    }
    
    /// Sets the date
    pub fn with_date(mut self, date: chrono::NaiveDate) -> Self {
        self.date = date;
        self
    }
    
    /// Formats the change as markdown
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str(&format!(
            "- {} **{}**",
            self.change_type.emoji(),
            self.change_type.name()
        ));
        
        if let Some(element) = &self.element {
            md.push_str(&format!(" `{}`", element));
        }
        
        md.push_str(&format!(" ({}): {}\n", self.component, self.description));
        
        if !self.references.is_empty() {
            md.push_str(&format!("  - References: {}\n", self.references.join(", ")));
        }
        
        if let Some(guide) = &self.migration_guide {
            md.push_str(&format!("  - Migration: {}\n", guide));
        }
        
        if let (Some(before), Some(after)) = (&self.code_before, &self.code_after) {
            md.push_str("  - Before:\n    ```rust\n    ");
            md.push_str(&before.replace('\n', "\n    "));
            md.push_str("\n    ```\n");
            md.push_str("  - After:\n    ```rust\n    ");
            md.push_str(&after.replace('\n', "\n    "));
            md.push_str("\n    ```\n");
        }
        
        md
    }
}

/// Changelog for a version
#[derive(Debug, Clone)]
pub struct Changelog {
    /// Version
    pub version: Version,
    
    /// Release date
    pub release_date: chrono::NaiveDate,
    
    /// Is this the latest version?
    pub is_latest: bool,
    
    /// Changes in this version
    pub changes: Vec<ApiChange>,
    
    /// Additional release notes
    pub notes: Option<String>,
}

impl Changelog {
    /// Creates a new changelog
    pub fn new(version: Version) -> Self {
        Self {
            version,
            release_date: chrono::Utc::now().date_naive(),
            is_latest: false,
            changes: Vec::new(),
            notes: None,
        }
    }
    
    /// Marks as latest
    pub fn mark_as_latest(mut self) -> Self {
        self.is_latest = true;
        self
    }
    
    /// Sets the release date
    pub fn with_release_date(mut self, date: chrono::NaiveDate) -> Self {
        self.release_date = date;
        self
    }
    
    /// Adds a change
    pub fn add_change(&mut self, change: ApiChange) {
        self.changes.push(change);
    }
    
    /// Sets release notes
    pub fn with_notes<S: Into<String>>(mut self, notes: S) -> Self {
        self.notes = Some(notes.into());
        self
    }
    
    /// Gets changes by type
    pub fn get_changes_by_type(&self, change_type: ChangeType) -> Vec<&ApiChange> {
        self.changes
            .iter()
            .filter(|c| c.change_type == change_type)
            .collect()
    }
    
    /// Checks if this version has breaking changes
    pub fn has_breaking_changes(&self) -> bool {
        self.changes.iter().any(|c| c.change_type == ChangeType::Breaking)
    }
    
    /// Generates markdown changelog
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("## Version {} ({})\n\n", 
            self.version,
            self.release_date.format("%Y-%m-%d")
        ));
        
        if self.is_latest {
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
        let mut performance = Vec::new();
        let mut docs = Vec::new();
        let mut internal = Vec::new();
        
        for change in &self.changes {
            match change.change_type {
                ChangeType::Breaking => breaking.push(change),
                ChangeType::Feature => features.push(change),
                ChangeType::Fix => fixes.push(change),
                ChangeType::Deprecation => deprecations.push(change),
                ChangeType::Performance => performance.push(change),
                ChangeType::Documentation => docs.push(change),
                ChangeType::Internal => internal.push(change),
            }
        }
        
        if !breaking.is_empty() {
            md.push_str("### ⚠️ Breaking Changes\n\n");
            for change in breaking {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        if !features.is_empty() {
            md.push_str("### ✨ New Features\n\n");
            for change in features {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        if !deprecations.is_empty() {
            md.push_str("### ⚰️ Deprecations\n\n");
            for change in deprecations {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        if !performance.is_empty() {
            md.push_str("### ⚡ Performance Improvements\n\n");
            for change in performance {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        if !fixes.is_empty() {
            md.push_str("### 🐛 Bug Fixes\n\n");
            for change in fixes {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        if !docs.is_empty() {
            md.push_str("### 📝 Documentation\n\n");
            for change in docs {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        if !internal.is_empty() {
            md.push_str("### 🔧 Internal Changes\n\n");
            for change in internal {
                md.push_str(&change.to_markdown());
            }
            md.push('\n');
        }
        
        md
    }
}

/// Compatibility matrix entry
#[derive(Debug, Clone)]
pub struct CompatibilityMatrixEntry {
    /// Core API version
    pub core_version: Version,
    
    /// Language binding versions
    pub binding_versions: HashMap<String, Version>,
    
    /// Platform compatibility
    pub platforms: Vec<String>,
    
    /// Minimum system requirements
    pub requirements: Option<String>,
    
    /// Compatibility notes
    pub notes: Option<String>,
}

impl CompatibilityMatrixEntry {
    /// Creates a new compatibility matrix entry
    pub fn new(core_version: Version) -> Self {
        Self {
            core_version,
            binding_versions: HashMap::new(),
            platforms: Vec::new(),
            requirements: None,
            notes: None,
        }
    }
    
    /// Adds a binding version
    pub fn add_binding<S: Into<String>>(mut self, language: S, version: Version) -> Self {
        self.binding_versions.insert(language.into(), version);
        self
    }
    
    /// Adds a platform
    pub fn add_platform<S: Into<String>>(mut self, platform: S) -> Self {
        self.platforms.push(platform.into());
        self
    }
    
    /// Sets requirements
    pub fn with_requirements<S: Into<String>>(mut self, requirements: S) -> Self {
        self.requirements = Some(requirements.into());
        self
    }
    
    /// Sets notes
    pub fn with_notes<S: Into<String>>(mut self, notes: S) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Change tracker for API changes
pub struct ChangeTracker {
    /// All changelogs by version
    changelogs: HashMap<Version, Changelog>,
    
    /// All changes by ID
    changes: HashMap<String, ApiChange>,
    
    /// Compatibility matrix
    compatibility_matrix: Vec<CompatibilityMatrixEntry>,
    
    /// Current version
    current_version: Version,
}

impl ChangeTracker {
    /// Creates a new change tracker
    pub fn new(current_version: Version) -> Self {
        Self {
            changelogs: HashMap::new(),
            changes: HashMap::new(),
            compatibility_matrix: Vec::new(),
            current_version,
        }
    }
    
    /// Adds a changelog
    pub fn add_changelog(&mut self, changelog: Changelog) {
        // Mark previous latest as not latest
        for cl in self.changelogs.values_mut() {
            if cl.is_latest {
                cl.is_latest = false;
            }
        }
        
        // Add all changes to the changes map
        for change in &changelog.changes {
            self.changes.insert(change.id.clone(), change.clone());
        }
        
        self.changelogs.insert(changelog.version.clone(), changelog);
    }
    
    /// Gets a changelog by version
    pub fn get_changelog(&self, version: &Version) -> Option<&Changelog> {
        self.changelogs.get(version)
    }
    
    /// Gets all changelogs sorted by version (newest first)
    pub fn get_all_changelogs(&self) -> Vec<&Changelog> {
        let mut changelogs: Vec<_> = self.changelogs.values().collect();
        changelogs.sort_by(|a, b| b.version.cmp(&a.version));
        changelogs
    }
    
    /// Gets a change by ID
    pub fn get_change(&self, id: &str) -> Option<&ApiChange> {
        self.changes.get(id)
    }
    
    /// Gets all changes of a specific type
    pub fn get_changes_by_type(&self, change_type: ChangeType) -> Vec<&ApiChange> {
        self.changes
            .values()
            .filter(|c| c.change_type == change_type)
            .collect()
    }
    
    /// Gets all breaking changes
    pub fn get_breaking_changes(&self) -> Vec<&ApiChange> {
        self.get_changes_by_type(ChangeType::Breaking)
    }
    
    /// Adds a compatibility matrix entry
    pub fn add_compatibility_entry(&mut self, entry: CompatibilityMatrixEntry) {
        self.compatibility_matrix.push(entry);
        self.compatibility_matrix.sort_by(|a, b| b.core_version.cmp(&a.core_version));
    }
    
    /// Gets compatibility matrix entry for a version
    pub fn get_compatibility_entry(&self, version: &Version) -> Option<&CompatibilityMatrixEntry> {
        self.compatibility_matrix
            .iter()
            .find(|e| &e.core_version == version)
    }
    
    /// Generates full changelog document
    pub fn generate_changelog_document(&self) -> String {
        let mut doc = String::new();
        
        doc.push_str("# Kizuna API Changelog\n\n");
        doc.push_str("All notable changes to the Kizuna API are documented in this file.\n\n");
        doc.push_str("The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),\n");
        doc.push_str("and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n");
        
        for changelog in self.get_all_changelogs() {
            doc.push_str(&changelog.to_markdown());
            doc.push_str("---\n\n");
        }
        
        doc
    }
    
    /// Generates compatibility matrix document
    pub fn generate_compatibility_matrix(&self) -> String {
        let mut doc = String::new();
        
        doc.push_str("# Kizuna API Compatibility Matrix\n\n");
        doc.push_str("This document shows the compatibility between Kizuna core API versions ");
        doc.push_str("and language binding versions.\n\n");
        
        if self.compatibility_matrix.is_empty() {
            doc.push_str("No compatibility information available.\n");
            return doc;
        }
        
        // Generate table header
        doc.push_str("| Core API | Rust | Node.js | Python | Flutter | Platforms |\n");
        doc.push_str("|----------|------|---------|--------|---------|----------|\n");
        
        for entry in &self.compatibility_matrix {
            doc.push_str(&format!("| {} ", entry.core_version));
            
            for lang in &["rust", "nodejs", "python", "flutter"] {
                if let Some(version) = entry.binding_versions.get(*lang) {
                    doc.push_str(&format!("| {} ", version));
                } else {
                    doc.push_str("| - ");
                }
            }
            
            doc.push_str(&format!("| {} |\n", entry.platforms.join(", ")));
        }
        
        doc.push_str("\n");
        
        // Add notes if any
        for entry in &self.compatibility_matrix {
            if entry.notes.is_some() || entry.requirements.is_some() {
                doc.push_str(&format!("### Version {}\n\n", entry.core_version));
                
                if let Some(req) = &entry.requirements {
                    doc.push_str(&format!("**Requirements:** {}\n\n", req));
                }
                
                if let Some(notes) = &entry.notes {
                    doc.push_str(&format!("**Notes:** {}\n\n", notes));
                }
            }
        }
        
        doc
    }
    
    /// Detects changes between two versions
    pub fn detect_changes(&self, from: &Version, to: &Version) -> Result<Vec<&ApiChange>> {
        let mut changes = Vec::new();
        
        // Get all versions between from and to
        let mut versions: Vec<_> = self.changelogs.keys().collect();
        versions.sort();
        
        for version in versions {
            if version > from && version <= to {
                if let Some(changelog) = self.changelogs.get(version) {
                    changes.extend(changelog.changes.iter());
                }
            }
        }
        
        Ok(changes)
    }
    
    /// Validates changelog consistency
    pub fn validate(&self) -> Result<Vec<String>> {
        let mut warnings = Vec::new();
        
        // Check that all versions have changelogs
        if self.changelogs.is_empty() {
            warnings.push("No changelogs found".to_string());
        }
        
        // Check that there's exactly one latest version
        let latest_count = self.changelogs.values().filter(|c| c.is_latest).count();
        if latest_count == 0 {
            warnings.push("No version marked as latest".to_string());
        } else if latest_count > 1 {
            warnings.push("Multiple versions marked as latest".to_string());
        }
        
        // Check for duplicate change IDs
        let mut seen_ids = std::collections::HashSet::new();
        for change in self.changes.values() {
            if !seen_ids.insert(&change.id) {
                warnings.push(format!("Duplicate change ID: {}", change.id));
            }
        }
        
        // Check that breaking changes have migration guides
        for change in self.get_breaking_changes() {
            if change.migration_guide.is_none() {
                warnings.push(format!(
                    "Breaking change '{}' is missing a migration guide",
                    change.id
                ));
            }
        }
        
        Ok(warnings)
    }
}

impl Default for ChangeTracker {
    fn default() -> Self {
        let version = Version::parse(env!("CARGO_PKG_VERSION"))
            .unwrap_or_else(|_| Version::new(0, 1, 0));
        Self::new(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_change_type() {
        assert_eq!(ChangeType::Breaking.name(), "Breaking Change");
        assert_eq!(ChangeType::Feature.emoji(), "✨");
        assert!(ChangeType::Breaking.affects_compatibility());
        assert!(!ChangeType::Feature.affects_compatibility());
    }
    
    #[test]
    fn test_api_change() {
        let change = ApiChange::new(
            "change-001",
            ChangeType::Feature,
            "Core API",
            "Added new feature",
        )
        .with_element("new_function")
        .add_reference("#123");
        
        assert_eq!(change.id, "change-001");
        assert_eq!(change.element, Some("new_function".to_string()));
        assert_eq!(change.references.len(), 1);
    }
    
    #[test]
    fn test_changelog() {
        let mut changelog = Changelog::new(Version::new(1, 0, 0));
        
        changelog.add_change(ApiChange::new(
            "change-001",
            ChangeType::Feature,
            "Core",
            "New feature",
        ));
        
        assert_eq!(changelog.changes.len(), 1);
        assert!(!changelog.has_breaking_changes());
    }
    
    #[test]
    fn test_changelog_with_breaking_changes() {
        let mut changelog = Changelog::new(Version::new(2, 0, 0));
        
        changelog.add_change(ApiChange::new(
            "change-001",
            ChangeType::Breaking,
            "Core",
            "Breaking change",
        ));
        
        assert!(changelog.has_breaking_changes());
    }
    
    #[test]
    fn test_change_tracker() {
        let mut tracker = ChangeTracker::new(Version::new(1, 0, 0));
        
        let mut changelog = Changelog::new(Version::new(1, 0, 0));
        changelog.add_change(ApiChange::new(
            "change-001",
            ChangeType::Feature,
            "Core",
            "New feature",
        ));
        
        tracker.add_changelog(changelog);
        
        assert_eq!(tracker.get_all_changelogs().len(), 1);
        assert!(tracker.get_change("change-001").is_some());
    }
    
    #[test]
    fn test_compatibility_matrix_entry() {
        let entry = CompatibilityMatrixEntry::new(Version::new(1, 0, 0))
            .add_binding("rust", Version::new(1, 0, 0))
            .add_binding("nodejs", Version::new(1, 0, 0))
            .add_platform("Linux")
            .add_platform("macOS");
        
        assert_eq!(entry.binding_versions.len(), 2);
        assert_eq!(entry.platforms.len(), 2);
    }
    
    #[test]
    fn test_change_detection() {
        let mut tracker = ChangeTracker::new(Version::new(1, 2, 0));
        
        let mut changelog_1_0 = Changelog::new(Version::new(1, 0, 0));
        changelog_1_0.add_change(ApiChange::new(
            "change-001",
            ChangeType::Feature,
            "Core",
            "Feature 1",
        ));
        tracker.add_changelog(changelog_1_0);
        
        let mut changelog_1_1 = Changelog::new(Version::new(1, 1, 0));
        changelog_1_1.add_change(ApiChange::new(
            "change-002",
            ChangeType::Feature,
            "Core",
            "Feature 2",
        ));
        tracker.add_changelog(changelog_1_1);
        
        let changes = tracker.detect_changes(
            &Version::new(0, 9, 0),
            &Version::new(1, 1, 0)
        ).unwrap();
        
        assert_eq!(changes.len(), 2);
    }
}
