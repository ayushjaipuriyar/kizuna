/// Integration tests for the versioning system
#[cfg(test)]
mod integration_tests {
    use super::super::{
        IntegratedVersionManager, ApiVersion, DeprecationInfo, MigrationGuide, MigrationStep,
        Changelog, ApiChange, ChangeType, CompatibilityMatrixEntry,
    };
    use semver::Version;
    
    #[test]
    fn test_complete_versioning_workflow() {
        // Create manager
        let mut manager = IntegratedVersionManager::new(Version::new(1, 2, 0));
        
        // Register versions
        let v1_0 = ApiVersion::new(1, 0, 0)
            .with_release_date(chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        manager.register_version(v1_0);
        
        let v1_2 = ApiVersion::new(1, 2, 0)
            .mark_as_latest()
            .with_release_date(chrono::NaiveDate::from_ymd_opt(2024, 5, 15).unwrap());
        manager.register_version(v1_2);
        
        // Add changelog
        let mut changelog = Changelog::new(Version::new(1, 2, 0))
            .mark_as_latest()
            .with_notes("Test release");
        
        changelog.add_change(ApiChange::new(
            "test-001",
            ChangeType::Feature,
            "Core",
            "Test feature",
        ));
        
        manager.add_changelog(changelog);
        
        // Register deprecation
        let deprecation = DeprecationInfo::new(
            "old_function",
            Version::new(1, 2, 0),
            "Test deprecation",
        )
        .with_replacement("new_function");
        
        manager.register_deprecation(deprecation);
        
        // Verify
        assert_eq!(manager.current_version(), &Version::new(1, 2, 0));
        assert!(manager.is_deprecated("old_function"));
        assert!(manager.get_changelog(&Version::new(1, 2, 0)).is_some());
    }
    
    #[test]
    fn test_compatibility_checking() {
        let manager = IntegratedVersionManager::new(Version::new(1, 0, 0));
        
        let v1_0 = Version::new(1, 0, 0);
        let v1_1 = Version::new(1, 1, 0);
        let v2_0 = Version::new(2, 0, 0);
        
        // Same major version should be backward compatible
        let check = manager.check_compatibility(&v1_0, &v1_1).unwrap();
        assert_eq!(check.level, super::super::versioning::CompatibilityLevel::BackwardCompatible);
        
        // Different major version should be incompatible
        let check = manager.check_compatibility(&v1_0, &v2_0).unwrap();
        assert_eq!(check.level, super::super::versioning::CompatibilityLevel::Incompatible);
    }
    
    #[test]
    fn test_deprecation_warnings() {
        let mut manager = IntegratedVersionManager::new(Version::new(1, 0, 0));
        
        let deprecation = DeprecationInfo::new(
            "deprecated_api",
            Version::new(1, 0, 0),
            "Test",
        );
        
        manager.register_deprecation(deprecation);
        
        assert!(manager.is_deprecated("deprecated_api"));
        assert!(!manager.is_deprecated("active_api"));
    }
    
    #[test]
    fn test_migration_guide() {
        let mut manager = IntegratedVersionManager::new(Version::new(2, 0, 0));
        
        let mut migration = MigrationGuide::new(
            Version::new(1, 0, 0),
            Version::new(2, 0, 0),
            "Test Migration",
            "Test overview",
        );
        
        migration.add_step(MigrationStep::new(
            1,
            "Step 1",
            "Description",
        ));
        
        manager.register_migration_guide(migration);
        
        let guide = manager.get_migration_guide(
            &Version::new(1, 0, 0),
            &Version::new(2, 0, 0),
        );
        
        assert!(guide.is_some());
        assert_eq!(guide.unwrap().steps.len(), 1);
    }
    
    #[test]
    fn test_change_tracking() {
        let mut manager = IntegratedVersionManager::new(Version::new(1, 2, 0));
        
        let mut changelog_1_0 = Changelog::new(Version::new(1, 0, 0));
        changelog_1_0.add_change(ApiChange::new(
            "change-1",
            ChangeType::Feature,
            "Core",
            "Feature 1",
        ));
        manager.add_changelog(changelog_1_0);
        
        let mut changelog_1_1 = Changelog::new(Version::new(1, 1, 0));
        changelog_1_1.add_change(ApiChange::new(
            "change-2",
            ChangeType::Feature,
            "Core",
            "Feature 2",
        ));
        manager.add_changelog(changelog_1_1);
        
        let changes = manager.get_changes_between(
            &Version::new(0, 9, 0),
            &Version::new(1, 1, 0),
        ).unwrap();
        
        assert_eq!(changes.len(), 2);
    }
    
    #[test]
    fn test_compatibility_matrix() {
        let mut manager = IntegratedVersionManager::new(Version::new(1, 0, 0));
        
        let entry = CompatibilityMatrixEntry::new(Version::new(1, 0, 0))
            .add_binding("rust", Version::new(1, 0, 0))
            .add_binding("nodejs", Version::new(1, 0, 0))
            .add_platform("Linux")
            .add_platform("macOS");
        
        manager.add_compatibility_entry(entry);
        
        // Verify the entry was added (we can't directly access it, but we can validate)
        let warnings = manager.validate().unwrap();
        // Should have no warnings about missing compatibility entries
        assert!(!warnings.iter().any(|w| w.contains("compatibility")));
    }
    
    #[test]
    fn test_sample_data_loading() {
        let mut manager = IntegratedVersionManager::new(Version::new(1, 0, 0));
        
        manager.load_sample_data().unwrap();
        
        // Verify sample data was loaded
        assert!(manager.get_changelog(&Version::new(1, 0, 0)).is_some());
        assert!(manager.is_deprecated("old_api_function"));
    }
}
