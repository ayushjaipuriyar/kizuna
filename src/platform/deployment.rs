// Platform-specific deployment automation
//
// This module provides deployment automation including package generation,
// code signing, and distribution to platform-specific stores and channels.

use crate::platform::{
    PlatformResult, PlatformError, OperatingSystem, Architecture, BuildTarget,
};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Deployment configuration
#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    pub version: String,
    pub target: BuildTarget,
    pub package_format: PackageFormat,
    pub signing_config: Option<SigningConfig>,
    pub distribution_channels: Vec<DistributionChannel>,
}

impl DeploymentConfig {
    /// Create a new deployment configuration
    pub fn new(version: String, target: BuildTarget) -> Self {
        let package_format = PackageFormat::default_for_platform(&target.platform);
        
        Self {
            version,
            target,
            package_format,
            signing_config: None,
            distribution_channels: vec![],
        }
    }
    
    /// Set package format
    pub fn with_format(mut self, format: PackageFormat) -> Self {
        self.package_format = format;
        self
    }
    
    /// Set signing configuration
    pub fn with_signing(mut self, config: SigningConfig) -> Self {
        self.signing_config = Some(config);
        self
    }
    
    /// Add distribution channel
    pub fn with_channel(mut self, channel: DistributionChannel) -> Self {
        self.distribution_channels.push(channel);
        self
    }
}

/// Package format
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PackageFormat {
    // Linux formats
    #[default]
    Deb,
    Rpm,
    Flatpak,
    Snap,
    AppImage,
    
    // macOS formats
    AppBundle,
    Dmg,
    Pkg,
    
    // Windows formats
    Msi,
    Msix,
    Exe,
    Zip,
    
    // Android formats
    Aab,
    Apk,
    
    // iOS formats
    Ipa,
    
    // Container formats
    Docker,
    Oci,
    
    // Web formats
    Wasm,
    Pwa,
}

impl PackageFormat {
    /// Get default package format for a platform
    pub fn default_for_platform(platform: &OperatingSystem) -> Self {
        match platform {
            OperatingSystem::Linux => PackageFormat::Deb,
            OperatingSystem::MacOS => PackageFormat::Dmg,
            OperatingSystem::Windows => PackageFormat::Msi,
            OperatingSystem::Android => PackageFormat::Aab,
            OperatingSystem::iOS => PackageFormat::Ipa,
            OperatingSystem::WebBrowser => PackageFormat::Wasm,
            OperatingSystem::Container => PackageFormat::Docker,
            OperatingSystem::Unknown => PackageFormat::Zip,
        }
    }
    
    /// Get file extension for this format
    pub fn extension(&self) -> &str {
        match self {
            PackageFormat::Deb => "deb",
            PackageFormat::Rpm => "rpm",
            PackageFormat::Flatpak => "flatpak",
            PackageFormat::Snap => "snap",
            PackageFormat::AppImage => "AppImage",
            PackageFormat::AppBundle => "app",
            PackageFormat::Dmg => "dmg",
            PackageFormat::Pkg => "pkg",
            PackageFormat::Msi => "msi",
            PackageFormat::Msix => "msix",
            PackageFormat::Exe => "exe",
            PackageFormat::Zip => "zip",
            PackageFormat::Aab => "aab",
            PackageFormat::Apk => "apk",
            PackageFormat::Ipa => "ipa",
            PackageFormat::Docker => "tar",
            PackageFormat::Oci => "tar",
            PackageFormat::Wasm => "wasm",
            PackageFormat::Pwa => "zip",
        }
    }
}

/// Code signing configuration
#[derive(Debug, Clone)]
pub struct SigningConfig {
    pub identity: String,
    pub certificate_path: Option<PathBuf>,
    pub keychain_password: Option<String>,
    pub timestamp_server: Option<String>,
}

impl SigningConfig {
    /// Create a new signing configuration
    pub fn new(identity: String) -> Self {
        Self {
            identity,
            certificate_path: None,
            keychain_password: None,
            timestamp_server: None,
        }
    }
    
    /// Set certificate path
    pub fn with_certificate(mut self, path: PathBuf) -> Self {
        self.certificate_path = Some(path);
        self
    }
    
    /// Set keychain password
    pub fn with_keychain_password(mut self, password: String) -> Self {
        self.keychain_password = Some(password);
        self
    }
    
    /// Set timestamp server
    pub fn with_timestamp_server(mut self, server: String) -> Self {
        self.timestamp_server = Some(server);
        self
    }
}

/// Distribution channel
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistributionChannel {
    // App stores
    AppleAppStore,
    MacAppStore,
    MicrosoftStore,
    GooglePlayStore,
    
    // Package repositories
    AptRepository,
    YumRepository,
    Flatpak,
    Snapcraft,
    Homebrew,
    Chocolatey,
    
    // Container registries
    DockerHub,
    GitHubContainerRegistry,
    AmazonEcr,
    GoogleContainerRegistry,
    
    // Direct distribution
    GitHubReleases,
    DirectDownload,
    
    // Custom
    Custom(String),
}

/// Deployment package
#[derive(Debug, Clone)]
pub struct DeploymentPackage {
    pub config: DeploymentConfig,
    pub package_path: PathBuf,
    pub checksum: String,
    pub size_bytes: u64,
    pub signed: bool,
}

impl DeploymentPackage {
    /// Create a new deployment package
    pub fn new(config: DeploymentConfig, package_path: PathBuf) -> PlatformResult<Self> {
        let metadata = std::fs::metadata(&package_path)
            .map_err(|e| PlatformError::IoError(e))?;
        
        let size_bytes = metadata.len();
        let checksum = Self::calculate_checksum(&package_path)?;
        
        Ok(Self {
            config,
            package_path,
            checksum,
            size_bytes,
            signed: false,
        })
    }
    
    /// Calculate SHA256 checksum
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
    
    /// Sign the package
    pub fn sign(&mut self, signing_config: &SigningConfig) -> PlatformResult<()> {
        match self.config.target.platform {
            OperatingSystem::MacOS | OperatingSystem::iOS => {
                self.sign_macos(signing_config)?;
            }
            OperatingSystem::Windows => {
                self.sign_windows(signing_config)?;
            }
            _ => {
                return Err(PlatformError::FeatureUnavailable(
                    format!("Code signing not supported for {:?}", self.config.target.platform)
                ));
            }
        }
        
        self.signed = true;
        Ok(())
    }
    
    /// Sign macOS package
    fn sign_macos(&self, config: &SigningConfig) -> PlatformResult<()> {
        // In a real implementation, this would call codesign
        log::info!("Signing macOS package with identity: {}", config.identity);
        
        // Placeholder for actual signing logic
        // codesign --force --deep --sign "$identity" "$package_path"
        
        Ok(())
    }
    
    /// Sign Windows package
    fn sign_windows(&self, config: &SigningConfig) -> PlatformResult<()> {
        // In a real implementation, this would call signtool
        log::info!("Signing Windows package with identity: {}", config.identity);
        
        // Placeholder for actual signing logic
        // signtool sign /f "$certificate" /p "$password" /t "$timestamp" "$package_path"
        
        Ok(())
    }
    
    /// Validate the package
    pub fn validate(&self) -> PlatformResult<PackageValidation> {
        let mut validation = PackageValidation {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };
        
        // Check if package exists
        if !self.package_path.exists() {
            validation.is_valid = false;
            validation.errors.push("Package file not found".to_string());
            return Ok(validation);
        }
        
        // Verify checksum
        let current_checksum = Self::calculate_checksum(&self.package_path)?;
        if current_checksum != self.checksum {
            validation.is_valid = false;
            validation.errors.push("Checksum mismatch".to_string());
        }
        
        // Check if signing is required but not done
        if self.config.signing_config.is_some() && !self.signed {
            validation.warnings.push("Package should be signed but is not".to_string());
        }
        
        // Platform-specific validation
        match self.config.target.platform {
            OperatingSystem::MacOS | OperatingSystem::iOS => {
                if self.config.signing_config.is_some() && !self.signed {
                    validation.errors.push("macOS/iOS packages must be signed".to_string());
                    validation.is_valid = false;
                }
            }
            _ => {}
        }
        
        Ok(validation)
    }
}

/// Package validation result
#[derive(Debug, Clone)]
pub struct PackageValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Deployment manager
pub struct DeploymentManager {
    packages: HashMap<BuildTarget, Vec<DeploymentPackage>>,
}

impl DeploymentManager {
    /// Create a new deployment manager
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }
    
    /// Register a deployment package
    pub fn register_package(&mut self, package: DeploymentPackage) {
        let target = package.config.target.clone();
        self.packages.entry(target).or_insert_with(Vec::new).push(package);
    }
    
    /// Get all packages for a target
    pub fn packages_for_target(&self, target: &BuildTarget) -> Vec<&DeploymentPackage> {
        self.packages.get(target).map(|v| v.iter().collect()).unwrap_or_default()
    }
    
    /// Get all packages
    pub fn all_packages(&self) -> Vec<&DeploymentPackage> {
        self.packages.values().flat_map(|v| v.iter()).collect()
    }
    
    /// Validate all packages
    pub fn validate_all(&self) -> PlatformResult<DeploymentValidationReport> {
        let mut report = DeploymentValidationReport {
            total_packages: 0,
            valid_packages: 0,
            invalid_packages: 0,
            results: HashMap::new(),
        };
        
        for (target, packages) in &self.packages {
            for package in packages {
                report.total_packages += 1;
                
                match package.validate() {
                    Ok(validation) => {
                        if validation.is_valid {
                            report.valid_packages += 1;
                        } else {
                            report.invalid_packages += 1;
                        }
                        report.results.insert(
                            format!("{}-{}", target.target_triple, package.config.package_format.extension()),
                            validation,
                        );
                    }
                    Err(e) => {
                        report.invalid_packages += 1;
                        report.results.insert(
                            format!("{}-{}", target.target_triple, package.config.package_format.extension()),
                            PackageValidation {
                                is_valid: false,
                                errors: vec![format!("Validation error: {}", e)],
                                warnings: vec![],
                            },
                        );
                    }
                }
            }
        }
        
        Ok(report)
    }
    
    /// Generate deployment manifest
    pub fn generate_manifest(&self, version: &str) -> DeploymentManifest {
        let mut manifest = DeploymentManifest {
            version: version.to_string(),
            packages: HashMap::new(),
        };
        
        for (target, packages) in &self.packages {
            for package in packages {
                let key = format!(
                    "{}-{}-{}",
                    target.platform.as_str(),
                    target.architecture.as_str(),
                    package.config.package_format.extension()
                );
                
                manifest.packages.insert(key, PackageInfo {
                    path: package.package_path.to_string_lossy().to_string(),
                    checksum: package.checksum.clone(),
                    size_bytes: package.size_bytes,
                    signed: package.signed,
                    format: package.config.package_format.clone(),
                });
            }
        }
        
        manifest
    }
}

impl Default for DeploymentManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Deployment validation report
#[derive(Debug, Clone)]
pub struct DeploymentValidationReport {
    pub total_packages: usize,
    pub valid_packages: usize,
    pub invalid_packages: usize,
    pub results: HashMap<String, PackageValidation>,
}

impl DeploymentValidationReport {
    /// Check if all packages are valid
    pub fn all_valid(&self) -> bool {
        self.invalid_packages == 0 && self.total_packages > 0
    }
    
    /// Generate summary
    pub fn summary(&self) -> String {
        format!(
            "Deployment Validation Summary:\n\
             Total Packages: {}\n\
             Valid: {}\n\
             Invalid: {}\n\
             Success Rate: {:.1}%",
            self.total_packages,
            self.valid_packages,
            self.invalid_packages,
            if self.total_packages > 0 {
                (self.valid_packages as f64 / self.total_packages as f64) * 100.0
            } else {
                0.0
            }
        )
    }
}

/// Deployment manifest
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeploymentManifest {
    pub version: String,
    pub packages: HashMap<String, PackageInfo>,
}

impl DeploymentManifest {
    /// Export as JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Package information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PackageInfo {
    pub path: String,
    pub checksum: String,
    pub size_bytes: u64,
    pub signed: bool,
    #[serde(skip)]
    pub format: PackageFormat,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_config() {
        let target = BuildTarget::new(OperatingSystem::Linux, Architecture::X86_64);
        let config = DeploymentConfig::new("1.0.0".to_string(), target);
        
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.package_format, PackageFormat::Deb);
    }

    #[test]
    fn test_package_format_defaults() {
        assert_eq!(
            PackageFormat::default_for_platform(&OperatingSystem::Linux),
            PackageFormat::Deb
        );
        assert_eq!(
            PackageFormat::default_for_platform(&OperatingSystem::MacOS),
            PackageFormat::Dmg
        );
        assert_eq!(
            PackageFormat::default_for_platform(&OperatingSystem::Windows),
            PackageFormat::Msi
        );
    }

    #[test]
    fn test_signing_config() {
        let config = SigningConfig::new("Developer ID".to_string())
            .with_timestamp_server("http://timestamp.example.com".to_string());
        
        assert_eq!(config.identity, "Developer ID");
        assert!(config.timestamp_server.is_some());
    }

    #[test]
    fn test_deployment_manager() {
        let manager = DeploymentManager::new();
        assert_eq!(manager.all_packages().len(), 0);
    }

    #[test]
    fn test_distribution_channels() {
        let config = DeploymentConfig::new(
            "1.0.0".to_string(),
            BuildTarget::new(OperatingSystem::MacOS, Architecture::ARM64)
        )
        .with_channel(DistributionChannel::MacAppStore)
        .with_channel(DistributionChannel::GitHubReleases);
        
        assert_eq!(config.distribution_channels.len(), 2);
    }
}
