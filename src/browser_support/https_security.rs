//! HTTPS and Secure Context Security for Browser Support
//!
//! This module provides HTTPS enforcement, secure cookie management,
//! and secure context validation for browser clients.

use uuid::Uuid;
use crate::browser_support::types::BrowserInfo;
use crate::browser_support::error::{BrowserSupportError, BrowserResult};

/// HTTPS enforcement middleware for browser API server
pub struct HTTPSEnforcer {
    /// Whether to enforce HTTPS in production
    enforce_https: bool,
    /// Allowed origins for development
    dev_origins: Vec<String>,
}

impl HTTPSEnforcer {
    /// Create a new HTTPS enforcer
    pub fn new(enforce_https: bool) -> Self {
        Self {
            enforce_https,
            dev_origins: vec!["http://localhost".to_string(), "http://127.0.0.1".to_string()],
        }
    }
    
    /// Create enforcer for production (strict HTTPS)
    pub fn production() -> Self {
        Self::new(true)
    }
    
    /// Create enforcer for development (allows localhost HTTP)
    pub fn development() -> Self {
        Self::new(false)
    }
    
    /// Add a development origin
    pub fn add_dev_origin(&mut self, origin: String) {
        self.dev_origins.push(origin);
    }
    
    /// Check if a request is using HTTPS or is from an allowed dev origin
    pub fn validate_request(&self, scheme: &str, host: &str) -> BrowserResult<()> {
        // Always allow HTTPS
        if scheme == "https" {
            return Ok(());
        }
        
        // In development mode, allow configured dev origins
        if !self.enforce_https {
            for dev_origin in &self.dev_origins {
                if dev_origin.contains(host) {
                    return Ok(());
                }
            }
        }
        
        // Reject HTTP in production or non-dev origins
        Err(BrowserSupportError::HTTPSRequired(
            format!("HTTPS is required for {}://{}", scheme, host)
        ))
    }
    
    /// Get security headers for HTTP responses
    pub fn get_security_headers(&self) -> Vec<(String, String)> {
        vec![
            // Strict Transport Security - force HTTPS for 1 year
            ("Strict-Transport-Security".to_string(), "max-age=31536000; includeSubDomains".to_string()),
            
            // Content Security Policy - restrict resource loading
            ("Content-Security-Policy".to_string(), self.get_csp_header()),
            
            // X-Frame-Options - prevent clickjacking
            ("X-Frame-Options".to_string(), "DENY".to_string()),
            
            // X-Content-Type-Options - prevent MIME sniffing
            ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
            
            // Referrer-Policy - control referrer information
            ("Referrer-Policy".to_string(), "strict-origin-when-cross-origin".to_string()),
            
            // Permissions-Policy - control browser features
            ("Permissions-Policy".to_string(), "geolocation=(), microphone=(), camera=()".to_string()),
        ]
    }
    
    /// Get Content Security Policy header value
    fn get_csp_header(&self) -> String {
        let directives = vec![
            "default-src 'self'",
            "script-src 'self' 'unsafe-inline'", // Allow inline scripts for WebRTC
            "style-src 'self' 'unsafe-inline'",
            "img-src 'self' data: blob:",
            "font-src 'self'",
            "connect-src 'self' wss: https:",
            "media-src 'self' blob:",
            "object-src 'none'",
            "base-uri 'self'",
            "form-action 'self'",
            "frame-ancestors 'none'",
            "upgrade-insecure-requests",
        ];
        
        directives.join("; ")
    }
}

/// Secure cookie manager for browser sessions
pub struct SecureCookieManager {
    /// Cookie domain
    domain: Option<String>,
    /// Cookie path
    path: String,
    /// Whether to use secure flag
    secure: bool,
    /// Whether to use HttpOnly flag
    http_only: bool,
    /// SameSite policy
    same_site: SameSitePolicy,
    /// Cookie max age in seconds
    max_age: i64,
}

/// SameSite cookie policy
#[derive(Debug, Clone, Copy)]
pub enum SameSitePolicy {
    Strict,
    Lax,
    None,
}

impl SecureCookieManager {
    /// Create a new secure cookie manager
    pub fn new() -> Self {
        Self {
            domain: None,
            path: "/".to_string(),
            secure: true,
            http_only: true,
            same_site: SameSitePolicy::Strict,
            max_age: 3600, // 1 hour
        }
    }
    
    /// Set cookie domain
    pub fn with_domain(mut self, domain: String) -> Self {
        self.domain = Some(domain);
        self
    }
    
    /// Set cookie path
    pub fn with_path(mut self, path: String) -> Self {
        self.path = path;
        self
    }
    
    /// Set secure flag
    pub fn with_secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }
    
    /// Set HttpOnly flag
    pub fn with_http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }
    
    /// Set SameSite policy
    pub fn with_same_site(mut self, same_site: SameSitePolicy) -> Self {
        self.same_site = same_site;
        self
    }
    
    /// Set max age
    pub fn with_max_age(mut self, max_age: i64) -> Self {
        self.max_age = max_age;
        self
    }
    
    /// Create a session cookie header
    pub fn create_session_cookie(&self, session_id: &Uuid) -> String {
        let mut cookie = format!("session_id={}; Path={}", session_id, self.path);
        
        if let Some(ref domain) = self.domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }
        
        if self.secure {
            cookie.push_str("; Secure");
        }
        
        if self.http_only {
            cookie.push_str("; HttpOnly");
        }
        
        match self.same_site {
            SameSitePolicy::Strict => cookie.push_str("; SameSite=Strict"),
            SameSitePolicy::Lax => cookie.push_str("; SameSite=Lax"),
            SameSitePolicy::None => cookie.push_str("; SameSite=None"),
        }
        
        cookie.push_str(&format!("; Max-Age={}", self.max_age));
        
        cookie
    }
    
    /// Create a cookie deletion header
    pub fn delete_session_cookie(&self) -> String {
        let mut cookie = format!("session_id=; Path={}; Max-Age=0", self.path);
        
        if let Some(ref domain) = self.domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }
        
        cookie
    }
    
    /// Parse session ID from cookie header
    pub fn parse_session_id(&self, cookie_header: &str) -> Option<Uuid> {
        for cookie in cookie_header.split(';') {
            let cookie = cookie.trim();
            if cookie.starts_with("session_id=") {
                if let Some(value) = cookie.strip_prefix("session_id=") {
                    if let Ok(uuid) = Uuid::parse_str(value) {
                        return Some(uuid);
                    }
                }
            }
        }
        None
    }
}

impl Default for SecureCookieManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Secure context validator for browser APIs
pub struct SecureContextValidator {
    /// Required security features
    required_features: Vec<SecureFeature>,
}

/// Secure features required for browser APIs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureFeature {
    HTTPS,
    WebRTC,
    ClipboardAPI,
    ServiceWorker,
    PushNotifications,
}

impl SecureContextValidator {
    /// Create a new secure context validator
    pub fn new() -> Self {
        Self {
            required_features: vec![SecureFeature::HTTPS],
        }
    }
    
    /// Add a required feature
    pub fn require_feature(mut self, feature: SecureFeature) -> Self {
        if !self.required_features.contains(&feature) {
            self.required_features.push(feature);
        }
        self
    }
    
    /// Validate browser context supports required features
    pub fn validate_context(&self, browser_info: &BrowserInfo) -> BrowserResult<()> {
        for feature in &self.required_features {
            match feature {
                SecureFeature::HTTPS => {
                    // HTTPS is enforced by HTTPSEnforcer
                }
                SecureFeature::WebRTC => {
                    if !browser_info.supports_webrtc {
                        return Err(BrowserSupportError::BrowserCompatibilityError {
                            browser: format!("{:?}", browser_info.browser_type),
                            issue: "WebRTC not supported".to_string(),
                        });
                    }
                }
                SecureFeature::ClipboardAPI => {
                    if !browser_info.supports_clipboard_api {
                        return Err(BrowserSupportError::BrowserCompatibilityError {
                            browser: format!("{:?}", browser_info.browser_type),
                            issue: "Clipboard API not supported".to_string(),
                        });
                    }
                }
                SecureFeature::ServiceWorker => {
                    // Service workers require HTTPS (except localhost)
                    // This is validated by the browser itself
                }
                SecureFeature::PushNotifications => {
                    // Push notifications require HTTPS and service workers
                    // This is validated by the browser itself
                }
            }
        }
        
        Ok(())
    }
    
    /// Get list of missing features
    pub fn get_missing_features(&self, browser_info: &BrowserInfo) -> Vec<SecureFeature> {
        let mut missing = Vec::new();
        
        for feature in &self.required_features {
            match feature {
                SecureFeature::WebRTC => {
                    if !browser_info.supports_webrtc {
                        missing.push(*feature);
                    }
                }
                SecureFeature::ClipboardAPI => {
                    if !browser_info.supports_clipboard_api {
                        missing.push(*feature);
                    }
                }
                _ => {}
            }
        }
        
        missing
    }
}

impl Default for SecureContextValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Session security manager combining all security features
pub struct SessionSecurityManager {
    /// HTTPS enforcer
    https_enforcer: HTTPSEnforcer,
    /// Cookie manager
    cookie_manager: SecureCookieManager,
    /// Context validator
    context_validator: SecureContextValidator,
}

impl SessionSecurityManager {
    /// Create a new session security manager
    pub fn new(
        https_enforcer: HTTPSEnforcer,
        cookie_manager: SecureCookieManager,
        context_validator: SecureContextValidator,
    ) -> Self {
        Self {
            https_enforcer,
            cookie_manager,
            context_validator,
        }
    }
    
    /// Create for production environment
    pub fn production() -> Self {
        Self {
            https_enforcer: HTTPSEnforcer::production(),
            cookie_manager: SecureCookieManager::new(),
            context_validator: SecureContextValidator::new()
                .require_feature(SecureFeature::HTTPS)
                .require_feature(SecureFeature::WebRTC),
        }
    }
    
    /// Create for development environment
    pub fn development() -> Self {
        Self {
            https_enforcer: HTTPSEnforcer::development(),
            cookie_manager: SecureCookieManager::new().with_secure(false),
            context_validator: SecureContextValidator::new(),
        }
    }
    
    /// Validate a request and browser context
    pub fn validate_request(
        &self,
        scheme: &str,
        host: &str,
        browser_info: &BrowserInfo,
    ) -> BrowserResult<()> {
        // Validate HTTPS
        self.https_enforcer.validate_request(scheme, host)?;
        
        // Validate browser context
        self.context_validator.validate_context(browser_info)?;
        
        Ok(())
    }
    
    /// Get security headers
    pub fn get_security_headers(&self) -> Vec<(String, String)> {
        self.https_enforcer.get_security_headers()
    }
    
    /// Create session cookie
    pub fn create_session_cookie(&self, session_id: &Uuid) -> String {
        self.cookie_manager.create_session_cookie(session_id)
    }
    
    /// Delete session cookie
    pub fn delete_session_cookie(&self) -> String {
        self.cookie_manager.delete_session_cookie()
    }
    
    /// Parse session ID from cookie
    pub fn parse_session_id(&self, cookie_header: &str) -> Option<Uuid> {
        self.cookie_manager.parse_session_id(cookie_header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_support::types::BrowserType;
    
    #[test]
    fn test_https_enforcer_production() {
        let enforcer = HTTPSEnforcer::production();
        
        // HTTPS should be allowed
        assert!(enforcer.validate_request("https", "example.com").is_ok());
        
        // HTTP should be rejected
        assert!(enforcer.validate_request("http", "example.com").is_err());
        
        // Even localhost HTTP should be rejected in production
        assert!(enforcer.validate_request("http", "localhost").is_err());
    }
    
    #[test]
    fn test_https_enforcer_development() {
        let enforcer = HTTPSEnforcer::development();
        
        // HTTPS should be allowed
        assert!(enforcer.validate_request("https", "example.com").is_ok());
        
        // Localhost HTTP should be allowed in development
        assert!(enforcer.validate_request("http", "localhost").is_ok());
        assert!(enforcer.validate_request("http", "127.0.0.1").is_ok());
        
        // Other HTTP should still be rejected
        assert!(enforcer.validate_request("http", "example.com").is_err());
    }
    
    #[test]
    fn test_security_headers() {
        let enforcer = HTTPSEnforcer::production();
        let headers = enforcer.get_security_headers();
        
        // Check that important headers are present
        assert!(headers.iter().any(|(k, _)| k == "Strict-Transport-Security"));
        assert!(headers.iter().any(|(k, _)| k == "Content-Security-Policy"));
        assert!(headers.iter().any(|(k, _)| k == "X-Frame-Options"));
        assert!(headers.iter().any(|(k, _)| k == "X-Content-Type-Options"));
    }
    
    #[test]
    fn test_secure_cookie_manager() {
        let manager = SecureCookieManager::new();
        let session_id = Uuid::new_v4();
        
        // Create cookie
        let cookie = manager.create_session_cookie(&session_id);
        
        // Check cookie attributes
        assert!(cookie.contains(&session_id.to_string()));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        assert!(cookie.contains("Max-Age="));
        
        // Parse session ID
        let parsed = manager.parse_session_id(&cookie);
        assert_eq!(parsed, Some(session_id));
    }
    
    #[test]
    fn test_cookie_deletion() {
        let manager = SecureCookieManager::new();
        let delete_cookie = manager.delete_session_cookie();
        
        assert!(delete_cookie.contains("Max-Age=0"));
    }
    
    #[test]
    fn test_secure_context_validator() {
        let validator = SecureContextValidator::new()
            .require_feature(SecureFeature::WebRTC)
            .require_feature(SecureFeature::ClipboardAPI);
        
        // Browser with all features
        let good_browser = BrowserInfo {
            user_agent: "Mozilla/5.0".to_string(),
            browser_type: BrowserType::Chrome,
            version: "120.0".to_string(),
            platform: "Linux".to_string(),
            supports_webrtc: true,
            supports_clipboard_api: true,
        };
        
        assert!(validator.validate_context(&good_browser).is_ok());
        
        // Browser missing WebRTC
        let bad_browser = BrowserInfo {
            user_agent: "Mozilla/5.0".to_string(),
            browser_type: BrowserType::Safari,
            version: "12.0".to_string(),
            platform: "iOS".to_string(),
            supports_webrtc: false,
            supports_clipboard_api: true,
        };
        
        assert!(validator.validate_context(&bad_browser).is_err());
        
        let missing = validator.get_missing_features(&bad_browser);
        assert!(missing.contains(&SecureFeature::WebRTC));
    }
    
    #[test]
    fn test_session_security_manager() {
        let manager = SessionSecurityManager::production();
        
        let browser_info = BrowserInfo {
            user_agent: "Mozilla/5.0".to_string(),
            browser_type: BrowserType::Chrome,
            version: "120.0".to_string(),
            platform: "Linux".to_string(),
            supports_webrtc: true,
            supports_clipboard_api: true,
        };
        
        // HTTPS request should be valid
        assert!(manager.validate_request("https", "example.com", &browser_info).is_ok());
        
        // HTTP request should be invalid
        assert!(manager.validate_request("http", "example.com", &browser_info).is_err());
        
        // Get security headers
        let headers = manager.get_security_headers();
        assert!(!headers.is_empty());
    }
}
