// Output formatting and display system for CLI

pub mod table;
pub mod json;
pub mod progress;
pub mod styling;

pub use table::{TableFormatter, TableFormatterImpl};
pub use json::{JSONFormatter, CSVFormatter, MinimalFormatter};
pub use progress::{ProgressRenderer, ProgressDisplay};
pub use styling::{ColorManager, StyleManager};

use crate::cli::{CLIError, CLIResult, OutputFormat, TableData, ProgressInfo};

/// Main output formatter that delegates to specific formatters
pub struct OutputFormatter {
    table_formatter: TableFormatterImpl,
    json_formatter: JSONFormatter,
    csv_formatter: CSVFormatter,
    minimal_formatter: MinimalFormatter,
    progress_renderer: ProgressRenderer,
    style_manager: StyleManager,
}

impl OutputFormatter {
    pub fn new(color_mode: crate::cli::ColorMode) -> Self {
        let style_manager = StyleManager::new(color_mode);
        Self {
            table_formatter: TableFormatterImpl::new(style_manager.clone()),
            json_formatter: JSONFormatter::new(),
            csv_formatter: CSVFormatter::new(),
            minimal_formatter: MinimalFormatter::new(),
            progress_renderer: ProgressRenderer::new(style_manager.clone()),
            style_manager,
        }
    }

    pub fn format_table(&self, data: TableData, style: crate::cli::TableStyle) -> CLIResult<String> {
        self.table_formatter.format(data, style)
    }

    pub fn format_json(&self, data: serde_json::Value, pretty: bool) -> CLIResult<String> {
        self.json_formatter.format(data, pretty)
    }

    pub fn format_csv(&self, data: TableData) -> CLIResult<String> {
        self.csv_formatter.format(data)
    }

    pub fn format_minimal(&self, data: TableData) -> CLIResult<String> {
        self.minimal_formatter.format(data)
    }

    pub fn render_progress(&self, progress: ProgressInfo) -> CLIResult<ProgressDisplay> {
        self.progress_renderer.render(progress)
    }

    pub fn apply_styling(&self, text: &str, style: crate::cli::TextStyle) -> CLIResult<String> {
        self.style_manager.apply_style(text, style)
    }
}
