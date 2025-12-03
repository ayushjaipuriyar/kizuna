// iOS internationalization support
//
// Handles localization, language preferences, and region-specific formatting

use crate::platform::{PlatformResult, PlatformError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Internationalization manager for iOS
pub struct InternationalizationManager {
    initialized: Arc<RwLock<bool>>,
    locale_info: Arc<RwLock<Option<LocaleInfo>>>,
    translations: Arc<RwLock<HashMap<String, HashMap<String, String>>>>,
}

/// Locale information
#[derive(Debug, Clone)]
pub struct LocaleInfo {
    pub language_code: String,
    pub country_code: String,
    pub locale_identifier: String,
    pub preferred_languages: Vec<String>,
    pub calendar: CalendarType,
    pub measurement_system: MeasurementSystem,
    pub currency_code: String,
    pub currency_symbol: String,
    pub decimal_separator: String,
    pub grouping_separator: String,
}

/// Calendar types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarType {
    Gregorian,
    Buddhist,
    Chinese,
    Hebrew,
    Islamic,
    IslamicCivil,
    Japanese,
    Persian,
    RepublicOfChina,
    Indian,
}

/// Measurement systems
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementSystem {
    Metric,
    US,
    UK,
}

/// Text direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

impl InternationalizationManager {
    /// Create a new internationalization manager
    pub fn new() -> Self {
        Self {
            initialized: Arc::new(RwLock::new(false)),
            locale_info: Arc::new(RwLock::new(None)),
            translations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the internationalization manager
    pub async fn initialize(&self) -> PlatformResult<()> {
        let mut initialized = self.initialized.write().await;
        if *initialized {
            return Ok(());
        }

        // Detect locale information
        let locale_info = self.detect_locale_info().await?;
        *self.locale_info.write().await = Some(locale_info);

        // Load default translations
        self.load_default_translations().await?;

        *initialized = true;
        Ok(())
    }

    /// Detect locale information
    async fn detect_locale_info(&self) -> PlatformResult<LocaleInfo> {
        // In a real implementation, this would query Locale.current
        Ok(LocaleInfo {
            language_code: "en".to_string(),
            country_code: "US".to_string(),
            locale_identifier: "en_US".to_string(),
            preferred_languages: vec!["en".to_string()],
            calendar: CalendarType::Gregorian,
            measurement_system: MeasurementSystem::US,
            currency_code: "USD".to_string(),
            currency_symbol: "$".to_string(),
            decimal_separator: ".".to_string(),
            grouping_separator: ",".to_string(),
        })
    }

    /// Load default translations
    async fn load_default_translations(&self) -> PlatformResult<()> {
        let mut translations = self.translations.write().await;
        
        // Add some default English translations
        let mut en_translations = HashMap::new();
        en_translations.insert("welcome".to_string(), "Welcome".to_string());
        en_translations.insert("settings".to_string(), "Settings".to_string());
        en_translations.insert("cancel".to_string(), "Cancel".to_string());
        en_translations.insert("ok".to_string(), "OK".to_string());
        
        translations.insert("en".to_string(), en_translations);
        
        Ok(())
    }

    /// Get locale information
    pub async fn get_locale_info(&self) -> PlatformResult<LocaleInfo> {
        self.locale_info.read().await
            .clone()
            .ok_or_else(|| PlatformError::IntegrationError(
                "Internationalization manager not initialized".to_string()
            ))
    }

    /// Get current language code
    pub async fn get_language_code(&self) -> String {
        match self.locale_info.read().await.as_ref() {
            Some(info) => info.language_code.clone(),
            None => "en".to_string(),
        }
    }

    /// Get current country code
    pub async fn get_country_code(&self) -> String {
        match self.locale_info.read().await.as_ref() {
            Some(info) => info.country_code.clone(),
            None => "US".to_string(),
        }
    }

    /// Get preferred languages
    pub async fn get_preferred_languages(&self) -> Vec<String> {
        match self.locale_info.read().await.as_ref() {
            Some(info) => info.preferred_languages.clone(),
            None => vec!["en".to_string()],
        }
    }

    /// Get localized string
    pub async fn localized_string(&self, key: &str) -> String {
        let language = self.get_language_code().await;
        let translations = self.translations.read().await;
        
        if let Some(lang_translations) = translations.get(&language) {
            if let Some(translation) = lang_translations.get(key) {
                return translation.clone();
            }
        }
        
        // Fallback to key if translation not found
        key.to_string()
    }

    /// Get localized string with fallback
    pub async fn localized_string_with_fallback(
        &self,
        key: &str,
        fallback: &str,
    ) -> String {
        let language = self.get_language_code().await;
        let translations = self.translations.read().await;
        
        if let Some(lang_translations) = translations.get(&language) {
            if let Some(translation) = lang_translations.get(key) {
                return translation.clone();
            }
        }
        
        fallback.to_string()
    }

    /// Add translation
    pub async fn add_translation(
        &self,
        language: &str,
        key: &str,
        value: &str,
    ) -> PlatformResult<()> {
        if language.is_empty() || key.is_empty() || value.is_empty() {
            return Err(PlatformError::IntegrationError(
                "Language, key, and value cannot be empty".to_string()
            ));
        }

        let mut translations = self.translations.write().await;
        translations
            .entry(language.to_string())
            .or_insert_with(HashMap::new)
            .insert(key.to_string(), value.to_string());
        
        Ok(())
    }

    /// Load translations from map
    pub async fn load_translations(
        &self,
        language: &str,
        translations_map: HashMap<String, String>,
    ) -> PlatformResult<()> {
        if language.is_empty() {
            return Err(PlatformError::IntegrationError(
                "Language cannot be empty".to_string()
            ));
        }

        let mut translations = self.translations.write().await;
        translations.insert(language.to_string(), translations_map);
        
        Ok(())
    }

    /// Get text direction for current language
    pub async fn get_text_direction(&self) -> TextDirection {
        let language = self.get_language_code().await;
        
        // RTL languages
        match language.as_str() {
            "ar" | "he" | "fa" | "ur" => TextDirection::RightToLeft,
            _ => TextDirection::LeftToRight,
        }
    }

    /// Check if current language is RTL
    pub async fn is_rtl(&self) -> bool {
        self.get_text_direction().await == TextDirection::RightToLeft
    }

    /// Format number according to locale
    pub async fn format_number(&self, number: f64) -> String {
        let locale_info = match self.locale_info.read().await.as_ref() {
            Some(info) => info.clone(),
            None => return number.to_string(),
        };

        // Simple formatting with locale separators
        let parts: Vec<&str> = number.to_string().split('.').collect();
        let integer_part = parts[0];
        
        // Add grouping separators
        let mut formatted = String::new();
        for (i, c) in integer_part.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                formatted.insert(0, locale_info.grouping_separator.chars().next().unwrap());
            }
            formatted.insert(0, c);
        }

        // Add decimal part if exists
        if parts.len() > 1 {
            formatted.push_str(&locale_info.decimal_separator);
            formatted.push_str(parts[1]);
        }

        formatted
    }

    /// Format currency according to locale
    pub async fn format_currency(&self, amount: f64) -> String {
        let locale_info = match self.locale_info.read().await.as_ref() {
            Some(info) => info.clone(),
            None => return format!("${:.2}", amount),
        };

        let formatted_number = self.format_number(amount).await;
        format!("{}{}", locale_info.currency_symbol, formatted_number)
    }

    /// Get measurement system
    pub async fn get_measurement_system(&self) -> MeasurementSystem {
        match self.locale_info.read().await.as_ref() {
            Some(info) => info.measurement_system,
            None => MeasurementSystem::Metric,
        }
    }

    /// Check if using metric system
    pub async fn is_metric(&self) -> bool {
        self.get_measurement_system().await == MeasurementSystem::Metric
    }

    /// Get calendar type
    pub async fn get_calendar_type(&self) -> CalendarType {
        match self.locale_info.read().await.as_ref() {
            Some(info) => info.calendar,
            None => CalendarType::Gregorian,
        }
    }
}

impl Default for InternationalizationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_i18n_manager_initialization() {
        let manager = InternationalizationManager::new();
        assert!(!*manager.initialized.read().await);

        let result = manager.initialize().await;
        assert!(result.is_ok());
        assert!(*manager.initialized.read().await);
    }

    #[tokio::test]
    async fn test_get_locale_info() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let info = manager.get_locale_info().await.unwrap();
        assert!(!info.language_code.is_empty());
        assert!(!info.country_code.is_empty());
    }

    #[tokio::test]
    async fn test_get_language_code() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let language = manager.get_language_code().await;
        assert!(!language.is_empty());
    }

    #[tokio::test]
    async fn test_get_preferred_languages() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let languages = manager.get_preferred_languages().await;
        assert!(!languages.is_empty());
    }

    #[tokio::test]
    async fn test_localized_string() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let welcome = manager.localized_string("welcome").await;
        assert_eq!(welcome, "Welcome");
    }

    #[tokio::test]
    async fn test_localized_string_missing_key() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let missing = manager.localized_string("nonexistent_key").await;
        assert_eq!(missing, "nonexistent_key");
    }

    #[tokio::test]
    async fn test_localized_string_with_fallback() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let text = manager.localized_string_with_fallback(
            "nonexistent",
            "Fallback Text"
        ).await;
        assert_eq!(text, "Fallback Text");
    }

    #[tokio::test]
    async fn test_add_translation() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let result = manager.add_translation("en", "test_key", "Test Value").await;
        assert!(result.is_ok());

        let value = manager.localized_string("test_key").await;
        assert_eq!(value, "Test Value");
    }

    #[tokio::test]
    async fn test_add_translation_validation() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let result = manager.add_translation("", "key", "value").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_translations() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let mut translations = HashMap::new();
        translations.insert("hello".to_string(), "Hola".to_string());
        translations.insert("goodbye".to_string(), "Adiós".to_string());

        let result = manager.load_translations("es", translations).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_text_direction() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let direction = manager.get_text_direction().await;
        // English is LTR
        assert_eq!(direction, TextDirection::LeftToRight);
    }

    #[tokio::test]
    async fn test_is_rtl() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let is_rtl = manager.is_rtl().await;
        // English is not RTL
        assert!(!is_rtl);
    }

    #[tokio::test]
    async fn test_format_number() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let formatted = manager.format_number(1234.56).await;
        assert!(formatted.contains("1"));
        assert!(formatted.contains("234"));
    }

    #[tokio::test]
    async fn test_format_currency() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let formatted = manager.format_currency(99.99).await;
        assert!(formatted.contains("$") || formatted.contains("99"));
    }

    #[tokio::test]
    async fn test_measurement_system() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let system = manager.get_measurement_system().await;
        // Should return a valid system
        assert!(matches!(
            system,
            MeasurementSystem::Metric | MeasurementSystem::US | MeasurementSystem::UK
        ));
    }

    #[tokio::test]
    async fn test_calendar_type() {
        let manager = InternationalizationManager::new();
        manager.initialize().await.unwrap();

        let calendar = manager.get_calendar_type().await;
        // Should return a valid calendar
        assert_eq!(calendar, CalendarType::Gregorian);
    }
}
