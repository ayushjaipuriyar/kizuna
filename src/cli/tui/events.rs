// Event handling for TUI

use crossterm::event::{self, Event, KeyEvent, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

/// Event handler for TUI input
pub struct EventHandler {
    // Future: Add event filtering and processing
}

impl EventHandler {
    /// Create a new event handler
    pub fn new() -> Self {
        Self {}
    }

    /// Process an event
    pub fn process(&self, event: Event) -> Option<Event> {
        // For now, pass through all events
        // Future: Add filtering, debouncing, etc.
        Some(event)
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Event loop that polls for terminal events
pub struct EventLoop {
    sender: mpsc::Sender<Event>,
}

impl EventLoop {
    /// Create a new event loop
    pub fn new(sender: mpsc::Sender<Event>) -> Self {
        Self { sender }
    }

    /// Run the event loop
    pub async fn run(self) {
        loop {
            // Poll for events with timeout
            if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(event) = event::read() {
                    // Send event to main loop
                    if self.sender.send(event).await.is_err() {
                        // Channel closed, exit loop
                        break;
                    }
                }
            }

            // Small delay to prevent busy loop
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
