// Text User Interface module

mod app;
mod events;
mod widgets;
mod peer_view;
mod file_browser_view;
mod transfer_view;
mod operation_monitor;

pub use app::{TUIApp, TUIManager};
pub use events::{EventHandler, EventLoop};
pub use widgets::{PeerListWidget, FileBrowserWidget, ProgressWidget, FileEntry};
pub use peer_view::{PeerView, PeerAction};
pub use file_browser_view::{FileBrowserView, FileAction};
pub use transfer_view::{TransferView, TransferAction};
pub use operation_monitor::{OperationMonitor, OperationControl, LogLevel, LogEntry};

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::types::{PeerInfo, OperationStatus, TUIState, ViewType};
use async_trait::async_trait;
use std::path::PathBuf;

/// TUI interface trait
#[async_trait]
pub trait TUIInterface {
    /// Start the TUI application
    async fn start_tui(&self) -> CLIResult<TUISession>;

    /// Render peer list widget
    async fn render_peer_list(&self, peers: Vec<PeerInfo>) -> CLIResult<PeerListWidget>;

    /// Render file browser widget
    async fn render_file_browser(&self, path: PathBuf) -> CLIResult<FileBrowserWidget>;

    /// Render progress view widget
    async fn render_progress_view(&self, operations: Vec<OperationStatus>) -> CLIResult<ProgressWidget>;

    /// Handle input event
    async fn handle_input_event(&self, event: InputEvent) -> CLIResult<UIAction>;
}

/// TUI session handle
#[derive(Debug)]
pub struct TUISession {
    pub state: TUIState,
    pub running: bool,
}

impl TUISession {
    /// Create a new TUI session
    pub fn new() -> Self {
        Self {
            state: TUIState {
                current_view: ViewType::PeerList,
                selected_peer: None,
                file_browser_path: PathBuf::from("."),
                active_operations: Vec::new(),
                peer_list: Vec::new(),
                navigation_stack: Vec::new(),
            },
            running: true,
        }
    }

    /// Stop the TUI session
    pub fn stop(&mut self) {
        self.running = false;
    }
}

impl Default for TUISession {
    fn default() -> Self {
        Self::new()
    }
}

/// Input event types
#[derive(Debug, Clone)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

/// Keyboard event
#[derive(Debug, Clone)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

/// Key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Enter,
    Escape,
    Backspace,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    F(u8),
}

/// Key modifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Mouse event
#[derive(Debug, Clone)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
}

/// Mouse event kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    Click,
    DoubleClick,
    Scroll(ScrollDirection),
}

/// Scroll direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

/// UI action resulting from input
#[derive(Debug, Clone)]
pub enum UIAction {
    None,
    Navigate(ViewType),
    SelectPeer(usize),
    SelectFile(usize),
    ExecuteCommand(String),
    Quit,
}
