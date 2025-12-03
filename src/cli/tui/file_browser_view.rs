// File browser view for TUI

use crate::cli::tui::widgets::FileEntry;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::fs;
use std::path::PathBuf;

/// File browser view state
#[derive(Debug, Clone)]
pub struct FileBrowserView {
    pub current_path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_index: usize,
    pub selected_files: Vec<PathBuf>,
    pub show_hidden: bool,
}

impl FileBrowserView {
    /// Create a new file browser view
    pub fn new(path: PathBuf) -> Self {
        let mut view = Self {
            current_path: path,
            entries: Vec::new(),
            selected_index: 0,
            selected_files: Vec::new(),
            show_hidden: false,
        };
        view.refresh_entries();
        view
    }

    /// Refresh directory entries
    pub fn refresh_entries(&mut self) {
        self.entries.clear();

        // Add parent directory entry if not at root
        if self.current_path.parent().is_some() {
            self.entries.push(FileEntry {
                name: "..".to_string(),
                path: self.current_path.parent().unwrap().to_path_buf(),
                is_directory: true,
                size: None,
            });
        }

        // Read directory entries
        if let Ok(read_dir) = fs::read_dir(&self.current_path) {
            let mut entries: Vec<FileEntry> = read_dir
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    // Skip hidden files if not showing them
                    if !self.show_hidden && name.starts_with('.') {
                        return None;
                    }

                    let is_directory = path.is_dir();
                    let size = if !is_directory {
                        entry.metadata().ok().map(|m| m.len())
                    } else {
                        None
                    };

                    Some(FileEntry {
                        name,
                        path,
                        is_directory,
                        size,
                    })
                })
                .collect();

            // Sort: directories first, then files, alphabetically
            entries.sort_by(|a, b| {
                match (a.is_directory, b.is_directory) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                }
            });

            self.entries.extend(entries);
        }

        // Reset selection if out of bounds
        if self.selected_index >= self.entries.len() && !self.entries.is_empty() {
            self.selected_index = self.entries.len() - 1;
        }
    }

    /// Render the file browser view
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(5)])
            .split(area);

        // Render file list
        self.render_file_list(frame, chunks[0]);

        // Render selection info
        self.render_selection_info(frame, chunks[1]);
    }

    /// Render file list
    fn render_file_list(&self, frame: &mut Frame, area: Rect) {
        if self.entries.is_empty() {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!("Files: {}", self.current_path.display()));
            let paragraph = Paragraph::new(vec![
                Line::from("Directory is empty or cannot be read."),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Press "),
                    Span::styled("h", Style::default().fg(Color::Yellow)),
                    Span::raw(" to toggle hidden files"),
                ]),
            ])
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
                let icon = if entry.is_directory {
                    "📁"
                } else {
                    "📄"
                };

                let size_str = if let Some(size) = entry.size {
                    format_size(size)
                } else if entry.is_directory && entry.name != ".." {
                    "<DIR>".to_string()
                } else {
                    String::new()
                };

                let is_selected = self.selected_files.contains(&entry.path);
                let checkbox = if is_selected { "[✓]" } else { "[ ]" };

                let name_color = if entry.is_directory {
                    Color::Cyan
                } else if is_selected {
                    Color::Green
                } else {
                    Color::White
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", checkbox),
                        Style::default().fg(if is_selected {
                            Color::Green
                        } else {
                            Color::Gray
                        }),
                    ),
                    Span::raw(format!("{} ", icon)),
                    Span::styled(
                        format!("{:<40}", truncate(&entry.name, 40)),
                        Style::default().fg(name_color),
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

        let title = format!(
            "Files: {} (Press Space to select, Enter to open)",
            self.current_path.display()
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

    /// Render selection info
    fn render_selection_info(&self, frame: &mut Frame, area: Rect) {
        let selected_count = self.selected_files.len();
        let total_size: u64 = self
            .selected_files
            .iter()
            .filter_map(|path| fs::metadata(path).ok())
            .filter(|m| m.is_file())
            .map(|m| m.len())
            .sum();

        let lines = vec![
            Line::from(vec![
                Span::styled("Selected: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{} file(s)", selected_count),
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Total Size: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format_size(total_size),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::styled("[Space]", Style::default().fg(Color::Yellow)),
                Span::raw(" Select  "),
                Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
                Span::raw(" Open  "),
                Span::styled("[h]", Style::default().fg(Color::Yellow)),
                Span::raw(" Toggle Hidden  "),
                Span::styled("[s]", Style::default().fg(Color::Yellow)),
                Span::raw(" Send Selected"),
            ]),
        ];

        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title("Selection"))
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
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

    /// Toggle selection of current entry
    pub fn toggle_selection(&mut self) {
        if let Some(entry) = self.entries.get(self.selected_index) {
            // Don't allow selecting parent directory
            if entry.name == ".." {
                return;
            }

            let path = entry.path.clone();
            if let Some(pos) = self.selected_files.iter().position(|p| p == &path) {
                self.selected_files.remove(pos);
            } else {
                self.selected_files.push(path);
            }
        }
    }

    /// Open selected entry (navigate into directory or select file)
    pub fn open_selected(&mut self) -> Option<FileAction> {
        if let Some(entry) = self.entries.get(self.selected_index).cloned() {
            if entry.is_directory {
                self.current_path = entry.path.clone();
                self.selected_index = 0;
                self.refresh_entries();
                Some(FileAction::NavigateToDirectory(entry.path))
            } else {
                // Toggle selection for files
                let path = entry.path.clone();
                self.toggle_selection();
                Some(FileAction::SelectFile(path))
            }
        } else {
            None
        }
    }

    /// Toggle hidden files visibility
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.refresh_entries();
    }

    /// Get selected entry
    pub fn get_selected(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_index)
    }

    /// Get all selected files
    pub fn get_selected_files(&self) -> &[PathBuf] {
        &self.selected_files
    }

    /// Clear all selections
    pub fn clear_selections(&mut self) {
        self.selected_files.clear();
    }

    /// Navigate to parent directory
    pub fn navigate_up(&mut self) {
        if let Some(parent) = self.current_path.parent() {
            self.current_path = parent.to_path_buf();
            self.selected_index = 0;
            self.refresh_entries();
        }
    }

    /// Navigate to home directory
    pub fn navigate_home(&mut self) {
        if let Some(home) = dirs::home_dir() {
            self.current_path = home;
            self.selected_index = 0;
            self.refresh_entries();
        }
    }
}

/// File action types
#[derive(Debug, Clone)]
pub enum FileAction {
    NavigateToDirectory(PathBuf),
    SelectFile(PathBuf),
    SendFiles(Vec<PathBuf>),
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

/// Truncate string to max length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
