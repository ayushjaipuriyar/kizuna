// Command Template System
//
// This module provides command template creation, validation, and management
// with parameter placeholders and type checking.

use crate::command_execution::{
    error::{CommandError, CommandResult as CmdResult},
    types::*,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// Unique identifier for a template
pub type TemplateId = Uuid;

/// Command template with parameter placeholders
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTemplate {
    pub template_id: TemplateId,
    pub name: String,
    pub description: String,
    pub command: String,
    pub parameters: Vec<TemplateParameter>,
    pub sandbox_config: SandboxConfig,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub version: u32,
    pub owner: PeerId,
    pub tags: Vec<String>,
}

/// Template parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    pub name: String,
    pub description: String,
    pub param_type: ParameterType,
    pub required: bool,
    pub default_value: Option<String>,
    pub validation_pattern: Option<String>,
}

/// Parameter types for validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Path,
    Url,
    Email,
    Custom(String),
}

/// Template instantiation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInstantiationRequest {
    pub template_id: TemplateId,
    pub parameter_values: HashMap<String, String>,
    pub requester: PeerId,
}

/// Validation result for template parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

/// Validation error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub parameter_name: String,
    pub error_message: String,
}

/// Template manager for storage and operations
pub struct TemplateManager {
    templates: HashMap<TemplateId, CommandTemplate>,
    storage_path: Option<PathBuf>,
}

impl TemplateManager {
    /// Create a new template manager
    pub fn new(storage_path: Option<PathBuf>) -> Self {
        Self {
            templates: HashMap::new(),
            storage_path,
        }
    }

    /// Create a new command template
    pub fn create_template(
        &mut self,
        name: String,
        description: String,
        command: String,
        parameters: Vec<TemplateParameter>,
        sandbox_config: SandboxConfig,
        owner: PeerId,
        tags: Vec<String>,
    ) -> CmdResult<CommandTemplate> {
        // Validate template command contains parameter placeholders
        self.validate_template_command(&command, &parameters)?;

        let template = CommandTemplate {
            template_id: Uuid::new_v4(),
            name,
            description,
            command,
            parameters,
            sandbox_config,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: 1,
            owner,
            tags,
        };

        self.templates.insert(template.template_id, template.clone());
        
        // Persist to storage if configured
        if self.storage_path.is_some() {
            self.save_template(&template)?;
        }

        Ok(template)
    }

    /// Get a template by ID
    pub fn get_template(&self, template_id: &TemplateId) -> CmdResult<&CommandTemplate> {
        self.templates
            .get(template_id)
            .ok_or_else(|| CommandError::TemplateNotFound(template_id.to_string()))
    }

    /// Update an existing template
    pub fn update_template(
        &mut self,
        template_id: &TemplateId,
        name: Option<String>,
        description: Option<String>,
        command: Option<String>,
        parameters: Option<Vec<TemplateParameter>>,
        sandbox_config: Option<SandboxConfig>,
        tags: Option<Vec<String>>,
    ) -> CmdResult<CommandTemplate> {
        // Validate command if provided
        if let Some(ref cmd) = command {
            let template = self.templates
                .get(template_id)
                .ok_or_else(|| CommandError::TemplateNotFound(template_id.to_string()))?;
            let params = parameters.as_ref().unwrap_or(&template.parameters);
            self.validate_template_command(cmd, params)?;
        }

        // Now update the template
        let template = self.templates
            .get_mut(template_id)
            .ok_or_else(|| CommandError::TemplateNotFound(template_id.to_string()))?;

        if let Some(name) = name {
            template.name = name;
        }
        if let Some(description) = description {
            template.description = description;
        }
        if let Some(command) = command {
            template.command = command;
        }
        if let Some(parameters) = parameters {
            template.parameters = parameters;
        }
        if let Some(sandbox_config) = sandbox_config {
            template.sandbox_config = sandbox_config;
        }
        if let Some(tags) = tags {
            template.tags = tags;
        }

        template.updated_at = chrono::Utc::now();
        template.version += 1;

        let updated_template = template.clone();

        // Persist to storage if configured
        if self.storage_path.is_some() {
            self.save_template(&updated_template)?;
        }

        Ok(updated_template)
    }

    /// Delete a template
    pub fn delete_template(&mut self, template_id: &TemplateId) -> CmdResult<()> {
        self.templates
            .remove(template_id)
            .ok_or_else(|| CommandError::TemplateNotFound(template_id.to_string()))?;

        // Remove from storage if configured
        if let Some(storage_path) = &self.storage_path {
            let template_file = storage_path.join(format!("{}.json", template_id));
            if template_file.exists() {
                std::fs::remove_file(template_file)
                    .map_err(|e| CommandError::StorageError(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// List all templates
    pub fn list_templates(&self) -> Vec<&CommandTemplate> {
        self.templates.values().collect()
    }

    /// Search templates by name or tags
    pub fn search_templates(&self, query: &str) -> Vec<&CommandTemplate> {
        let query_lower = query.to_lowercase();
        self.templates
            .values()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower)
                    || t.description.to_lowercase().contains(&query_lower)
                    || t.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Validate template parameters
    pub fn validate_parameters(
        &self,
        template_id: &TemplateId,
        parameter_values: &HashMap<String, String>,
    ) -> CmdResult<ValidationResult> {
        let template = self.get_template(template_id)?;
        let mut errors = Vec::new();

        // Check required parameters
        for param in &template.parameters {
            if param.required && !parameter_values.contains_key(&param.name) {
                errors.push(ValidationError {
                    parameter_name: param.name.clone(),
                    error_message: format!("Required parameter '{}' is missing", param.name),
                });
                continue;
            }

            if let Some(value) = parameter_values.get(&param.name) {
                // Validate parameter type
                if let Err(e) = self.validate_parameter_type(value, &param.param_type) {
                    errors.push(ValidationError {
                        parameter_name: param.name.clone(),
                        error_message: e,
                    });
                }

                // Validate against pattern if provided
                if let Some(pattern) = &param.validation_pattern {
                    if let Err(e) = self.validate_parameter_pattern(value, pattern) {
                        errors.push(ValidationError {
                            parameter_name: param.name.clone(),
                            error_message: e,
                        });
                    }
                }
            }
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
        })
    }

    /// Instantiate a template with parameter values
    pub fn instantiate_template(
        &self,
        request: TemplateInstantiationRequest,
    ) -> CmdResult<CommandRequest> {
        let template = self.get_template(&request.template_id)?;

        // Validate parameters
        let validation = self.validate_parameters(&request.template_id, &request.parameter_values)?;
        if !validation.valid {
            return Err(CommandError::ValidationError(format!(
                "Parameter validation failed: {:?}",
                validation.errors
            )));
        }

        // Substitute parameters in command
        let mut command = template.command.clone();
        let mut parameter_values = request.parameter_values.clone();

        // Add default values for missing optional parameters
        for param in &template.parameters {
            if !param.required && !parameter_values.contains_key(&param.name) {
                if let Some(default) = &param.default_value {
                    parameter_values.insert(param.name.clone(), default.clone());
                }
            }
        }

        // Replace placeholders
        for (name, value) in &parameter_values {
            let placeholder = format!("{{{{{}}}}}", name);
            command = command.replace(&placeholder, value);
        }

        // Create command request
        Ok(CommandRequest {
            request_id: Uuid::new_v4(),
            command,
            arguments: vec![],
            working_directory: None,
            environment: HashMap::new(),
            timeout: template.sandbox_config.max_execution_time,
            sandbox_config: template.sandbox_config.clone(),
            requester: request.requester,
            created_at: chrono::Utc::now(),
        })
    }

    /// Validate template command contains valid parameter placeholders
    fn validate_template_command(
        &self,
        command: &str,
        parameters: &[TemplateParameter],
    ) -> CmdResult<()> {
        // Extract placeholders from command
        let placeholders = self.extract_placeholders(command);

        // Check that all placeholders have corresponding parameters
        for placeholder in &placeholders {
            if !parameters.iter().any(|p| &p.name == placeholder) {
                return Err(CommandError::ValidationError(format!(
                    "Placeholder '{{{{{}}}}}' has no corresponding parameter definition",
                    placeholder
                )));
            }
        }

        Ok(())
    }

    /// Extract parameter placeholders from command string
    fn extract_placeholders(&self, command: &str) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut chars = command.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '{' {
                if chars.peek() == Some(&'{') {
                    chars.next(); // consume second '{'
                    let mut placeholder = String::new();
                    
                    while let Some(c) = chars.next() {
                        if c == '}' {
                            if chars.peek() == Some(&'}') {
                                chars.next(); // consume second '}'
                                placeholders.push(placeholder);
                                break;
                            }
                        } else {
                            placeholder.push(c);
                        }
                    }
                }
            }
        }
        
        placeholders
    }

    /// Validate parameter value matches expected type
    fn validate_parameter_type(&self, value: &str, param_type: &ParameterType) -> Result<(), String> {
        match param_type {
            ParameterType::String => Ok(()),
            ParameterType::Integer => {
                value.parse::<i64>()
                    .map(|_| ())
                    .map_err(|_| format!("Value '{}' is not a valid integer", value))
            }
            ParameterType::Float => {
                value.parse::<f64>()
                    .map(|_| ())
                    .map_err(|_| format!("Value '{}' is not a valid float", value))
            }
            ParameterType::Boolean => {
                value.parse::<bool>()
                    .map(|_| ())
                    .map_err(|_| format!("Value '{}' is not a valid boolean", value))
            }
            ParameterType::Path => {
                // Basic path validation
                if value.is_empty() {
                    Err("Path cannot be empty".to_string())
                } else {
                    Ok(())
                }
            }
            ParameterType::Url => {
                // Basic URL validation
                if value.starts_with("http://") || value.starts_with("https://") {
                    Ok(())
                } else {
                    Err(format!("Value '{}' is not a valid URL", value))
                }
            }
            ParameterType::Email => {
                // Basic email validation
                if value.contains('@') && value.contains('.') {
                    Ok(())
                } else {
                    Err(format!("Value '{}' is not a valid email", value))
                }
            }
            ParameterType::Custom(_) => Ok(()), // Custom types require external validation
        }
    }

    /// Validate parameter value matches pattern
    fn validate_parameter_pattern(&self, value: &str, pattern: &str) -> Result<(), String> {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| format!("Invalid validation pattern: {}", e))?;
        
        if regex.is_match(value) {
            Ok(())
        } else {
            Err(format!("Value '{}' does not match pattern '{}'", value, pattern))
        }
    }

    /// Save template to storage
    fn save_template(&self, template: &CommandTemplate) -> CmdResult<()> {
        if let Some(storage_path) = &self.storage_path {
            std::fs::create_dir_all(storage_path)
                .map_err(|e| CommandError::StorageError(e.to_string()))?;

            let template_file = storage_path.join(format!("{}.json", template.template_id));
            let json = serde_json::to_string_pretty(template)?;

            std::fs::write(template_file, json)
                .map_err(|e| CommandError::StorageError(e.to_string()))?;
        }

        Ok(())
    }

    /// Load templates from storage
    pub fn load_templates(&mut self) -> CmdResult<usize> {
        if let Some(storage_path) = &self.storage_path {
            if !storage_path.exists() {
                return Ok(0);
            }

            let entries = std::fs::read_dir(storage_path)
                .map_err(|e| CommandError::StorageError(e.to_string()))?;

            let mut count = 0;
            for entry in entries {
                let entry = entry.map_err(|e| CommandError::StorageError(e.to_string()))?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let json = std::fs::read_to_string(&path)
                        .map_err(|e| CommandError::StorageError(e.to_string()))?;

                    let template: CommandTemplate = serde_json::from_str(&json)?;

                    self.templates.insert(template.template_id, template);
                    count += 1;
                }
            }

            Ok(count)
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_template() {
        let mut manager = TemplateManager::new(None);
        
        let params = vec![
            TemplateParameter {
                name: "filename".to_string(),
                description: "File to process".to_string(),
                param_type: ParameterType::Path,
                required: true,
                default_value: None,
                validation_pattern: None,
            },
        ];

        let result = manager.create_template(
            "List File".to_string(),
            "List contents of a file".to_string(),
            "cat {{filename}}".to_string(),
            params,
            SandboxConfig::default(),
            "test_peer".to_string(),
            vec!["file".to_string()],
        );

        assert!(result.is_ok());
        let template = result.unwrap();
        assert_eq!(template.name, "List File");
        assert_eq!(template.version, 1);
    }

    #[test]
    fn test_validate_parameters() {
        let mut manager = TemplateManager::new(None);
        
        let params = vec![
            TemplateParameter {
                name: "count".to_string(),
                description: "Number of items".to_string(),
                param_type: ParameterType::Integer,
                required: true,
                default_value: None,
                validation_pattern: None,
            },
        ];

        let template = manager.create_template(
            "Test".to_string(),
            "Test template".to_string(),
            "echo {{count}}".to_string(),
            params,
            SandboxConfig::default(),
            "test_peer".to_string(),
            vec![],
        ).unwrap();

        // Valid parameter
        let mut values = HashMap::new();
        values.insert("count".to_string(), "42".to_string());
        let result = manager.validate_parameters(&template.template_id, &values).unwrap();
        assert!(result.valid);

        // Invalid parameter type
        let mut values = HashMap::new();
        values.insert("count".to_string(), "not_a_number".to_string());
        let result = manager.validate_parameters(&template.template_id, &values).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_instantiate_template() {
        let mut manager = TemplateManager::new(None);
        
        let params = vec![
            TemplateParameter {
                name: "message".to_string(),
                description: "Message to display".to_string(),
                param_type: ParameterType::String,
                required: true,
                default_value: None,
                validation_pattern: None,
            },
        ];

        let template = manager.create_template(
            "Echo".to_string(),
            "Echo a message".to_string(),
            "echo {{message}}".to_string(),
            params,
            SandboxConfig::default(),
            "test_peer".to_string(),
            vec![],
        ).unwrap();

        let mut values = HashMap::new();
        values.insert("message".to_string(), "Hello World".to_string());

        let request = TemplateInstantiationRequest {
            template_id: template.template_id,
            parameter_values: values,
            requester: "test_peer".to_string(),
        };

        let result = manager.instantiate_template(request).unwrap();
        assert_eq!(result.command, "echo Hello World");
    }

    #[test]
    fn test_extract_placeholders() {
        let manager = TemplateManager::new(None);
        
        let command = "echo {{name}} and {{value}}";
        let placeholders = manager.extract_placeholders(command);
        
        assert_eq!(placeholders.len(), 2);
        assert!(placeholders.contains(&"name".to_string()));
        assert!(placeholders.contains(&"value".to_string()));
    }
}
