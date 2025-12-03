// Command Execution Module
//
// This module provides secure remote command execution capabilities with sandboxing,
// authorization controls, and cross-platform compatibility.

pub mod manager;
pub mod sandbox;
pub mod auth;
pub mod script;
pub mod error;
pub mod types;
pub mod platform;
pub mod system_info;
pub mod notification;
pub mod template;
pub mod template_sharing;
pub mod scheduler;
pub mod history;
pub mod audit;
pub mod security_integration;
pub mod transport_integration;
pub mod api;

// Re-export main types and traits
pub use error::{CommandError, CommandResult as CmdResult};
pub use types::*;
pub use manager::CommandManager;
pub use sandbox::SandboxEngine;
pub use auth::AuthorizationManager;
pub use script::ScriptEngine;
pub use platform::{UnifiedCommandManager, CommandTranslator, Platform};
pub use system_info::SystemInfoProvider;
pub use notification::{
    NotificationManager, NotificationBackend, NotificationCapabilities, NotificationRecord,
    NotificationFormatter, NotificationBuilder, FormattedNotification, NotificationStyle,
    DeliveryService, DeliveryTracker, DeliveryInfo, DeliveryAnalytics,
};
pub use template::{
    TemplateManager, CommandTemplate, TemplateParameter, ParameterType,
    TemplateInstantiationRequest, ValidationResult, ValidationError, TemplateId,
};
pub use template_sharing::{
    TemplateSharingManager, TemplateShareRequest, TemplatePermissions,
    SharedTemplate, SyncStatus, TemplateUpdate,
};
pub use scheduler::{
    Scheduler, ScheduledTask, ScheduledTaskType, Schedule, ScheduleType,
    ScheduledExecutionResult, ScheduleId,
};
pub use history::{
    HistoryManager, SqliteHistoryManager, HistoryFilter,
};
pub use audit::{
    AuditLogger, SqliteAuditLogger, AuditLogEntry, AuditEventType,
    AuditSeverity, AuditFilter, create_authorization_log, create_security_event_log,
};
pub use security_integration::{
    CommandSecurityIntegration, EncryptedCommandMessage, CommandMessageType,
    CommandMessage, SecureCommandTransmission,
};
pub use transport_integration::{
    CommandTransportIntegration, CommandExecutionApi, CommandExecutionConfig,
};
pub use api::{
    CommandExecution, CommandExecutionBuilder, CommandExecutionEvent,
    CommandExecutionCallback,
};
