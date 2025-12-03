// macOS app bundle creation and management

use crate::platform::{PlatformResult, PlatformError};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;

/// App bundle structure
#[derive(Debug, Clone)]
pub struct AppBundle {
    pub name: String,
    pub identifier: String,
    pub version: String,
    pub executable_name: String,
    pub icon_path: Option<PathBuf>,
    pub category: Option<String>,
    pub copyright: Option<String>,
    pub minimum_system_version: Option<String>,
}

impl AppBundle {
    pub fn new(name: String, identifier: String, version: String) -> Self {
        let executable_name = name.clone();
        Self {
            name,
            identifier,
            version,
            executable_name,
            icon_path: None,
            category: None,
            copyright: None,
            minimum_system_version: Some("10.15".to_string()),
        }
    }

    /// Create the app bundle directory structure
    pub fn create_bundle(&self, output_dir: &Path, binary_path: &Path) -> PlatformResult<PathBuf> {
        let bundle_name = format!("{}.app", self.name);
        let bundle_path = output_dir.join(&bundle_name);
        
        // Create directory structure
        let contents_dir = bundle_path.join("Contents");
        let macos_dir = contents_dir.join("MacOS");
        let resources_dir = contents_dir.join("Resources");
        
        fs::create_dir_all(&macos_dir)
            .map_err(|e| PlatformError::IntegrationError(format!("Failed to create MacOS dir: {}", e)))?;
        fs::create_dir_all(&resources_dir)
            .map_err(|e| PlatformError::IntegrationError(format!("Failed to create Resources dir: {}", e)))?;
        
        // Copy executable
        let exe_dest = macos_dir.join(&self.executable_name);
        fs::copy(binary_path, &exe_dest)
            .map_err(|e| PlatformError::IntegrationError(format!("Failed to copy executable: {}", e)))?;
        
        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&exe_dest)
                .map_err(|e| PlatformError::IntegrationError(format!("Failed to get permissions: {}", e)))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&exe_dest, perms)
                .map_err(|e| PlatformError::IntegrationError(format!("Failed to set permissions: {}", e)))?;
        }
        
        // Copy icon if provided
        if let Some(icon_path) = &self.icon_path {
            if icon_path.exists() {
                let icon_dest = resources_dir.join(format!("{}.icns", self.name));
                fs::copy(icon_path, icon_dest)
                    .map_err(|e| PlatformError::IntegrationError(format!("Failed to copy icon: {}", e)))?;
            }
        }
        
        // Create Info.plist
        self.create_info_plist(&contents_dir)?;
        
        // Create PkgInfo
        self.create_pkg_info(&contents_dir)?;
        
        Ok(bundle_path)
    }

    /// Create Info.plist file
    fn create_info_plist(&self, contents_dir: &Path) -> PlatformResult<()> {
        let plist_path = contents_dir.join("Info.plist");
        let mut file = fs::File::create(&plist_path)
            .map_err(|e| PlatformError::IntegrationError(format!("Failed to create Info.plist: {}", e)))?;
        
        let icon_file = self.icon_path.as_ref()
            .map(|_| format!("{}.icns", self.name))
            .unwrap_or_default();
        
        let min_version = self.minimum_system_version.as_deref().unwrap_or("10.15");
        let category = self.category.as_deref().unwrap_or("public.app-category.utilities");
        let copyright = self.copyright.as_deref().unwrap_or("");
        
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>{}</string>
    <key>CFBundleIdentifier</key>
    <string>{}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>{}</string>
    <key>CFBundleVersion</key>
    <string>{}</string>
    <key>LSMinimumSystemVersion</key>
    <string>{}</string>
    <key>LSApplicationCategoryType</key>
    <string>{}</string>
    <key>NSHumanReadableCopyright</key>
    <string>{}</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
    {}
</dict>
</plist>"#,
            self.executable_name,
            self.identifier,
            self.name,
            self.version,
            self.version,
            min_version,
            category,
            copyright,
            if !icon_file.is_empty() {
                format!("<key>CFBundleIconFile</key>\n    <string>{}</string>", icon_file)
            } else {
                String::new()
            }
        );
        
        file.write_all(plist_content.as_bytes())
            .map_err(|e| PlatformError::IntegrationError(format!("Failed to write Info.plist: {}", e)))?;
        
        Ok(())
    }

    /// Create PkgInfo file
    fn create_pkg_info(&self, contents_dir: &Path) -> PlatformResult<()> {
        let pkginfo_path = contents_dir.join("PkgInfo");
        let mut file = fs::File::create(&pkginfo_path)
            .map_err(|e| PlatformError::IntegrationError(format!("Failed to create PkgInfo: {}", e)))?;
        
        file.write_all(b"APPL????")
            .map_err(|e| PlatformError::IntegrationError(format!("Failed to write PkgInfo: {}", e)))?;
        
        Ok(())
    }
}

/// Create a DMG disk image from an app bundle
pub fn create_dmg(
    app_bundle_path: &Path,
    output_path: &Path,
    volume_name: &str,
) -> PlatformResult<()> {
    use std::process::Command;
    
    // Create temporary directory for DMG contents
    let temp_dir = std::env::temp_dir().join(format!("dmg-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir)
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to create temp dir: {}", e)))?;
    
    // Copy app bundle to temp directory
    let app_name = app_bundle_path.file_name()
        .ok_or_else(|| PlatformError::IntegrationError("Invalid app bundle path".to_string()))?;
    let temp_app = temp_dir.join(app_name);
    
    copy_dir_recursive(app_bundle_path, &temp_app)?;
    
    // Create DMG
    let output = Command::new("hdiutil")
        .arg("create")
        .arg("-volname")
        .arg(volume_name)
        .arg("-srcfolder")
        .arg(&temp_dir)
        .arg("-ov")
        .arg("-format")
        .arg("UDZO")
        .arg(output_path)
        .output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to run hdiutil: {}", e)))?;
    
    // Clean up temp directory
    let _ = fs::remove_dir_all(&temp_dir);
    
    if !output.status.success() {
        return Err(PlatformError::IntegrationError(
            format!("DMG creation failed: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    Ok(())
}

/// Helper function to recursively copy directories
fn copy_dir_recursive(src: &Path, dst: &Path) -> PlatformResult<()> {
    fs::create_dir_all(dst)
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to create directory: {}", e)))?;
    
    for entry in fs::read_dir(src)
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to read directory: {}", e)))? {
        let entry = entry
            .map_err(|e| PlatformError::IntegrationError(format!("Failed to read entry: {}", e)))?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());
        
        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)
                .map_err(|e| PlatformError::IntegrationError(format!("Failed to copy file: {}", e)))?;
        }
    }
    
    Ok(())
}

/// Check if running on Apple Silicon
pub fn is_apple_silicon() -> bool {
    #[cfg(target_arch = "aarch64")]
    {
        true
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        false
    }
}

/// Get the current architecture string
pub fn get_architecture() -> &'static str {
    if is_apple_silicon() {
        "arm64"
    } else {
        "x86_64"
    }
}

/// Create a universal binary from Intel and ARM binaries
pub fn create_universal_binary(
    intel_binary: &Path,
    arm_binary: &Path,
    output_path: &Path,
) -> PlatformResult<()> {
    use std::process::Command;
    
    let output = Command::new("lipo")
        .arg("-create")
        .arg("-output")
        .arg(output_path)
        .arg(intel_binary)
        .arg(arm_binary)
        .output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to run lipo: {}", e)))?;
    
    if !output.status.success() {
        return Err(PlatformError::IntegrationError(
            format!("Universal binary creation failed: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    Ok(())
}
