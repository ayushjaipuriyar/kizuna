// CLI Output Formatting Demo
// Demonstrates the output formatting capabilities of the CLI system

use kizuna::cli::{
    ColorMode, OutputFormatter, TableData, TableStyle, ProgressInfo, TextStyle, Color,
};
use std::time::Duration;

fn main() {
    println!("=== CLI Output Formatting Demo ===\n");

    // Create output formatter with color support
    let formatter = OutputFormatter::new(ColorMode::Auto);

    // Demo 1: Table formatting
    println!("1. Table Formatting:");
    println!("-------------------");
    let table_data = TableData {
        headers: vec![
            "Peer Name".to_string(),
            "Device Type".to_string(),
            "Status".to_string(),
            "Last Seen".to_string(),
        ],
        rows: vec![
            vec![
                "Alice's Laptop".to_string(),
                "Desktop".to_string(),
                "Connected".to_string(),
                "2 mins ago".to_string(),
            ],
            vec![
                "Bob's Phone".to_string(),
                "Mobile".to_string(),
                "Disconnected".to_string(),
                "1 hour ago".to_string(),
            ],
            vec![
                "Server-01".to_string(),
                "Server".to_string(),
                "Connected".to_string(),
                "Just now".to_string(),
            ],
        ],
    };

    match formatter.format_table(table_data.clone(), TableStyle::default()) {
        Ok(table) => println!("{}", table),
        Err(e) => eprintln!("Error formatting table: {}", e),
    }

    // Demo 2: JSON output
    println!("\n2. JSON Output:");
    println!("---------------");
    let json_formatter = kizuna::cli::JSONFormatter::new();
    match json_formatter.table_to_json(table_data.clone()) {
        Ok(json) => match json_formatter.format(json, true) {
            Ok(formatted) => println!("{}", formatted),
            Err(e) => eprintln!("Error formatting JSON: {}", e),
        },
        Err(e) => eprintln!("Error converting to JSON: {}", e),
    }

    // Demo 3: CSV output
    println!("\n3. CSV Output:");
    println!("--------------");
    let csv_formatter = kizuna::cli::CSVFormatter::new();
    match csv_formatter.format(table_data.clone()) {
        Ok(csv) => println!("{}", csv),
        Err(e) => eprintln!("Error formatting CSV: {}", e),
    }

    // Demo 4: Minimal output
    println!("\n4. Minimal Output (tab-separated):");
    println!("-----------------------------------");
    let minimal_formatter = kizuna::cli::MinimalFormatter::new();
    match minimal_formatter.format(table_data) {
        Ok(minimal) => println!("{}", minimal),
        Err(e) => eprintln!("Error formatting minimal: {}", e),
    }

    // Demo 5: Progress bars
    println!("\n5. Progress Display:");
    println!("--------------------");
    
    let progress_renderer = kizuna::cli::ProgressRenderer::new(
        kizuna::cli::StyleManager::new(ColorMode::Auto)
    );

    // Progress at 50%
    let progress = ProgressInfo {
        current: 512 * 1024 * 1024, // 512 MB
        total: Some(1024 * 1024 * 1024), // 1 GB
        rate: Some(10.5 * 1024.0 * 1024.0), // 10.5 MB/s
        eta: Some(Duration::from_secs(49)),
        message: Some("Transferring file.zip".to_string()),
    };

    match progress_renderer.render(progress) {
        Ok(display) => {
            println!("{}", display.bar);
            println!("{}", display.status);
            if !display.details.is_empty() {
                println!("{}", display.details);
            }
        }
        Err(e) => eprintln!("Error rendering progress: {}", e),
    }

    // Demo 6: Status indicators
    println!("\n6. Status Indicators:");
    println!("---------------------");
    let statuses = vec!["completed", "failed", "warning", "running", "cancelled"];
    for status in statuses {
        match progress_renderer.render_status_indicator(status) {
            Ok(indicator) => println!("{}", indicator),
            Err(e) => eprintln!("Error rendering status: {}", e),
        }
    }

    // Demo 7: Color and styling
    println!("\n7. Color and Styling:");
    println!("---------------------");
    let style_manager = kizuna::cli::StyleManager::new(ColorMode::Auto);
    
    if let Ok(success) = style_manager.success("✓ Operation completed successfully") {
        println!("{}", success);
    }
    
    if let Ok(error) = style_manager.error("✗ Operation failed") {
        println!("{}", error);
    }
    
    if let Ok(warning) = style_manager.warning("⚠ Warning: Low disk space") {
        println!("{}", warning);
    }
    
    if let Ok(info) = style_manager.info("ℹ Information: 3 peers discovered") {
        println!("{}", info);
    }

    println!("\n=== Demo Complete ===");
}
