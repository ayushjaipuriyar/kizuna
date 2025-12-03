/// Code examples and usage patterns
use super::{Result, DocError};
use std::collections::HashMap;
use std::path::PathBuf;

/// Programming language for examples
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExampleLanguage {
    Rust,
    JavaScript,
    TypeScript,
    Python,
    Dart,
}

impl ExampleLanguage {
    /// Gets the file extension for this language
    pub fn extension(&self) -> &str {
        match self {
            Self::Rust => "rs",
            Self::JavaScript => "js",
            Self::TypeScript => "ts",
            Self::Python => "py",
            Self::Dart => "dart",
        }
    }
    
    /// Gets the language name
    pub fn name(&self) -> &str {
        match self {
            Self::Rust => "Rust",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Python => "Python",
            Self::Dart => "Dart",
        }
    }
}

/// Code example category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExampleCategory {
    GettingStarted,
    Discovery,
    FileTransfer,
    Streaming,
    Security,
    Plugins,
    Advanced,
}

/// A code example
#[derive(Debug, Clone)]
pub struct CodeExample {
    /// Example title
    pub title: String,
    /// Example description
    pub description: String,
    /// Programming language
    pub language: ExampleLanguage,
    /// Category
    pub category: ExampleCategory,
    /// Source code
    pub code: String,
    /// Expected output (optional)
    pub expected_output: Option<String>,
    /// Additional notes
    pub notes: Option<String>,
}

impl CodeExample {
    /// Creates a new code example
    pub fn new(
        title: String,
        description: String,
        language: ExampleLanguage,
        category: ExampleCategory,
        code: String,
    ) -> Self {
        Self {
            title,
            description,
            language,
            category,
            code,
            expected_output: None,
            notes: None,
        }
    }
    
    /// Sets the expected output
    pub fn with_output(mut self, output: String) -> Self {
        self.expected_output = Some(output);
        self
    }
    
    /// Sets additional notes
    pub fn with_notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }
    
    /// Validates the example code
    pub fn validate(&self) -> Result<()> {
        if self.code.is_empty() {
            return Err(DocError::ExampleError("Code cannot be empty".to_string()));
        }
        
        // Basic syntax validation based on language
        match self.language {
            ExampleLanguage::Rust => self.validate_rust_syntax(),
            ExampleLanguage::Python => self.validate_python_syntax(),
            _ => Ok(()), // Other languages not validated yet
        }
    }
    
    /// Formats the example as markdown
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("### {}\n\n", self.title));
        md.push_str(&format!("{}\n\n", self.description));
        
        md.push_str(&format!("```{}\n", self.language.extension()));
        md.push_str(&self.code);
        if !self.code.ends_with('\n') {
            md.push('\n');
        }
        md.push_str("```\n\n");
        
        if let Some(output) = &self.expected_output {
            md.push_str("**Expected Output:**\n\n");
            md.push_str("```\n");
            md.push_str(output);
            if !output.ends_with('\n') {
                md.push('\n');
            }
            md.push_str("```\n\n");
        }
        
        if let Some(notes) = &self.notes {
            md.push_str(&format!("**Note:** {}\n\n", notes));
        }
        
        md
    }
    
    fn validate_rust_syntax(&self) -> Result<()> {
        // Basic Rust syntax checks
        let open_braces = self.code.matches('{').count();
        let close_braces = self.code.matches('}').count();
        
        if open_braces != close_braces {
            return Err(DocError::ExampleError(
                "Mismatched braces in Rust code".to_string()
            ));
        }
        
        Ok(())
    }
    
    fn validate_python_syntax(&self) -> Result<()> {
        // Basic Python syntax checks
        let open_parens = self.code.matches('(').count();
        let close_parens = self.code.matches(')').count();
        
        if open_parens != close_parens {
            return Err(DocError::ExampleError(
                "Mismatched parentheses in Python code".to_string()
            ));
        }
        
        Ok(())
    }
}

/// Manages code examples
pub struct ExampleManager {
    examples: HashMap<ExampleCategory, Vec<CodeExample>>,
}

impl ExampleManager {
    /// Creates a new example manager
    pub fn new() -> Self {
        Self {
            examples: HashMap::new(),
        }
    }
    
    /// Adds an example
    pub fn add_example(&mut self, example: CodeExample) -> Result<()> {
        example.validate()?;
        
        self.examples
            .entry(example.category)
            .or_insert_with(Vec::new)
            .push(example);
        
        Ok(())
    }
    
    /// Gets examples by category
    pub fn get_by_category(&self, category: ExampleCategory) -> Vec<&CodeExample> {
        self.examples
            .get(&category)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }
    
    /// Gets examples by language
    pub fn get_by_language(&self, language: ExampleLanguage) -> Vec<&CodeExample> {
        self.examples
            .values()
            .flatten()
            .filter(|e| e.language == language)
            .collect()
    }
    
    /// Gets all examples
    pub fn get_all(&self) -> Vec<&CodeExample> {
        self.examples.values().flatten().collect()
    }
    
    /// Generates a getting started guide
    pub fn generate_getting_started(&self, language: ExampleLanguage) -> Result<String> {
        let mut guide = String::new();
        
        guide.push_str(&format!("# Getting Started with Kizuna ({})\n\n", language.name()));
        
        guide.push_str("## Installation\n\n");
        guide.push_str(&self.generate_installation_section(language));
        
        guide.push_str("\n## Quick Start\n\n");
        
        let examples = self.get_by_category(ExampleCategory::GettingStarted);
        let lang_examples: Vec<_> = examples
            .into_iter()
            .filter(|e| e.language == language)
            .collect();
        
        if lang_examples.is_empty() {
            return Err(DocError::ExampleError(
                format!("No getting started examples for {}", language.name())
            ));
        }
        
        for example in lang_examples {
            guide.push_str(&example.to_markdown());
        }
        
        Ok(guide)
    }
    
    /// Generates a tutorial
    pub fn generate_tutorial(&self, category: ExampleCategory, language: ExampleLanguage) -> Result<String> {
        let mut tutorial = String::new();
        
        let category_name = match category {
            ExampleCategory::GettingStarted => "Getting Started",
            ExampleCategory::Discovery => "Peer Discovery",
            ExampleCategory::FileTransfer => "File Transfer",
            ExampleCategory::Streaming => "Media Streaming",
            ExampleCategory::Security => "Security",
            ExampleCategory::Plugins => "Plugin Development",
            ExampleCategory::Advanced => "Advanced Topics",
        };
        
        tutorial.push_str(&format!("# {} Tutorial ({})\n\n", category_name, language.name()));
        
        let examples = self.get_by_category(category);
        let lang_examples: Vec<_> = examples
            .into_iter()
            .filter(|e| e.language == language)
            .collect();
        
        if lang_examples.is_empty() {
            return Err(DocError::ExampleError(
                format!("No examples for {} in {}", category_name, language.name())
            ));
        }
        
        for example in lang_examples {
            tutorial.push_str(&example.to_markdown());
        }
        
        Ok(tutorial)
    }
    
    /// Writes examples to files
    pub fn write_examples(&self, output_dir: &PathBuf) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(output_dir)?;
        
        let mut written_files = Vec::new();
        
        for (category, examples) in &self.examples {
            let category_dir = output_dir.join(format!("{:?}", category).to_lowercase());
            std::fs::create_dir_all(&category_dir)?;
            
            for (idx, example) in examples.iter().enumerate() {
                let filename = format!(
                    "{:02}_{}.{}",
                    idx + 1,
                    example.title.to_lowercase().replace(' ', "_"),
                    example.language.extension()
                );
                
                let filepath = category_dir.join(filename);
                std::fs::write(&filepath, &example.code)?;
                written_files.push(filepath);
            }
        }
        
        Ok(written_files)
    }
    
    fn generate_installation_section(&self, language: ExampleLanguage) -> String {
        match language {
            ExampleLanguage::Rust => {
                "Add to your `Cargo.toml`:\n\n```toml\n[dependencies]\nkizuna = \"1.0\"\n```\n"
                    .to_string()
            }
            ExampleLanguage::JavaScript | ExampleLanguage::TypeScript => {
                "```bash\nnpm install kizuna-node\n```\n".to_string()
            }
            ExampleLanguage::Python => {
                "```bash\npip install kizuna\n```\n".to_string()
            }
            ExampleLanguage::Dart => {
                "Add to your `pubspec.yaml`:\n\n```yaml\ndependencies:\n  kizuna: ^1.0.0\n```\n"
                    .to_string()
            }
        }
    }
    
    /// Loads default examples
    pub fn load_default_examples(&mut self) -> Result<()> {
        // Rust getting started example
        self.add_example(CodeExample::new(
            "Basic Initialization".to_string(),
            "Initialize a Kizuna instance and discover peers".to_string(),
            ExampleLanguage::Rust,
            ExampleCategory::GettingStarted,
            r#"use kizuna::developer_api::{KizunaAPI, KizunaInstance, KizunaConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Kizuna
    let config = KizunaConfig::default();
    let instance = KizunaInstance::initialize(config).await?;
    
    // Discover peers
    let mut peers = instance.discover_peers().await?;
    
    // Shutdown
    instance.shutdown().await?;
    Ok(())
}
"#.to_string(),
        ))?;
        
        // Python getting started example
        self.add_example(CodeExample::new(
            "Basic Initialization".to_string(),
            "Initialize a Kizuna instance and discover peers".to_string(),
            ExampleLanguage::Python,
            ExampleCategory::GettingStarted,
            r#"import kizuna
import asyncio

async def main():
    # Initialize Kizuna
    instance = await kizuna.initialize({})
    
    # Discover peers
    async for peer in instance.discover_peers():
        print(f'Found peer: {peer.name}')
    
    # Shutdown
    await instance.shutdown()

asyncio.run(main())
"#.to_string(),
        ))?;
        
        // JavaScript getting started example
        self.add_example(CodeExample::new(
            "Basic Initialization".to_string(),
            "Initialize a Kizuna instance and discover peers".to_string(),
            ExampleLanguage::JavaScript,
            ExampleCategory::GettingStarted,
            r#"const kizuna = require('kizuna-node');

async function main() {
    // Initialize Kizuna
    const instance = await kizuna.initialize({});
    
    // Discover peers
    const peers = await instance.discoverPeers();
    console.log('Found peers:', peers);
    
    // Shutdown
    await instance.shutdown();
}

main().catch(console.error);
"#.to_string(),
        ))?;
        
        // Dart getting started example
        self.add_example(CodeExample::new(
            "Basic Initialization".to_string(),
            "Initialize a Kizuna instance and discover peers".to_string(),
            ExampleLanguage::Dart,
            ExampleCategory::GettingStarted,
            r#"import 'package:kizuna/kizuna.dart';

void main() async {
    // Initialize Kizuna
    final instance = await Kizuna.initialize({});
    
    // Discover peers
    final peers = instance.discoverPeers();
    await for (final peer in peers) {
        print('Found peer: ${peer.name}');
    }
    
    // Shutdown
    await instance.shutdown();
}
"#.to_string(),
        ))?;
        
        // File transfer example (Rust)
        self.add_example(CodeExample::new(
            "File Transfer".to_string(),
            "Transfer a file to a peer".to_string(),
            ExampleLanguage::Rust,
            ExampleCategory::FileTransfer,
            r#"use kizuna::developer_api::{KizunaAPI, KizunaInstance};
use std::path::PathBuf;

async fn transfer_file(instance: &KizunaInstance, peer_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = PathBuf::from("document.pdf");
    let handle = instance.transfer_file(file_path, peer_id.into()).await?;
    
    println!("Transfer started: {:?}", handle.transfer_id());
    Ok(())
}
"#.to_string(),
        ))?;
        
        // Plugin example (Rust)
        self.add_example(CodeExample::new(
            "Custom Discovery Plugin".to_string(),
            "Create a custom discovery plugin".to_string(),
            ExampleLanguage::Rust,
            ExampleCategory::Plugins,
            r#"use kizuna::developer_api::plugins::{Plugin, PluginContext};

struct MyDiscoveryPlugin;

impl Plugin for MyDiscoveryPlugin {
    fn name(&self) -> &str {
        "my-discovery"
    }
    
    fn version(&self) -> semver::Version {
        semver::Version::new(1, 0, 0)
    }
    
    fn initialize(&mut self, context: PluginContext) -> Result<(), Box<dyn std::error::Error>> {
        println!("Plugin initialized");
        Ok(())
    }
    
    fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Plugin shutdown");
        Ok(())
    }
}
"#.to_string(),
        ))?;
        
        Ok(())
    }
}

impl Default for ExampleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_code_example_creation() {
        let example = CodeExample::new(
            "Test".to_string(),
            "Test example".to_string(),
            ExampleLanguage::Rust,
            ExampleCategory::GettingStarted,
            "fn main() {}".to_string(),
        );
        
        assert_eq!(example.title, "Test");
        assert_eq!(example.language, ExampleLanguage::Rust);
    }
    
    #[test]
    fn test_example_validation() {
        let valid_example = CodeExample::new(
            "Test".to_string(),
            "Test".to_string(),
            ExampleLanguage::Rust,
            ExampleCategory::GettingStarted,
            "fn main() {}".to_string(),
        );
        
        assert!(valid_example.validate().is_ok());
        
        let invalid_example = CodeExample::new(
            "Test".to_string(),
            "Test".to_string(),
            ExampleLanguage::Rust,
            ExampleCategory::GettingStarted,
            "".to_string(),
        );
        
        assert!(invalid_example.validate().is_err());
    }
    
    #[test]
    fn test_example_manager() {
        let mut manager = ExampleManager::new();
        
        let example = CodeExample::new(
            "Test".to_string(),
            "Test".to_string(),
            ExampleLanguage::Rust,
            ExampleCategory::GettingStarted,
            "fn main() {}".to_string(),
        );
        
        assert!(manager.add_example(example).is_ok());
        assert_eq!(manager.get_by_category(ExampleCategory::GettingStarted).len(), 1);
    }
}
