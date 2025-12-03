/// Automated documentation generation system
use super::{Result, DocError};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Documentation format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocFormat {
    /// Rust documentation (rustdoc)
    Rustdoc,
    /// Markdown format
    Markdown,
    /// HTML format
    Html,
    /// JSON API specification
    Json,
}

/// Generated documentation
#[derive(Debug, Clone)]
pub struct Documentation {
    /// Documentation format
    pub format: DocFormat,
    /// Documentation content
    pub content: String,
    /// Metadata
    pub metadata: DocMetadata,
}

/// Documentation metadata
#[derive(Debug, Clone)]
pub struct DocMetadata {
    /// API version
    pub version: String,
    /// Generation timestamp
    pub generated_at: chrono::DateTime<chrono::Utc>,
    /// Language (for language-specific docs)
    pub language: Option<String>,
    /// Additional metadata
    pub extra: HashMap<String, String>,
}

/// Documentation generator
pub struct DocGenerator {
    /// Output directory
    output_dir: PathBuf,
    /// API version
    version: String,
    /// Include examples
    include_examples: bool,
}

impl DocGenerator {
    /// Creates a new documentation generator
    pub fn new(output_dir: PathBuf, version: String) -> Self {
        Self {
            output_dir,
            version,
            include_examples: true,
        }
    }
    
    /// Sets whether to include examples
    pub fn with_examples(mut self, include: bool) -> Self {
        self.include_examples = include;
        self
    }
    
    /// Generates Rust API documentation
    pub fn generate_rust_docs(&self) -> Result<Documentation> {
        let content = self.generate_rustdoc_content()?;
        
        Ok(Documentation {
            format: DocFormat::Rustdoc,
            content,
            metadata: DocMetadata {
                version: self.version.clone(),
                generated_at: chrono::Utc::now(),
                language: Some("rust".to_string()),
                extra: HashMap::new(),
            },
        })
    }
    
    /// Generates Node.js API documentation
    pub fn generate_nodejs_docs(&self) -> Result<Documentation> {
        let content = self.generate_jsdoc_content()?;
        
        Ok(Documentation {
            format: DocFormat::Markdown,
            content,
            metadata: DocMetadata {
                version: self.version.clone(),
                generated_at: chrono::Utc::now(),
                language: Some("javascript".to_string()),
                extra: HashMap::new(),
            },
        })
    }
    
    /// Generates Python API documentation
    pub fn generate_python_docs(&self) -> Result<Documentation> {
        let content = self.generate_python_doc_content()?;
        
        Ok(Documentation {
            format: DocFormat::Markdown,
            content,
            metadata: DocMetadata {
                version: self.version.clone(),
                generated_at: chrono::Utc::now(),
                language: Some("python".to_string()),
                extra: HashMap::new(),
            },
        })
    }
    
    /// Generates Flutter API documentation
    pub fn generate_flutter_docs(&self) -> Result<Documentation> {
        let content = self.generate_dartdoc_content()?;
        
        Ok(Documentation {
            format: DocFormat::Markdown,
            content,
            metadata: DocMetadata {
                version: self.version.clone(),
                generated_at: chrono::Utc::now(),
                language: Some("dart".to_string()),
                extra: HashMap::new(),
            },
        })
    }
    
    /// Generates comprehensive API reference
    pub fn generate_api_reference(&self) -> Result<Documentation> {
        let mut content = String::new();
        
        content.push_str(&format!("# Kizuna API Reference v{}\n\n", self.version));
        content.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        // Core API section
        content.push_str("## Core API\n\n");
        content.push_str(&self.generate_core_api_docs()?);
        
        // Language bindings section
        content.push_str("\n## Language Bindings\n\n");
        content.push_str("### Rust\n\n");
        content.push_str(&self.generate_rust_api_summary()?);
        content.push_str("\n### Node.js\n\n");
        content.push_str(&self.generate_nodejs_api_summary()?);
        content.push_str("\n### Python\n\n");
        content.push_str(&self.generate_python_api_summary()?);
        content.push_str("\n### Flutter\n\n");
        content.push_str(&self.generate_flutter_api_summary()?);
        
        // Plugin system section
        content.push_str("\n## Plugin System\n\n");
        content.push_str(&self.generate_plugin_api_docs()?);
        
        Ok(Documentation {
            format: DocFormat::Markdown,
            content,
            metadata: DocMetadata {
                version: self.version.clone(),
                generated_at: chrono::Utc::now(),
                language: None,
                extra: HashMap::new(),
            },
        })
    }
    
    /// Writes documentation to file
    pub fn write_to_file(&self, doc: &Documentation, filename: &str) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.output_dir)?;
        
        let extension = match doc.format {
            DocFormat::Rustdoc | DocFormat::Markdown => "md",
            DocFormat::Html => "html",
            DocFormat::Json => "json",
        };
        
        let filepath = self.output_dir.join(format!("{}.{}", filename, extension));
        std::fs::write(&filepath, &doc.content)?;
        
        Ok(filepath)
    }
    
    // Private helper methods
    
    fn generate_rustdoc_content(&self) -> Result<String> {
        let mut content = String::new();
        
        content.push_str("# Kizuna Rust API Documentation\n\n");
        content.push_str("## Overview\n\n");
        content.push_str("The Kizuna Rust API provides comprehensive access to all Kizuna functionality ");
        content.push_str("with native performance and type safety.\n\n");
        
        content.push_str("## Core Traits\n\n");
        content.push_str("### `KizunaAPI`\n\n");
        content.push_str("Main API trait for Kizuna functionality.\n\n");
        content.push_str("```rust\n");
        content.push_str("#[async_trait]\n");
        content.push_str("pub trait KizunaAPI: Send + Sync {\n");
        content.push_str("    async fn initialize(config: KizunaConfig) -> Result<KizunaInstance>;\n");
        content.push_str("    async fn discover_peers(&self) -> Result<Stream<PeerInfo>>;\n");
        content.push_str("    async fn connect_to_peer(&self, peer_id: PeerId) -> Result<PeerConnection>;\n");
        content.push_str("    async fn transfer_file(&self, file: PathBuf, peer_id: PeerId) -> Result<TransferHandle>;\n");
        content.push_str("    async fn start_stream(&self, config: StreamConfig) -> Result<StreamHandle>;\n");
        content.push_str("    async fn shutdown(&self) -> Result<()>;\n");
        content.push_str("}\n");
        content.push_str("```\n\n");
        
        if self.include_examples {
            content.push_str("## Example Usage\n\n");
            content.push_str("```rust\n");
            content.push_str("use kizuna::developer_api::{KizunaAPI, KizunaInstance, KizunaConfig};\n\n");
            content.push_str("#[tokio::main]\n");
            content.push_str("async fn main() -> Result<(), Box<dyn std::error::Error>> {\n");
            content.push_str("    let config = KizunaConfig::default();\n");
            content.push_str("    let instance = KizunaInstance::initialize(config).await?;\n");
            content.push_str("    \n");
            content.push_str("    // Discover peers\n");
            content.push_str("    let mut peers = instance.discover_peers().await?;\n");
            content.push_str("    \n");
            content.push_str("    // Shutdown\n");
            content.push_str("    instance.shutdown().await?;\n");
            content.push_str("    Ok(())\n");
            content.push_str("}\n");
            content.push_str("```\n\n");
        }
        
        Ok(content)
    }
    
    fn generate_jsdoc_content(&self) -> Result<String> {
        let mut content = String::new();
        
        content.push_str("# Kizuna Node.js API Documentation\n\n");
        content.push_str("## Installation\n\n");
        content.push_str("```bash\n");
        content.push_str("npm install kizuna-node\n");
        content.push_str("```\n\n");
        
        content.push_str("## API Reference\n\n");
        content.push_str("### `initialize(config)`\n\n");
        content.push_str("Initializes a new Kizuna instance.\n\n");
        content.push_str("**Parameters:**\n");
        content.push_str("- `config` (Object): Configuration object\n\n");
        content.push_str("**Returns:** Promise<KizunaInstance>\n\n");
        
        if self.include_examples {
            content.push_str("**Example:**\n\n");
            content.push_str("```javascript\n");
            content.push_str("const kizuna = require('kizuna-node');\n\n");
            content.push_str("async function main() {\n");
            content.push_str("  const instance = await kizuna.initialize({});\n");
            content.push_str("  const peers = await instance.discoverPeers();\n");
            content.push_str("  await instance.shutdown();\n");
            content.push_str("}\n");
            content.push_str("```\n\n");
        }
        
        Ok(content)
    }
    
    fn generate_python_doc_content(&self) -> Result<String> {
        let mut content = String::new();
        
        content.push_str("# Kizuna Python API Documentation\n\n");
        content.push_str("## Installation\n\n");
        content.push_str("```bash\n");
        content.push_str("pip install kizuna\n");
        content.push_str("```\n\n");
        
        content.push_str("## API Reference\n\n");
        content.push_str("### `initialize(config: dict) -> KizunaInstance`\n\n");
        content.push_str("Initializes a new Kizuna instance.\n\n");
        content.push_str("**Parameters:**\n");
        content.push_str("- `config` (dict): Configuration dictionary\n\n");
        content.push_str("**Returns:** KizunaInstance\n\n");
        
        if self.include_examples {
            content.push_str("**Example:**\n\n");
            content.push_str("```python\n");
            content.push_str("import kizuna\n");
            content.push_str("import asyncio\n\n");
            content.push_str("async def main():\n");
            content.push_str("    instance = await kizuna.initialize({})\n");
            content.push_str("    async for peer in instance.discover_peers():\n");
            content.push_str("        print(f'Found peer: {peer.name}')\n");
            content.push_str("    await instance.shutdown()\n\n");
            content.push_str("asyncio.run(main())\n");
            content.push_str("```\n\n");
        }
        
        Ok(content)
    }
    
    fn generate_dartdoc_content(&self) -> Result<String> {
        let mut content = String::new();
        
        content.push_str("# Kizuna Flutter API Documentation\n\n");
        content.push_str("## Installation\n\n");
        content.push_str("Add to your `pubspec.yaml`:\n\n");
        content.push_str("```yaml\n");
        content.push_str("dependencies:\n");
        content.push_str("  kizuna: ^1.0.0\n");
        content.push_str("```\n\n");
        
        content.push_str("## API Reference\n\n");
        content.push_str("### `Kizuna.initialize(config)`\n\n");
        content.push_str("Initializes a new Kizuna instance.\n\n");
        content.push_str("**Parameters:**\n");
        content.push_str("- `config` (Map<String, dynamic>): Configuration map\n\n");
        content.push_str("**Returns:** Future<KizunaInstance>\n\n");
        
        if self.include_examples {
            content.push_str("**Example:**\n\n");
            content.push_str("```dart\n");
            content.push_str("import 'package:kizuna/kizuna.dart';\n\n");
            content.push_str("void main() async {\n");
            content.push_str("  final instance = await Kizuna.initialize({});\n");
            content.push_str("  final peers = instance.discoverPeers();\n");
            content.push_str("  await instance.shutdown();\n");
            content.push_str("}\n");
            content.push_str("```\n\n");
        }
        
        Ok(content)
    }
    
    fn generate_core_api_docs(&self) -> Result<String> {
        let mut content = String::new();
        
        content.push_str("The core API provides the foundational interface for all Kizuna operations.\n\n");
        content.push_str("### Key Methods\n\n");
        content.push_str("- `initialize(config)` - Initialize Kizuna instance\n");
        content.push_str("- `discover_peers()` - Discover peers on the network\n");
        content.push_str("- `connect_to_peer(peer_id)` - Connect to a specific peer\n");
        content.push_str("- `transfer_file(file, peer_id)` - Transfer a file to a peer\n");
        content.push_str("- `start_stream(config)` - Start a media stream\n");
        content.push_str("- `shutdown()` - Shutdown the instance\n\n");
        
        Ok(content)
    }
    
    fn generate_rust_api_summary(&self) -> Result<String> {
        Ok("Native Rust API with full async/await support and zero-cost abstractions.\n\n".to_string())
    }
    
    fn generate_nodejs_api_summary(&self) -> Result<String> {
        Ok("Promise-based JavaScript API with TypeScript definitions.\n\n".to_string())
    }
    
    fn generate_python_api_summary(&self) -> Result<String> {
        Ok("Pythonic API with asyncio support and comprehensive type hints.\n\n".to_string())
    }
    
    fn generate_flutter_api_summary(&self) -> Result<String> {
        Ok("Dart API with Stream support for cross-platform Flutter applications.\n\n".to_string())
    }
    
    fn generate_plugin_api_docs(&self) -> Result<String> {
        let mut content = String::new();
        
        content.push_str("The plugin system allows extending Kizuna functionality.\n\n");
        content.push_str("### Plugin Trait\n\n");
        content.push_str("```rust\n");
        content.push_str("pub trait Plugin {\n");
        content.push_str("    fn name(&self) -> &str;\n");
        content.push_str("    fn version(&self) -> Version;\n");
        content.push_str("    fn initialize(&mut self, context: PluginContext) -> Result<()>;\n");
        content.push_str("    fn shutdown(&mut self) -> Result<()>;\n");
        content.push_str("}\n");
        content.push_str("```\n\n");
        
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_doc_generator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let generator = DocGenerator::new(temp_dir.path().to_path_buf(), "1.0.0".to_string());
        assert_eq!(generator.version, "1.0.0");
        assert!(generator.include_examples);
    }
    
    #[test]
    fn test_generate_rust_docs() {
        let temp_dir = TempDir::new().unwrap();
        let generator = DocGenerator::new(temp_dir.path().to_path_buf(), "1.0.0".to_string());
        let doc = generator.generate_rust_docs().unwrap();
        
        assert_eq!(doc.format, DocFormat::Rustdoc);
        assert!(doc.content.contains("Kizuna Rust API"));
        assert!(doc.content.contains("KizunaAPI"));
    }
    
    #[test]
    fn test_generate_api_reference() {
        let temp_dir = TempDir::new().unwrap();
        let generator = DocGenerator::new(temp_dir.path().to_path_buf(), "1.0.0".to_string());
        let doc = generator.generate_api_reference().unwrap();
        
        assert_eq!(doc.format, DocFormat::Markdown);
        assert!(doc.content.contains("API Reference"));
        assert!(doc.content.contains("Core API"));
        assert!(doc.content.contains("Language Bindings"));
    }
}
