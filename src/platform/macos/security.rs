// macOS security features including Gatekeeper and code signing

use crate::platform::{PlatformResult, PlatformError};
use std::path::Path;
use std::process::Command;

/// Check if the application is code signed
pub fn is_code_signed() -> PlatformResult<bool> {
    let exe_path = std::env::current_exe()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to get executable path: {}", e)))?;
    
    check_code_signature(&exe_path)
}

/// Check if a specific file/bundle is code signed
pub fn check_code_signature(path: &Path) -> PlatformResult<bool> {
    let output = Command::new("codesign")
        .arg("--verify")
        .arg("--verbose")
        .arg(path)
        .output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to run codesign: {}", e)))?;
    
    Ok(output.status.success())
}

/// Get code signing information for a file
pub fn get_code_signing_info(path: &Path) -> PlatformResult<CodeSigningInfo> {
    let output = Command::new("codesign")
        .arg("--display")
        .arg("--verbose=4")
        .arg(path)
        .output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to run codesign: {}", e)))?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    let mut info = CodeSigningInfo {
        signed: output.status.success(),
        identifier: None,
        team_identifier: None,
        authority: Vec::new(),
        entitlements: Vec::new(),
    };
    
    for line in stderr.lines() {
        if line.contains("Identifier=") {
            if let Some(id) = line.split("Identifier=").nth(1) {
                info.identifier = Some(id.trim().to_string());
            }
        } else if line.contains("TeamIdentifier=") {
            if let Some(team) = line.split("TeamIdentifier=").nth(1) {
                info.team_identifier = Some(team.trim().to_string());
            }
        } else if line.contains("Authority=") {
            if let Some(auth) = line.split("Authority=").nth(1) {
                info.authority.push(auth.trim().to_string());
            }
        }
    }
    
    Ok(info)
}

/// Code signing information
#[derive(Debug, Clone)]
pub struct CodeSigningInfo {
    pub signed: bool,
    pub identifier: Option<String>,
    pub team_identifier: Option<String>,
    pub authority: Vec<String>,
    pub entitlements: Vec<String>,
}

/// Sign a binary or app bundle
pub fn sign_binary(
    path: &Path,
    identity: &str,
    entitlements: Option<&Path>,
) -> PlatformResult<()> {
    let mut cmd = Command::new("codesign");
    cmd.arg("--force")
        .arg("--sign")
        .arg(identity)
        .arg("--timestamp");
    
    if let Some(ent_path) = entitlements {
        cmd.arg("--entitlements").arg(ent_path);
    }
    
    cmd.arg(path);
    
    let output = cmd.output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to run codesign: {}", e)))?;
    
    if !output.status.success() {
        return Err(PlatformError::IntegrationError(
            format!("Code signing failed: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    Ok(())
}

/// Check Gatekeeper status
pub fn check_gatekeeper_status() -> PlatformResult<bool> {
    let output = Command::new("spctl")
        .arg("--status")
        .output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to run spctl: {}", e)))?;
    
    let status = String::from_utf8_lossy(&output.stdout);
    Ok(status.contains("assessments enabled"))
}

/// Assess if an application will pass Gatekeeper
pub fn assess_gatekeeper(path: &Path) -> PlatformResult<GatekeeperAssessment> {
    let output = Command::new("spctl")
        .arg("--assess")
        .arg("--verbose")
        .arg("--type")
        .arg("execute")
        .arg(path)
        .output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to run spctl: {}", e)))?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    Ok(GatekeeperAssessment {
        accepted: output.status.success(),
        message: stderr.to_string(),
    })
}

/// Gatekeeper assessment result
#[derive(Debug, Clone)]
pub struct GatekeeperAssessment {
    pub accepted: bool,
    pub message: String,
}

/// Submit app for notarization
pub fn submit_for_notarization(
    path: &Path,
    apple_id: &str,
    password: &str,
    team_id: &str,
) -> PlatformResult<String> {
    // First, create a zip archive for notarization
    let zip_path = path.with_extension("zip");
    
    let output = Command::new("ditto")
        .arg("-c")
        .arg("-k")
        .arg("--keepParent")
        .arg(path)
        .arg(&zip_path)
        .output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to create zip: {}", e)))?;
    
    if !output.status.success() {
        return Err(PlatformError::IntegrationError(
            format!("Failed to create zip: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    // Submit for notarization using notarytool
    let output = Command::new("xcrun")
        .arg("notarytool")
        .arg("submit")
        .arg(&zip_path)
        .arg("--apple-id")
        .arg(apple_id)
        .arg("--password")
        .arg(password)
        .arg("--team-id")
        .arg(team_id)
        .arg("--wait")
        .output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to submit for notarization: {}", e)))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    if !output.status.success() {
        return Err(PlatformError::IntegrationError(
            format!("Notarization failed: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    // Extract submission ID from output
    let submission_id = stdout
        .lines()
        .find(|line| line.contains("id:"))
        .and_then(|line| line.split("id:").nth(1))
        .map(|s| s.trim().to_string())
        .ok_or_else(|| PlatformError::IntegrationError("Failed to extract submission ID".to_string()))?;
    
    Ok(submission_id)
}

/// Staple notarization ticket to app bundle
pub fn staple_notarization(path: &Path) -> PlatformResult<()> {
    let output = Command::new("xcrun")
        .arg("stapler")
        .arg("staple")
        .arg(path)
        .output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to staple: {}", e)))?;
    
    if !output.status.success() {
        return Err(PlatformError::IntegrationError(
            format!("Stapling failed: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    Ok(())
}

/// Check if running with hardened runtime
pub fn is_hardened_runtime() -> PlatformResult<bool> {
    let exe_path = std::env::current_exe()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to get executable path: {}", e)))?;
    
    let output = Command::new("codesign")
        .arg("--display")
        .arg("--verbose")
        .arg(&exe_path)
        .output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to run codesign: {}", e)))?;
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(stderr.contains("runtime"))
}

/// Get entitlements for a binary
pub fn get_entitlements(path: &Path) -> PlatformResult<String> {
    let output = Command::new("codesign")
        .arg("--display")
        .arg("--entitlements")
        .arg("-")
        .arg(path)
        .output()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to run codesign: {}", e)))?;
    
    if !output.status.success() {
        return Err(PlatformError::IntegrationError(
            format!("Failed to get entitlements: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
