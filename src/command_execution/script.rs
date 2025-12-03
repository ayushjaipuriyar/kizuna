use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use crate::command_execution::{
    error::{CommandError, CommandResult as CmdResult},
    types::*,
    sandbox::{Sandbox, SandboxEngine, DefaultSandboxEngine},
};

/// Parsed script ready for parameter substitution
#[derive(Debug, Clone)]
pub struct ParsedScript {
    pub content: String,
    pub language: ScriptLanguage,
    pub parameters: Vec<String>, // List of parameter names found in script
}

/// Executable script with parameters substituted
#[derive(Debug, Clone)]
pub struct ExecutableScript {
    pub content: String,
    pub language: ScriptLanguage,
    pub interpreter_path: String,
    pub environment: HashMap<String, String>,
}

/// Script validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ScriptError>,
    pub warnings: Vec<String>,
}

/// Script execution progress callback
pub type ProgressCallback = Box<dyn Fn(ScriptProgress) + Send + Sync>;

/// Script execution progress information
#[derive(Debug, Clone)]
pub struct ScriptProgress {
    pub stage: ExecutionStage,
    pub message: String,
    pub percentage: Option<u8>,
}

/// Stages of script execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStage {
    Parsing,
    Validating,
    Preparing,
    Executing,
    Collecting,
    Cleaning,
    Complete,
}

/// Script Engine trait for multi-language script execution
#[async_trait]
pub trait ScriptEngine: Send + Sync {
    /// Parse a script and identify parameters
    async fn parse_script(
        &self,
        content: String,
        language: ScriptLanguage,
    ) -> CmdResult<ParsedScript>;

    /// Substitute parameters in a parsed script
    async fn substitute_parameters(
        &self,
        script: ParsedScript,
        params: HashMap<String, String>,
    ) -> CmdResult<ExecutableScript>;

    /// Execute a script within a sandbox
    async fn execute_script(
        &self,
        script: ExecutableScript,
        sandbox: &Sandbox,
    ) -> CmdResult<ScriptResult>;

    /// Validate script syntax without executing
    async fn validate_script_syntax(
        &self,
        content: String,
        language: ScriptLanguage,
    ) -> CmdResult<ValidationResult>;

    /// Detect script language from content
    async fn detect_language(&self, content: &str) -> CmdResult<ScriptLanguage>;
}

/// Default implementation of ScriptEngine
pub struct DefaultScriptEngine {
    sandbox_engine: DefaultSandboxEngine,
}

impl DefaultScriptEngine {
    /// Create a new script engine
    pub fn new() -> Self {
        Self {
            sandbox_engine: DefaultSandboxEngine::new(),
        }
    }

    /// Create execution status update
    fn create_progress(&self, stage: ExecutionStage, message: &str, percentage: Option<u8>) -> ScriptProgress {
        ScriptProgress {
            stage,
            message: message.to_string(),
            percentage,
        }
    }

    /// Get interpreter path for a given language
    fn get_interpreter_path(&self, language: ScriptLanguage) -> CmdResult<String> {
        match language {
            ScriptLanguage::Bash => {
                // Try to find bash, fallback to sh
                #[cfg(not(target_os = "windows"))]
                {
                    if std::path::Path::new("/bin/bash").exists() {
                        Ok("/bin/bash".to_string())
                    } else if std::path::Path::new("/usr/bin/bash").exists() {
                        Ok("/usr/bin/bash".to_string())
                    } else {
                        Ok("/bin/sh".to_string())
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    // On Windows, try Git Bash or WSL bash
                    Ok("bash".to_string())
                }
            }
            ScriptLanguage::PowerShell => {
                #[cfg(target_os = "windows")]
                {
                    Ok("powershell.exe".to_string())
                }
                #[cfg(not(target_os = "windows"))]
                {
                    // Try pwsh (PowerShell Core) on Unix systems
                    Ok("pwsh".to_string())
                }
            }
            ScriptLanguage::Python => {
                // Try python3 first, then python
                Ok("python3".to_string())
            }
            ScriptLanguage::JavaScript => {
                // Use node
                Ok("node".to_string())
            }
            ScriptLanguage::Batch => {
                #[cfg(target_os = "windows")]
                {
                    Ok("cmd.exe".to_string())
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Err(CommandError::invalid_request(
                        "Batch scripts are only supported on Windows"
                    ))
                }
            }
            ScriptLanguage::Auto => {
                Err(CommandError::invalid_request(
                    "Cannot get interpreter for Auto language - detect language first"
                ))
            }
        }
    }

    /// Get file extension for a script language
    fn get_file_extension(&self, language: ScriptLanguage) -> &str {
        match language {
            ScriptLanguage::Bash => "sh",
            ScriptLanguage::PowerShell => "ps1",
            ScriptLanguage::Python => "py",
            ScriptLanguage::JavaScript => "js",
            ScriptLanguage::Batch => "bat",
            ScriptLanguage::Auto => "txt",
        }
    }

    /// Extract parameters from script content
    fn extract_parameters(&self, content: &str, language: ScriptLanguage) -> Vec<String> {
        let mut parameters = Vec::new();
        
        match language {
            ScriptLanguage::Bash => {
                // Look for ${PARAM_NAME} or $PARAM_NAME patterns
                let re = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}|\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();
                for cap in re.captures_iter(content) {
                    if let Some(param) = cap.get(1).or_else(|| cap.get(2)) {
                        let param_name = param.as_str().to_string();
                        if !parameters.contains(&param_name) {
                            parameters.push(param_name);
                        }
                    }
                }
            }
            ScriptLanguage::PowerShell => {
                // Look for $PARAM_NAME patterns
                let re = Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();
                for cap in re.captures_iter(content) {
                    if let Some(param) = cap.get(1) {
                        let param_name = param.as_str().to_string();
                        if !parameters.contains(&param_name) {
                            parameters.push(param_name);
                        }
                    }
                }
            }
            ScriptLanguage::Python => {
                // Look for {PARAM_NAME} patterns (for format strings)
                let re = Regex::new(r"\{([A-Za-z_][A-Za-z0-9_]*)\}").unwrap();
                for cap in re.captures_iter(content) {
                    if let Some(param) = cap.get(1) {
                        let param_name = param.as_str().to_string();
                        if !parameters.contains(&param_name) {
                            parameters.push(param_name);
                        }
                    }
                }
            }
            ScriptLanguage::JavaScript => {
                // Look for ${PARAM_NAME} patterns (template literals)
                let re = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").unwrap();
                for cap in re.captures_iter(content) {
                    if let Some(param) = cap.get(1) {
                        let param_name = param.as_str().to_string();
                        if !parameters.contains(&param_name) {
                            parameters.push(param_name);
                        }
                    }
                }
            }
            ScriptLanguage::Batch => {
                // Look for %PARAM_NAME% patterns
                let re = Regex::new(r"%([A-Za-z_][A-Za-z0-9_]*)%").unwrap();
                for cap in re.captures_iter(content) {
                    if let Some(param) = cap.get(1) {
                        let param_name = param.as_str().to_string();
                        if !parameters.contains(&param_name) {
                            parameters.push(param_name);
                        }
                    }
                }
            }
            ScriptLanguage::Auto => {}
        }
        
        parameters
    }

    /// Perform parameter substitution in script content
    fn substitute_params(&self, content: &str, params: &HashMap<String, String>, language: ScriptLanguage) -> String {
        let mut result = content.to_string();
        
        match language {
            ScriptLanguage::Bash => {
                // Replace ${PARAM_NAME} and $PARAM_NAME patterns
                for (key, value) in params {
                    // Escape special characters in the value for shell safety
                    let escaped_value = value.replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('$', "\\$")
                        .replace('`', "\\`");
                    
                    result = result.replace(&format!("${{{}}}", key), &escaped_value);
                    // Be careful with $PARAM_NAME to avoid replacing partial matches
                    let re = Regex::new(&format!(r"\${}(?![A-Za-z0-9_])", regex::escape(key))).unwrap();
                    result = re.replace_all(&result, escaped_value.as_str()).to_string();
                }
            }
            ScriptLanguage::PowerShell => {
                // Replace $PARAM_NAME patterns
                for (key, value) in params {
                    // Escape special characters for PowerShell
                    let escaped_value = value.replace('`', "``")
                        .replace('$', "`$")
                        .replace('"', "`\"");
                    
                    let re = Regex::new(&format!(r"\${}(?![A-Za-z0-9_])", regex::escape(key))).unwrap();
                    result = re.replace_all(&result, escaped_value.as_str()).to_string();
                }
            }
            ScriptLanguage::Python => {
                // Replace {PARAM_NAME} patterns
                for (key, value) in params {
                    // For Python, we'll use simple string replacement
                    // In production, consider using proper Python string formatting
                    result = result.replace(&format!("{{{}}}", key), value);
                }
            }
            ScriptLanguage::JavaScript => {
                // Replace ${PARAM_NAME} patterns
                for (key, value) in params {
                    // Escape special characters for JavaScript
                    let escaped_value = value.replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('`', "\\`")
                        .replace('$', "\\$");
                    
                    result = result.replace(&format!("${{{}}}", key), &escaped_value);
                }
            }
            ScriptLanguage::Batch => {
                // Replace %PARAM_NAME% patterns
                for (key, value) in params {
                    result = result.replace(&format!("%{}%", key), value);
                }
            }
            ScriptLanguage::Auto => {}
        }
        
        result
    }

    /// Parse error messages from script output with enhanced context
    fn parse_errors(&self, stderr: &str, language: ScriptLanguage) -> Vec<ScriptError> {
        let mut errors = Vec::new();
        
        match language {
            ScriptLanguage::Bash => {
                // Parse bash error format: "script.sh: line 5: command not found"
                let re = Regex::new(r"line (\d+):(.+)").unwrap();
                for cap in re.captures_iter(stderr) {
                    if let (Some(line), Some(msg)) = (cap.get(1), cap.get(2)) {
                        if let Ok(line_num) = line.as_str().parse::<usize>() {
                            errors.push(ScriptError {
                                line: Some(line_num),
                                message: self.format_error_message(msg.as_str().trim(), language),
                                error_type: "BashError".to_string(),
                            });
                        }
                    }
                }
                
                // If no structured errors found, add the whole stderr as one error
                if errors.is_empty() && !stderr.is_empty() {
                    errors.push(ScriptError {
                        line: None,
                        message: self.format_error_message(stderr, language),
                        error_type: "BashError".to_string(),
                    });
                }
            }
            ScriptLanguage::PowerShell => {
                // Parse PowerShell error format
                let re = Regex::new(r"At line:(\d+) char:(\d+)").unwrap();
                let mut found_errors = false;
                for cap in re.captures_iter(stderr) {
                    if let Some(line) = cap.get(1) {
                        if let Ok(line_num) = line.as_str().parse::<usize>() {
                            // Extract the error message from the full stderr
                            let error_msg = self.extract_powershell_error(stderr, line_num);
                            errors.push(ScriptError {
                                line: Some(line_num),
                                message: self.format_error_message(&error_msg, language),
                                error_type: "PowerShellError".to_string(),
                            });
                            found_errors = true;
                        }
                    }
                }
                
                if !found_errors && !stderr.is_empty() {
                    errors.push(ScriptError {
                        line: None,
                        message: self.format_error_message(stderr, language),
                        error_type: "PowerShellError".to_string(),
                    });
                }
            }
            ScriptLanguage::Python => {
                // Parse Python error format: "  File "script.py", line 5"
                let re = Regex::new(r#"File ".*", line (\d+)"#).unwrap();
                let mut found_errors = false;
                for cap in re.captures_iter(stderr) {
                    if let Some(line) = cap.get(1) {
                        if let Ok(line_num) = line.as_str().parse::<usize>() {
                            // Extract the specific error and traceback
                            let error_msg = self.extract_python_error(stderr, line_num);
                            errors.push(ScriptError {
                                line: Some(line_num),
                                message: self.format_error_message(&error_msg, language),
                                error_type: "PythonError".to_string(),
                            });
                            found_errors = true;
                        }
                    }
                }
                
                if !found_errors && !stderr.is_empty() {
                    errors.push(ScriptError {
                        line: None,
                        message: self.format_error_message(stderr, language),
                        error_type: "PythonError".to_string(),
                    });
                }
            }
            ScriptLanguage::JavaScript => {
                // Parse Node.js error format
                if !stderr.is_empty() {
                    errors.push(ScriptError {
                        line: None,
                        message: self.format_error_message(stderr, language),
                        error_type: "JavaScriptError".to_string(),
                    });
                }
            }
            ScriptLanguage::Batch => {
                if !stderr.is_empty() {
                    errors.push(ScriptError {
                        line: None,
                        message: self.format_error_message(stderr, language),
                        error_type: "BatchError".to_string(),
                    });
                }
            }
            ScriptLanguage::Auto => {}
        }
        
        errors
    }

    /// Format error message with additional context
    fn format_error_message(&self, message: &str, language: ScriptLanguage) -> String {
        let trimmed = message.trim();
        if trimmed.is_empty() {
            return format!("{:?} script execution failed", language);
        }
        
        // Limit error message length for readability
        if trimmed.len() > 500 {
            format!("{}...\n(error truncated)", &trimmed[..500])
        } else {
            trimmed.to_string()
        }
    }

    /// Extract PowerShell error message with context
    fn extract_powershell_error(&self, stderr: &str, line_num: usize) -> String {
        let lines: Vec<&str> = stderr.lines().collect();
        let mut error_lines = Vec::new();
        
        for (i, line) in lines.iter().enumerate() {
            if line.contains(&format!("At line:{}", line_num)) {
                // Include this line and the next few lines for context
                error_lines.push(*line);
                for j in (i + 1)..std::cmp::min(i + 5, lines.len()) {
                    error_lines.push(lines[j]);
                }
                break;
            }
        }
        
        if error_lines.is_empty() {
            stderr.to_string()
        } else {
            error_lines.join("\n")
        }
    }

    /// Extract Python error message with traceback
    fn extract_python_error(&self, stderr: &str, line_num: usize) -> String {
        let lines: Vec<&str> = stderr.lines().collect();
        let mut error_lines = Vec::new();
        let mut in_traceback = false;
        
        for line in lines.iter() {
            if line.contains("Traceback") {
                in_traceback = true;
            }
            
            if in_traceback {
                error_lines.push(*line);
                
                // Stop after the actual error message (lines that don't start with whitespace)
                if !line.starts_with(' ') && !line.starts_with('\t') && !line.contains("Traceback") {
                    break;
                }
            }
        }
        
        if error_lines.is_empty() {
            stderr.to_string()
        } else {
            error_lines.join("\n")
        }
    }

    /// Count lines executed (approximation based on script content)
    fn count_lines_executed(&self, content: &str) -> usize {
        content.lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("//")
            })
            .count()
    }

    /// Setup isolated environment for script execution
    fn setup_environment(&self, base_env: &HashMap<String, String>, sandbox_config: &SandboxConfig) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // If environment isolation is enabled, start with minimal environment
        if sandbox_config.environment_isolation {
            // Add only essential environment variables
            env.insert("PATH".to_string(), std::env::var("PATH").unwrap_or_default());
            env.insert("HOME".to_string(), std::env::var("HOME").unwrap_or_default());
            
            // Add temp directory if available
            if let Some(temp_dir) = &sandbox_config.temp_directory {
                env.insert("TMPDIR".to_string(), temp_dir.to_string_lossy().to_string());
                env.insert("TEMP".to_string(), temp_dir.to_string_lossy().to_string());
                env.insert("TMP".to_string(), temp_dir.to_string_lossy().to_string());
            }
        } else {
            // Copy current environment
            for (key, value) in std::env::vars() {
                env.insert(key, value);
            }
        }

        // Add script-specific environment variables
        for (key, value) in base_env {
            env.insert(key.clone(), value.clone());
        }

        env
    }

    /// Cleanup script execution environment
    async fn cleanup_environment(&self, temp_dir: &PathBuf, script_path: &PathBuf) -> CmdResult<()> {
        // Remove script file
        if script_path.exists() {
            tokio::fs::remove_file(script_path).await
                .map_err(|e| CommandError::execution_error(format!("Failed to remove script file: {}", e)))?;
        }

        // Clean up any temporary files created during execution
        // (This is a basic cleanup - in production, you might want more sophisticated cleanup)
        let mut entries = tokio::fs::read_dir(temp_dir).await
            .map_err(|e| CommandError::execution_error(format!("Failed to read temp directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| CommandError::execution_error(format!("Failed to read directory entry: {}", e)))? {
            let path = entry.path();
            if path.is_file() {
                // Only remove files that look like temporary script files
                if let Some(filename) = path.file_name() {
                    if filename.to_string_lossy().starts_with("script_") {
                        let _ = tokio::fs::remove_file(&path).await;
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for DefaultScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ScriptEngine for DefaultScriptEngine {
    async fn parse_script(
        &self,
        content: String,
        language: ScriptLanguage,
    ) -> CmdResult<ParsedScript> {
        // Auto-detect language if needed
        let detected_language = if language == ScriptLanguage::Auto {
            self.detect_language(&content).await?
        } else {
            language
        };

        // Extract parameters from the script
        let parameters = self.extract_parameters(&content, detected_language);

        Ok(ParsedScript {
            content,
            language: detected_language,
            parameters,
        })
    }

    async fn substitute_parameters(
        &self,
        script: ParsedScript,
        params: HashMap<String, String>,
    ) -> CmdResult<ExecutableScript> {
        // Perform parameter substitution
        let substituted_content = self.substitute_params(&script.content, &params, script.language);

        // Get interpreter path
        let interpreter_path = self.get_interpreter_path(script.language)?;

        // Setup environment variables from parameters
        let mut environment = HashMap::new();
        for (key, value) in params.iter() {
            environment.insert(key.clone(), value.clone());
        }

        // Add language-specific environment variables
        match script.language {
            ScriptLanguage::Python => {
                environment.insert("PYTHONUNBUFFERED".to_string(), "1".to_string());
            }
            ScriptLanguage::JavaScript => {
                environment.insert("NODE_ENV".to_string(), "production".to_string());
            }
            _ => {}
        }

        Ok(ExecutableScript {
            content: substituted_content,
            language: script.language,
            interpreter_path,
            environment,
        })
    }

    async fn execute_script(
        &self,
        script: ExecutableScript,
        sandbox: &Sandbox,
    ) -> CmdResult<ScriptResult> {
        let start_time = Instant::now();
        let request_id = uuid::Uuid::new_v4();

        // Create a temporary script file in the sandbox temp directory
        let temp_dir = sandbox.temp_dir.as_ref()
            .ok_or_else(|| CommandError::sandbox_error("Sandbox has no temp directory"))?;

        let script_filename = format!("script_{}.{}", 
            uuid::Uuid::new_v4(), 
            self.get_file_extension(script.language)
        );
        let script_path = temp_dir.join(&script_filename);

        // Write script content to file
        let mut file = fs::File::create(&script_path).await
            .map_err(|e| CommandError::execution_error(format!("Failed to create script file: {}", e)))?;
        
        file.write_all(script.content.as_bytes()).await
            .map_err(|e| CommandError::execution_error(format!("Failed to write script file: {}", e)))?;
        
        file.flush().await
            .map_err(|e| CommandError::execution_error(format!("Failed to flush script file: {}", e)))?;

        // Make script executable on Unix systems
        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&script_path).await
                .map_err(|e| CommandError::execution_error(format!("Failed to get file metadata: {}", e)))?
                .permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&script_path, perms).await
                .map_err(|e| CommandError::execution_error(format!("Failed to set file permissions: {}", e)))?;
        }

        // Build command arguments based on language
        let args = match script.language {
            ScriptLanguage::Bash => vec![script_path.to_string_lossy().to_string()],
            ScriptLanguage::PowerShell => vec![
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-File".to_string(),
                script_path.to_string_lossy().to_string(),
            ],
            ScriptLanguage::Python => vec![script_path.to_string_lossy().to_string()],
            ScriptLanguage::JavaScript => vec![script_path.to_string_lossy().to_string()],
            ScriptLanguage::Batch => vec![
                "/C".to_string(),
                script_path.to_string_lossy().to_string(),
            ],
            ScriptLanguage::Auto => {
                return Err(CommandError::invalid_request(
                    "Cannot execute script with Auto language"
                ));
            }
        };

        // Setup environment for script execution
        let _environment = self.setup_environment(&script.environment, &sandbox.config);
        // Note: The sandbox engine handles environment setup internally
        // This is here for documentation and future enhancement

        // Execute the script in the sandbox
        let result = self.sandbox_engine.execute_in_sandbox(
            sandbox,
            &script.interpreter_path,
            &args,
        ).await;

        // Parse errors from stderr if execution succeeded
        let (errors, exit_code, output) = match result {
            Ok(res) => {
                let errors = self.parse_errors(&res.stderr, script.language);
                (errors, res.exit_code, res.stdout)
            }
            Err(e) => {
                // Clean up before returning error
                let _ = self.cleanup_environment(temp_dir, &script_path).await;
                return Err(e);
            }
        };

        // Count lines executed
        let lines_executed = self.count_lines_executed(&script.content);

        // Clean up script execution environment
        let _ = self.cleanup_environment(temp_dir, &script_path).await;

        Ok(ScriptResult {
            request_id,
            exit_code,
            output,
            errors,
            execution_time: start_time.elapsed(),
            lines_executed,
        })
    }

    async fn validate_script_syntax(
        &self,
        content: String,
        language: ScriptLanguage,
    ) -> CmdResult<ValidationResult> {
        let detected_language = if language == ScriptLanguage::Auto {
            self.detect_language(&content).await?
        } else {
            language
        };

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Basic validation checks
        if content.trim().is_empty() {
            errors.push(ScriptError {
                line: None,
                message: "Script content is empty".to_string(),
                error_type: "ValidationError".to_string(),
            });
            return Ok(ValidationResult {
                is_valid: false,
                errors,
                warnings,
            });
        }

        // Language-specific validation
        match detected_language {
            ScriptLanguage::Bash => {
                // Check for common bash syntax issues
                if content.contains("#!/bin/bash") && !content.starts_with("#!/bin/bash") {
                    warnings.push("Shebang should be on the first line".to_string());
                }
            }
            ScriptLanguage::PowerShell => {
                // Check for common PowerShell issues
                if content.contains("$ErrorActionPreference") {
                    warnings.push("Script modifies error action preference".to_string());
                }
            }
            ScriptLanguage::Python => {
                // Check for Python syntax basics
                let lines: Vec<&str> = content.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.ends_with(':') && i + 1 < lines.len() {
                        let next_line = lines[i + 1].trim();
                        if !next_line.is_empty() && !next_line.starts_with(' ') && !next_line.starts_with('\t') {
                            warnings.push(format!("Line {} may have indentation issues", i + 2));
                        }
                    }
                }
            }
            ScriptLanguage::JavaScript => {
                // Basic JavaScript checks
                let open_braces = content.matches('{').count();
                let close_braces = content.matches('}').count();
                if open_braces != close_braces {
                    errors.push(ScriptError {
                        line: None,
                        message: "Mismatched braces".to_string(),
                        error_type: "SyntaxError".to_string(),
                    });
                }
            }
            ScriptLanguage::Batch => {
                // Basic batch file checks
                if content.to_lowercase().contains("del /f /q") {
                    warnings.push("Script contains potentially dangerous delete commands".to_string());
                }
            }
            ScriptLanguage::Auto => {}
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    async fn detect_language(&self, content: &str) -> CmdResult<ScriptLanguage> {
        let trimmed = content.trim();
        
        // Check shebang line
        if let Some(first_line) = trimmed.lines().next() {
            if first_line.starts_with("#!") {
                if first_line.contains("bash") || first_line.contains("/sh") {
                    return Ok(ScriptLanguage::Bash);
                } else if first_line.contains("python") {
                    return Ok(ScriptLanguage::Python);
                } else if first_line.contains("node") {
                    return Ok(ScriptLanguage::JavaScript);
                } else if first_line.contains("pwsh") || first_line.contains("powershell") {
                    return Ok(ScriptLanguage::PowerShell);
                }
            }
        }

        // Check for language-specific patterns
        if trimmed.contains("@echo off") || trimmed.contains("@ECHO OFF") {
            return Ok(ScriptLanguage::Batch);
        }

        if trimmed.contains("param(") || trimmed.contains("$PSVersionTable") {
            return Ok(ScriptLanguage::PowerShell);
        }

        if trimmed.contains("def ") && trimmed.contains(":") {
            return Ok(ScriptLanguage::Python);
        }

        if trimmed.contains("function ") && trimmed.contains("{") {
            return Ok(ScriptLanguage::JavaScript);
        }

        if trimmed.contains("echo ") || trimmed.contains("export ") {
            return Ok(ScriptLanguage::Bash);
        }

        // Default to bash for Unix-like systems, batch for Windows
        #[cfg(target_os = "windows")]
        {
            Ok(ScriptLanguage::Batch)
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(ScriptLanguage::Bash)
        }
    }
}
