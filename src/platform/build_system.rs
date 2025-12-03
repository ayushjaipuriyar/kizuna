// Cross-platform build system and validation
//
// This module provides build system integration, cross-compilation support,
// and build artifact validation for all supported platforms.

use crate::platform::{
    PlatformResult, PlatformError, PlatformInfo, OperatingSystem, Architecture,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Build target configuration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildTarget {
    pub platform: OperatingSystem,
    pub architecture: Architecture,
    pub target_triple: String,
}

impl BuildTarget {
    /// Create a new build target
    pub fn new(platform: OperatingSystem, architecture: Architecture) -> Self {
        let target_triple = Self::generate_target_triple(&platform, &architecture);
        Self {
            platform,
            architecture,
            target_triple,
        }
    }
    
    /// Generate Rust target triple from platform and architecture
    fn generate_target_triple(platform: &OperatingSystem, arch: &Architecture) -> String {
        match (platform, arch) {
            (OperatingSystem::Linux, Architecture::X86_64) => "x86_64-unknown-linux-gnu".to_string(),
            (OperatingSystem::Linux, Architecture::ARM64) => "aarch64-unknown-linux-gnu".to_string(),
            (OperatingSystem::MacOS, Architecture::X86_64) => "x86_64-apple-darwin".to_string(),
            (OperatingSystem::MacOS, Architecture::ARM64) => "aarch64-apple-darwin".to_string(),
            (OperatingSystem::Windows, Architecture::X86_64) => "x86_64-pc-windows-msvc".to_string(),
            (OperatingSystem::Windows, Architecture::ARM64) => "aarch64-pc-windows-msvc".to_string(),
            (OperatingSystem::Android, Architecture::ARM64) => "aarch64-linux-android".to_string(),
            (OperatingSystem::Android, Architecture::ARM32) => "armv7-linux-androideabi".to_string(),
            (OperatingSystem::iOS, Architecture::ARM64) => "aarch64-apple-ios".to_string(),
            (OperatingSystem::WebBrowser, Architecture::WASM32) => "wasm32-unknown-unknown".to_string(),
            _ => "unknown".to_string(),
        }
    }
    
    /// Get all supported build targets
    pub fn all_targets() -> Vec<BuildTarget> {
        vec![
            // Linux
            BuildTarget::new(OperatingSystem::Linux, Architecture::X86_64),
            BuildTarget::new(OperatingSystem::Linux, Architecture::ARM64),
            // macOS
            BuildTarget::new(OperatingSystem::MacOS, Architecture::X86_64),
            BuildTarget::new(OperatingSystem::MacOS, Architecture::ARM64),
            // Windows
            BuildTarget::new(OperatingSystem::Windows, Architecture::X86_64),
            BuildTarget::new(OperatingSystem::Windows, Architecture::ARM64),
            // WebAssembly
            BuildTarget::new(OperatingSystem::WebBrowser, Architecture::WASM32),
        ]
    }
    
    /// Check if this target can be built on the current host
    pub fn can_build_on_host(&self, host: &PlatformInfo) -> bool {
        // Native builds are always possible
        if self.platform == host.os && self.architecture == host.architecture {
            return true;
        }
        
        // Cross-compilation possibilities
        match (&host.os, &self.platform) {
            // Linux can cross-compile to most targets
            (OperatingSystem::Linux, _) => true,
            // macOS can build for both Intel and Apple Silicon
            (OperatingSystem::MacOS, OperatingSystem::MacOS) => true,
            // macOS can build for iOS
            (OperatingSystem::MacOS, OperatingSystem::iOS) => true,
            // Windows can build for different Windows architectures
            (OperatingSystem::Windows, OperatingSystem::Windows) => true,
            // Any platform can build WASM
            (_, OperatingSystem::WebBrowser) => true,
            _ => false,
        }
    }
}

/// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub target: BuildTarget,
    pub optimization_level: OptimizationLevel,
    pub features: Vec<String>,
    pub profile: BuildProfile,
    pub output_dir: PathBuf,
}

impl BuildConfig {
    /// Create a new build configuration
    pub fn new(target: BuildTarget) -> Self {
        Self {
            target,
            optimization_level: OptimizationLevel::Release,
            features: vec![],
            profile: BuildProfile::Release,
            output_dir: PathBuf::from("target"),
        }
    }
    
    /// Set optimization level
    pub fn with_optimization(mut self, level: OptimizationLevel) -> Self {
        self.optimization_level = level;
        self
    }
    
    /// Add a feature flag
    pub fn with_feature(mut self, feature: String) -> Self {
        self.features.push(feature);
        self
    }
    
    /// Set build profile
    pub fn with_profile(mut self, profile: BuildProfile) -> Self {
        self.profile = profile;
        self
    }
    
    /// Set output directory
    pub fn with_output_dir(mut self, dir: PathBuf) -> Self {
        self.output_dir = dir;
        self
    }
}

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    Debug,
    Release,
    Size,
    Speed,
}

/// Build profile
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildProfile {
    Debug,
    Release,
    Test,
    Bench,
}

/// Build artifact information
#[derive(Debug, Clone)]
pub struct BuildArtifact {
    pub target: BuildTarget,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub checksum: String,
    pub build_time: std::time::SystemTime,
}

impl BuildArtifact {
    /// Create a new build artifact from a file
    pub fn from_file(target: BuildTarget, path: PathBuf) -> PlatformResult<Self> {
        let metadata = std::fs::metadata(&path)
            .map_err(|e| PlatformError::IoError(e))?;
        
        let size_bytes = metadata.len();
        let build_time = metadata.modified()
            .map_err(|e| PlatformError::IoError(e))?;
        
        // Calculate checksum
        let checksum = Self::calculate_checksum(&path)?;
        
        Ok(Self {
            target,
            path,
            size_bytes,
            checksum,
            build_time,
        })
    }
    
    /// Calculate SHA256 checksum of the artifact
    fn calculate_checksum(path: &Path) -> PlatformResult<String> {
        use sha2::{Sha256, Digest};
        use std::io::Read;
        
        let mut file = std::fs::File::open(path)
            .map_err(|e| PlatformError::IoError(e))?;
        
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];
        
        loop {
            let n = file.read(&mut buffer)
                .map_err(|e| PlatformError::IoError(e))?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }
    
    /// Validate the artifact
    pub fn validate(&self) -> PlatformResult<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };
        
        // Check if file exists
        if !self.path.exists() {
            result.is_valid = false;
            result.errors.push(format!("Artifact not found: {:?}", self.path));
            return Ok(result);
        }
        
        // Verify checksum
        let current_checksum = Self::calculate_checksum(&self.path)?;
        if current_checksum != self.checksum {
            result.is_valid = false;
            result.errors.push("Checksum mismatch".to_string());
        }
        
        // Check file size
        let metadata = std::fs::metadata(&self.path)
            .map_err(|e| PlatformError::IoError(e))?;
        
        if metadata.len() != self.size_bytes {
            result.warnings.push("File size changed".to_string());
        }
        
        // Platform-specific validation
        match self.target.platform {
            OperatingSystem::Linux | OperatingSystem::MacOS => {
                // Check if binary is executable
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = metadata.permissions();
                    if perms.mode() & 0o111 == 0 {
                        result.warnings.push("Binary is not executable".to_string());
                    }
                }
            }
            OperatingSystem::Windows => {
                // Check if it's a valid PE executable
                if !self.path.extension().map_or(false, |ext| ext == "exe") {
                    result.warnings.push("Windows binary should have .exe extension".to_string());
                }
            }
            OperatingSystem::WebBrowser => {
                // Check if it's a valid WASM module
                if !self.path.extension().map_or(false, |ext| ext == "wasm") {
                    result.errors.push("WASM artifact should have .wasm extension".to_string());
                    result.is_valid = false;
                }
            }
            _ => {}
        }
        
        Ok(result)
    }
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Build system manager
pub struct BuildSystemManager {
    supported_targets: Vec<BuildTarget>,
    host_info: PlatformInfo,
}

impl BuildSystemManager {
    /// Create a new build system manager
    pub fn new(host_info: PlatformInfo) -> Self {
        Self {
            supported_targets: BuildTarget::all_targets(),
            host_info,
        }
    }
    
    /// Get all supported build targets
    pub fn supported_targets(&self) -> &[BuildTarget] {
        &self.supported_targets
    }
    
    /// Get targets that can be built on the current host
    pub fn buildable_targets(&self) -> Vec<&BuildTarget> {
        self.supported_targets
            .iter()
            .filter(|target| target.can_build_on_host(&self.host_info))
            .collect()
    }
    
    /// Validate a build configuration
    pub fn validate_config(&self, config: &BuildConfig) -> PlatformResult<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };
        
        // Check if target is supported
        if !self.supported_targets.contains(&config.target) {
            result.is_valid = false;
            result.errors.push(format!(
                "Unsupported target: {} {}",
                config.target.platform.as_str(),
                config.target.architecture.as_str()
            ));
        }
        
        // Check if target can be built on this host
        if !config.target.can_build_on_host(&self.host_info) {
            result.warnings.push(format!(
                "Cross-compilation required for target: {}",
                config.target.target_triple
            ));
        }
        
        // Validate output directory
        if !config.output_dir.exists() {
            result.warnings.push(format!(
                "Output directory does not exist: {:?}",
                config.output_dir
            ));
        }
        
        Ok(result)
    }
    
    /// Generate build matrix for CI/CD
    pub fn generate_build_matrix(&self) -> BuildMatrix {
        let mut matrix = BuildMatrix {
            targets: HashMap::new(),
        };
        
        for target in &self.supported_targets {
            let config = BuildConfig::new(target.clone());
            matrix.targets.insert(target.clone(), config);
        }
        
        matrix
    }
    
    /// Validate all artifacts in a directory
    pub fn validate_artifacts(&self, artifact_dir: &Path) -> PlatformResult<ArtifactValidationReport> {
        let mut report = ArtifactValidationReport {
            total_artifacts: 0,
            valid_artifacts: 0,
            invalid_artifacts: 0,
            results: HashMap::new(),
        };
        
        // Scan for artifacts
        for target in &self.supported_targets {
            let artifact_path = self.get_artifact_path(artifact_dir, target);
            
            if artifact_path.exists() {
                report.total_artifacts += 1;
                
                match BuildArtifact::from_file(target.clone(), artifact_path) {
                    Ok(artifact) => {
                        match artifact.validate() {
                            Ok(validation) => {
                                if validation.is_valid {
                                    report.valid_artifacts += 1;
                                } else {
                                    report.invalid_artifacts += 1;
                                }
                                report.results.insert(target.clone(), validation);
                            }
                            Err(e) => {
                                report.invalid_artifacts += 1;
                                report.results.insert(
                                    target.clone(),
                                    ValidationResult {
                                        is_valid: false,
                                        errors: vec![format!("Validation error: {}", e)],
                                        warnings: vec![],
                                    },
                                );
                            }
                        }
                    }
                    Err(e) => {
                        report.invalid_artifacts += 1;
                        report.results.insert(
                            target.clone(),
                            ValidationResult {
                                is_valid: false,
                                errors: vec![format!("Failed to load artifact: {}", e)],
                                warnings: vec![],
                            },
                        );
                    }
                }
            }
        }
        
        Ok(report)
    }
    
    /// Get expected artifact path for a target
    fn get_artifact_path(&self, base_dir: &Path, target: &BuildTarget) -> PathBuf {
        let binary_name = match target.platform {
            OperatingSystem::Windows => "kizuna.exe",
            OperatingSystem::WebBrowser => "kizuna_bg.wasm",
            _ => "kizuna",
        };
        
        base_dir
            .join(target.platform.as_str())
            .join(&target.target_triple)
            .join(binary_name)
    }
}

/// Build matrix for CI/CD
#[derive(Debug, Clone)]
pub struct BuildMatrix {
    pub targets: HashMap<BuildTarget, BuildConfig>,
}

impl BuildMatrix {
    /// Export as GitHub Actions matrix JSON
    pub fn to_github_actions_json(&self) -> String {
        let targets: Vec<_> = self.targets.keys()
            .map(|t| serde_json::json!({
                "platform": t.platform.as_str(),
                "arch": t.architecture.as_str(),
                "target": t.target_triple,
            }))
            .collect();
        
        serde_json::json!({
            "include": targets
        }).to_string()
    }
}

/// Artifact validation report
#[derive(Debug, Clone)]
pub struct ArtifactValidationReport {
    pub total_artifacts: usize,
    pub valid_artifacts: usize,
    pub invalid_artifacts: usize,
    pub results: HashMap<BuildTarget, ValidationResult>,
}

impl ArtifactValidationReport {
    /// Check if all artifacts are valid
    pub fn all_valid(&self) -> bool {
        self.invalid_artifacts == 0 && self.total_artifacts > 0
    }
    
    /// Generate a summary report
    pub fn summary(&self) -> String {
        format!(
            "Artifact Validation Summary:\n\
             Total: {}\n\
             Valid: {}\n\
             Invalid: {}\n\
             Success Rate: {:.1}%",
            self.total_artifacts,
            self.valid_artifacts,
            self.invalid_artifacts,
            if self.total_artifacts > 0 {
                (self.valid_artifacts as f64 / self.total_artifacts as f64) * 100.0
            } else {
                0.0
            }
        )
    }
}

// Extension traits for platform types
impl OperatingSystem {
    /// Get string representation
    pub fn as_str(&self) -> &str {
        match self {
            OperatingSystem::Linux => "linux",
            OperatingSystem::MacOS => "macos",
            OperatingSystem::Windows => "windows",
            OperatingSystem::Android => "android",
            OperatingSystem::iOS => "ios",
            OperatingSystem::WebBrowser => "wasm",
            OperatingSystem::Container => "container",
            OperatingSystem::Unknown => "unknown",
        }
    }
}

impl Architecture {
    /// Get string representation
    pub fn as_str(&self) -> &str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::ARM64 => "arm64",
            Architecture::ARM32 => "arm32",
            Architecture::WASM32 => "wasm32",
            Architecture::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_target_creation() {
        let target = BuildTarget::new(OperatingSystem::Linux, Architecture::X86_64);
        assert_eq!(target.target_triple, "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn test_all_targets() {
        let targets = BuildTarget::all_targets();
        assert!(!targets.is_empty());
        assert!(targets.len() >= 7); // At least 7 major targets
    }

    #[test]
    fn test_build_config() {
        let target = BuildTarget::new(OperatingSystem::Linux, Architecture::X86_64);
        let config = BuildConfig::new(target.clone())
            .with_optimization(OptimizationLevel::Release)
            .with_feature("full-features".to_string());
        
        assert_eq!(config.target, target);
        assert_eq!(config.optimization_level, OptimizationLevel::Release);
        assert_eq!(config.features.len(), 1);
    }

    #[test]
    fn test_build_system_manager() {
        let host_info = crate::platform::detect_platform().unwrap();
        let manager = BuildSystemManager::new(host_info);
        
        let supported = manager.supported_targets();
        assert!(!supported.is_empty());
        
        let buildable = manager.buildable_targets();
        assert!(!buildable.is_empty());
    }

    #[test]
    fn test_build_matrix_generation() {
        let host_info = crate::platform::detect_platform().unwrap();
        let manager = BuildSystemManager::new(host_info);
        
        let matrix = manager.generate_build_matrix();
        assert!(!matrix.targets.is_empty());
    }
}
