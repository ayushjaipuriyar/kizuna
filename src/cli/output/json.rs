// JSON, CSV, and minimal output formatters

use crate::cli::{CLIError, CLIResult, TableData};
use serde_json;

/// JSON output formatter
pub struct JSONFormatter;

impl JSONFormatter {
    pub fn new() -> Self {
        Self
    }

    /// Format data as JSON
    pub fn format(&self, data: serde_json::Value, pretty: bool) -> CLIResult<String> {
        if pretty {
            serde_json::to_string_pretty(&data)
                .map_err(|e| CLIError::FormatError(format!("JSON formatting error: {}", e)))
        } else {
            serde_json::to_string(&data)
                .map_err(|e| CLIError::FormatError(format!("JSON formatting error: {}", e)))
        }
    }

    /// Convert table data to JSON array of objects
    pub fn table_to_json(&self, data: TableData) -> CLIResult<serde_json::Value> {
        let mut rows = Vec::new();
        
        for row in data.rows {
            let mut obj = serde_json::Map::new();
            for (i, value) in row.iter().enumerate() {
                if let Some(header) = data.headers.get(i) {
                    obj.insert(header.clone(), serde_json::Value::String(value.clone()));
                }
            }
            rows.push(serde_json::Value::Object(obj));
        }

        Ok(serde_json::Value::Array(rows))
    }
}

/// CSV output formatter
pub struct CSVFormatter;

impl CSVFormatter {
    pub fn new() -> Self {
        Self
    }

    /// Format table data as CSV
    pub fn format(&self, data: TableData) -> CLIResult<String> {
        let mut output = String::new();

        // Write headers
        if !data.headers.is_empty() {
            output.push_str(&self.format_row(&data.headers));
            output.push('\n');
        }

        // Write rows
        for row in &data.rows {
            output.push_str(&self.format_row(row));
            output.push('\n');
        }

        Ok(output)
    }

    /// Format a single row with CSV escaping
    fn format_row(&self, cells: &[String]) -> String {
        cells
            .iter()
            .map(|cell| self.escape_csv_field(cell))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Escape a CSV field (quote if necessary)
    fn escape_csv_field(&self, field: &str) -> String {
        // Quote if contains comma, quote, or newline
        if field.contains(',') || field.contains('"') || field.contains('\n') {
            format!("\"{}\"", field.replace('"', "\"\""))
        } else {
            field.to_string()
        }
    }
}

/// Minimal output formatter (tab-separated, no headers)
pub struct MinimalFormatter;

impl MinimalFormatter {
    pub fn new() -> Self {
        Self
    }

    /// Format table data in minimal format
    pub fn format(&self, data: TableData) -> CLIResult<String> {
        let mut output = String::new();

        // Only output data rows, no headers
        for row in &data.rows {
            output.push_str(&row.join("\t"));
            output.push('\n');
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_formatting() {
        let formatter = JSONFormatter::new();
        let value = serde_json::json!({"name": "test", "value": 42});
        
        let pretty = formatter.format(value.clone(), true).unwrap();
        assert!(pretty.contains("name"));
        assert!(pretty.contains("test"));
        
        let compact = formatter.format(value, false).unwrap();
        assert!(compact.contains("name"));
        assert!(!compact.contains("  ")); // No indentation
    }

    #[test]
    fn test_table_to_json() {
        let formatter = JSONFormatter::new();
        let data = TableData {
            headers: vec!["name".to_string(), "age".to_string()],
            rows: vec![
                vec!["Alice".to_string(), "30".to_string()],
                vec!["Bob".to_string(), "25".to_string()],
            ],
        };

        let json = formatter.table_to_json(data).unwrap();
        assert!(json.is_array());
        let array = json.as_array().unwrap();
        assert_eq!(array.len(), 2);
    }

    #[test]
    fn test_csv_formatting() {
        let formatter = CSVFormatter::new();
        let data = TableData {
            headers: vec!["name".to_string(), "age".to_string()],
            rows: vec![
                vec!["Alice".to_string(), "30".to_string()],
                vec!["Bob".to_string(), "25".to_string()],
            ],
        };

        let csv = formatter.format(data).unwrap();
        assert!(csv.contains("name,age"));
        assert!(csv.contains("Alice,30"));
        assert!(csv.contains("Bob,25"));
    }

    #[test]
    fn test_csv_escaping() {
        let formatter = CSVFormatter::new();
        let data = TableData {
            headers: vec!["name".to_string()],
            rows: vec![
                vec!["Smith, John".to_string()],
                vec!["O\"Brien".to_string()],
            ],
        };

        let csv = formatter.format(data).unwrap();
        assert!(csv.contains("\"Smith, John\""));
        assert!(csv.contains("\"O\"\"Brien\""));
    }

    #[test]
    fn test_minimal_formatting() {
        let formatter = MinimalFormatter::new();
        let data = TableData {
            headers: vec!["name".to_string(), "age".to_string()],
            rows: vec![
                vec!["Alice".to_string(), "30".to_string()],
                vec!["Bob".to_string(), "25".to_string()],
            ],
        };

        let minimal = formatter.format(data).unwrap();
        assert!(!minimal.contains("name")); // No headers
        assert!(minimal.contains("Alice\t30"));
        assert!(minimal.contains("Bob\t25"));
    }
}
