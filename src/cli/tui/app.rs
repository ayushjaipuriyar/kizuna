// TUI Application and Manager

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::types::{PeerInfo, OperationStatus, TUIState, ViewType, PeerId};
use crate::cli::tui::events::{EventHandler, EventLoop};
use crate::cli::tui::widgets::{PeerListWidget, FileBrowserWidget, ProgressWidget};
use crate::cli::tui::peer_view::PeerView;
use crate::cli::tui::file_browser_view::FileBrowserView;
use crate::cli::tui::transfer_view::TransferView;
use crate::cli::tui::operation_monitor::OperationMonitor;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// TUI Application
pub struct TUIApp {
    pub state: TUIState,
    pub running: bool,
    event_handler: EventHandler,
    peer_view: PeerView,
    file_browser_view: FileBrowserView,
    transfer_view: TransferView,
    operation_monitor: OperationMonitor,
}

impl TUIApp {
    /// Create a new TUI application
    pub fn new() -> Self {
        let initial_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            state: TUIState {
                current_view: ViewType::PeerList,
                selected_peer: None,
                file_browser_path: initial_path.clone(),
                active_operations: Vec::new(),
                peer_list: Vec::new(),
                navigation_stack: Vec::new(),
            },
            running: true,
            event_handler: EventHandler::new(),
            peer_view: PeerView::new(Vec::new()),
            file_browser_view: FileBrowserView::new(initial_path),
            transfer_view: TransferView::new(Vec::new()),
            operation_monitor: OperationMonitor::new(),
        }
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> CLIResult<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.running = false;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
            }
            KeyCode::Tab => {
                self.next_view();
            }
            KeyCode::BackTab => {
                self.previous_view();
            }
            KeyCode::Up => {
                self.handle_up();
            }
            KeyCode::Down => {
                self.handle_down();
            }
            KeyCode::Enter => {
                self.handle_enter()?;
            }
            KeyCode::Char('1') => {
                self.state.current_view = ViewType::PeerList;
            }
            KeyCode::Char('2') => {
                self.state.current_view = ViewType::FileBrowser;
            }
            KeyCode::Char('3') => {
                self.state.current_view = ViewType::TransferProgress;
            }
            KeyCode::Char('l') => {
                // Toggle logs in operation monitor or transfer view
                if self.state.current_view == ViewType::TransferProgress {
                    self.operation_monitor.toggle_logs();
                }
            }
            KeyCode::Char(' ') => {
                // Handle space key for file selection
                if self.state.current_view == ViewType::FileBrowser {
                    self.file_browser_view.toggle_selection();
                }
            }
            KeyCode::Char('h') => {
                // Toggle hidden files in file browser
                if self.state.current_view == ViewType::FileBrowser {
                    self.file_browser_view.toggle_hidden();
                }
            }
            KeyCode::Char('s') => {
                // Send selected files
                if self.state.current_view == ViewType::FileBrowser {
                    self.handle_send_files()?;
                }
            }
            KeyCode::Char(c) => {
                // Handle view-specific actions
                match self.state.current_view {
                    ViewType::PeerList => {
                        self.handle_peer_action(c)?;
                    }
                    ViewType::TransferProgress => {
                        self.handle_operation_control(c)?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle peer-specific actions
    fn handle_peer_action(&mut self, key: char) -> CLIResult<()> {
        use crate::cli::tui::peer_view::PeerAction;

        if let Some(peer) = self.peer_view.get_selected() {
            if let Some(action) = PeerAction::from_char(key, peer.connection_status) {
                // Store the action for processing
                // In a real implementation, this would trigger actual peer operations
                match action {
                    PeerAction::Connect => {
                        // TODO: Trigger connection to peer
                    }
                    PeerAction::Disconnect => {
                        // TODO: Trigger disconnection from peer
                    }
                    PeerAction::ToggleTrust => {
                        // TODO: Toggle trust status
                    }
                    PeerAction::Block => {
                        // TODO: Block peer
                    }
                    PeerAction::Unblock => {
                        // TODO: Unblock peer
                    }
                    PeerAction::Retry => {
                        // TODO: Retry connection
                    }
                    PeerAction::Cancel => {
                        // TODO: Cancel connection attempt
                    }
                }
            }
        }
        Ok(())
    }

    /// Handle file sending
    fn handle_send_files(&mut self) -> CLIResult<()> {
        let selected_files = self.file_browser_view.get_selected_files();
        if !selected_files.is_empty() {
            // TODO: Trigger file transfer to selected peer
            // This would create new OperationStatus entries
            self.file_browser_view.clear_selections();
        }
        Ok(())
    }

    /// Handle operation control actions
    fn handle_operation_control(&mut self, key: char) -> CLIResult<()> {
        use crate::cli::tui::operation_monitor::OperationControl;

        if let Some(control) = OperationControl::from_char(key) {
            match control {
                OperationControl::Pause => {
                    // TODO: Pause selected operation
                }
                OperationControl::Resume => {
                    // TODO: Resume paused operation
                }
                OperationControl::Cancel => {
                    // TODO: Cancel operation
                }
                OperationControl::Retry => {
                    // TODO: Retry failed operation
                }
                OperationControl::ClearCompleted => {
                    self.operation_monitor.clear_completed();
                }
            }
        }
        Ok(())
    }

    /// Navigate to next view
    fn next_view(&mut self) {
        self.state.current_view = match self.state.current_view {
            ViewType::PeerList => ViewType::FileBrowser,
            ViewType::FileBrowser => ViewType::TransferProgress,
            ViewType::TransferProgress => ViewType::PeerList,
            ViewType::StreamViewer => ViewType::CommandTerminal,
            ViewType::CommandTerminal => ViewType::Settings,
            ViewType::Settings => ViewType::PeerList,
        };
    }

    /// Navigate to previous view
    fn previous_view(&mut self) {
        self.state.current_view = match self.state.current_view {
            ViewType::PeerList => ViewType::TransferProgress,
            ViewType::FileBrowser => ViewType::PeerList,
            ViewType::TransferProgress => ViewType::FileBrowser,
            ViewType::StreamViewer => ViewType::Settings,
            ViewType::CommandTerminal => ViewType::StreamViewer,
            ViewType::Settings => ViewType::CommandTerminal,
        };
    }

    /// Handle up arrow key
    fn handle_up(&mut self) {
        // Implementation depends on current view
        match self.state.current_view {
            ViewType::PeerList => {
                self.peer_view.select_previous();
            }
            ViewType::FileBrowser => {
                self.file_browser_view.select_previous();
            }
            ViewType::TransferProgress => {
                if !self.operation_monitor.show_logs {
                    self.operation_monitor.select_previous();
                }
            }
            _ => {}
        }
    }

    /// Handle down arrow key
    fn handle_down(&mut self) {
        // Implementation depends on current view
        match self.state.current_view {
            ViewType::PeerList => {
                self.peer_view.select_next();
            }
            ViewType::FileBrowser => {
                self.file_browser_view.select_next();
            }
            ViewType::TransferProgress => {
                if !self.operation_monitor.show_logs {
                    self.operation_monitor.select_next();
                }
            }
            _ => {}
        }
    }

    /// Handle enter key
    fn handle_enter(&mut self) -> CLIResult<()> {
        match self.state.current_view {
            ViewType::PeerList => {
                self.peer_view.toggle_details();
            }
            ViewType::FileBrowser => {
                self.file_browser_view.open_selected();
            }
            ViewType::TransferProgress => {
                self.transfer_view.toggle_details();
            }
            _ => {}
        }
        Ok(())
    }

    /// Render the TUI
    pub fn render(&self, frame: &mut Frame) {
        let area = frame.size();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        // Render header
        self.render_header(frame, chunks[0]);

        // Render main content based on current view
        match self.state.current_view {
            ViewType::PeerList => {
                self.render_peer_list(frame, chunks[1]);
            }
            ViewType::FileBrowser => {
                self.render_file_browser(frame, chunks[1]);
            }
            ViewType::TransferProgress => {
                self.render_progress(frame, chunks[1]);
            }
            ViewType::StreamViewer => {
                self.render_stream_viewer(frame, chunks[1]);
            }
            ViewType::CommandTerminal => {
                self.render_command_terminal(frame, chunks[1]);
            }
            ViewType::Settings => {
                self.render_settings(frame, chunks[1]);
            }
        }

        // Render footer
        self.render_footer(frame, chunks[2]);
    }

    /// Render header with tabs
    fn render_header(&self, frame: &mut Frame, area: Rect) {
        let titles = vec!["Peers (1)", "Files (2)", "Transfers (3)"];
        let index = match self.state.current_view {
            ViewType::PeerList => 0,
            ViewType::FileBrowser => 1,
            ViewType::TransferProgress => 2,
            _ => 0,
        };

        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Kizuna TUI"))
            .select(index)
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(tabs, area);
    }

    /// Render peer list view
    fn render_peer_list(&self, frame: &mut Frame, area: Rect) {
        self.peer_view.render(frame, area);
    }

    /// Render file browser view
    fn render_file_browser(&self, frame: &mut Frame, area: Rect) {
        self.file_browser_view.render(frame, area);
    }

    /// Render progress view
    fn render_progress(&self, frame: &mut Frame, area: Rect) {
        // Use operation monitor for detailed monitoring
        self.operation_monitor.render(frame, area);
    }

    /// Render stream viewer
    fn render_stream_viewer(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Stream Viewer");
        let paragraph = Paragraph::new("Stream viewer not yet implemented")
            .block(block)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(paragraph, area);
    }

    /// Render command terminal
    fn render_command_terminal(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Command Terminal");
        let paragraph = Paragraph::new("Command terminal not yet implemented")
            .block(block)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(paragraph, area);
    }

    /// Render settings
    fn render_settings(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Settings");
        let paragraph = Paragraph::new("Settings not yet implemented")
            .block(block)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(paragraph, area);
    }

    /// Render footer with help text
    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let help_text = vec![
            Span::raw("Press "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" to quit, "),
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" to switch views, "),
            Span::styled("1-3", Style::default().fg(Color::Yellow)),
            Span::raw(" for quick navigation"),
        ];

        let paragraph = Paragraph::new(Line::from(help_text))
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));

        frame.render_widget(paragraph, area);
    }

    /// Update peer list
    pub fn update_peers(&mut self, peers: Vec<PeerInfo>) {
        self.state.peer_list = peers.clone();
        self.peer_view.update_peers(peers);
    }

    /// Update operations
    pub fn update_operations(&mut self, operations: Vec<OperationStatus>) {
        self.state.active_operations = operations.clone();
        self.transfer_view.update_operations(operations.clone());
        self.operation_monitor.update_operations(operations);
    }

    /// Get file browser view
    pub fn file_browser_view(&self) -> &FileBrowserView {
        &self.file_browser_view
    }

    /// Get mutable file browser view
    pub fn file_browser_view_mut(&mut self) -> &mut FileBrowserView {
        &mut self.file_browser_view
    }

    /// Get transfer view
    pub fn transfer_view(&self) -> &TransferView {
        &self.transfer_view
    }

    /// Get mutable transfer view
    pub fn transfer_view_mut(&mut self) -> &mut TransferView {
        &mut self.transfer_view
    }

    /// Get operation monitor
    pub fn operation_monitor(&self) -> &OperationMonitor {
        &self.operation_monitor
    }

    /// Get mutable operation monitor
    pub fn operation_monitor_mut(&mut self) -> &mut OperationMonitor {
        &mut self.operation_monitor
    }

    /// Add log to operation monitor
    pub fn add_log(&mut self, level: crate::cli::tui::operation_monitor::LogLevel, operation_id: uuid::Uuid, message: String) {
        self.operation_monitor.add_log(level, operation_id, message);
    }
}

impl Default for TUIApp {
    fn default() -> Self {
        Self::new()
    }
}

/// TUI Manager handles terminal setup and event loop
pub struct TUIManager {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app: TUIApp,
}

impl TUIManager {
    /// Create a new TUI manager
    pub fn new() -> CLIResult<Self> {
        // Setup terminal
        enable_raw_mode().map_err(|e| CLIError::TUIError(e.to_string()))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| CLIError::TUIError(e.to_string()))?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)
            .map_err(|e| CLIError::TUIError(e.to_string()))?;

        Ok(Self {
            terminal,
            app: TUIApp::new(),
        })
    }

    /// Run the TUI application
    pub async fn run(&mut self) -> CLIResult<()> {
        let (tx, mut rx) = mpsc::channel(100);
        let event_loop = EventLoop::new(tx);

        // Spawn event loop
        let event_handle = tokio::spawn(async move {
            event_loop.run().await
        });

        // Main render loop
        while self.app.running {
            // Render
            self.terminal
                .draw(|f| self.app.render(f))
                .map_err(|e| CLIError::TUIError(e.to_string()))?;

            // Handle events
            if let Ok(event) = rx.try_recv() {
                match event {
                    crossterm::event::Event::Key(key) => {
                        self.app.handle_key(key)?;
                    }
                    crossterm::event::Event::Resize(_, _) => {
                        // Terminal will auto-resize
                    }
                    _ => {}
                }
            }

            // Small delay to prevent busy loop
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Cleanup
        event_handle.abort();
        self.cleanup()?;

        Ok(())
    }

    /// Cleanup terminal state
    fn cleanup(&mut self) -> CLIResult<()> {
        disable_raw_mode().map_err(|e| CLIError::TUIError(e.to_string()))?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .map_err(|e| CLIError::TUIError(e.to_string()))?;
        self.terminal
            .show_cursor()
            .map_err(|e| CLIError::TUIError(e.to_string()))?;
        Ok(())
    }

    /// Get mutable reference to app
    pub fn app_mut(&mut self) -> &mut TUIApp {
        &mut self.app
    }
}

impl Drop for TUIManager {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
