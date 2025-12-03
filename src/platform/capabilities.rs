// Platform capability management

use crate::platform::{
    PlatformResult, PlatformError, PlatformCapabilities, Feature,
    OperatingSystem, Architecture,
};
use std::collections::HashMap;

/// Capability manager for tracking and managing platform features
pub struct CapabilityManager {
    capabilities: PlatformCapabilities,
    feature_registry: HashMap<Feature, bool>,
}

impl CapabilityManager {
    /// Create a new capability manager with detected capabilities
    pub fn new(capabilities: PlatformCapabilities) -> Self {
        let mut manager = Self {
            capabilities,
            feature_registry: HashMap::new(),
        };
        
        manager.initialize_feature_registry();
        manager
    }

    /// Initialize the feature registry based on capabilities
    fn initialize_feature_registry(&mut self) {
        // Register features based on capabilities
        self.feature_registry.insert(
            Feature::Clipboard,
            self.capabilities.gui_framework.is_some()
        );
        
        self.feature_registry.insert(
            Feature::FileTransfer,
            true // Always available
        );
        
        self.feature_registry.insert(
            Feature::Streaming,
            self.capabilities.hardware_acceleration.len() > 0
        );
        
        self.feature_registry.insert(
            Feature::CommandExecution,
            self.capabilities.gui_framework != Some(crate::platform::GUIFramework::Web)
        );
        
        self.feature_registry.insert(
            Feature::Discovery,
            self.capabilities.network_features.mdns || 
            self.capabilities.network_features.bluetooth
        );
        
        self.feature_registry.insert(
            Feature::SystemTray,
            self.capabilities.system_tray
        );
        
        self.feature_registry.insert(
            Feature::Notifications,
            self.capabilities.notifications
        );
        
        self.feature_registry.insert(
            Feature::AutoStart,
            self.capabilities.auto_start
        );
        
        self.feature_registry.insert(
            Feature::FileAssociations,
            self.capabilities.file_associations
        );
    }

    /// Check if a feature is available
    pub fn is_feature_available(&self, feature: Feature) -> bool {
        self.feature_registry.get(&feature).copied().unwrap_or(false)
    }

    /// Get all available features
    pub fn available_features(&self) -> Vec<Feature> {
        self.feature_registry
            .iter()
            .filter(|(_, available)| **available)
            .map(|(feature, _)| *feature)
            .collect()
    }

    /// Get platform capabilities
    pub fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }

    /// Enable a feature (if supported by platform)
    pub fn enable_feature(&mut self, feature: Feature) -> PlatformResult<()> {
        if !self.can_enable_feature(feature) {
            return Err(PlatformError::FeatureUnavailable(
                format!("{:?} is not supported on this platform", feature)
            ));
        }
        
        self.feature_registry.insert(feature, true);
        Ok(())
    }

    /// Disable a feature
    pub fn disable_feature(&mut self, feature: Feature) {
        self.feature_registry.insert(feature, false);
    }

    /// Check if a feature can be enabled on this platform
    fn can_enable_feature(&self, feature: Feature) -> bool {
        match feature {
            Feature::Clipboard => self.capabilities.gui_framework.is_some(),
            Feature::FileTransfer => true,
            Feature::Streaming => !self.capabilities.hardware_acceleration.is_empty(),
            Feature::CommandExecution => {
                self.capabilities.gui_framework != Some(crate::platform::GUIFramework::Web)
            }
            Feature::Discovery => {
                self.capabilities.network_features.mdns || 
                self.capabilities.network_features.bluetooth
            }
            Feature::SystemTray => self.capabilities.system_tray,
            Feature::Notifications => self.capabilities.notifications,
            Feature::AutoStart => self.capabilities.auto_start,
            Feature::FileAssociations => self.capabilities.file_associations,
        }
    }

    /// Get graceful degradation options for a feature
    pub fn get_fallback_options(&self, feature: Feature) -> Vec<FallbackOption> {
        match feature {
            Feature::Clipboard => {
                let mut options = Vec::new();
                if self.capabilities.network_features.websocket {
                    options.push(FallbackOption {
                        name: "network-based clipboard sync".to_string(),
                        description: "Use network synchronization instead of native clipboard".to_string(),
                        performance_impact: PerformanceImpact::Medium,
                    });
                }
                options
            }
            Feature::Discovery => {
                let mut options = Vec::new();
                if self.capabilities.network_features.tcp {
                    options.push(FallbackOption {
                        name: "TCP-based discovery".to_string(),
                        description: "Use TCP port scanning for peer discovery".to_string(),
                        performance_impact: PerformanceImpact::Low,
                    });
                }
                if self.capabilities.network_features.udp {
                    options.push(FallbackOption {
                        name: "UDP broadcast discovery".to_string(),
                        description: "Use UDP broadcasts for peer discovery".to_string(),
                        performance_impact: PerformanceImpact::Low,
                    });
                }
                options
            }
            Feature::Streaming => {
                vec![FallbackOption {
                    name: "software encoding".to_string(),
                    description: "Use CPU-based video encoding instead of hardware acceleration".to_string(),
                    performance_impact: PerformanceImpact::High,
                }]
            }
            Feature::SystemTray => {
                let mut options = Vec::new();
                if self.capabilities.notifications {
                    options.push(FallbackOption {
                        name: "notification-based status".to_string(),
                        description: "Use notifications for status updates instead of system tray".to_string(),
                        performance_impact: PerformanceImpact::Low,
                    });
                }
                options
            }
            Feature::CommandExecution => {
                vec![FallbackOption {
                    name: "restricted execution".to_string(),
                    description: "Execute commands with restricted permissions".to_string(),
                    performance_impact: PerformanceImpact::Low,
                }]
            }
            _ => vec![],
        }
    }
    
    /// Apply graceful degradation for unavailable features
    pub fn apply_graceful_degradation(&mut self, feature: Feature) -> PlatformResult<()> {
        if self.is_feature_available(feature) {
            return Ok(());
        }
        
        let fallbacks = self.get_fallback_options(feature);
        if fallbacks.is_empty() {
            return Err(PlatformError::FeatureUnavailable(
                format!("{:?} has no fallback options on this platform", feature)
            ));
        }
        
        // Enable the feature with degraded functionality
        self.feature_registry.insert(feature, true);
        Ok(())
    }
    
    /// Get recommended optimizations for this platform
    pub fn get_platform_optimizations(&self) -> Vec<PlatformOptimization> {
        let mut optimizations = Vec::new();
        
        // Hardware acceleration optimizations
        if self.capabilities.hardware_acceleration.contains(&crate::platform::HardwareFeature::SIMD) {
            optimizations.push(PlatformOptimization {
                name: "SIMD vectorization".to_string(),
                category: OptimizationCategory::CPU,
                enabled: true,
                impact: PerformanceImpact::Medium,
            });
        }
        
        if self.capabilities.hardware_acceleration.contains(&crate::platform::HardwareFeature::GPU) {
            optimizations.push(PlatformOptimization {
                name: "GPU acceleration".to_string(),
                category: OptimizationCategory::GPU,
                enabled: true,
                impact: PerformanceImpact::High,
            });
        }
        
        if self.capabilities.hardware_acceleration.contains(&crate::platform::HardwareFeature::Crypto) {
            optimizations.push(PlatformOptimization {
                name: "Hardware crypto".to_string(),
                category: OptimizationCategory::Security,
                enabled: true,
                impact: PerformanceImpact::Medium,
            });
        }
        
        // Network optimizations
        if self.capabilities.network_features.quic {
            optimizations.push(PlatformOptimization {
                name: "QUIC protocol".to_string(),
                category: OptimizationCategory::Network,
                enabled: true,
                impact: PerformanceImpact::Medium,
            });
        }
        
        if self.capabilities.network_features.webrtc {
            optimizations.push(PlatformOptimization {
                name: "WebRTC data channels".to_string(),
                category: OptimizationCategory::Network,
                enabled: true,
                impact: PerformanceImpact::Low,
            });
        }
        
        // Security optimizations
        if self.capabilities.security_features.keychain {
            optimizations.push(PlatformOptimization {
                name: "System keychain".to_string(),
                category: OptimizationCategory::Security,
                enabled: true,
                impact: PerformanceImpact::Low,
            });
        }
        
        if self.capabilities.security_features.secure_enclave {
            optimizations.push(PlatformOptimization {
                name: "Secure enclave".to_string(),
                category: OptimizationCategory::Security,
                enabled: true,
                impact: PerformanceImpact::Low,
            });
        }
        
        optimizations
    }
    
    /// Select best optimization strategy for current platform
    pub fn select_optimization_strategy(&self) -> OptimizationStrategy {
        let optimizations = self.get_platform_optimizations();
        
        OptimizationStrategy {
            cpu_optimizations: optimizations.iter()
                .filter(|o| o.category == OptimizationCategory::CPU)
                .cloned()
                .collect(),
            gpu_optimizations: optimizations.iter()
                .filter(|o| o.category == OptimizationCategory::GPU)
                .cloned()
                .collect(),
            network_optimizations: optimizations.iter()
                .filter(|o| o.category == OptimizationCategory::Network)
                .cloned()
                .collect(),
            security_optimizations: optimizations.iter()
                .filter(|o| o.category == OptimizationCategory::Security)
                .cloned()
                .collect(),
        }
    }
}

/// Fallback option for unavailable features
#[derive(Debug, Clone)]
pub struct FallbackOption {
    pub name: String,
    pub description: String,
    pub performance_impact: PerformanceImpact,
}

/// Performance impact level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceImpact {
    Low,
    Medium,
    High,
}

/// Platform optimization
#[derive(Debug, Clone)]
pub struct PlatformOptimization {
    pub name: String,
    pub category: OptimizationCategory,
    pub enabled: bool,
    pub impact: PerformanceImpact,
}

/// Optimization category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationCategory {
    CPU,
    GPU,
    Network,
    Security,
    Memory,
}

/// Optimization strategy
#[derive(Debug, Clone)]
pub struct OptimizationStrategy {
    pub cpu_optimizations: Vec<PlatformOptimization>,
    pub gpu_optimizations: Vec<PlatformOptimization>,
    pub network_optimizations: Vec<PlatformOptimization>,
    pub security_optimizations: Vec<PlatformOptimization>,
}

/// Get default capabilities for a platform
pub fn default_capabilities_for_platform(
    os: OperatingSystem,
    arch: Architecture,
) -> PlatformResult<PlatformCapabilities> {
    crate::platform::detection::detect_capabilities(&os, &arch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_manager_creation() {
        let caps = PlatformCapabilities::default();
        let manager = CapabilityManager::new(caps);
        
        // File transfer should always be available
        assert!(manager.is_feature_available(Feature::FileTransfer));
    }

    #[test]
    fn test_feature_availability() {
        let mut caps = PlatformCapabilities::default();
        caps.system_tray = true;
        
        let manager = CapabilityManager::new(caps);
        assert!(manager.is_feature_available(Feature::SystemTray));
    }

    #[test]
    fn test_enable_disable_feature() {
        let mut caps = PlatformCapabilities::default();
        caps.notifications = true;
        
        let mut manager = CapabilityManager::new(caps);
        
        // Should be able to enable notifications
        assert!(manager.enable_feature(Feature::Notifications).is_ok());
        assert!(manager.is_feature_available(Feature::Notifications));
        
        // Disable it
        manager.disable_feature(Feature::Notifications);
        assert!(!manager.is_feature_available(Feature::Notifications));
    }

    #[test]
    fn test_fallback_options() {
        let mut caps = PlatformCapabilities::default();
        caps.network_features.tcp = true;
        caps.network_features.udp = true;
        
        let manager = CapabilityManager::new(caps);
        let fallbacks = manager.get_fallback_options(Feature::Discovery);
        
        assert!(!fallbacks.is_empty());
        assert!(fallbacks.iter().any(|f| f.name.contains("TCP")));
    }
    
    #[test]
    fn test_graceful_degradation() {
        let mut caps = PlatformCapabilities::default();
        caps.network_features.websocket = true;
        caps.system_tray = false;
        
        let mut manager = CapabilityManager::new(caps);
        
        // System tray not available, but has fallback
        assert!(!manager.is_feature_available(Feature::SystemTray));
        
        // Apply graceful degradation
        let result = manager.apply_graceful_degradation(Feature::SystemTray);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_platform_optimizations() {
        let mut caps = PlatformCapabilities::default();
        caps.hardware_acceleration.insert(crate::platform::HardwareFeature::SIMD);
        caps.hardware_acceleration.insert(crate::platform::HardwareFeature::GPU);
        
        let manager = CapabilityManager::new(caps);
        let optimizations = manager.get_platform_optimizations();
        
        assert!(!optimizations.is_empty());
        assert!(optimizations.iter().any(|o| o.category == OptimizationCategory::CPU));
        assert!(optimizations.iter().any(|o| o.category == OptimizationCategory::GPU));
    }
    
    #[test]
    fn test_optimization_strategy() {
        let mut caps = PlatformCapabilities::default();
        caps.hardware_acceleration.insert(crate::platform::HardwareFeature::SIMD);
        caps.network_features.quic = true;
        
        let manager = CapabilityManager::new(caps);
        let strategy = manager.select_optimization_strategy();
        
        assert!(!strategy.cpu_optimizations.is_empty());
        assert!(!strategy.network_optimizations.is_empty());
    }
}
