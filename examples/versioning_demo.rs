/// Demonstration of the API versioning, deprecation, and change tracking system
use kizuna::developer_api::core::{
    IntegratedVersionManager, ApiVersion, DeprecationInfo, MigrationGuide, MigrationStep,
    Changelog, ApiChange, ChangeType, CompatibilityMatrixEntry,
};
use semver::Version;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Kizuna API Versioning System Demo ===\n");
    
    // Create an integrated version manager
    let mut manager = IntegratedVersionManager::new(Version::new(1, 2, 0));
    
    println!("Current API Version: {}\n", manager.current_version());
    
    // Register API versions
    println!("--- Registering API Versions ---");
    
    let v1_0_0 = ApiVersion::new(1, 0, 0)
        .with_release_date(chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
    manager.register_version(v1_0_0);
    println!("✓ Registered v1.0.0");
    
    let v1_1_0 = ApiVersion::new(1, 1, 0)
        .with_release_date(chrono::NaiveDate::from_ymd_opt(2024, 3, 1).unwrap());
    manager.register_version(v1_1_0);
    println!("✓ Registered v1.1.0");
    
    let v1_2_0 = ApiVersion::new(1, 2, 0)
        .mark_as_latest()
        .with_release_date(chrono::NaiveDate::from_ymd_opt(2024, 5, 15).unwrap());
    manager.register_version(v1_2_0);
    println!("✓ Registered v1.2.0 (latest)\n");
    
    // Add changelogs
    println!("--- Adding Changelogs ---");
    
    let mut changelog_1_0 = Changelog::new(Version::new(1, 0, 0))
        .with_release_date(chrono::NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
        .with_notes("First stable release with core functionality.");
    
    changelog_1_0.add_change(
        ApiChange::new(
            "feature-001",
            ChangeType::Feature,
            "Core API",
            "Initial stable API with discovery, transport, and file transfer",
        )
    );
    
    manager.add_changelog(changelog_1_0);
    println!("✓ Added changelog for v1.0.0");
    
    let mut changelog_1_2 = Changelog::new(Version::new(1, 2, 0))
        .mark_as_latest()
        .with_release_date(chrono::NaiveDate::from_ymd_opt(2024, 5, 15).unwrap())
        .with_notes("Enhanced API with improved performance and new features.");
    
    changelog_1_2.add_change(
        ApiChange::new(
            "feature-002",
            ChangeType::Feature,
            "Core API",
            "Added streaming API with hardware acceleration support",
        )
        .with_element("start_stream")
    );
    
    changelog_1_2.add_change(
        ApiChange::new(
            "deprecation-001",
            ChangeType::Deprecation,
            "Core API",
            "Deprecated old_connect_method in favor of connect_to_peer",
        )
        .with_element("old_connect_method")
        .with_migration_guide("Replace old_connect_method() with connect_to_peer()")
    );
    
    changelog_1_2.add_change(
        ApiChange::new(
            "performance-001",
            ChangeType::Performance,
            "File Transfer",
            "Improved file transfer speed by 40% with parallel chunking",
        )
    );
    
    manager.add_changelog(changelog_1_2);
    println!("✓ Added changelog for v1.2.0\n");
    
    // Register deprecations
    println!("--- Registering Deprecations ---");
    
    let deprecation = DeprecationInfo::new(
        "old_connect_method",
        Version::new(1, 2, 0),
        "Replaced by more efficient connect_to_peer method",
    )
    .with_replacement("connect_to_peer")
    .with_removal_version(Version::new(2, 0, 0))
    .with_migration_guide("Update all calls from old_connect_method(peer_id) to connect_to_peer(peer_id)");
    
    manager.register_deprecation(deprecation);
    println!("✓ Registered deprecation for old_connect_method\n");
    
    // Add migration guide
    println!("--- Adding Migration Guides ---");
    
    let mut migration = MigrationGuide::new(
        Version::new(1, 0, 0),
        Version::new(2, 0, 0),
        "Migration Guide: v1.x to v2.0",
        "This guide helps you migrate from Kizuna API v1.x to v2.0 with breaking changes.",
    );
    
    migration.add_breaking_change("Removed old_connect_method - use connect_to_peer instead");
    migration.add_breaking_change("Changed configuration format - see step 1");
    
    migration.add_step(
        MigrationStep::new(
            1,
            "Update connection calls",
            "Replace all old_connect_method calls with connect_to_peer",
        )
        .with_code_examples(
            "// Old way\ninstance.old_connect_method(peer_id)?;",
            "// New way\ninstance.connect_to_peer(peer_id).await?;",
        )
        .with_effort(15)
    );
    
    migration.add_step(
        MigrationStep::new(
            2,
            "Update configuration format",
            "Migrate to new TOML-based configuration",
        )
        .with_effort(30)
    );
    
    manager.register_migration_guide(migration);
    println!("✓ Added migration guide from v1.0 to v2.0\n");
    
    // Add compatibility matrix
    println!("--- Adding Compatibility Matrix ---");
    
    let compat_entry = CompatibilityMatrixEntry::new(Version::new(1, 2, 0))
        .add_binding("rust", Version::new(1, 2, 0))
        .add_binding("nodejs", Version::new(1, 2, 0))
        .add_binding("python", Version::new(1, 2, 0))
        .add_binding("flutter", Version::new(1, 1, 0))
        .add_platform("Linux")
        .add_platform("macOS")
        .add_platform("Windows")
        .add_platform("Android")
        .add_platform("iOS")
        .with_requirements("Rust 1.70+, Node.js 18+, Python 3.8+, Flutter 3.0+");
    
    manager.add_compatibility_entry(compat_entry);
    println!("✓ Added compatibility matrix for v1.2.0\n");
    
    // Check compatibility between versions
    println!("--- Checking Version Compatibility ---");
    
    let v1_0 = Version::new(1, 0, 0);
    let v1_2 = Version::new(1, 2, 0);
    let v2_0 = Version::new(2, 0, 0);
    
    let compat_check = manager.check_compatibility(&v1_0, &v1_2)?;
    println!("Compatibility from v1.0.0 to v1.2.0:");
    println!("  Level: {:?}", compat_check.level);
    println!("  Notes: {}", compat_check.notes.join(", "));
    
    let compat_check_breaking = manager.check_compatibility(&v1_2, &v2_0)?;
    println!("\nCompatibility from v1.2.0 to v2.0.0:");
    println!("  Level: {:?}", compat_check_breaking.level);
    println!("  Breaking changes: {}", compat_check_breaking.breaking_changes.len());
    println!("  Migration required: {}", compat_check_breaking.requires_migration());
    
    // Check deprecation status
    println!("\n--- Checking Deprecation Status ---");
    
    if manager.is_deprecated("old_connect_method") {
        println!("⚠️  old_connect_method is deprecated");
        if let Some(info) = manager.get_deprecation_info("old_connect_method") {
            println!("   Deprecated since: v{}", info.deprecated_since);
            if let Some(removal) = &info.removal_version {
                println!("   Will be removed in: v{}", removal);
            }
            if let Some(replacement) = &info.replacement {
                println!("   Use instead: {}", replacement);
            }
        }
    }
    
    // Get changes between versions
    println!("\n--- Changes Between Versions ---");
    
    let changes = manager.get_changes_between(&v1_0, &v1_2)?;
    println!("Changes from v1.0.0 to v1.2.0: {} changes", changes.len());
    for change in changes {
        println!("  {} {} ({}): {}", 
            change.change_type.emoji(),
            change.change_type.name(),
            change.component,
            change.description
        );
    }
    
    // Validate the version management system
    println!("\n--- Validating Version System ---");
    
    let warnings = manager.validate()?;
    if warnings.is_empty() {
        println!("✓ Version system is valid with no warnings");
    } else {
        println!("⚠️  Validation warnings:");
        for warning in warnings {
            println!("  - {}", warning);
        }
    }
    
    // Generate documentation
    println!("\n--- Generating Documentation ---");
    
    let output_dir = std::path::Path::new("target/version_docs");
    match manager.generate_all_docs(output_dir) {
        Ok(files) => {
            println!("✓ Generated {} documentation files:", files.len());
            for file in files {
                println!("  - {}", file.display());
            }
        }
        Err(e) => {
            println!("⚠️  Failed to generate docs: {}", e);
        }
    }
    
    // Display version report
    println!("\n--- Version Report Summary ---");
    println!("{}", manager.generate_version_report());
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}
