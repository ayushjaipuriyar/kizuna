// Windows installer and distribution support

use crate::platform::{PlatformResult, PlatformError};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Windows installer manager for MSI and MSIX packages
pub struct InstallerManager {
    app_name: String,
    app_version: String,
    publisher: String,
}

impl InstallerManager {
    pub fn new(app_name: String, app_version: String, publisher: String) -> Self {
        Self {
            app_name,
            app_version,
            publisher,
        }
    }

    /// Create MSI installer configuration
    pub fn create_msi_config(&self) -> PlatformResult<MSIConfig> {
        Ok(MSIConfig {
            product_name: self.app_name.clone(),
            product_version: self.app_version.clone(),
            manufacturer: self.publisher.clone(),
            upgrade_code: self.generate_upgrade_code(),
            install_dir: format!("ProgramFiles\\{}", self.app_name),
            features: vec![
                InstallerFeature {
                    id: "MainApplication".to_string(),
                    title: "Main Application".to_string(),
                    description: "Core application files".to_string(),
                    level: 1,
                },
                InstallerFeature {
                    id: "Documentation".to_string(),
                    title: "Documentation".to_string(),
                    description: "User documentation and help files".to_string(),
                    level: 2,
                },
            ],
            shortcuts: vec![
                Shortcut {
                    name: self.app_name.clone(),
                    target: format!("[INSTALLDIR]\\{}.exe", self.app_name),
                    location: ShortcutLocation::StartMenu,
                },
                Shortcut {
                    name: self.app_name.clone(),
                    target: format!("[INSTALLDIR]\\{}.exe", self.app_name),
                    location: ShortcutLocation::Desktop,
                },
            ],
            registry_keys: vec![
                RegistryKey {
                    root: "HKCU".to_string(),
                    key: format!("Software\\{}", self.app_name),
                    name: "InstallPath".to_string(),
                    value: "[INSTALLDIR]".to_string(),
                },
            ],
        })
    }

    /// Create MSIX package configuration for Microsoft Store
    pub fn create_msix_config(&self) -> PlatformResult<MSIXConfig> {
        Ok(MSIXConfig {
            package_name: self.app_name.clone(),
            package_version: self.app_version.clone(),
            publisher: self.publisher.clone(),
            publisher_display_name: self.publisher.clone(),
            identity_name: format!("com.{}.{}", self.publisher.to_lowercase(), self.app_name.to_lowercase()),
            capabilities: vec![
                "internetClient".to_string(),
                "privateNetworkClientServer".to_string(),
            ],
            target_device_families: vec![
                "Windows.Desktop".to_string(),
                "Windows.Universal".to_string(),
            ],
            min_version: "10.0.17763.0".to_string(),
            max_version_tested: "10.0.22621.0".to_string(),
        })
    }

    /// Generate WiX XML for MSI installer
    pub fn generate_wix_xml(&self, config: &MSIConfig) -> PlatformResult<String> {
        let mut xml = String::new();
        
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<Wix xmlns=\"http://schemas.microsoft.com/wix/2006/wi\">\n");
        xml.push_str(&format!("  <Product Id=\"*\" Name=\"{}\" Language=\"1033\" Version=\"{}\" Manufacturer=\"{}\" UpgradeCode=\"{}\">\n",
            config.product_name, config.product_version, config.manufacturer, config.upgrade_code));
        xml.push_str("    <Package InstallerVersion=\"200\" Compressed=\"yes\" InstallScope=\"perMachine\" />\n");
        xml.push_str("    <MajorUpgrade DowngradeErrorMessage=\"A newer version is already installed.\" />\n");
        xml.push_str("    <MediaTemplate EmbedCab=\"yes\" />\n");
        
        // Add features
        for feature in &config.features {
            xml.push_str(&format!("    <Feature Id=\"{}\" Title=\"{}\" Level=\"{}\">\n",
                feature.id, feature.title, feature.level));
            xml.push_str(&format!("      <ComponentGroupRef Id=\"{}Components\" />\n", feature.id));
            xml.push_str("    </Feature>\n");
        }
        
        xml.push_str("  </Product>\n");
        xml.push_str("</Wix>\n");
        
        Ok(xml)
    }

    /// Generate AppxManifest.xml for MSIX package
    pub fn generate_appx_manifest(&self, config: &MSIXConfig) -> PlatformResult<String> {
        let mut xml = String::new();
        
        xml.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
        xml.push_str("<Package xmlns=\"http://schemas.microsoft.com/appx/manifest/foundation/windows10\"\n");
        xml.push_str("         xmlns:uap=\"http://schemas.microsoft.com/appx/manifest/uap/windows10\">\n");
        xml.push_str(&format!("  <Identity Name=\"{}\" Publisher=\"{}\" Version=\"{}\" />\n",
            config.identity_name, config.publisher, config.package_version));
        xml.push_str("  <Properties>\n");
        xml.push_str(&format!("    <DisplayName>{}</DisplayName>\n", config.package_name));
        xml.push_str(&format!("    <PublisherDisplayName>{}</PublisherDisplayName>\n", config.publisher_display_name));
        xml.push_str("  </Properties>\n");
        xml.push_str("  <Dependencies>\n");
        xml.push_str(&format!("    <TargetDeviceFamily Name=\"Windows.Desktop\" MinVersion=\"{}\" MaxVersionTested=\"{}\" />\n",
            config.min_version, config.max_version_tested));
        xml.push_str("  </Dependencies>\n");
        xml.push_str("  <Capabilities>\n");
        for capability in &config.capabilities {
            xml.push_str(&format!("    <Capability Name=\"{}\" />\n", capability));
        }
        xml.push_str("  </Capabilities>\n");
        xml.push_str("</Package>\n");
        
        Ok(xml)
    }

    /// Build MSI installer package
    pub fn build_msi(&self, source_dir: &Path, output_dir: &Path) -> PlatformResult<PathBuf> {
        let config = self.create_msi_config()?;
        let wix_xml = self.generate_wix_xml(&config)?;
        
        // Write WiX XML to temporary file
        let wix_file = output_dir.join("installer.wxs");
        std::fs::write(&wix_file, wix_xml)
            .map_err(|e| PlatformError::SystemError(format!("Failed to write WiX file: {}", e)))?;
        
        // In production, you would call WiX toolset (candle.exe and light.exe) here
        // For now, we'll return the expected output path
        let msi_path = output_dir.join(format!("{}-{}.msi", self.app_name, self.app_version));
        
        Ok(msi_path)
    }

    /// Build MSIX package
    pub fn build_msix(&self, source_dir: &Path, output_dir: &Path) -> PlatformResult<PathBuf> {
        let config = self.create_msix_config()?;
        let manifest = self.generate_appx_manifest(&config)?;
        
        // Write AppxManifest.xml to source directory
        let manifest_file = source_dir.join("AppxManifest.xml");
        std::fs::write(&manifest_file, manifest)
            .map_err(|e| PlatformError::SystemError(format!("Failed to write manifest: {}", e)))?;
        
        // In production, you would call MakeAppx.exe here
        // For now, we'll return the expected output path
        let msix_path = output_dir.join(format!("{}-{}.msix", self.app_name, self.app_version));
        
        Ok(msix_path)
    }

    /// Sign installer package with code signing certificate
    pub fn sign_package(&self, package_path: &Path, cert_path: &Path) -> PlatformResult<()> {
        // In production, you would call SignTool.exe here
        // For now, we'll just validate the paths exist
        if !package_path.exists() {
            return Err(PlatformError::SystemError(
                format!("Package not found: {}", package_path.display())
            ));
        }
        
        Ok(())
    }

    /// Generate upgrade code (UUID) for MSI
    fn generate_upgrade_code(&self) -> String {
        // In production, this should be a stable UUID for the application
        // For now, we'll generate a deterministic one based on app name
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        self.app_name.hash(&mut hasher);
        let hash = hasher.finish();
        
        format!("{{{:08X}-{:04X}-{:04X}-{:04X}-{:012X}}}",
            (hash >> 32) as u32,
            ((hash >> 16) & 0xFFFF) as u16,
            (hash & 0xFFFF) as u16,
            ((hash >> 48) & 0xFFFF) as u16,
            hash & 0xFFFFFFFFFFFF)
    }
}

#[derive(Debug, Clone)]
pub struct MSIConfig {
    pub product_name: String,
    pub product_version: String,
    pub manufacturer: String,
    pub upgrade_code: String,
    pub install_dir: String,
    pub features: Vec<InstallerFeature>,
    pub shortcuts: Vec<Shortcut>,
    pub registry_keys: Vec<RegistryKey>,
}

#[derive(Debug, Clone)]
pub struct MSIXConfig {
    pub package_name: String,
    pub package_version: String,
    pub publisher: String,
    pub publisher_display_name: String,
    pub identity_name: String,
    pub capabilities: Vec<String>,
    pub target_device_families: Vec<String>,
    pub min_version: String,
    pub max_version_tested: String,
}

#[derive(Debug, Clone)]
pub struct InstallerFeature {
    pub id: String,
    pub title: String,
    pub description: String,
    pub level: u32,
}

#[derive(Debug, Clone)]
pub struct Shortcut {
    pub name: String,
    pub target: String,
    pub location: ShortcutLocation,
}

#[derive(Debug, Clone)]
pub enum ShortcutLocation {
    StartMenu,
    Desktop,
    Startup,
}

#[derive(Debug, Clone)]
pub struct RegistryKey {
    pub root: String,
    pub key: String,
    pub name: String,
    pub value: String,
}
