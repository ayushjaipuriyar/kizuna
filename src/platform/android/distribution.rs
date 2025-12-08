// Android distribution and packaging
//
// Handles Android app bundle (AAB) generation, APK generation,
// and Android-specific update and installation mechanisms

use crate::platform::{PlatformResult, PlatformError};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Android package format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AndroidPackageFormat {
    /// Android App Bundle for Google Play Store
    Aab,
    /// APK for direct installation and sideloading
    Apk,
}

impl AndroidPackageFormat {
    /// Get file extension for this format
    pub fn extension(&self) -> &str {
        match self {
            AndroidPackageFormat::Aab => "aab",
            AndroidPackageFormat::Apk => "apk",
        }
    }
}

/// Android build configuration
#[derive(Debug, Clone)]
pub struct AndroidBuildConfig {
    pub package_name: String,
    pub version_name: String,
    pub version_code: u32,
    pub min_sdk_version: u32,
    pub target_sdk_version: u32,
    pub compile_sdk_version: u32,
    pub architectures: Vec<AndroidArchitecture>,
    pub build_type: AndroidBuildType,
}

impl AndroidBuildConfig {
    /// Create a new Android build configuration
    pub fn new(package_name: String, version_name: String, version_code: u32) -> Self {
        Self {
            package_name,
            version_name,
            version_code,
            min_sdk_version: 24, // Android 7.0
            target_sdk_version: 33, // Android 13
            compile_sdk_version: 33,
            architectures: vec![
                AndroidArchitecture::Arm64V8a,
                AndroidArchitecture::ArmV7a,
                AndroidArchitecture::X86_64,
            ],
            build_type: AndroidBuildType::Release,
        }
    }

    /// Set SDK versions
    pub fn with_sdk_versions(mut self, min: u32, target: u32, compile: u32) -> Self {
        self.min_sdk_version = min;
        self.target_sdk_version = target;
        self.compile_sdk_version = compile;
        self
    }

    /// Set architectures
    pub fn with_architectures(mut self, architectures: Vec<AndroidArchitecture>) -> Self {
        self.architectures = architectures;
        self
    }

    /// Set build type
    pub fn with_build_type(mut self, build_type: AndroidBuildType) -> Self {
        self.build_type = build_type;
        self
    }
}

/// Android architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AndroidArchitecture {
    /// ARM 64-bit (most modern devices)
    Arm64V8a,
    /// ARM 32-bit (older devices)
    ArmV7a,
    /// x86 64-bit (emulators and some tablets)
    X86_64,
    /// x86 32-bit (older emulators)
    X86,
}

impl AndroidArchitecture {
    /// Get ABI name for this architecture
    pub fn abi_name(&self) -> &str {
        match self {
            AndroidArchitecture::Arm64V8a => "arm64-v8a",
            AndroidArchitecture::ArmV7a => "armeabi-v7a",
            AndroidArchitecture::X86_64 => "x86_64",
            AndroidArchitecture::X86 => "x86",
        }
    }

    /// Get Rust target triple for this architecture
    pub fn rust_target(&self) -> &str {
        match self {
            AndroidArchitecture::Arm64V8a => "aarch64-linux-android",
            AndroidArchitecture::ArmV7a => "armv7-linux-androideabi",
            AndroidArchitecture::X86_64 => "x86_64-linux-android",
            AndroidArchitecture::X86 => "i686-linux-android",
        }
    }
}

/// Android build type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AndroidBuildType {
    Debug,
    Release,
}

/// Android signing configuration
#[derive(Debug, Clone)]
pub struct AndroidSigningConfig {
    pub keystore_path: PathBuf,
    pub keystore_password: String,
    pub key_alias: String,
    pub key_password: String,
}

impl AndroidSigningConfig {
    /// Create a new signing configuration
    pub fn new(
        keystore_path: PathBuf,
        keystore_password: String,
        key_alias: String,
        key_password: String,
    ) -> Self {
        Self {
            keystore_path,
            keystore_password,
            key_alias,
            key_password,
        }
    }
}

/// Android package
#[derive(Debug, Clone)]
pub struct AndroidPackage {
    pub format: AndroidPackageFormat,
    pub config: AndroidBuildConfig,
    pub package_path: PathBuf,
    pub checksum: String,
    pub size_bytes: u64,
    pub signed: bool,
}

impl AndroidPackage {
    /// Create a new Android package from a file
    pub fn from_file(
        format: AndroidPackageFormat,
        config: AndroidBuildConfig,
        package_path: PathBuf,
    ) -> PlatformResult<Self> {
        let metadata = std::fs::metadata(&package_path)
            .map_err(|e| PlatformError::IoError(e))?;

        let size_bytes = metadata.len();
        let checksum = Self::calculate_checksum(&package_path)?;

        Ok(Self {
            format,
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
    pub fn sign(&mut self, signing_config: &AndroidSigningConfig) -> PlatformResult<()> {
        // Verify keystore exists
        if !signing_config.keystore_path.exists() {
            return Err(PlatformError::DeploymentError(
                format!("Keystore not found: {:?}", signing_config.keystore_path)
            ));
        }

        // In a real implementation, this would call jarsigner or apksigner
        log::info!(
            "Signing Android package with keystore: {:?}, alias: {}",
            signing_config.keystore_path,
            signing_config.key_alias
        );

        // Placeholder for actual signing logic
        // For AAB: bundletool sign --bundle=app.aab --ks=keystore.jks --ks-pass=pass:password --ks-key-alias=key --key-pass=pass:password
        // For APK: apksigner sign --ks keystore.jks --ks-pass pass:password --key-pass pass:password app.apk

        self.signed = true;
        Ok(())
    }

    /// Validate the package
    pub fn validate(&self) -> PlatformResult<AndroidPackageValidation> {
        let mut validation = AndroidPackageValidation {
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

        // Check file extension
        let extension = self.package_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        if extension != self.format.extension() {
            validation.warnings.push(format!(
                "File extension '{}' does not match format '{}'",
                extension,
                self.format.extension()
            ));
        }

        // Check if package should be signed
        if self.format == AndroidPackageFormat::Aab && !self.signed {
            validation.errors.push("AAB packages must be signed for Play Store distribution".to_string());
            validation.is_valid = false;
        }

        if self.format == AndroidPackageFormat::Apk && !self.signed {
            validation.warnings.push("APK should be signed for distribution".to_string());
        }

        Ok(validation)
    }
}

/// Android package validation result
#[derive(Debug, Clone)]
pub struct AndroidPackageValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Android distribution manager
pub struct AndroidDistributionManager {
    packages: Vec<AndroidPackage>,
    build_configs: HashMap<String, AndroidBuildConfig>,
}

impl AndroidDistributionManager {
    /// Create a new Android distribution manager
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            build_configs: HashMap::new(),
        }
    }

    /// Register a build configuration
    pub fn register_build_config(&mut self, name: String, config: AndroidBuildConfig) {
        self.build_configs.insert(name, config);
    }

    /// Get a build configuration
    pub fn get_build_config(&self, name: &str) -> Option<&AndroidBuildConfig> {
        self.build_configs.get(name)
    }

    /// Create an AAB package
    pub async fn create_aab(
        &mut self,
        config: AndroidBuildConfig,
        native_libs_dir: &Path,
        output_dir: &Path,
    ) -> PlatformResult<AndroidPackage> {
        // Validate input directories
        if !native_libs_dir.exists() {
            return Err(PlatformError::DeploymentError(
                format!("Native libraries directory not found: {:?}", native_libs_dir)
            ));
        }

        // Create output directory if it doesn't exist
        std::fs::create_dir_all(output_dir)
            .map_err(|e| PlatformError::IoError(e))?;

        // Generate AAB filename
        let aab_filename = format!(
            "{}-{}.aab",
            config.package_name.replace('.', "_"),
            config.version_name
        );
        let aab_path = output_dir.join(aab_filename);

        // In a real implementation, this would:
        // 1. Compile native libraries for each architecture
        // 2. Create Android manifest
        // 3. Package resources
        // 4. Create base module
        // 5. Build AAB using bundletool
        log::info!("Creating AAB package: {:?}", aab_path);

        // Placeholder: create a dummy file for testing
        std::fs::write(&aab_path, b"AAB_PLACEHOLDER")
            .map_err(|e| PlatformError::IoError(e))?;

        let package = AndroidPackage::from_file(
            AndroidPackageFormat::Aab,
            config,
            aab_path,
        )?;

        self.packages.push(package.clone());
        Ok(package)
    }

    /// Create an APK package
    pub async fn create_apk(
        &mut self,
        config: AndroidBuildConfig,
        native_libs_dir: &Path,
        output_dir: &Path,
    ) -> PlatformResult<AndroidPackage> {
        // Validate input directories
        if !native_libs_dir.exists() {
            return Err(PlatformError::DeploymentError(
                format!("Native libraries directory not found: {:?}", native_libs_dir)
            ));
        }

        // Create output directory if it doesn't exist
        std::fs::create_dir_all(output_dir)
            .map_err(|e| PlatformError::IoError(e))?;

        // Generate APK filename
        let apk_filename = format!(
            "{}-{}.apk",
            config.package_name.replace('.', "_"),
            config.version_name
        );
        let apk_path = output_dir.join(apk_filename);

        // In a real implementation, this would:
        // 1. Compile native libraries for each architecture
        // 2. Create Android manifest
        // 3. Package resources
        // 4. Create DEX files
        // 5. Build APK using aapt2 and apkbuilder
        log::info!("Creating APK package: {:?}", apk_path);

        // Placeholder: create a dummy file for testing
        std::fs::write(&apk_path, b"APK_PLACEHOLDER")
            .map_err(|e| PlatformError::IoError(e))?;

        let package = AndroidPackage::from_file(
            AndroidPackageFormat::Apk,
            config,
            apk_path,
        )?;

        self.packages.push(package.clone());
        Ok(package)
    }

    /// Sign a package
    pub fn sign_package(
        &mut self,
        package_index: usize,
        signing_config: &AndroidSigningConfig,
    ) -> PlatformResult<()> {
        if package_index >= self.packages.len() {
            return Err(PlatformError::DeploymentError(
                "Invalid package index".to_string()
            ));
        }

        self.packages[package_index].sign(signing_config)?;
        Ok(())
    }

    /// Validate all packages
    pub fn validate_all(&self) -> PlatformResult<AndroidDistributionValidation> {
        let mut report = AndroidDistributionValidation {
            total_packages: self.packages.len(),
            valid_packages: 0,
            invalid_packages: 0,
            results: HashMap::new(),
        };

        for (index, package) in self.packages.iter().enumerate() {
            match package.validate() {
                Ok(validation) => {
                    if validation.is_valid {
                        report.valid_packages += 1;
                    } else {
                        report.invalid_packages += 1;
                    }
                    report.results.insert(
                        format!("package_{}", index),
                        validation,
                    );
                }
                Err(e) => {
                    report.invalid_packages += 1;
                    report.results.insert(
                        format!("package_{}", index),
                        AndroidPackageValidation {
                            is_valid: false,
                            errors: vec![format!("Validation error: {}", e)],
                            warnings: vec![],
                        },
                    );
                }
            }
        }

        Ok(report)
    }

    /// Get all packages
    pub fn packages(&self) -> &[AndroidPackage] {
        &self.packages
    }

    /// Generate distribution manifest
    pub fn generate_manifest(&self, version: &str) -> AndroidDistributionManifest {
        let mut manifest = AndroidDistributionManifest {
            version: version.to_string(),
            packages: HashMap::new(),
        };

        for (index, package) in self.packages.iter().enumerate() {
            let key = format!(
                "{}-{}-{}",
                package.config.package_name,
                package.config.version_name,
                package.format.extension()
            );

            manifest.packages.insert(key, AndroidPackageInfo {
                format: package.format,
                path: package.package_path.to_string_lossy().to_string(),
                checksum: package.checksum.clone(),
                size_bytes: package.size_bytes,
                signed: package.signed,
                version_name: package.config.version_name.clone(),
                version_code: package.config.version_code,
                min_sdk_version: package.config.min_sdk_version,
                target_sdk_version: package.config.target_sdk_version,
            });
        }

        manifest
    }
}

impl Default for AndroidDistributionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Android distribution validation report
#[derive(Debug, Clone)]
pub struct AndroidDistributionValidation {
    pub total_packages: usize,
    pub valid_packages: usize,
    pub invalid_packages: usize,
    pub results: HashMap<String, AndroidPackageValidation>,
}

impl AndroidDistributionValidation {
    /// Check if all packages are valid
    pub fn all_valid(&self) -> bool {
        self.invalid_packages == 0 && self.total_packages > 0
    }

    /// Generate summary
    pub fn summary(&self) -> String {
        format!(
            "Android Distribution Validation Summary:\n\
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

/// Android distribution manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidDistributionManifest {
    pub version: String,
    pub packages: HashMap<String, AndroidPackageInfo>,
}

impl AndroidDistributionManifest {
    /// Export as JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Load from JSON
    pub fn from_json(json: &str) -> PlatformResult<Self> {
        serde_json::from_str(json)
            .map_err(|e| PlatformError::DeploymentError(format!("Failed to parse manifest: {}", e)))
    }
}

/// Android package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndroidPackageInfo {
    pub format: AndroidPackageFormat,
    pub path: String,
    pub checksum: String,
    pub size_bytes: u64,
    pub signed: bool,
    pub version_name: String,
    pub version_code: u32,
    pub min_sdk_version: u32,
    pub target_sdk_version: u32,
}

/// Android update mechanism
pub struct AndroidUpdateManager {
    current_version: String,
    update_channel: UpdateChannel,
}

impl AndroidUpdateManager {
    /// Create a new update manager
    pub fn new(current_version: String) -> Self {
        Self {
            current_version,
            update_channel: UpdateChannel::Stable,
        }
    }

    /// Set update channel
    pub fn with_channel(mut self, channel: UpdateChannel) -> Self {
        self.update_channel = channel;
        self
    }

    /// Check for updates
    pub async fn check_for_updates(&self, update_url: &str) -> PlatformResult<Option<UpdateInfo>> {
        // In a real implementation, this would:
        // 1. Fetch update manifest from server
        // 2. Compare versions
        // 3. Return update info if available
        log::info!("Checking for updates from: {}", update_url);

        // Placeholder: no updates available
        Ok(None)
    }

    /// Download update
    pub async fn download_update(
        &self,
        update_info: &UpdateInfo,
        output_path: &Path,
    ) -> PlatformResult<PathBuf> {
        // In a real implementation, this would:
        // 1. Download the update package
        // 2. Verify checksum
        // 3. Save to output path
        log::info!("Downloading update: {} to {:?}", update_info.version, output_path);

        // Placeholder
        Ok(output_path.to_path_buf())
    }

    /// Install update
    pub async fn install_update(&self, package_path: &Path) -> PlatformResult<()> {
        // In a real implementation, this would:
        // 1. Verify package signature
        // 2. Trigger Android package installer
        // 3. Handle installation result
        log::info!("Installing update from: {:?}", package_path);

        if !package_path.exists() {
            return Err(PlatformError::DeploymentError(
                "Update package not found".to_string()
            ));
        }

        // Placeholder: installation would be handled by Android system
        Ok(())
    }

    /// Get current version
    pub fn current_version(&self) -> &str {
        &self.current_version
    }

    /// Get update channel
    pub fn update_channel(&self) -> UpdateChannel {
        self.update_channel
    }
}

/// Update channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateChannel {
    Stable,
    Beta,
    Alpha,
}

/// Update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub version_code: u32,
    pub download_url: String,
    pub checksum: String,
    pub size_bytes: u64,
    pub release_notes: String,
    pub required: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_android_architecture_abi_names() {
        assert_eq!(AndroidArchitecture::Arm64V8a.abi_name(), "arm64-v8a");
        assert_eq!(AndroidArchitecture::ArmV7a.abi_name(), "armeabi-v7a");
        assert_eq!(AndroidArchitecture::X86_64.abi_name(), "x86_64");
        assert_eq!(AndroidArchitecture::X86.abi_name(), "x86");
    }

    #[test]
    fn test_android_architecture_rust_targets() {
        assert_eq!(AndroidArchitecture::Arm64V8a.rust_target(), "aarch64-linux-android");
        assert_eq!(AndroidArchitecture::ArmV7a.rust_target(), "armv7-linux-androideabi");
        assert_eq!(AndroidArchitecture::X86_64.rust_target(), "x86_64-linux-android");
        assert_eq!(AndroidArchitecture::X86.rust_target(), "i686-linux-android");
    }

    #[test]
    fn test_package_format_extension() {
        assert_eq!(AndroidPackageFormat::Aab.extension(), "aab");
        assert_eq!(AndroidPackageFormat::Apk.extension(), "apk");
    }

    #[test]
    fn test_build_config_creation() {
        let config = AndroidBuildConfig::new(
            "com.example.app".to_string(),
            "1.0.0".to_string(),
            1,
        );

        assert_eq!(config.package_name, "com.example.app");
        assert_eq!(config.version_name, "1.0.0");
        assert_eq!(config.version_code, 1);
        assert_eq!(config.min_sdk_version, 24);
        assert_eq!(config.target_sdk_version, 33);
    }

    #[test]
    fn test_build_config_with_sdk_versions() {
        let config = AndroidBuildConfig::new(
            "com.example.app".to_string(),
            "1.0.0".to_string(),
            1,
        )
        .with_sdk_versions(26, 33, 33);

        assert_eq!(config.min_sdk_version, 26);
        assert_eq!(config.target_sdk_version, 33);
        assert_eq!(config.compile_sdk_version, 33);
    }

    #[test]
    fn test_distribution_manager_creation() {
        let manager = AndroidDistributionManager::new();
        assert_eq!(manager.packages().len(), 0);
    }

    #[test]
    fn test_register_build_config() {
        let mut manager = AndroidDistributionManager::new();
        let config = AndroidBuildConfig::new(
            "com.example.app".to_string(),
            "1.0.0".to_string(),
            1,
        );

        manager.register_build_config("default".to_string(), config);
        assert!(manager.get_build_config("default").is_some());
    }

    #[tokio::test]
    async fn test_create_aab_package() {
        let temp_dir = TempDir::new().unwrap();
        let native_libs_dir = temp_dir.path().join("libs");
        let output_dir = temp_dir.path().join("output");

        std::fs::create_dir_all(&native_libs_dir).unwrap();

        let mut manager = AndroidDistributionManager::new();
        let config = AndroidBuildConfig::new(
            "com.example.app".to_string(),
            "1.0.0".to_string(),
            1,
        );

        let result = manager.create_aab(config, &native_libs_dir, &output_dir).await;
        assert!(result.is_ok());

        let package = result.unwrap();
        assert_eq!(package.format, AndroidPackageFormat::Aab);
        assert!(package.package_path.exists());
    }

    #[tokio::test]
    async fn test_create_apk_package() {
        let temp_dir = TempDir::new().unwrap();
        let native_libs_dir = temp_dir.path().join("libs");
        let output_dir = temp_dir.path().join("output");

        std::fs::create_dir_all(&native_libs_dir).unwrap();

        let mut manager = AndroidDistributionManager::new();
        let config = AndroidBuildConfig::new(
            "com.example.app".to_string(),
            "1.0.0".to_string(),
            1,
        );

        let result = manager.create_apk(config, &native_libs_dir, &output_dir).await;
        assert!(result.is_ok());

        let package = result.unwrap();
        assert_eq!(package.format, AndroidPackageFormat::Apk);
        assert!(package.package_path.exists());
    }

    #[test]
    fn test_package_validation() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("test.apk");

        // Create a test package file
        let mut file = std::fs::File::create(&package_path).unwrap();
        file.write_all(b"TEST_APK_CONTENT").unwrap();

        let config = AndroidBuildConfig::new(
            "com.example.app".to_string(),
            "1.0.0".to_string(),
            1,
        );

        let package = AndroidPackage::from_file(
            AndroidPackageFormat::Apk,
            config,
            package_path,
        ).unwrap();

        let validation = package.validate().unwrap();
        assert!(validation.is_valid);
    }

    #[test]
    fn test_package_checksum() {
        let temp_dir = TempDir::new().unwrap();
        let package_path = temp_dir.path().join("test.apk");

        // Create a test package file
        std::fs::write(&package_path, b"TEST_CONTENT").unwrap();

        let config = AndroidBuildConfig::new(
            "com.example.app".to_string(),
            "1.0.0".to_string(),
            1,
        );

        let package = AndroidPackage::from_file(
            AndroidPackageFormat::Apk,
            config,
            package_path,
        ).unwrap();

        assert!(!package.checksum.is_empty());
        assert_eq!(package.checksum.len(), 64); // SHA256 hex string length
    }

    #[test]
    fn test_distribution_manifest() {
        let manager = AndroidDistributionManager::new();
        let manifest = manager.generate_manifest("1.0.0");

        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.packages.len(), 0);
    }

    #[test]
    fn test_manifest_json_serialization() {
        let manifest = AndroidDistributionManifest {
            version: "1.0.0".to_string(),
            packages: HashMap::new(),
        };

        let json = manifest.to_json();
        assert!(json.contains("1.0.0"));

        let parsed = AndroidDistributionManifest::from_json(&json).unwrap();
        assert_eq!(parsed.version, "1.0.0");
    }

    #[test]
    fn test_update_manager_creation() {
        let manager = AndroidUpdateManager::new("1.0.0".to_string());
        assert_eq!(manager.current_version(), "1.0.0");
        assert_eq!(manager.update_channel(), UpdateChannel::Stable);
    }

    #[test]
    fn test_update_manager_with_channel() {
        let manager = AndroidUpdateManager::new("1.0.0".to_string())
            .with_channel(UpdateChannel::Beta);

        assert_eq!(manager.update_channel(), UpdateChannel::Beta);
    }

    #[tokio::test]
    async fn test_validate_all_packages() {
        let temp_dir = TempDir::new().unwrap();
        let native_libs_dir = temp_dir.path().join("libs");
        let output_dir = temp_dir.path().join("output");

        std::fs::create_dir_all(&native_libs_dir).unwrap();

        let mut manager = AndroidDistributionManager::new();
        let config = AndroidBuildConfig::new(
            "com.example.app".to_string(),
            "1.0.0".to_string(),
            1,
        );

        manager.create_apk(config, &native_libs_dir, &output_dir).await.unwrap();

        let validation = manager.validate_all().unwrap();
        assert_eq!(validation.total_packages, 1);
    }
}
