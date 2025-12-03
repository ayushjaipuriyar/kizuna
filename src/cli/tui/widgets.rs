// TUI Widgets

use crate::cli::types::{
    ConnectionStatus, OperationState, OperationStatus, OperationType, PeerInfo, ProgressInfo,
    TrustStatus,
};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
    Frame,
};
use std::path::PathBuf;

/// Peer list widget
#[derive(Debug, Clone)]
pub struct PeerListWidget {
    pub peers: Vec<PeerInfo>,
    pub selected_index: usize,
}

impl PeerListWidget {
    /// Create a new peer list widget
    pub fn new(peers: Vec<PeerInfo>, selected_index: usize) -> Self {
        Self {
            peers,
            selected_index,
        }
    }

    /// Render the widget
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if self.peers.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Peers (0)");
            let paragraph = Paragraph::new("No peers discovered. Press 'd' to discover.")
                .block(block)
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(paragraph, area);
            return;
        }

        let items: Vec<ListItem> = self
            .peers
            .iter()
            .enumerate()
            .map(|(i, peer)| {
                let status_color = match peer.connection_status {
                    ConnectionStatus::Connected => Color::Green,
                    ConnectionStatus::Disconnected => Color::Gray,
                    ConnectionStatus::Connecting => Color::Yellow,
                    ConnectionStatus::Error => Color::Red,
                };

                let trust_icon = match peer.trust_status {
                    TrustStatus::Trusted => "✓",
                    TrustStatus::Untrusted => "?",
                    TrustStatus::Blocked => "✗",
                };

                let status_text = format!("{:?}", peer.connection_status);
                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", trust_icon),
                        Style::default().fg(match peer.trust_status {
                            TrustStatus::Trusted => Color::Green,
                            TrustStatus::Untrusted => Color::Yellow,
                            TrustStatus::Blocked => Color::Red,
                        }),
                    ),
                    Span::styled(
                        format!("{:<20}", peer.name),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" | "),
                    Span::styled(
                        format!("{:<15}", peer.device_type),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(" | "),
                    Span::styled(status_text, Style::default().fg(status_color)),
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

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Peers ({})", self.peers.len())),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(list, area);
    }

    /// Select next peer
    pub fn select_next(&mut self) {
        if !self.peers.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.peers.len();
        }
    }

    /// Select previous peer
    pub fn select_previous(&mut self) {
        if !self.peers.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.peers.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Get selected peer
    pub fn get_selected(&self) -> Option<&PeerInfo> {
        self.peers.get(self.selected_index)
    }
}

/// File entry in browser
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub size: Option<u64>,
}

/// File browser widget
#[derive(Debug, Clone)]
pub struct FileBrowserWidget {
    pub current_path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
}

impl FileBrowserWidget {
    /// Create a new file browser widget
    pub fn new(current_path: PathBuf, entries: Vec<FileEntry>, selected_index: usize) -> Self {
        Self {
            current_path,
            entries,
            selected_index,
        }
    }

    /// Render the widget
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if self.entries.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!("Files: {}", self.current_path.display()));
            let paragraph = Paragraph::new("No files in directory")
                .block(block)
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(paragraph, area);
            return;
        }

        let items: Vec<ListItem> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let icon = if entry.is_directory { "📁" } else { "📄" };
                let size_str = if let Some(size) = entry.size {
                    format_size(size)
                } else {
                    String::new()
                };

                let line = Line::from(vec![
                    Span::raw(format!("{} ", icon)),
                    Span::styled(
                        format!("{:<40}", entry.name),
                        Style::default().fg(if entry.is_directory {
                            Color::Cyan
                        } else {
                            Color::White
                        }),
                    ),
                    Span::styled(
                        format!("{:>10}", size_str),
                        Style::default().fg(Color::Gray),
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

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Files: {}", self.current_path.display())),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(list, area);
    }

    /// Select next entry
    pub fn select_next(&mut self) {
        if !self.entries.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.entries.len();
        }
    }

    /// Select previous entry
    pub fn select_previous(&mut self) {
        if !self.entries.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.entries.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Get selected entry
    pub fn get_selected(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_index)
    }
}

/// Progress widget
#[derive(Debug, Clone)]
pub struct ProgressWidget {
    pub operations: Vec<OperationStatus>,
}

impl ProgressWidget {
    /// Create a new progress widget
    pub fn new(operations: Vec<OperationStatus>) -> Self {
        Self { operations }
    }

    /// Render the widget
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if self.operations.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Active Operations (0)");
            let paragraph = Paragraph::new("No active operations")
                .block(block)
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(paragraph, area);
            return;
        }

        let rows: Vec<Row> = self
            .operations
            .iter()
            .map(|op| {
                let op_type = format!("{:?}", op.operation_type);
                let status = match &op.status {
                    OperationState::Starting => "Starting",
                    OperationState::InProgress => "In Progress",
                    OperationState::Completed => "Completed",
                    OperationState::Failed(_) => "Failed",
                    OperationState::Cancelled => "Cancelled",
                };

                let progress_str = if let Some(ref progress) = op.progress {
                    if let Some(total) = progress.total {
                        let percentage = (progress.current as f64 / total as f64 * 100.0) as u64;
                        format!("{}%", percentage)
                    } else {
                        format!("{}", format_size(progress.current))
                    }
                } else {
                    "-".to_string()
                };

                let rate_str = if let Some(ref progress) = op.progress {
                    if let Some(rate) = progress.rate {
                        format!("{}/s", format_size(rate as u64))
                    } else {
                        "-".to_string()
                    }
                } else {
                    "-".to_string()
                };

                let status_color = match op.status {
                    OperationState::Starting => Color::Yellow,
                    OperationState::InProgress => Color::Cyan,
                    OperationState::Completed => Color::Green,
                    OperationState::Failed(_) => Color::Red,
                    OperationState::Cancelled => Color::Gray,
                };

                Row::new(vec![
                    op_type,
                    format!("{}", op.peer_id),
                    status.to_string(),
                    progress_str,
                    rate_str,
                ])
                .style(Style::default().fg(status_color))
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(15),
                Constraint::Length(36),
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Length(12),
            ],
        )
        .header(
            Row::new(vec!["Type", "Peer ID", "Status", "Progress", "Rate"])
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Active Operations ({})", self.operations.len())),
        );

        frame.render_widget(table, area);
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
