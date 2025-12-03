// Transfer queue and progress view for TUI

use crate::cli::types::{OperationState, OperationStatus, OperationType, ProgressInfo};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Row, Table, Wrap},
    Frame,
};

/// Transfer queue view state
#[derive(Debug, Clone)]
pub struct TransferView {
    pub operations: Vec<OperationStatus>,
    pub selected_index: usize,
    pub show_details: bool,
}

impl TransferView {
    /// Create a new transfer view
    pub fn new(operations: Vec<OperationStatus>) -> Self {
        Self {
            operations,
            selected_index: 0,
            show_details: false,
        }
    }

    /// Render the transfer view
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if self.show_details && !self.operations.is_empty() {
            // Split view: list on left, details on right
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);

            self.render_operation_list(frame, chunks[0]);
            self.render_operation_details(frame, chunks[1]);
        } else {
            // Full width list
            self.render_operation_list(frame, area);
        }
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
                    Span::styled("2", Style::default().fg(Color::Yellow)),
                    Span::raw(" to browse files and start transfers"),
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
                let op_type_str = match op.operation_type {
                    OperationType::FileTransfer => "📁 Transfer",
                    OperationType::CameraStream => "📹 Stream",
                    OperationType::CommandExecution => "⚙️  Command",
                    OperationType::ClipboardSync => "📋 Clipboard",
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

                let progress_str = if let Some(ref progress) = op.progress {
                    if let Some(total) = progress.total {
                        let percentage = (progress.current as f64 / total as f64 * 100.0) as u64;
                        format!("{}%", percentage)
                    } else {
                        format_size(progress.current)
                    }
                } else {
                    "-".to_string()
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{:<15}", op_type_str),
                        Style::default().fg(Color::White),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:<12}", status_str),
                        Style::default().fg(status_color),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:>8}", progress_str),
                        Style::default().fg(Color::Cyan),
                    ),
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

        let title = if self.show_details {
            format!(
                "Active Operations ({}) - Press Enter for details",
                self.operations.len()
            )
        } else {
            format!(
                "Active Operations ({}) - Press Enter to view details",
                self.operations.len()
            )
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(list, area);
    }

    /// Render operation details
    fn render_operation_details(&self, frame: &mut Frame, area: Rect) {
        if let Some(op) = self.operations.get(self.selected_index) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8),
                    Constraint::Length(4),
                    Constraint::Min(5),
                    Constraint::Length(4),
                ])
                .split(area);

            // Basic info
            self.render_operation_info(frame, chunks[0], op);

            // Progress bar
            if let Some(ref progress) = op.progress {
                self.render_progress_bar(frame, chunks[1], progress);
            }

            // Status message
            self.render_status_message(frame, chunks[2], op);

            // Actions
            self.render_operation_actions(frame, chunks[3], op);
        }
    }

    /// Render operation basic info
    fn render_operation_info(&self, frame: &mut Frame, area: Rect, op: &OperationStatus) {
        let op_type_str = format!("{:?}", op.operation_type);
        let status_str = match &op.status {
            OperationState::Starting => "Starting".to_string(),
            OperationState::InProgress => "In Progress".to_string(),
            OperationState::Completed => "Completed".to_string(),
            OperationState::Failed(err) => format!("Failed: {}", err),
            OperationState::Cancelled => "Cancelled".to_string(),
        };

        let status_color = match op.status {
            OperationState::Starting => Color::Yellow,
            OperationState::InProgress => Color::Cyan,
            OperationState::Completed => Color::Green,
            OperationState::Failed(_) => Color::Red,
            OperationState::Cancelled => Color::Gray,
        };

        let elapsed = chrono::Utc::now()
            .signed_duration_since(op.started_at)
            .to_std()
            .unwrap_or_default();

        let lines = vec![
            Line::from(vec![
                Span::styled("Type: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    op_type_str,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Gray)),
                Span::styled(status_str, Style::default().fg(status_color)),
            ]),
            Line::from(vec![
                Span::styled("Peer ID: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    op.peer_id.to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(vec![
                Span::styled("Started: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    op.started_at.format("%H:%M:%S").to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("Elapsed: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format_duration(elapsed),
                    Style::default().fg(Color::White),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Operation Details"),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render progress bar
    fn render_progress_bar(&self, frame: &mut Frame, area: Rect, progress: &ProgressInfo) {
        let percentage = if let Some(total) = progress.total {
            ((progress.current as f64 / total as f64) * 100.0) as u16
        } else {
            0
        };

        let label = if let Some(total) = progress.total {
            format!(
                "{} / {} ({}%)",
                format_size(progress.current),
                format_size(total),
                percentage
            )
        } else {
            format_size(progress.current)
        };

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .percent(percentage)
            .label(label);

        frame.render_widget(gauge, area);
    }

    /// Render status message
    fn render_status_message(&self, frame: &mut Frame, area: Rect, op: &OperationStatus) {
        let mut lines = Vec::new();

        if let Some(ref progress) = op.progress {
            if let Some(rate) = progress.rate {
                lines.push(Line::from(vec![
                    Span::styled("Speed: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{}/s", format_size(rate as u64)),
                        Style::default().fg(Color::Cyan),
                    ),
                ]));
            }

            if let Some(eta) = progress.eta {
                lines.push(Line::from(vec![
                    Span::styled("ETA: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format_duration(eta),
                        Style::default().fg(Color::Yellow),
                    ),
                ]));
            }

            if let Some(ref message) = progress.message {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Message: ", Style::default().fg(Color::Gray)),
                    Span::styled(message.clone(), Style::default().fg(Color::White)),
                ]));
            }
        }

        if lines.is_empty() {
            lines.push(Line::from(
                Span::styled("No additional information", Style::default().fg(Color::Gray)),
            ));
        }

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render operation actions
    fn render_operation_actions(&self, frame: &mut Frame, area: Rect, op: &OperationStatus) {
        let actions = match op.status {
            OperationState::Starting | OperationState::InProgress => {
                vec![("x", "Cancel", Color::Red)]
            }
            OperationState::Failed(_) => vec![("r", "Retry", Color::Green)],
            OperationState::Completed | OperationState::Cancelled => {
                vec![("d", "Dismiss", Color::Gray)]
            }
        };

        let lines: Vec<Line> = actions
            .iter()
            .map(|(key, action, color)| {
                Line::from(vec![
                    Span::styled(
                        format!("[{}]", key),
                        Style::default()
                            .fg(*color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(*action, Style::default().fg(Color::White)),
                ])
            })
            .collect();

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Actions"))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
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

    /// Toggle details view
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    /// Get selected operation
    pub fn get_selected(&self) -> Option<&OperationStatus> {
        self.operations.get(self.selected_index)
    }

    /// Update operations list
    pub fn update_operations(&mut self, operations: Vec<OperationStatus>) {
        self.operations = operations;
        if self.selected_index >= self.operations.len() && !self.operations.is_empty() {
            self.selected_index = self.operations.len() - 1;
        }
    }
}

/// Transfer action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferAction {
    Cancel,
    Retry,
    Dismiss,
}

impl TransferAction {
    /// Get action from key code
    pub fn from_char(c: char, status: &OperationState) -> Option<Self> {
        match (c, status) {
            ('x', OperationState::Starting) | ('x', OperationState::InProgress) => {
                Some(TransferAction::Cancel)
            }
            ('r', OperationState::Failed(_)) => Some(TransferAction::Retry),
            ('d', OperationState::Completed) | ('d', OperationState::Cancelled) => {
                Some(TransferAction::Dismiss)
            }
            _ => None,
        }
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

/// Format duration for display
fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
