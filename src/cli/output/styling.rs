// Color and styling management for terminal output

use crate::cli::{CLIError, CLIResult, Color, ColorMode, TextStyle};
use std::io::{self, IsTerminal};

/// Manages color and styling for terminal output
#[derive(Clone)]
pub struct StyleManager {
    color_mode: ColorMode,
    supports_color: bool,
}

impl StyleManager {
    pub fn new(color_mode: ColorMode) -> Self {
        let supports_color = Self::detect_color_support();
        Self {
            color_mode,
            supports_color,
        }
    }

    /// Detect if terminal supports color
    fn detect_color_support() -> bool {
        // Check if stdout is a terminal
        if !io::stdout().is_terminal() {
            return false;
        }

        // Check environment variables
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }

        if let Ok(term) = std::env::var("TERM") {
            if term == "dumb" {
                return false;
            }
            if term.contains("color") || term.contains("256") {
                return true;
            }
        }

        // Check for common color-supporting terminals
        if std::env::var("COLORTERM").is_ok() {
            return true;
        }

        // Default to true on Unix-like systems
        cfg!(unix)
    }

    /// Check if colors should be used
    fn should_use_color(&self) -> bool {
        match self.color_mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => self.supports_color,
        }
    }

    /// Apply text styling
    pub fn apply_style(&self, text: &str, style: TextStyle) -> CLIResult<String> {
        if !self.should_use_color() {
            return Ok(text.to_string());
        }

        let mut styled = String::new();
        let mut codes = Vec::new();

        // Add style codes
        if style.bold {
            codes.push("1");
        }
        if style.italic {
            codes.push("3");
        }
        if style.underline {
            codes.push("4");
        }

        // Add color code
        if let Some(color) = style.color {
            codes.push(self.color_to_code(color));
        }

        // Apply codes if any
        if !codes.is_empty() {
            styled.push_str("\x1b[");
            styled.push_str(&codes.join(";"));
            styled.push('m');
        }

        styled.push_str(text);

        // Reset if we applied any styling
        if !codes.is_empty() {
            styled.push_str("\x1b[0m");
        }

        Ok(styled)
    }

    /// Convert color enum to ANSI code
    fn color_to_code(&self, color: Color) -> &'static str {
        match color {
            Color::Black => "30",
            Color::Red => "31",
            Color::Green => "32",
            Color::Yellow => "33",
            Color::Blue => "34",
            Color::Magenta => "35",
            Color::Cyan => "36",
            Color::White => "37",
            Color::Gray => "90",
        }
    }

    /// Create a colored string (convenience method)
    pub fn colorize(&self, text: &str, color: Color) -> CLIResult<String> {
        self.apply_style(
            text,
            TextStyle {
                color: Some(color),
                ..Default::default()
            },
        )
    }

    /// Create a bold string (convenience method)
    pub fn bold(&self, text: &str) -> CLIResult<String> {
        self.apply_style(
            text,
            TextStyle {
                bold: true,
                ..Default::default()
            },
        )
    }

    /// Create a success message (green)
    pub fn success(&self, text: &str) -> CLIResult<String> {
        self.apply_style(
            text,
            TextStyle {
                color: Some(Color::Green),
                bold: true,
                ..Default::default()
            },
        )
    }

    /// Create an error message (red)
    pub fn error(&self, text: &str) -> CLIResult<String> {
        self.apply_style(
            text,
            TextStyle {
                color: Some(Color::Red),
                bold: true,
                ..Default::default()
            },
        )
    }

    /// Create a warning message (yellow)
    pub fn warning(&self, text: &str) -> CLIResult<String> {
        self.apply_style(
            text,
            TextStyle {
                color: Some(Color::Yellow),
                bold: true,
                ..Default::default()
            },
        )
    }

    /// Create an info message (cyan)
    pub fn info(&self, text: &str) -> CLIResult<String> {
        self.apply_style(
            text,
            TextStyle {
                color: Some(Color::Cyan),
                ..Default::default()
            },
        )
    }
}

/// Color manager for managing color schemes
pub struct ColorManager {
    style_manager: StyleManager,
}

impl ColorManager {
    pub fn new(color_mode: ColorMode) -> Self {
        Self {
            style_manager: StyleManager::new(color_mode),
        }
    }

    /// Get style manager
    pub fn style_manager(&self) -> &StyleManager {
        &self.style_manager
    }

    /// Apply a color scheme to text
    pub fn apply_scheme(&self, text: &str, scheme: ColorScheme) -> CLIResult<String> {
        match scheme {
            ColorScheme::Success => self.style_manager.success(text),
            ColorScheme::Error => self.style_manager.error(text),
            ColorScheme::Warning => self.style_manager.warning(text),
            ColorScheme::Info => self.style_manager.info(text),
            ColorScheme::Default => Ok(text.to_string()),
        }
    }
}

/// Predefined color schemes
#[derive(Debug, Clone, Copy)]
pub enum ColorScheme {
    Success,
    Error,
    Warning,
    Info,
    Default,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_disabled() {
        let manager = StyleManager::new(ColorMode::Never);
        let styled = manager.apply_style(
            "test",
            TextStyle {
                bold: true,
                color: Some(Color::Red),
                ..Default::default()
            },
        ).unwrap();
        assert_eq!(styled, "test");
    }

    #[test]
    fn test_color_enabled() {
        let manager = StyleManager::new(ColorMode::Always);
        let styled = manager.apply_style(
            "test",
            TextStyle {
                bold: true,
                color: Some(Color::Red),
                ..Default::default()
            },
        ).unwrap();
        assert!(styled.contains("\x1b["));
        assert!(styled.contains("test"));
    }

    #[test]
    fn test_convenience_methods() {
        let manager = StyleManager::new(ColorMode::Always);
        
        let success = manager.success("ok").unwrap();
        assert!(success.contains("ok"));
        
        let error = manager.error("fail").unwrap();
        assert!(error.contains("fail"));
        
        let warning = manager.warning("warn").unwrap();
        assert!(warning.contains("warn"));
    }
}
