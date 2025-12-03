// macOS Spotlight integration

use crate::platform::{PlatformResult, PlatformError};
use std::path::Path;
use std::process::Command;

/// Index a file for Spotlight search
pub fn index_file(file_path: &Path) -> PlatformResult<()> {
    // Spotlight automatically indexes files in standard locations
    // This function can be used to trigger manual indexing if needed
    
    if !file_path.exists() {
        return Err(PlatformError::IntegrationError(
            format!("File does not exist: {:?}", file_path)
        ));
    }
    
    // Use mdimport to manually index a file
    let output = Command::new("mdimport")
        .arg(file_path)
        .output()
        .map_err(|e| PlatformError::IntegrationError(
            format!("Failed to run mdimport: {}", e)
        ))?;
    
    if !output.status.success() {
        return Err(PlatformError::IntegrationError(
            format!("mdimport failed: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    Ok(())
}

/// Search for files using Spotlight
pub fn spotlight_search(query: &str, limit: usize) -> PlatformResult<Vec<String>> {
    let output = Command::new("mdfind")
        .arg("-name")
        .arg(query)
        .arg("-limit")
        .arg(limit.to_string())
        .output()
        .map_err(|e| PlatformError::IntegrationError(
            format!("Failed to run mdfind: {}", e)
        ))?;
    
    if !output.status.success() {
        return Err(PlatformError::IntegrationError(
            format!("mdfind failed: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    let results = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();
    
    Ok(results)
}

/// Get metadata for a file using Spotlight
pub fn get_file_metadata(file_path: &Path) -> PlatformResult<std::collections::HashMap<String, String>> {
    use std::collections::HashMap;
    
    if !file_path.exists() {
        return Err(PlatformError::IntegrationError(
            format!("File does not exist: {:?}", file_path)
        ));
    }
    
    let output = Command::new("mdls")
        .arg(file_path)
        .output()
        .map_err(|e| PlatformError::IntegrationError(
            format!("Failed to run mdls: {}", e)
        ))?;
    
    if !output.status.success() {
        return Err(PlatformError::IntegrationError(
            format!("mdls failed: {}", String::from_utf8_lossy(&output.stderr))
        ));
    }
    
    let mut metadata = HashMap::new();
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    for line in output_str.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            metadata.insert(key, value);
        }
    }
    
    Ok(metadata)
}
