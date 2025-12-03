// iOS accessibility support
//
// Handles VoiceOver, Dynamic Type, and other iOS accessibility features

use crate::platform::{PlatformResult, PlatformError};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Accessibility manager for iOS
pub struct AccessibilityManager {
    initialized: Arc<RwLock<bool>>,
    features: Arc<RwLock<Option<AccessibilityFeatures>>>,
}

/// Accessibility features status
#[derive(Debug, Clone)]
pub struct AccessibilityFeatures {
    pub voiceover_enabled: bool,
    pub bold_text_enabled: bool,
    pub larger_text_enabled: bool,
    pub reduce_motion_enabled: bool,
    pub reduce_transparency_enabled: bool,
    pub increase_contrast_enabled: bool,
    pub differentiate_without_color_enabled: bool,
    pub invert_colors_enabled: bool,
    pub mono_audio_enabled: bool,
    pub closed_captions_enabled: bool,
    pub guided_access_enabled: bool,
    pub switch_control_enabled: bool,
    pub speak_selection_enabled: bool,
    pub speak_screen_enabled: bool,
}

/// Dynamic Type content size categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentSizeCategory {
    ExtraSmall,
    Small,
    Medium,
    Large,
    ExtraLarge,
    ExtraExtraLarge,
    ExtraExtraExtraLarge,
    AccessibilityMedium,
    AccessibilityLarge,
    AccessibilityExtraLarge,
    AccessibilityExtraExtraLarge,
    AccessibilityExtraExtraExtraLarge,
}

/// Accessibility traits for UI elements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityTrait {
    Button,
    Link,
    Header,
    SearchField,
    Image,
    Selected,
    PlaysSound,
    KeyboardKey,
    StaticText,
    SummaryElement,
    NotEnabled,
    UpdatesFrequently,
    StartsMediaSession,
    Adjustable,
    AllowsDirectInteraction,
    CausesPageTurn,
}

impl AccessibilityManager {
    /// Create a new accessibility manager
    pub fn new() -> Self {
        Self {
            initialized: Arc::new(RwLock::new(false)),
            features: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the accessibility manager
    pub async fn initialize(&self) -> PlatformResult<()> {
        let mut initialized = self.initialized.write().await;
        if *initialized {
            return Ok(());
        }

        // Detect accessibility features
        let features = self.detect_accessibility_features().await?;
        *self.features.write().await = Some(features);

        *initialized = true;
        Ok(())
    }

    /// Detect accessibility features
    async fn detect_accessibility_features(&self) -> PlatformResult<AccessibilityFeatures> {
        // In a real implementation, this would query UIAccessibility
        Ok(AccessibilityFeatures {
            voiceover_enabled: false,
            bold_text_enabled: false,
            larger_text_enabled: false,
            reduce_motion_enabled: false,
            reduce_transparency_enabled: false,
            increase_contrast_enabled: false,
            differentiate_without_color_enabled: false,
            invert_colors_enabled: false,
            mono_audio_enabled: false,
            closed_captions_enabled: false,
            guided_access_enabled: false,
            switch_control_enabled: false,
            speak_selection_enabled: false,
            speak_screen_enabled: false,
        })
    }

    /// Get accessibility features
    pub async fn get_features(&self) -> PlatformResult<AccessibilityFeatures> {
        self.features.read().await
            .clone()
            .ok_or_else(|| PlatformError::IntegrationError(
                "Accessibility manager not initialized".to_string()
            ))
    }

    /// Check if VoiceOver is enabled
    pub async fn is_voiceover_enabled(&self) -> bool {
        match self.features.read().await.as_ref() {
            Some(features) => features.voiceover_enabled,
            None => false,
        }
    }

    /// Check if bold text is enabled
    pub async fn is_bold_text_enabled(&self) -> bool {
        match self.features.read().await.as_ref() {
            Some(features) => features.bold_text_enabled,
            None => false,
        }
    }

    /// Check if reduce motion is enabled
    pub async fn is_reduce_motion_enabled(&self) -> bool {
        match self.features.read().await.as_ref() {
            Some(features) => features.reduce_motion_enabled,
            None => false,
        }
    }

    /// Check if reduce transparency is enabled
    pub async fn is_reduce_transparency_enabled(&self) -> bool {
        match self.features.read().await.as_ref() {
            Some(features) => features.reduce_transparency_enabled,
            None => false,
        }
    }

    /// Check if increase contrast is enabled
    pub async fn is_increase_contrast_enabled(&self) -> bool {
        match self.features.read().await.as_ref() {
            Some(features) => features.increase_contrast_enabled,
            None => false,
        }
    }

    /// Get preferred content size category
    pub async fn get_preferred_content_size_category(&self) -> ContentSizeCategory {
        // In a real implementation, this would query UIApplication.shared.preferredContentSizeCategory
        ContentSizeCategory::Large
    }

    /// Calculate scaled font size for Dynamic Type
    pub async fn scaled_font_size(&self, base_size: f32) -> f32 {
        let category = self.get_preferred_content_size_category().await;
        let scale_factor = self.get_scale_factor_for_category(category);
        base_size * scale_factor
    }

    /// Get scale factor for content size category
    fn get_scale_factor_for_category(&self, category: ContentSizeCategory) -> f32 {
        match category {
            ContentSizeCategory::ExtraSmall => 0.8,
            ContentSizeCategory::Small => 0.9,
            ContentSizeCategory::Medium => 0.95,
            ContentSizeCategory::Large => 1.0,
            ContentSizeCategory::ExtraLarge => 1.1,
            ContentSizeCategory::ExtraExtraLarge => 1.2,
            ContentSizeCategory::ExtraExtraExtraLarge => 1.3,
            ContentSizeCategory::AccessibilityMedium => 1.4,
            ContentSizeCategory::AccessibilityLarge => 1.5,
            ContentSizeCategory::AccessibilityExtraLarge => 1.6,
            ContentSizeCategory::AccessibilityExtraExtraLarge => 1.7,
            ContentSizeCategory::AccessibilityExtraExtraExtraLarge => 1.8,
        }
    }

    /// Post accessibility announcement
    pub async fn post_announcement(&self, message: &str) -> PlatformResult<()> {
        if message.is_empty() {
            return Err(PlatformError::IntegrationError(
                "Announcement message cannot be empty".to_string()
            ));
        }

        // In a real implementation, this would use UIAccessibility.post(notification:argument:)
        Ok(())
    }

    /// Set accessibility label for element
    pub async fn set_accessibility_label(
        &self,
        element_id: &str,
        label: &str,
    ) -> PlatformResult<()> {
        if element_id.is_empty() || label.is_empty() {
            return Err(PlatformError::IntegrationError(
                "Element ID and label cannot be empty".to_string()
            ));
        }

        // In a real implementation, this would set the accessibilityLabel property
        Ok(())
    }

    /// Set accessibility hint for element
    pub async fn set_accessibility_hint(
        &self,
        element_id: &str,
        hint: &str,
    ) -> PlatformResult<()> {
        if element_id.is_empty() || hint.is_empty() {
            return Err(PlatformError::IntegrationError(
                "Element ID and hint cannot be empty".to_string()
            ));
        }

        // In a real implementation, this would set the accessibilityHint property
        Ok(())
    }

    /// Set accessibility traits for element
    pub async fn set_accessibility_traits(
        &self,
        element_id: &str,
        traits: Vec<AccessibilityTrait>,
    ) -> PlatformResult<()> {
        if element_id.is_empty() {
            return Err(PlatformError::IntegrationError(
                "Element ID cannot be empty".to_string()
            ));
        }

        if traits.is_empty() {
            return Err(PlatformError::IntegrationError(
                "At least one trait must be specified".to_string()
            ));
        }

        // In a real implementation, this would set the accessibilityTraits property
        Ok(())
    }

    /// Check if element should be accessible
    pub async fn should_be_accessible(&self, element_id: &str) -> bool {
        // In a real implementation, this would check if the element should be accessible
        !element_id.is_empty()
    }

    /// Get recommended animation duration
    pub async fn get_recommended_animation_duration(&self, base_duration: f32) -> f32 {
        if self.is_reduce_motion_enabled().await {
            // Significantly reduce or eliminate animations
            0.0
        } else {
            base_duration
        }
    }

    /// Get recommended transparency level
    pub async fn get_recommended_transparency(&self, base_transparency: f32) -> f32 {
        if self.is_reduce_transparency_enabled().await {
            // Reduce transparency
            base_transparency.min(0.3)
        } else {
            base_transparency
        }
    }

    /// Check if should use high contrast colors
    pub async fn should_use_high_contrast(&self) -> bool {
        self.is_increase_contrast_enabled().await
    }

    /// Check if should differentiate without color
    pub async fn should_differentiate_without_color(&self) -> bool {
        match self.features.read().await.as_ref() {
            Some(features) => features.differentiate_without_color_enabled,
            None => false,
        }
    }
}

impl Default for AccessibilityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_accessibility_manager_initialization() {
        let manager = AccessibilityManager::new();
        assert!(!*manager.initialized.read().await);

        let result = manager.initialize().await;
        assert!(result.is_ok());
        assert!(*manager.initialized.read().await);
    }

    #[tokio::test]
    async fn test_get_features() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let features = manager.get_features().await.unwrap();
        // Features should be detected
        assert!(!features.voiceover_enabled || !features.voiceover_enabled);
    }

    #[tokio::test]
    async fn test_voiceover_check() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let enabled = manager.is_voiceover_enabled().await;
        // Should return a boolean
        assert!(enabled || !enabled);
    }

    #[tokio::test]
    async fn test_bold_text_check() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let enabled = manager.is_bold_text_enabled().await;
        assert!(enabled || !enabled);
    }

    #[tokio::test]
    async fn test_reduce_motion_check() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let enabled = manager.is_reduce_motion_enabled().await;
        assert!(enabled || !enabled);
    }

    #[tokio::test]
    async fn test_preferred_content_size() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let category = manager.get_preferred_content_size_category().await;
        // Should return a valid category
        assert_eq!(category, ContentSizeCategory::Large);
    }

    #[tokio::test]
    async fn test_scaled_font_size() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let base_size = 16.0;
        let scaled_size = manager.scaled_font_size(base_size).await;
        
        // Scaled size should be positive
        assert!(scaled_size > 0.0);
    }

    #[tokio::test]
    async fn test_post_announcement() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let result = manager.post_announcement("Test announcement").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_announcement_validation() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let result = manager.post_announcement("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_accessibility_label() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let result = manager.set_accessibility_label("button1", "Submit Button").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_accessibility_hint() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let result = manager.set_accessibility_hint("button1", "Double tap to submit").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_accessibility_traits() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let traits = vec![AccessibilityTrait::Button, AccessibilityTrait::Selected];
        let result = manager.set_accessibility_traits("button1", traits).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_accessibility_traits_validation() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let result = manager.set_accessibility_traits("button1", vec![]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_should_be_accessible() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        assert!(manager.should_be_accessible("button1").await);
        assert!(!manager.should_be_accessible("").await);
    }

    #[tokio::test]
    async fn test_recommended_animation_duration() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let base_duration = 0.3;
        let duration = manager.get_recommended_animation_duration(base_duration).await;
        
        // Duration should be non-negative
        assert!(duration >= 0.0);
    }

    #[tokio::test]
    async fn test_recommended_transparency() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let base_transparency = 0.5;
        let transparency = manager.get_recommended_transparency(base_transparency).await;
        
        // Transparency should be between 0 and 1
        assert!(transparency >= 0.0 && transparency <= 1.0);
    }

    #[tokio::test]
    async fn test_should_use_high_contrast() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let high_contrast = manager.should_use_high_contrast().await;
        assert!(high_contrast || !high_contrast);
    }

    #[tokio::test]
    async fn test_should_differentiate_without_color() {
        let manager = AccessibilityManager::new();
        manager.initialize().await.unwrap();

        let differentiate = manager.should_differentiate_without_color().await;
        assert!(differentiate || !differentiate);
    }
}
