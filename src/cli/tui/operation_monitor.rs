// Real-time operation monitoring dashboard for TUI

use crate::cli::types::{OperationState, OperationStatus, OperationType, ProgressInfo};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Sparkline, Wrap},
    Frame,
};
use std::collections::VecDeque;

/// Operation monitoring dashboard state
#[derive(Debug, Clone)]
pub struct OperationMonitor {
    pub operations: Vec<OperationStatus>,
    pub selected_index: usize,
    pub show_logs: bool,
    pub logs: VecDeque<LogEntry>,
    pub statistics: OperationStatistics,
    pub bandwidth_history: VecDeque<u64>,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub operation_id: uuid::Uuid,
    pub message: String,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Debug,
}

/// Operation statistics
#[derive(Debug, Clone, Default)]
pub struct OperationStatistics {
    pub total_operations: usize,
    pub active_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub total_bytes_transferred: u64,
    pub current_bandwidth: u64,
    pub average_bandwidth: u64,
}

impl OperationMonitor {
    /// Create a new operation monitor
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            selected_index: 0,
            show_logs: false,
            logs: VecDeque::with_capacity(1000),
            statistics: OperationStatistics::default(),
            bandwidth_history: VecDeque::with_capacity(60),
        }
    }

    /// Render the operation monitor
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),
                Constraint::Min(10),
                Constraint::Length(8),
            ])
            .split(area);

        // Render statistics
        self.render_statistics(frame, chunks[0]);

        // Render operation list or logs
        if self.show_logs {
            self.render_logs(frame, chunks[1]);
        } else {
            self.render_operation_list(frame, chunks[1]);
        }

        // Render bandwidth graph
        self.render_bandwidth_graph(frame, chunks[2]);
    }

    /// Render statistics panel
    fn render_statistics(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(area);

        // Total operations
        self.render_stat_box(
            frame,
            chunks[0],
            "Total",
            &self.statistics.total_operations.to_string(),
            Color::White,
        );

        // Active operations
        self.render_stat_box(
            frame,
            chunks[1],
            "Active",
            &self.statistics.active_operations.to_string(),
            Color::Cyan,
        );

        // Completed operations
        self.render_stat_box(
            frame,
            chunks[2],
            "Completed",
            &self.statistics.completed_operations.to_string(),
            Color::Green,
        );

        // Failed operations
        self.render_stat_box(
            frame,
            chunks[3],
            "Failed",
            &self.statistics.failed_operations.to_string(),
            Color::Red,
        );
    }

    /// Render a single stat box
    fn render_stat_box(&self, frame: &mut Frame, area: Rect, title: &str, value: &str, color: Color) {
        let lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                value,
                Style::default()
                    .fg(color)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title))
            .alignment(ratatui::layout::Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// Render operation list
    fn render_operation_list(&self, frame: &mut Frame, area: Rect) {
        if self.operations.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Active Operations (0)");
            let paragraph = Paragraph::new(vec![
                Line::from("No active operations."),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Press "),
                    Span::styled("l", Style::default().fg(Color::Yellow)),
                    Span::raw(" to view logs"),
                ]),
            ])
            .block(block)
            .style(Style::default().fg(Color::Gray));
            frame.render_widget(paragraph, area);
            return;
        }

        let items: Vec<ListItem> = self
            .operations
            .iter()
            .enumerate()
            .map(|(i, op)| {
                let op_type_icon = match op.operation_type {
                    OperationType::FileTransfer => "📁",
                    OperationType::CameraStream => "📹",
                    OperationType::CommandExecution => "⚙️",
                    OperationType::ClipboardSync => "📋",
                };

                let status_str = match &op.status {
                    OperationState::Starting => "Starting",
                    OperationState::InProgress => "In Progress",
                    OperationState::Completed => "Completed",
                    OperationState::Failed(_) => "Failed",
                    OperationState::Cancelled => "Cancelled",
                };

                let status_color = match op.status {
                    OperationState::Starting => Color::Yellow,
                    OperationState::InProgress => Color::Cyan,
                    OperationState::Completed => Color::Green,
                    OperationState::Failed(_) => Color::Red,
                    OperationState::Cancelled => Color::Gray,
                };

                let progress_bar = if let Some(ref progress) = op.progress {
                    if let Some(total) = progress.total {
                        let percentage = (progress.current as f64 / total as f64 * 100.0) as usize;
                        let filled = percentage / 5;
                        let empty = 20 - filled;
                        format!("[{}{}] {}%", "█".repeat(filled), "░".repeat(empty), percentage)
                    } else {
                        format!("{}", format_size(progress.current))
                    }
                } else {
                    "".to_string()
                };

                let rate_str = if let Some(ref progress) = op.progress {
                    if let Some(rate) = progress.rate {
                        format!("{}/s", format_size(rate as u64))
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                };

                let line = Line::from(vec![
                    Span::raw(format!("{} ", op_type_icon)),
                    Span::styled(
                        format!("{:<12}", status_str),
                        Style::default().fg(status_color),
                    ),
                    Span::raw(" "),
                    Span::styled(progress_bar, Style::default().fg(Color::Cyan)),
                    Span::raw(" "),
                    Span::styled(rate_str, Style::default().fg(Color::Yellow)),
                ]);

                let style = if i == self.selected_index {
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let title = format!(
            "Active Operations ({}) - Press l for logs, Enter for details",
            self.operations.len()
        );

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(list, area);
    }

    /// Render logs panel
    fn render_logs(&self, frame: &mut Frame, area: Rect) {
        if self.logs.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Operation Logs (0)");
            let paragraph = Paragraph::new(vec![
                Line::from("No logs available."),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Press "),
                    Span::styled("l", Style::default().fg(Color::Yellow)),
                    Span::raw(" to return to operations list"),
                ]),
            ])
            .block(block)
            .style(Style::default().fg(Color::Gray));
            frame.render_widget(paragraph, area);
            return;
        }

        let items: Vec<ListItem> = self
            .logs
            .iter()
            .rev()
            .take(area.height as usize - 2)
            .map(|log| {
                let level_str = match log.level {
                    LogLevel::Info => "INFO",
                    LogLevel::Warning => "WARN",
                    LogLevel::Error => "ERROR",
                    LogLevel::Debug => "DEBUG",
                };

                let level_color = match log.level {
                    LogLevel::Info => Color::Cyan,
                    LogLevel::Warning => Color::Yellow,
                    LogLevel::Error => Color::Red,
                    LogLevel::Debug => Color::Gray,
                };

                let timestamp = log.timestamp.format("%H:%M:%S").to_string();

                let line = Line::from(vec![
                    Span::styled(timestamp, Style::default().fg(Color::DarkGray)),
                    Span::raw(" "),
                    Span::styled(
                        format!("[{:<5}]", level_str),
                        Style::default().fg(level_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(&log.message, Style::default().fg(Color::White)),
                ]);

                ListItem::new(line)
            })
            .collect();

        let title = format!("Operation Logs ({}) - Press l to return", self.logs.len());

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));

        frame.render_widget(list, area);
    }

    /// Render bandwidth graph
    fn render_bandwidth_graph(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // Bandwidth stats
        let lines = vec![Line::from(vec![
            Span::styled("Current: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}/s", format_size(self.statistics.current_bandwidth)),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Average: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{}/s", format_size(self.statistics.average_bandwidth)),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled("Total: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format_size(self.statistics.total_bytes_transferred),
                Style::default().fg(Color::Green),
            ),
        ])];

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Bandwidth"));

        frame.render_widget(paragraph, chunks[0]);

        // Bandwidth graph
        if !self.bandwidth_history.is_empty() {
            let data: Vec<u64> = self.bandwidth_history.iter().copied().collect();
            let max_bandwidth = data.iter().max().copied().unwrap_or(1);

            let sparkline = Sparkline::default()
                .block(Block::default().borders(Borders::ALL).title("History (60s)"))
                .data(&data)
                .max(max_bandwidth)
                .style(Style::default().fg(Color::Cyan));

            frame.render_widget(sparkline, chunks[1]);
        }
    }

    /// Update operations
    pub fn update_operations(&mut self, operations: Vec<OperationStatus>) {
        self.operations = operations;
        self.update_statistics();

        if self.selected_index >= self.operations.len() && !self.operations.is_empty() {
            self.selected_index = self.operations.len() - 1;
        }
    }

    /// Update statistics
    fn update_statistics(&mut self) {
        self.statistics.total_operations = self.operations.len();
        self.statistics.active_operations = self
            .operations
            .iter()
            .filter(|op| matches!(op.status, OperationState::InProgress | OperationState::Starting))
            .count();
        self.statistics.completed_operations = self
            .operations
            .iter()
            .filter(|op| matches!(op.status, OperationState::Completed))
            .count();
        self.statistics.failed_operations = self
            .operations
            .iter()
            .filter(|op| matches!(op.status, OperationState::Failed(_)))
            .count();

        // Calculate current bandwidth
        let current_bandwidth: u64 = self
            .operations
            .iter()
            .filter_map(|op| op.progress.as_ref())
            .filter_map(|p| p.rate)
            .map(|r| r as u64)
            .sum();

        self.statistics.current_bandwidth = current_bandwidth;

        // Update bandwidth history
        self.bandwidth_history.push_back(current_bandwidth);
        if self.bandwidth_history.len() > 60 {
            self.bandwidth_history.pop_front();
        }

        // Calculate average bandwidth
        if !self.bandwidth_history.is_empty() {
            let sum: u64 = self.bandwidth_history.iter().sum();
            self.statistics.average_bandwidth = sum / self.bandwidth_history.len() as u64;
        }

        // Calculate total bytes transferred
        self.statistics.total_bytes_transferred = self
            .operations
            .iter()
            .filter_map(|op| op.progress.as_ref())
            .map(|p| p.current)
            .sum();
    }

    /// Add log entry
    pub fn add_log(&mut self, level: LogLevel, operation_id: uuid::Uuid, message: String) {
        let entry = LogEntry {
            timestamp: chrono::Utc::now(),
            level,
            operation_id,
            message,
        };

        self.logs.push_back(entry);

        // Keep only last 1000 logs
        if self.logs.len() > 1000 {
            self.logs.pop_front();
        }
    }

    /// Toggle logs view
    pub fn toggle_logs(&mut self) {
        self.show_logs = !self.show_logs;
    }

    /// Select next operation
    pub fn select_next(&mut self) {
        if !self.operations.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.operations.len();
        }
    }

    /// Select previous operation
    pub fn select_previous(&mut self) {
        if !self.operations.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.operations.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Get selected operation
    pub fn get_selected(&self) -> Option<&OperationStatus> {
        self.operations.get(self.selected_index)
    }

    /// Clear completed operations
    pub fn clear_completed(&mut self) {
        self.operations.retain(|op| {
            !matches!(
                op.status,
                OperationState::Completed | OperationState::Cancelled
            )
        });
        self.update_statistics();
    }
}

impl Default for OperationMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Format byte size for display
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Operation control actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationControl {
    Pause,
    Resume,
    Cancel,
    Retry,
    ClearCompleted,
}

impl OperationControl {
    /// Get control action from key code
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'p' => Some(OperationControl::Pause),
            'r' => Some(OperationControl::Resume),
            'x' => Some(OperationControl::Cancel),
            'c' => Some(OperationControl::ClearCompleted),
            _ => None,
        }
    }
}
