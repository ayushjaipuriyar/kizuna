/// Example demonstrating the documentation generation system
use kizuna::developer_api::docs::{
    DocGenerator, DocFormat, ExampleManager, ExampleLanguage, ExampleCategory,
    VersionManager, DocVersion, Changelog, ApiChange, ChangeType,
};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Kizuna Documentation System Demo ===\n");
    
    // 1. Documentation Generation
    println!("1. Generating API Documentation");
    println!("--------------------------------");
    
    let output_dir = PathBuf::from("target/docs");
    let generator = DocGenerator::new(output_dir.clone(), "1.0.0".to_string())
        .with_examples(true);
    
    // Generate Rust documentation
    let rust_docs = generator.generate_rust_docs()?;
    println!("✓ Generated Rust documentation ({} bytes)", rust_docs.content.len());
    
    // Generate Node.js documentation
    let nodejs_docs = generator.generate_nodejs_docs()?;
    println!("✓ Generated Node.js documentation ({} bytes)", nodejs_docs.content.len());
    
    // Generate Python documentation
    let python_docs = generator.generate_python_docs()?;
    println!("✓ Generated Python documentation ({} bytes)", python_docs.content.len());
    
    // Generate Flutter documentation
    let flutter_docs = generator.generate_flutter_docs()?;
    println!("✓ Generated Flutter documentation ({} bytes)", flutter_docs.content.len());
    
    // Generate comprehensive API reference
    let api_ref = generator.generate_api_reference()?;
    println!("✓ Generated API reference ({} bytes)", api_ref.content.len());
    
    // Write documentation to files
    let rust_path = generator.write_to_file(&rust_docs, "rust-api")?;
    println!("  Written to: {:?}", rust_path);
    
    let nodejs_path = generator.write_to_file(&nodejs_docs, "nodejs-api")?;
    println!("  Written to: {:?}", nodejs_path);
    
    let python_path = generator.write_to_file(&python_docs, "python-api")?;
    println!("  Written to: {:?}", python_path);
    
    let flutter_path = generator.write_to_file(&flutter_docs, "flutter-api")?;
    println!("  Written to: {:?}", flutter_path);
    
    let api_ref_path = generator.write_to_file(&api_ref, "api-reference")?;
    println!("  Written to: {:?}", api_ref_path);
    
    // 2. Code Examples and Usage Patterns
    println!("\n2. Managing Code Examples");
    println!("-------------------------");
    
    let mut example_manager = ExampleManager::new();
    
    // Load default examples
    example_manager.load_default_examples()?;
    println!("✓ Loaded default examples");
    
    // Get examples by category
    let getting_started = example_manager.get_by_category(ExampleCategory::GettingStarted);
    println!("  Getting Started examples: {}", getting_started.len());
    
    let file_transfer = example_manager.get_by_category(ExampleCategory::FileTransfer);
    println!("  File Transfer examples: {}", file_transfer.len());
    
    let plugins = example_manager.get_by_category(ExampleCategory::Plugins);
    println!("  Plugin examples: {}", plugins.len());
    
    // Get examples by language
    let rust_examples = example_manager.get_by_language(ExampleLanguage::Rust);
    println!("  Rust examples: {}", rust_examples.len());
    
    let python_examples = example_manager.get_by_language(ExampleLanguage::Python);
    println!("  Python examples: {}", python_examples.len());
    
    // Generate getting started guides
    let rust_guide = example_manager.generate_getting_started(ExampleLanguage::Rust)?;
    println!("✓ Generated Rust getting started guide ({} bytes)", rust_guide.len());
    
    let python_guide = example_manager.generate_getting_started(ExampleLanguage::Python)?;
    println!("✓ Generated Python getting started guide ({} bytes)", python_guide.len());
    
    // Generate tutorials
    let file_transfer_tutorial = example_manager.generate_tutorial(
        ExampleCategory::FileTransfer,
        ExampleLanguage::Rust
    )?;
    println!("✓ Generated file transfer tutorial ({} bytes)", file_transfer_tutorial.len());
    
    // Write examples to files
    let examples_dir = output_dir.join("examples");
    let written_files = example_manager.write_examples(&examples_dir)?;
    println!("✓ Written {} example files to {:?}", written_files.len(), examples_dir);
    
    // 3. Documentation Versioning
    println!("\n3. Documentation Versioning");
    println!("---------------------------");
    
    let mut version_manager = VersionManager::new();
    
    // Load sample version data
    version_manager.load_sample_data()?;
    println!("✓ Loaded sample version data");
    
    // Get latest version
    if let Some(latest) = version_manager.get_latest() {
        println!("  Latest version: {}", latest.version_string());
        println!("  Release date: {}", latest.release_date);
    }
    
    // Get all versions
    let all_versions = version_manager.get_all_versions();
    println!("  Total versions: {}", all_versions.len());
    
    // Generate changelog document
    let changelog_doc = version_manager.generate_changelog_document()?;
    println!("✓ Generated changelog document ({} bytes)", changelog_doc.len());
    
    // Generate compatibility matrix
    let compat_matrix = version_manager.generate_compatibility_matrix()?;
    println!("✓ Generated compatibility matrix ({} bytes)", compat_matrix.len());
    
    // Validate consistency
    let warnings = version_manager.validate_consistency()?;
    if warnings.is_empty() {
        println!("✓ Documentation consistency validated (no warnings)");
    } else {
        println!("⚠ Documentation warnings:");
        for warning in &warnings {
            println!("  - {}", warning);
        }
    }
    
    // Write version documentation
    let version_docs_dir = output_dir.join("versions");
    let version_files = version_manager.write_version_docs(&version_docs_dir)?;
    println!("✓ Written {} version documentation files", version_files.len());
    for file in &version_files {
        println!("  - {:?}", file);
    }
    
    // 4. Display Sample Documentation
    println!("\n4. Sample Documentation Preview");
    println!("-------------------------------");
    
    // Show a sample from the API reference
    let preview_lines: Vec<&str> = api_ref.content.lines().take(20).collect();
    println!("\nAPI Reference Preview (first 20 lines):");
    println!("```");
    for line in preview_lines {
        println!("{}", line);
    }
    println!("```");
    
    // Show a sample example
    if let Some(example) = getting_started.first() {
        println!("\nSample Code Example:");
        println!("```");
        println!("{}", example.to_markdown());
        println!("```");
    }
    
    // Show a sample changelog entry
    if let Some(version) = version_manager.get_latest() {
        if let Some(changelog) = version_manager.get_changelog(&version.version) {
            println!("\nSample Changelog Entry:");
            println!("```");
            let changelog_preview: Vec<&str> = changelog.to_markdown().lines().take(15).collect();
            for line in changelog_preview {
                println!("{}", line);
            }
            println!("```");
        }
    }
    
    println!("\n=== Documentation System Demo Complete ===");
    println!("\nGenerated documentation can be found in: {:?}", output_dir);
    
    Ok(())
}
