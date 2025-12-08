/// API deprecation policies and migration support system
use super::{KizunaError, Result};
use semver::Version;
use std::collections::HashMap;
use std::fmt;

/// Deprecation status of an API element
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeprecationStatus {
    /// API is active and not deprecated
    Active,
    
    /// API is soft deprecated - warning issued but still functional
    SoftDeprecated,
    
    /// API is hard deprecated - will be removed in next major version
    HardDeprecated,
    
    /// API has been removed
    Removed,
}

impl fmt::Display for DeprecationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::SoftDeprecated => write!(f, "Soft Deprecated"),
            Self::HardDeprecated => write!(f, "Hard Deprecated"),
            Self::Removed => write!(f, "Removed"),
        }
    }
}

/// Deprecation warning level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningLevel {
    /// Informational - no action required yet
    Info,
    
    /// Warning - action recommended
    Warning,
    
    /// Error - action required
    Error,
}

/// Deprecation information for an API element
#[derive(Debug, Clone)]
pub struct DeprecationInfo {
    /// API element identifier (function name, type name, etc.)
    pub element: String,
    
    /// Current deprecation status
    pub status: DeprecationStatus,
    
    /// Version when deprecation was announced
    pub deprecated_since: Version,
    
    /// Version when API will be removed (if known)
    pub removal_version: Option<Version>,
    
    /// Reason for deprecation
    pub reason: String,
    
    /// Replacement API (if any)
    pub replacement: Option<String>,
    
    /// Migration guide
    pub migration_guide: Option<String>,
    
    /// Warning level
    pub warning_level: WarningLevel,
}

impl DeprecationInfo {
    /// Creates a new deprecation info
    pub fn new<S: Into<String>>(
        element: S,
        deprecated_since: Version,
        reason: S,
    ) -> Self {
        Self {
            element: element.into(),
            status: DeprecationStatus::SoftDeprecated,
            deprecated_since,
            removal_version: None,
            reason: reason.into(),
            replacement: None,
            migration_guide: None,
            warning_level: WarningLevel::Warning,
        }
    }
    
    /// Sets the removal version
    pub fn with_removal_version(mut self, version: Version) -> Self {
        self.removal_version = Some(version);
        self.status = DeprecationStatus::HardDeprecated;
        self.warning_level = WarningLevel::Error;
        self
    }
    
    /// Sets the replacement API
    pub fn with_replacement<S: Into<String>>(mut self, replacement: S) -> Self {
        self.replacement = Some(replacement.into());
        self
    }
    
    /// Sets the migration guide
    pub fn with_migration_guide<S: Into<String>>(mut self, guide: S) -> Self {
        self.migration_guide = Some(guide.into());
        self
    }
    
    /// Sets the warning level
    pub fn with_warning_level(mut self, level: WarningLevel) -> Self {
        self.warning_level = level;
        self
    }
    
    /// Marks as removed
    pub fn mark_as_removed(mut self) -> Self {
        self.status = DeprecationStatus::Removed;
        self.warning_level = WarningLevel::Error;
        self
    }
    
    /// Generates a deprecation warning message
    pub fn warning_message(&self) -> String {
        let mut msg = format!(
            "[{}] '{}' is {} (since v{})",
            self.warning_level_emoji(),
            self.element,
            self.status,
            self.deprecated_since
        );
        
        if let Some(removal) = &self.removal_version {
            msg.push_str(&format!(" and will be removed in v{}", removal));
        }
        
        msg.push_str(&format!(": {}", self.reason));
        
        if let Some(replacement) = &self.replacement {
            msg.push_str(&format!("\n  → Use '{}' instead", replacement));
        }
        
        if let Some(guide) = &self.migration_guide {
            msg.push_str(&format!("\n  → Migration: {}", guide));
        }
        
        msg
    }
    
    /// Gets the emoji for the warning level
    fn warning_level_emoji(&self) -> &str {
        match self.warning_level {
            WarningLevel::Info => "ℹ️",
            WarningLevel::Warning => "⚠️",
            WarningLevel::Error => "❌",
        }
    }
    
    /// Checks if this deprecation should block usage
    pub fn should_block(&self) -> bool {
        self.status == DeprecationStatus::Removed
    }
}

/// Migration step for upgrading between versions
#[derive(Debug, Clone)]
pub struct MigrationStep {
    /// Step number
    pub step_number: usize,
    
    /// Step title
    pub title: String,
    
    /// Detailed description
    pub description: String,
    
    /// Code example (before)
    pub code_before: Option<String>,
    
    /// Code example (after)
    pub code_after: Option<String>,
    
    /// Is this step required?
    pub required: bool,
    
    /// Estimated effort (in minutes)
    pub estimated_effort: Option<u32>,
}

impl MigrationStep {
    /// Creates a new migration step
    pub fn new<S: Into<String>>(step_number: usize, title: S, description: S) -> Self {
        Self {
            step_number,
            title: title.into(),
            description: description.into(),
            code_before: None,
            code_after: None,
            required: true,
            estimated_effort: None,
        }
    }
    
    /// Adds code examples
    pub fn with_code_examples<S: Into<String>>(mut self, before: S, after: S) -> Self {
        self.code_before = Some(before.into());
        self.code_after = Some(after.into());
        self
    }
    
    /// Marks as optional
    pub fn mark_as_optional(mut self) -> Self {
        self.required = false;
        self
    }
    
    /// Sets estimated effort
    pub fn with_effort(mut self, minutes: u32) -> Self {
        self.estimated_effort = Some(minutes);
        self
    }
    
    /// Formats as markdown
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str(&format!(
            "### Step {}: {}{}\n\n",
            self.step_number,
            self.title,
            if self.required { "" } else { " (Optional)" }
        ));
        
        md.push_str(&format!("{}\n\n", self.description));
        
        if let Some(effort) = self.estimated_effort {
            md.push_str(&format!("**Estimated effort:** {} minutes\n\n", effort));
        }
        
        if let (Some(before), Some(after)) = (&self.code_before, &self.code_after) {
            md.push_str("**Before:**\n```rust\n");
            md.push_str(before);
            md.push_str("\n```\n\n");
            
            md.push_str("**After:**\n```rust\n");
            md.push_str(after);
            md.push_str("\n```\n\n");
        }
        
        md
    }
}

/// Migration guide for upgrading between versions
#[derive(Debug, Clone)]
pub struct MigrationGuide {
    /// Source version
    pub from_version: Version,
    
    /// Target version
    pub to_version: Version,
    
    /// Guide title
    pub title: String,
    
    /// Overview
    pub overview: String,
    
    /// Migration steps
    pub steps: Vec<MigrationStep>,
    
    /// Breaking changes summary
    pub breaking_changes: Vec<String>,
    
    /// Total estimated effort (in minutes)
    pub total_effort: Option<u32>,
}

impl MigrationGuide {
    /// Creates a new migration guide
    pub fn new<S: Into<String>>(
        from: Version,
        to: Version,
        title: S,
        overview: S,
    ) -> Self {
        Self {
            from_version: from,
            to_version: to,
            title: title.into(),
            overview: overview.into(),
            steps: Vec::new(),
            breaking_changes: Vec::new(),
            total_effort: None,
        }
    }
    
    /// Adds a migration step
    pub fn add_step(&mut self, step: MigrationStep) {
        self.steps.push(step);
        self.recalculate_effort();
    }
    
    /// Adds a breaking change
    pub fn add_breaking_change<S: Into<String>>(&mut self, change: S) {
        self.breaking_changes.push(change.into());
    }
    
    /// Recalculates total effort
    fn recalculate_effort(&mut self) {
        let total: u32 = self.steps
            .iter()
            .filter_map(|s| s.estimated_effort)
            .sum();
        
        if total > 0 {
            self.total_effort = Some(total);
        }
    }
    
    /// Generates markdown documentation
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("# {}\n\n", self.title));
        md.push_str(&format!(
            "Migration guide from version {} to {}\n\n",
            self.from_version, self.to_version
        ));
        
        if let Some(effort) = self.total_effort {
            md.push_str(&format!("**Total estimated effort:** {} minutes\n\n", effort));
        }
        
        md.push_str("## Overview\n\n");
        md.push_str(&format!("{}\n\n", self.overview));
        
        if !self.breaking_changes.is_empty() {
            md.push_str("## Breaking Changes\n\n");
            for change in &self.breaking_changes {
                md.push_str(&format!("- {}\n", change));
            }
            md.push_str("\n");
        }
        
        md.push_str("## Migration Steps\n\n");
        for step in &self.steps {
            md.push_str(&step.to_markdown());
        }
        
        md
    }
}

/// Deprecation manager for tracking and warning about deprecated APIs
pub struct DeprecationManager {
    /// All deprecation information
    deprecations: HashMap<String, DeprecationInfo>,
    
    /// Migration guides
    migration_guides: HashMap<(Version, Version), MigrationGuide>,
    
    /// Current API version
    current_version: Version,
    
    /// Should warnings be emitted?
    emit_warnings: bool,
}

impl DeprecationManager {
    /// Creates a new deprecation manager
    pub fn new(current_version: Version) -> Self {
        Self {
            deprecations: HashMap::new(),
            migration_guides: HashMap::new(),
            current_version,
            emit_warnings: true,
        }
    }
    
    /// Registers a deprecation
    pub fn register_deprecation(&mut self, info: DeprecationInfo) {
        self.deprecations.insert(info.element.clone(), info);
    }
    
    /// Checks if an API element is deprecated
    pub fn is_deprecated(&self, element: &str) -> bool {
        self.deprecations.contains_key(element)
    }
    
    /// Gets deprecation info for an element
    pub fn get_deprecation_info(&self, element: &str) -> Option<&DeprecationInfo> {
        self.deprecations.get(element)
    }
    
    /// Checks if an API element should be blocked
    pub fn should_block(&self, element: &str) -> bool {
        self.deprecations
            .get(element)
            .map(|info| info.should_block())
            .unwrap_or(false)
    }
    
    /// Emits a deprecation warning
    pub fn warn(&self, element: &str) -> Result<()> {
        if !self.emit_warnings {
            return Ok(());
        }
        
        if let Some(info) = self.deprecations.get(element) {
            if info.should_block() {
                return Err(KizunaError::other(format!(
                    "API '{}' has been removed and is no longer available",
                    element
                )));
            }
            
            eprintln!("{}", info.warning_message());
        }
        
        Ok(())
    }
    
    /// Registers a migration guide
    pub fn register_migration_guide(&mut self, guide: MigrationGuide) {
        let key = (guide.from_version.clone(), guide.to_version.clone());
        self.migration_guides.insert(key, guide);
    }
    
    /// Gets a migration guide
    pub fn get_migration_guide(&self, from: &Version, to: &Version) -> Option<&MigrationGuide> {
        self.migration_guides.get(&(from.clone(), to.clone()))
    }
    
    /// Gets all migration guides for a source version
    pub fn get_migration_guides_from(&self, from: &Version) -> Vec<&MigrationGuide> {
        self.migration_guides
            .values()
            .filter(|g| &g.from_version == from)
            .collect()
    }
    
    /// Enables or disables warning emission
    pub fn set_emit_warnings(&mut self, emit: bool) {
        self.emit_warnings = emit;
    }
    
    /// Gets all deprecated elements
    pub fn get_all_deprecations(&self) -> Vec<&DeprecationInfo> {
        self.deprecations.values().collect()
    }
    
    /// Generates a deprecation report
    pub fn generate_deprecation_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("# API Deprecation Report\n\n");
        report.push_str(&format!("Current API Version: {}\n\n", self.current_version));
        
        // Group by status
        let mut soft_deprecated = Vec::new();
        let mut hard_deprecated = Vec::new();
        let mut removed = Vec::new();
        
        for info in self.deprecations.values() {
            match info.status {
                DeprecationStatus::SoftDeprecated => soft_deprecated.push(info),
                DeprecationStatus::HardDeprecated => hard_deprecated.push(info),
                DeprecationStatus::Removed => removed.push(info),
                DeprecationStatus::Active => {}
            }
        }
        
        if !hard_deprecated.is_empty() {
            report.push_str("## Hard Deprecated (Removal Scheduled)\n\n");
            for info in hard_deprecated {
                report.push_str(&format!("- **{}**: {}\n", info.element, info.reason));
                if let Some(removal) = &info.removal_version {
                    report.push_str(&format!("  - Removal: v{}\n", removal));
                }
                if let Some(replacement) = &info.replacement {
                    report.push_str(&format!("  - Replacement: {}\n", replacement));
                }
            }
            report.push_str("\n");
        }
        
        if !soft_deprecated.is_empty() {
            report.push_str("## Soft Deprecated\n\n");
            for info in soft_deprecated {
                report.push_str(&format!("- **{}**: {}\n", info.element, info.reason));
                if let Some(replacement) = &info.replacement {
                    report.push_str(&format!("  - Replacement: {}\n", replacement));
                }
            }
            report.push_str("\n");
        }
        
        if !removed.is_empty() {
            report.push_str("## Removed\n\n");
            for info in removed {
                report.push_str(&format!("- **{}**: {}\n", info.element, info.reason));
            }
            report.push_str("\n");
        }
        
        report
    }
    
    /// Generates migration guide documentation
    pub fn generate_migration_docs(&self) -> String {
        let mut docs = String::new();
        
        docs.push_str("# Migration Guides\n\n");
        
        let mut guides: Vec<_> = self.migration_guides.values().collect();
        guides.sort_by(|a, b| {
            b.from_version.cmp(&a.from_version)
                .then(b.to_version.cmp(&a.to_version))
        });
        
        for guide in guides {
            docs.push_str(&guide.to_markdown());
            docs.push_str("\n---\n\n");
        }
        
        docs
    }
}

impl Default for DeprecationManager {
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
    fn test_deprecation_info() {
        let info = DeprecationInfo::new(
            "old_function",
            Version::new(1, 0, 0),
            "Use new_function instead",
        )
        .with_replacement("new_function");
        
        assert_eq!(info.status, DeprecationStatus::SoftDeprecated);
        assert_eq!(info.replacement, Some("new_function".to_string()));
    }
    
    #[test]
    fn test_deprecation_warning_message() {
        let info = DeprecationInfo::new(
            "old_api",
            Version::new(1, 0, 0),
            "Replaced by better API",
        )
        .with_replacement("new_api")
        .with_removal_version(Version::new(2, 0, 0));
        
        let msg = info.warning_message();
        assert!(msg.contains("old_api"));
        assert!(msg.contains("new_api"));
        assert!(msg.contains("2.0.0"));
    }
    
    #[test]
    fn test_migration_step() {
        let step = MigrationStep::new(
            1,
            "Update imports",
            "Change old imports to new module structure",
        )
        .with_effort(5);
        
        assert_eq!(step.step_number, 1);
        assert_eq!(step.estimated_effort, Some(5));
    }
    
    #[test]
    fn test_migration_guide() {
        let mut guide = MigrationGuide::new(
            Version::new(1, 0, 0),
            Version::new(2, 0, 0),
            "v1 to v2 Migration",
            "Major version upgrade with breaking changes",
        );
        
        guide.add_step(MigrationStep::new(
            1,
            "Update config",
            "Update configuration format",
        ).with_effort(10));
        
        assert_eq!(guide.steps.len(), 1);
        assert_eq!(guide.total_effort, Some(10));
    }
    
    #[test]
    fn test_deprecation_manager() {
        let mut manager = DeprecationManager::new(Version::new(1, 0, 0));
        
        let info = DeprecationInfo::new(
            "old_function",
            Version::new(1, 0, 0),
            "Use new_function",
        );
        
        manager.register_deprecation(info);
        
        assert!(manager.is_deprecated("old_function"));
        assert!(!manager.is_deprecated("new_function"));
    }
    
    #[test]
    fn test_deprecation_blocking() {
        let mut manager = DeprecationManager::new(Version::new(2, 0, 0));
        
        let info = DeprecationInfo::new(
            "removed_api",
            Version::new(1, 0, 0),
            "API removed",
        )
        .mark_as_removed();
        
        manager.register_deprecation(info);
        
        assert!(manager.should_block("removed_api"));
        assert!(manager.warn("removed_api").is_err());
    }
}
