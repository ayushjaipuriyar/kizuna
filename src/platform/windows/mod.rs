// Windows platform module

pub mod win32;
pub mod registry;
pub mod networking;
pub mod installer;
pub mod updater;
pub mod architecture;
pub mod notifications;
pub mod performance;

#[cfg(test)]
mod tests;

pub use win32::{initialize_com, cleanup_com, initialize_winsock, cleanup_winsock};
pub use registry::RegistryManager;
pub use networking::{WindowsNetworking, NetworkAdapter, ConnectionStatus};
pub use installer::{InstallerManager, MSIConfig, MSIXConfig};
pub use updater::{UpdateManager, UpdateInfo, UpdateConfig, StoreUpdateManager};
pub use architecture::{WindowsArchitecture, ArchitectureOptimizer, OptimizationConfig};
pub use notifications::{NotificationManager, ToastNotification, BadgeManager, TileManager};
pub use performance::{PerformanceOptimizer, ProcessPriority, ThreadPriority, PowerManager};
