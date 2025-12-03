// iOS form factor support
//
// Handles adaptive UI for iPhone and iPad, size classes,
// and device-specific optimizations

use crate::platform::{PlatformResult, PlatformError};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Form factor manager for iOS
pub struct FormFactorManager {
    initialized: Arc<RwLock<bool>>,
    device_info: Arc<RwLock<Option<DeviceInfo>>>,
    layout_config: Arc<RwLock<LayoutConfiguration>>,
}

/// Device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_type: DeviceType,
    pub screen_size: ScreenSize,
    pub size_class: SizeClass,
    pub orientation: Orientation,
    pub scale_factor: f32,
    pub safe_area_insets: SafeAreaInsets,
}

/// Device types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    iPhone,
    iPad,
    iPadMini,
    iPadPro,
}

/// Screen size information
#[derive(Debug, Clone, Copy)]
pub struct ScreenSize {
    pub width: f32,
    pub height: f32,
    pub diagonal_inches: f32,
}

/// Size class (UIKit trait collection)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeClass {
    pub horizontal: SizeClassType,
    pub vertical: SizeClassType,
}

/// Size class types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeClassType {
    Compact,
    Regular,
    Unspecified,
}

/// Device orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    PortraitUpsideDown,
    LandscapeLeft,
    LandscapeRight,
    Unknown,
}

/// Safe area insets
#[derive(Debug, Clone, Copy)]
pub struct SafeAreaInsets {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

/// Layout configuration
#[derive(Debug, Clone)]
pub struct LayoutConfiguration {
    pub use_adaptive_layout: bool,
    pub column_count: usize,
    pub spacing: f32,
    pub margins: Margins,
    pub font_scale: f32,
}

/// Layout margins
#[derive(Debug, Clone, Copy)]
pub struct Margins {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl FormFactorManager {
    /// Create a new form factor manager
    pub fn new() -> Self {
        Self {
            initialized: Arc::new(RwLock::new(false)),
            device_info: Arc::new(RwLock::new(None)),
            layout_config: Arc::new(RwLock::new(LayoutConfiguration::default())),
        }
    }

    /// Initialize the form factor manager
    pub async fn initialize(&self) -> PlatformResult<()> {
        let mut initialized = self.initialized.write().await;
        if *initialized {
            return Ok(());
        }

        // Detect device information
        let device_info = self.detect_device_info().await?;
        *self.device_info.write().await = Some(device_info.clone());

        // Configure layout based on device
        self.configure_layout_for_device(&device_info).await?;

        *initialized = true;
        Ok(())
    }

    /// Detect device information
    async fn detect_device_info(&self) -> PlatformResult<DeviceInfo> {
        // In a real implementation, this would query UIDevice and UIScreen
        // For now, we'll return sensible defaults for iPhone
        Ok(DeviceInfo {
            device_type: DeviceType::iPhone,
            screen_size: ScreenSize {
                width: 390.0,
                height: 844.0,
                diagonal_inches: 6.1,
            },
            size_class: SizeClass {
                horizontal: SizeClassType::Compact,
                vertical: SizeClassType::Regular,
            },
            orientation: Orientation::Portrait,
            scale_factor: 3.0,
            safe_area_insets: SafeAreaInsets {
                top: 47.0,
                bottom: 34.0,
                left: 0.0,
                right: 0.0,
            },
        })
    }

    /// Configure layout for device
    async fn configure_layout_for_device(&self, device_info: &DeviceInfo) -> PlatformResult<()> {
        let mut config = LayoutConfiguration::default();

        match device_info.device_type {
            DeviceType::iPhone => {
                config.column_count = 1;
                config.spacing = 16.0;
                config.margins = Margins {
                    top: 16.0,
                    bottom: 16.0,
                    left: 16.0,
                    right: 16.0,
                };
                config.font_scale = 1.0;
            }
            DeviceType::iPad | DeviceType::iPadMini => {
                config.column_count = 2;
                config.spacing = 24.0;
                config.margins = Margins {
                    top: 24.0,
                    bottom: 24.0,
                    left: 24.0,
                    right: 24.0,
                };
                config.font_scale = 1.1;
            }
            DeviceType::iPadPro => {
                config.column_count = 3;
                config.spacing = 32.0;
                config.margins = Margins {
                    top: 32.0,
                    bottom: 32.0,
                    left: 32.0,
                    right: 32.0,
                };
                config.font_scale = 1.2;
            }
        }

        *self.layout_config.write().await = config;
        Ok(())
    }

    /// Get device information
    pub async fn get_device_info(&self) -> PlatformResult<DeviceInfo> {
        self.device_info.read().await
            .clone()
            .ok_or_else(|| PlatformError::IntegrationError(
                "Form factor manager not initialized".to_string()
            ))
    }

    /// Get layout configuration
    pub async fn get_layout_config(&self) -> LayoutConfiguration {
        self.layout_config.read().await.clone()
    }

    /// Check if device is iPhone
    pub async fn is_iphone(&self) -> bool {
        match self.device_info.read().await.as_ref() {
            Some(info) => info.device_type == DeviceType::iPhone,
            None => false,
        }
    }

    /// Check if device is iPad
    pub async fn is_ipad(&self) -> bool {
        match self.device_info.read().await.as_ref() {
            Some(info) => matches!(
                info.device_type,
                DeviceType::iPad | DeviceType::iPadMini | DeviceType::iPadPro
            ),
            None => false,
        }
    }

    /// Check if in landscape orientation
    pub async fn is_landscape(&self) -> bool {
        match self.device_info.read().await.as_ref() {
            Some(info) => matches!(
                info.orientation,
                Orientation::LandscapeLeft | Orientation::LandscapeRight
            ),
            None => false,
        }
    }

    /// Check if in portrait orientation
    pub async fn is_portrait(&self) -> bool {
        match self.device_info.read().await.as_ref() {
            Some(info) => matches!(
                info.orientation,
                Orientation::Portrait | Orientation::PortraitUpsideDown
            ),
            None => false,
        }
    }

    /// Update orientation
    pub async fn update_orientation(&self, orientation: Orientation) -> PlatformResult<()> {
        let mut device_info = self.device_info.write().await;
        if let Some(info) = device_info.as_mut() {
            info.orientation = orientation;
            
            // Swap width and height for landscape
            if matches!(orientation, Orientation::LandscapeLeft | Orientation::LandscapeRight) {
                std::mem::swap(&mut info.screen_size.width, &mut info.screen_size.height);
            }
        }
        Ok(())
    }

    /// Get recommended column count for current device
    pub async fn get_recommended_column_count(&self) -> usize {
        self.layout_config.read().await.column_count
    }

    /// Get safe area insets
    pub async fn get_safe_area_insets(&self) -> PlatformResult<SafeAreaInsets> {
        match self.device_info.read().await.as_ref() {
            Some(info) => Ok(info.safe_area_insets),
            None => Err(PlatformError::IntegrationError(
                "Form factor manager not initialized".to_string()
            )),
        }
    }

    /// Calculate adaptive font size
    pub async fn calculate_adaptive_font_size(&self, base_size: f32) -> f32 {
        let config = self.layout_config.read().await;
        base_size * config.font_scale
    }

    /// Get size class
    pub async fn get_size_class(&self) -> PlatformResult<SizeClass> {
        match self.device_info.read().await.as_ref() {
            Some(info) => Ok(info.size_class),
            None => Err(PlatformError::IntegrationError(
                "Form factor manager not initialized".to_string()
            )),
        }
    }

    /// Check if should use compact layout
    pub async fn should_use_compact_layout(&self) -> bool {
        match self.device_info.read().await.as_ref() {
            Some(info) => {
                info.size_class.horizontal == SizeClassType::Compact ||
                info.size_class.vertical == SizeClassType::Compact
            }
            None => false,
        }
    }

    /// Check if should use regular layout
    pub async fn should_use_regular_layout(&self) -> bool {
        match self.device_info.read().await.as_ref() {
            Some(info) => {
                info.size_class.horizontal == SizeClassType::Regular &&
                info.size_class.vertical == SizeClassType::Regular
            }
            None => false,
        }
    }

    /// Get screen scale factor
    pub async fn get_scale_factor(&self) -> f32 {
        match self.device_info.read().await.as_ref() {
            Some(info) => info.scale_factor,
            None => 1.0,
        }
    }

    /// Update layout configuration
    pub async fn update_layout_config(&self, config: LayoutConfiguration) {
        *self.layout_config.write().await = config;
    }
}

impl Default for FormFactorManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for LayoutConfiguration {
    fn default() -> Self {
        Self {
            use_adaptive_layout: true,
            column_count: 1,
            spacing: 16.0,
            margins: Margins {
                top: 16.0,
                bottom: 16.0,
                left: 16.0,
                right: 16.0,
            },
            font_scale: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_form_factor_manager_initialization() {
        let manager = FormFactorManager::new();
        assert!(!*manager.initialized.read().await);

        let result = manager.initialize().await;
        assert!(result.is_ok());
        assert!(*manager.initialized.read().await);
    }

    #[tokio::test]
    async fn test_get_device_info() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        let info = manager.get_device_info().await.unwrap();
        assert!(info.screen_size.width > 0.0);
        assert!(info.screen_size.height > 0.0);
    }

    #[tokio::test]
    async fn test_device_type_detection() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        // Default is iPhone
        assert!(manager.is_iphone().await || manager.is_ipad().await);
    }

    #[tokio::test]
    async fn test_orientation_detection() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        // Default is portrait
        assert!(manager.is_portrait().await || manager.is_landscape().await);
    }

    #[tokio::test]
    async fn test_update_orientation() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        let result = manager.update_orientation(Orientation::LandscapeLeft).await;
        assert!(result.is_ok());

        assert!(manager.is_landscape().await);
    }

    #[tokio::test]
    async fn test_get_layout_config() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        let config = manager.get_layout_config().await;
        assert!(config.column_count > 0);
        assert!(config.spacing > 0.0);
    }

    #[tokio::test]
    async fn test_recommended_column_count() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        let count = manager.get_recommended_column_count().await;
        assert!(count > 0);
        assert!(count <= 3);
    }

    #[tokio::test]
    async fn test_safe_area_insets() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        let insets = manager.get_safe_area_insets().await.unwrap();
        assert!(insets.top >= 0.0);
        assert!(insets.bottom >= 0.0);
    }

    #[tokio::test]
    async fn test_adaptive_font_size() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        let base_size = 16.0;
        let adaptive_size = manager.calculate_adaptive_font_size(base_size).await;
        assert!(adaptive_size >= base_size);
    }

    #[tokio::test]
    async fn test_size_class() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        let size_class = manager.get_size_class().await.unwrap();
        assert_ne!(size_class.horizontal, SizeClassType::Unspecified);
    }

    #[tokio::test]
    async fn test_compact_layout_check() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        // iPhone should use compact layout
        let is_compact = manager.should_use_compact_layout().await;
        assert!(is_compact);
    }

    #[tokio::test]
    async fn test_scale_factor() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        let scale = manager.get_scale_factor().await;
        assert!(scale >= 1.0);
        assert!(scale <= 3.0);
    }

    #[tokio::test]
    async fn test_update_layout_config() {
        let manager = FormFactorManager::new();
        manager.initialize().await.unwrap();

        let mut config = manager.get_layout_config().await;
        config.column_count = 4;

        manager.update_layout_config(config).await;

        let updated_config = manager.get_layout_config().await;
        assert_eq!(updated_config.column_count, 4);
    }
}
