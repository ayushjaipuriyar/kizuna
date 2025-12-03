// Peer management view for TUI

use crate::cli::types::{ConnectionStatus, PeerInfo, TrustStatus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Peer management view state
#[derive(Debug, Clone)]
pub struct PeerView {
    pub peers: Vec<PeerInfo>,
    pub selected_index: usize,
    pub show_details: bool,
}

impl PeerView {
    /// Create a new peer view
    pub fn new(peers: Vec<PeerInfo>) -> Self {
        Self {
            peers,
            selected_index: 0,
            show_details: false,
        }
    }

    /// Render the peer view
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if self.show_details && !self.peers.is_empty() {
            // Split view: list on left, details on right
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(area);

            self.render_peer_list(frame, chunks[0]);
            self.render_peer_details(frame, chunks[1]);
        } else {
            // Full width list
            self.render_peer_list(frame, area);
        }
    }

    /// Render peer list
    fn render_peer_list(&self, frame: &mut Frame, area: Rect) {
        if self.peers.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .title("Peers (0)");
            let paragraph = Paragraph::new(vec![
                Line::from("No peers discovered."),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Press "),
                    Span::styled("d", Style::default().fg(Color::Yellow)),
                    Span::raw(" to discover peers"),
                ]),
            ])
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

                let status_indicator = match peer.connection_status {
                    ConnectionStatus::Connected => "●",
                    ConnectionStatus::Disconnected => "○",
                    ConnectionStatus::Connecting => "◐",
                    ConnectionStatus::Error => "✗",
                };

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
                        format!("{} ", status_indicator),
                        Style::default().fg(status_color),
                    ),
                    Span::styled(
                        format!("{:<18}", truncate(&peer.name, 18)),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:<12}", truncate(&peer.device_type, 12)),
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
            format!("Peers ({}) - Press Enter for details", self.peers.len())
        } else {
            format!("Peers ({}) - Press Enter to view details", self.peers.len())
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

    /// Render peer details
    fn render_peer_details(&self, frame: &mut Frame, area: Rect) {
        if let Some(peer) = self.peers.get(self.selected_index) {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(10),
                    Constraint::Min(5),
                    Constraint::Length(5),
                ])
                .split(area);

            // Basic info
            self.render_basic_info(frame, chunks[0], peer);

            // Capabilities
            self.render_capabilities(frame, chunks[1], peer);

            // Actions
            self.render_actions(frame, chunks[2], peer);
        }
    }

    /// Render basic peer information
    fn render_basic_info(&self, frame: &mut Frame, area: Rect, peer: &PeerInfo) {
        let status_text = format!("{:?}", peer.connection_status);
        let status_color = match peer.connection_status {
            ConnectionStatus::Connected => Color::Green,
            ConnectionStatus::Disconnected => Color::Gray,
            ConnectionStatus::Connecting => Color::Yellow,
            ConnectionStatus::Error => Color::Red,
        };

        let trust_text = format!("{:?}", peer.trust_status);
        let trust_color = match peer.trust_status {
            TrustStatus::Trusted => Color::Green,
            TrustStatus::Untrusted => Color::Yellow,
            TrustStatus::Blocked => Color::Red,
        };

        let last_seen = if let Some(ts) = peer.last_seen {
            ts.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            "Never".to_string()
        };

        let lines = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(Color::Gray)),
                Span::styled(&peer.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Device Type: ", Style::default().fg(Color::Gray)),
                Span::styled(&peer.device_type, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Gray)),
                Span::styled(status_text, Style::default().fg(status_color)),
            ]),
            Line::from(vec![
                Span::styled("Trust: ", Style::default().fg(Color::Gray)),
                Span::styled(trust_text, Style::default().fg(trust_color)),
            ]),
            Line::from(vec![
                Span::styled("Last Seen: ", Style::default().fg(Color::Gray)),
                Span::styled(last_seen, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::Gray)),
                Span::styled(peer.id.to_string(), Style::default().fg(Color::DarkGray)),
            ]),
        ];

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Peer Details"))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    /// Render peer capabilities
    fn render_capabilities(&self, frame: &mut Frame, area: Rect, peer: &PeerInfo) {
        let items: Vec<ListItem> = peer
            .capabilities
            .iter()
            .map(|cap| {
                ListItem::new(Line::from(vec![
                    Span::raw("• "),
                    Span::styled(cap, Style::default().fg(Color::Cyan)),
                ]))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Capabilities ({})", peer.capabilities.len())),
        );

        frame.render_widget(list, area);
    }

    /// Render available actions
    fn render_actions(&self, frame: &mut Frame, area: Rect, peer: &PeerInfo) {
        let actions = match peer.connection_status {
            ConnectionStatus::Connected => vec![
                ("d", "Disconnect", Color::Red),
                ("t", "Toggle Trust", Color::Yellow),
                ("b", "Block", Color::Red),
            ],
            ConnectionStatus::Disconnected => vec![
                ("c", "Connect", Color::Green),
                ("t", "Toggle Trust", Color::Yellow),
                ("b", "Block", Color::Red),
            ],
            ConnectionStatus::Connecting => vec![
                ("x", "Cancel", Color::Yellow),
            ],
            ConnectionStatus::Error => vec![
                ("r", "Retry", Color::Green),
                ("b", "Block", Color::Red),
            ],
        };

        let lines: Vec<Line> = actions
            .iter()
            .map(|(key, action, color)| {
                Line::from(vec![
                    Span::styled(format!("[{}]", key), Style::default().fg(*color).add_modifier(Modifier::BOLD)),
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

    /// Toggle details view
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    /// Get selected peer
    pub fn get_selected(&self) -> Option<&PeerInfo> {
        self.peers.get(self.selected_index)
    }

    /// Update peers list
    pub fn update_peers(&mut self, peers: Vec<PeerInfo>) {
        self.peers = peers;
        if self.selected_index >= self.peers.len() && !self.peers.is_empty() {
            self.selected_index = self.peers.len() - 1;
        }
    }
}

/// Truncate string to max length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Peer action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerAction {
    Connect,
    Disconnect,
    ToggleTrust,
    Block,
    Unblock,
    Retry,
    Cancel,
}

impl PeerAction {
    /// Get action from key code
    pub fn from_char(c: char, status: ConnectionStatus) -> Option<Self> {
        match (c, status) {
            ('c', ConnectionStatus::Disconnected) => Some(PeerAction::Connect),
            ('d', ConnectionStatus::Connected) => Some(PeerAction::Disconnect),
            ('t', _) => Some(PeerAction::ToggleTrust),
            ('b', _) => Some(PeerAction::Block),
            ('r', ConnectionStatus::Error) => Some(PeerAction::Retry),
            ('x', ConnectionStatus::Connecting) => Some(PeerAction::Cancel),
            _ => None,
        }
    }
}
