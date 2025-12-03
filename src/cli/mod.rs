// CLI UX module for Kizuna
// Provides command-line interface and interactive TUI capabilities

pub mod completion;
pub mod config;
pub mod error;
pub mod filter;
pub mod handlers;
pub mod help;
pub mod history;
pub mod integration;
pub mod intelligent_completion;
pub mod output;
pub mod parser;
pub mod pipeline;
pub mod powershell_completion;
pub mod security_integration;
pub mod tui;
pub mod types;

pub use error::{CLIError, CLIResult};
pub use types::*;

// Re-export commonly used items
pub use completion::CompletionGenerator;
pub use history::{HistoryEntry, HistoryManager, HistoryStatistics};
pub use intelligent_completion::{Completion, CompletionContext, IntelligentCompletion};
pub use powershell_completion::PowerShellCompletion;

pub use parser::{
    ClapCommandParser, CommandContext, CommandExecutor, CommandParser, CommandPipeline,
    CommandRouter, CommandValidator, ValidationWarning,
};

pub use output::{
    OutputFormatter, TableFormatter, JSONFormatter, CSVFormatter, MinimalFormatter,
    ProgressRenderer, ProgressDisplay, StyleManager, ColorManager,
};

pub use pipeline::{
    PipelineInput, PipelineOutput, BatchOperationInput, BatchOperationOutput, OperationOutput,
};

pub use help::{
    HelpSystem, CommandHelp, HelpOption, HelpExample, SearchResult, MatchType,
};

pub use filter::{
    PeerFilter, FileFilter, FileEntry, OperationFilter, SearchEngine, SearchMatch,
};

pub use security_integration::{
    CLISecurityIntegration, CLISession,
};

pub use integration::CLISystemIntegration;
