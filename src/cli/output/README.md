# CLI Output Formatting System

This module provides comprehensive output formatting and display capabilities for the Kizuna CLI system.

## Components

### 1. Table Formatter (`table.rs`)
Formats structured data into visually appealing tables with:
- **Column alignment and styling**: Headers centered, data left-aligned
- **Responsive layout**: Automatically adapts to terminal width
- **Border rendering**: Unicode box-drawing characters for clean tables
- **Text truncation**: Handles overflow gracefully with ellipsis
- **Sortable and filterable**: Foundation for advanced table operations

**Key Features:**
- Automatic column width calculation
- Terminal width detection
- Proportional column resizing when space is limited
- Support for custom table styles

### 2. JSON/CSV/Minimal Formatters (`json.rs`)
Multiple output formats for different use cases:

#### JSON Formatter
- Pretty-printed or compact JSON output
- Converts table data to JSON array of objects
- Ideal for scripting and automation

#### CSV Formatter
- Standard CSV format with proper escaping
- Handles commas, quotes, and newlines in data
- Perfect for data export and analysis

#### Minimal Formatter
- Tab-separated values without headers
- Pipeline-friendly output
- Minimal overhead for scripting

### 3. Progress Display (`progress.rs`)
Real-time progress visualization with:
- **Progress bars**: Visual representation with percentage
- **Status information**: Current/total, speed, ETA
- **Color coding**: Green (complete), Cyan (in progress), Yellow (starting)
- **Indeterminate progress**: Animated spinner for unknown duration
- **Status indicators**: Colored symbols for different states

**Features:**
- Human-readable byte formatting (B, KB, MB, GB, TB)
- Duration formatting (seconds, minutes, hours)
- Customizable spinner animations
- Status-based color coding (✓ success, ✗ error, ⚠ warning, etc.)

### 4. Color and Styling Management (`styling.rs`)
Terminal color and text styling with:
- **Color detection**: Automatic terminal capability detection
- **Color modes**: Auto, Always, Never
- **Text styles**: Bold, italic, underline
- **Color palette**: 8 standard colors + gray
- **Convenience methods**: success(), error(), warning(), info()

**Features:**
- Respects NO_COLOR environment variable
- ANSI escape sequence generation
- Terminal capability detection
- Configurable color schemes

## Usage Examples

### Table Formatting
```rust
use kizuna::cli::{OutputFormatter, TableData, TableStyle, ColorMode};

let formatter = OutputFormatter::new(ColorMode::Auto);
let data = TableData {
    headers: vec!["Name".to_string(), "Status".to_string()],
    rows: vec![
        vec!["Alice".to_string(), "Connected".to_string()],
        vec!["Bob".to_string(), "Disconnected".to_string()],
    ],
};

let table = formatter.format_table(data, TableStyle::default())?;
println!("{}", table);
```

### JSON Output
```rust
use kizuna::cli::JSONFormatter;

let formatter = JSONFormatter::new();
let json = formatter.table_to_json(data)?;
let output = formatter.format(json, true)?; // pretty print
println!("{}", output);
```

### Progress Display
```rust
use kizuna::cli::{ProgressRenderer, ProgressInfo, StyleManager, ColorMode};
use std::time::Duration;

let renderer = ProgressRenderer::new(StyleManager::new(ColorMode::Auto));
let progress = ProgressInfo {
    current: 50,
    total: Some(100),
    rate: Some(10.0),
    eta: Some(Duration::from_secs(5)),
    message: Some("Transferring...".to_string()),
};

let display = renderer.render(progress)?;
println!("{}", display.bar);
println!("{}", display.status);
```

### Color and Styling
```rust
use kizuna::cli::{StyleManager, ColorMode, Color, TextStyle};

let manager = StyleManager::new(ColorMode::Auto);

// Convenience methods
println!("{}", manager.success("Operation completed")?);
println!("{}", manager.error("Operation failed")?);
println!("{}", manager.warning("Low disk space")?);

// Custom styling
let styled = manager.apply_style(
    "Important",
    TextStyle {
        bold: true,
        color: Some(Color::Red),
        ..Default::default()
    }
)?;
```

## Requirements Satisfied

This implementation satisfies the following requirements from the CLI UX specification:

- **Requirement 1.2**: Display peer information in structured format
- **Requirement 1.4**: Provide both JSON and human-readable output formats
- **Requirement 2.3**: Display transfer progress with speed and ETA information
- **Requirement 5.3**: Display streaming status and connection quality
- **Requirement 6.2**: Display command output in real-time with proper formatting
- **Requirement 7.3**: Display peer capabilities and status information
- **Requirement 9.2**: Support command-line options for output format configuration
- **Requirement 10.3**: Provide machine-readable output formats (JSON, CSV) for scripting

## Testing

The module includes comprehensive unit tests for:
- Table formatting with various data sizes
- Text truncation and padding
- CSV escaping and formatting
- JSON conversion and formatting
- Progress bar rendering
- Byte and duration formatting
- Color and styling application
- Terminal capability detection

Run tests with:
```bash
cargo test --lib cli::output
```

## Dependencies

- `terminal_size`: Terminal width detection
- `serde_json`: JSON serialization
- Standard library: I/O, formatting, time

## Architecture

The output system follows a modular design:
```
OutputFormatter (main facade)
├── TableFormatterImpl (table rendering)
├── JSONFormatter (JSON output)
├── CSVFormatter (CSV output)
├── MinimalFormatter (minimal output)
├── ProgressRenderer (progress display)
└── StyleManager (color and styling)
```

All formatters are independent and can be used directly or through the `OutputFormatter` facade.
