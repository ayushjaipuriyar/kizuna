// Progress display and real-time updates

use crate::cli::{CLIError, CLIResult, Color, ProgressInfo, TextStyle};
use crate::cli::output::styling::StyleManager;
use std::time::Duration;

/// Progress display structure
#[derive(Debug, Clone)]
pub struct ProgressDisplay {
    pub bar: String,
    pub status: String,
    pub details: String,
}

/// Progress renderer for creating progress bars and status displays
pub struct ProgressRenderer {
    style_manager: StyleManager,
}

impl ProgressRenderer {
    pub fn new(style_manager: StyleManager) -> Self {
        Self { style_manager }
    }

    /// Render progress information into a display
    pub fn render(&self, progress: ProgressInfo) -> CLIResult<ProgressDisplay> {
        let bar = self.render_progress_bar(&progress)?;
        let status = self.render_status(&progress)?;
        let details = self.render_details(&progress)?;

        Ok(ProgressDisplay {
            bar,
            status,
            details,
        })
    }

    /// Render a progress bar
    fn render_progress_bar(&self, progress: &ProgressInfo) -> CLIResult<String> {
        let bar_width = 40;
        
        if let Some(total) = progress.total {
            if total == 0 {
                return Ok(self.render_indeterminate_bar(bar_width));
            }

            let percentage = (progress.current as f64 / total as f64 * 100.0).min(100.0);
            let filled = ((progress.current as f64 / total as f64) * bar_width as f64) as usize;
            let filled = filled.min(bar_width);
            let empty = bar_width - filled;

            let bar = format!(
                "[{}{}] {:.1}%",
                "█".repeat(filled),
                "░".repeat(empty),
                percentage
            );

            // Color based on progress
            let color = if percentage >= 100.0 {
                Color::Green
            } else if percentage >= 50.0 {
                Color::Cyan
            } else {
                Color::Yellow
            };

            self.style_manager.apply_style(
                &bar,
                TextStyle {
                    color: Some(color),
                    ..Default::default()
                },
            )
        } else {
            // Indeterminate progress
            Ok(self.render_indeterminate_bar(bar_width))
        }
    }

    /// Render an indeterminate progress bar (spinner style)
    fn render_indeterminate_bar(&self, width: usize) -> String {
        let spinner_pos = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() / 100) as usize % width;

        let mut bar = vec!['░'; width];
        for i in 0..5.min(width) {
            let pos = (spinner_pos + i) % width;
            bar[pos] = '█';
        }

        format!("[{}] ...", bar.iter().collect::<String>())
    }

    /// Render status line with speed and ETA
    fn render_status(&self, progress: &ProgressInfo) -> CLIResult<String> {
        let mut parts = Vec::new();

        // Current/Total
        if let Some(total) = progress.total {
            parts.push(format!(
                "{} / {}",
                self.format_bytes(progress.current),
                self.format_bytes(total)
            ));
        } else {
            parts.push(self.format_bytes(progress.current));
        }

        // Speed
        if let Some(rate) = progress.rate {
            parts.push(format!("{}/s", self.format_bytes(rate as u64)));
        }

        // ETA
        if let Some(eta) = progress.eta {
            parts.push(format!("ETA: {}", self.format_duration(eta)));
        }

        Ok(parts.join(" | "))
    }

    /// Render additional details
    fn render_details(&self, progress: &ProgressInfo) -> CLIResult<String> {
        if let Some(ref message) = progress.message {
            self.style_manager.apply_style(
                message,
                TextStyle {
                    color: Some(Color::Gray),
                    ..Default::default()
                },
            )
        } else {
            Ok(String::new())
        }
    }

    /// Format bytes in human-readable format
    fn format_bytes(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_idx = 0;

        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }

        if unit_idx == 0 {
            format!("{} {}", bytes, UNITS[unit_idx])
        } else {
            format!("{:.2} {}", size, UNITS[unit_idx])
        }
    }

    /// Format duration in human-readable format
    fn format_duration(&self, duration: Duration) -> String {
        let secs = duration.as_secs();
        
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Create a spinner character based on frame number
    pub fn spinner(&self, frame: usize) -> char {
        const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        SPINNER_CHARS[frame % SPINNER_CHARS.len()]
    }

    /// Render a status indicator with color coding
    pub fn render_status_indicator(&self, status: &str) -> CLIResult<String> {
        let (symbol, color) = match status.to_lowercase().as_str() {
            "completed" | "success" | "done" => ("✓", Color::Green),
            "failed" | "error" => ("✗", Color::Red),
            "warning" | "pending" => ("⚠", Color::Yellow),
            "running" | "in_progress" => ("●", Color::Cyan),
            "cancelled" => ("○", Color::Gray),
            _ => ("•", Color::White),
        };

        let styled_symbol = self.style_manager.apply_style(
            symbol,
            TextStyle {
                color: Some(color),
                bold: true,
                ..Default::default()
            },
        )?;

        Ok(format!("{} {}", styled_symbol, status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::ColorMode;

    #[test]
    fn test_progress_bar_rendering() {
        let style_manager = StyleManager::new(ColorMode::Never);
        let renderer = ProgressRenderer::new(style_manager);

        let progress = ProgressInfo {
            current: 50,
            total: Some(100),
            rate: Some(10.0),
            eta: Some(Duration::from_secs(5)),
            message: Some("Transferring...".to_string()),
        };

        let display = renderer.render(progress).unwrap();
        assert!(display.bar.contains("50.0%"));
        assert!(display.status.contains("50 B"));
        assert!(display.status.contains("100 B"));
    }

    #[test]
    fn test_indeterminate_progress() {
        let style_manager = StyleManager::new(ColorMode::Never);
        let renderer = ProgressRenderer::new(style_manager);

        let progress = ProgressInfo {
            current: 1024,
            total: None,
            rate: None,
            eta: None,
            message: None,
        };

        let display = renderer.render(progress).unwrap();
        assert!(display.bar.contains("["));
        assert!(display.bar.contains("]"));
    }

    #[test]
    fn test_format_bytes() {
        let style_manager = StyleManager::new(ColorMode::Never);
        let renderer = ProgressRenderer::new(style_manager);

        assert_eq!(renderer.format_bytes(500), "500 B");
        assert_eq!(renderer.format_bytes(1024), "1.00 KB");
        assert_eq!(renderer.format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(renderer.format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        let style_manager = StyleManager::new(ColorMode::Never);
        let renderer = ProgressRenderer::new(style_manager);

        assert_eq!(renderer.format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(renderer.format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(renderer.format_duration(Duration::from_secs(3661)), "1h 1m");
    }

    #[test]
    fn test_status_indicator() {
        let style_manager = StyleManager::new(ColorMode::Never);
        let renderer = ProgressRenderer::new(style_manager);

        let success = renderer.render_status_indicator("completed").unwrap();
        assert!(success.contains("completed"));

        let error = renderer.render_status_indicator("failed").unwrap();
        assert!(error.contains("failed"));
    }
}
