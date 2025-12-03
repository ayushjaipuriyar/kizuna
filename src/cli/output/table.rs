// Table formatting for structured data display

use crate::cli::{CLIError, CLIResult, TableData, TableStyle};
use crate::cli::output::styling::StyleManager;
use std::cmp::max;

/// Trait for table formatting
pub trait TableFormatter {
    fn format(&self, data: TableData, style: TableStyle) -> CLIResult<String>;
    fn format_with_width(&self, data: TableData, style: TableStyle, max_width: usize) -> CLIResult<String>;
}

/// Implementation of table formatter
#[derive(Clone)]
pub struct TableFormatterImpl {
    style_manager: StyleManager,
}

impl TableFormatterImpl {
    pub fn new(style_manager: StyleManager) -> Self {
        Self { style_manager }
    }

    /// Get terminal width or default to 80
    fn get_terminal_width(&self) -> usize {
        terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80)
    }

    /// Calculate column widths based on content and available space
    fn calculate_column_widths(&self, data: &TableData, max_width: usize) -> Vec<usize> {
        if data.headers.is_empty() {
            return Vec::new();
        }

        let num_cols = data.headers.len();
        let mut widths = vec![0; num_cols];

        // Calculate minimum width needed for each column
        for (i, header) in data.headers.iter().enumerate() {
            widths[i] = header.len();
        }

        for row in &data.rows {
            for (i, cell) in row.iter().enumerate().take(num_cols) {
                widths[i] = max(widths[i], cell.len());
            }
        }

        // Account for borders and padding
        let border_overhead = if num_cols > 0 {
            3 * num_cols + 1 // "| " + " |" for each column
        } else {
            0
        };

        let total_content_width: usize = widths.iter().sum();
        let total_needed = total_content_width + border_overhead;

        // If we exceed max width, proportionally reduce column widths
        if total_needed > max_width && max_width > border_overhead {
            let available = max_width - border_overhead;
            let scale = available as f64 / total_content_width as f64;
            
            for width in &mut widths {
                *width = max(3, (*width as f64 * scale) as usize); // Minimum 3 chars per column
            }
        }

        widths
    }

    /// Truncate text to fit within width
    fn truncate(&self, text: &str, width: usize) -> String {
        if text.len() <= width {
            text.to_string()
        } else if width > 3 {
            format!("{}...", &text[..width - 3])
        } else {
            text.chars().take(width).collect()
        }
    }

    /// Pad text to width with alignment
    fn pad(&self, text: &str, width: usize, align: Alignment) -> String {
        let text_len = text.len();
        if text_len >= width {
            return text.to_string();
        }

        let padding = width - text_len;
        match align {
            Alignment::Left => format!("{}{}", text, " ".repeat(padding)),
            Alignment::Right => format!("{}{}", " ".repeat(padding), text),
            Alignment::Center => {
                let left_pad = padding / 2;
                let right_pad = padding - left_pad;
                format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
            }
        }
    }

    /// Create a horizontal border line
    fn create_border(&self, widths: &[usize], style: BorderStyle) -> String {
        let (left, mid, right, fill) = match style {
            BorderStyle::Top => ("┌", "┬", "┐", "─"),
            BorderStyle::Middle => ("├", "┼", "┤", "─"),
            BorderStyle::Bottom => ("└", "┴", "┘", "─"),
        };

        let mut line = String::from(left);
        for (i, &width) in widths.iter().enumerate() {
            line.push_str(&fill.repeat(width + 2)); // +2 for padding
            if i < widths.len() - 1 {
                line.push_str(mid);
            }
        }
        line.push_str(right);
        line
    }

    /// Format a row with given widths
    fn format_row(&self, cells: &[String], widths: &[usize], align: Alignment) -> String {
        let mut row = String::from("│");
        for (i, cell) in cells.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(10);
            let truncated = self.truncate(cell, width);
            let padded = self.pad(&truncated, width, align);
            row.push(' ');
            row.push_str(&padded);
            row.push_str(" │");
        }
        row
    }
}

impl TableFormatter for TableFormatterImpl {
    fn format(&self, data: TableData, style: TableStyle) -> CLIResult<String> {
        let max_width = self.get_terminal_width();
        self.format_with_width(data, style, max_width)
    }

    fn format_with_width(&self, data: TableData, style: TableStyle, max_width: usize) -> CLIResult<String> {
        if data.headers.is_empty() {
            return Ok(String::new());
        }

        let widths = self.calculate_column_widths(&data, max_width);
        let mut output = String::new();

        // Top border
        if style.borders {
            output.push_str(&self.create_border(&widths, BorderStyle::Top));
            output.push('\n');
        }

        // Header row
        let header_row = self.format_row(&data.headers, &widths, Alignment::Center);
        let styled_header = self.style_manager.apply_style(&header_row, style.header_style)
            .unwrap_or_else(|_| header_row.clone());
        output.push_str(&styled_header);
        output.push('\n');

        // Middle border after header
        if style.borders && !data.rows.is_empty() {
            output.push_str(&self.create_border(&widths, BorderStyle::Middle));
            output.push('\n');
        }

        // Data rows
        for row in &data.rows {
            let data_row = self.format_row(row, &widths, Alignment::Left);
            let styled_row = self.style_manager.apply_style(&data_row, style.row_style)
                .unwrap_or_else(|_| data_row.clone());
            output.push_str(&styled_row);
            output.push('\n');
        }

        // Bottom border
        if style.borders {
            output.push_str(&self.create_border(&widths, BorderStyle::Bottom));
            output.push('\n');
        }

        Ok(output)
    }
}

#[derive(Debug, Clone, Copy)]
enum Alignment {
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone, Copy)]
enum BorderStyle {
    Top,
    Middle,
    Bottom,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{ColorMode, TextStyle};

    #[test]
    fn test_empty_table() {
        let style_manager = StyleManager::new(ColorMode::Never);
        let formatter = TableFormatterImpl::new(style_manager);
        let data = TableData {
            headers: vec![],
            rows: vec![],
        };
        let result = formatter.format(data, TableStyle::default()).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_simple_table() {
        let style_manager = StyleManager::new(ColorMode::Never);
        let formatter = TableFormatterImpl::new(style_manager);
        let data = TableData {
            headers: vec!["Name".to_string(), "Age".to_string()],
            rows: vec![
                vec!["Alice".to_string(), "30".to_string()],
                vec!["Bob".to_string(), "25".to_string()],
            ],
        };
        let result = formatter.format(data, TableStyle::default()).unwrap();
        assert!(result.contains("Name"));
        assert!(result.contains("Alice"));
        assert!(result.contains("Bob"));
    }

    #[test]
    fn test_truncation() {
        let style_manager = StyleManager::new(ColorMode::Never);
        let formatter = TableFormatterImpl::new(style_manager);
        let truncated = formatter.truncate("Very long text that needs truncation", 10);
        assert_eq!(truncated.len(), 10);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_padding() {
        let style_manager = StyleManager::new(ColorMode::Never);
        let formatter = TableFormatterImpl::new(style_manager);
        
        let left = formatter.pad("test", 10, Alignment::Left);
        assert_eq!(left.len(), 10);
        assert!(left.starts_with("test"));
        
        let right = formatter.pad("test", 10, Alignment::Right);
        assert_eq!(right.len(), 10);
        assert!(right.ends_with("test"));
    }
}
