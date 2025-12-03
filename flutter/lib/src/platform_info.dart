/// Platform information and feature detection
/// 
/// This file provides utilities for detecting the current platform
/// and checking which features are supported.

library platform_info;

/// Platform information
class PlatformInfo {
  /// Platform name (android, ios, windows, macos, linux, web)
  final String platform;
  
  /// Platform version
  final String version;
  
  /// Supported features on this platform
  final List<String> supportedFeatures;
  
  PlatformInfo({
    required this.platform,
    required this.version,
    required this.supportedFeatures,
  });
  
  /// Gets the current platform information
  static PlatformInfo getCurrentPlatform() {
    throw UnimplementedError('This will be implemented by Flutter Rust Bridge');
  }
  
  /// Checks if a feature is supported on the current platform
  bool isFeatureSupported(String feature) {
    return supportedFeatures.contains(feature);
  }
}
