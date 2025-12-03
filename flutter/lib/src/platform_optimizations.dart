/// Platform-specific optimizations
/// 
/// This file provides utilities for getting platform-specific
/// optimization recommendations.

library platform_optimizations;

/// Platform-specific optimizations
class PlatformOptimizations {
  /// Gets recommended buffer size for the platform
  static int getRecommendedBufferSize() {
    throw UnimplementedError('This will be implemented by Flutter Rust Bridge');
  }
  
  /// Gets recommended concurrent transfer limit for the platform
  static int getMaxConcurrentTransfers() {
    throw UnimplementedError('This will be implemented by Flutter Rust Bridge');
  }
  
  /// Checks if background execution is supported
  static bool supportsBackgroundExecution() {
    throw UnimplementedError('This will be implemented by Flutter Rust Bridge');
  }
  
  /// Gets platform-specific network preferences
  static NetworkPreferences getNetworkPreferences() {
    throw UnimplementedError('This will be implemented by Flutter Rust Bridge');
  }
}

/// Network preferences for platform-specific optimization
class NetworkPreferences {
  /// Prefer WiFi over other network types
  final bool preferWifi;
  
  /// Allow cellular data usage
  final bool allowCellular;
  
  /// Prefer low latency over bandwidth
  final bool preferLowLatency;
  
  /// Prefer low power consumption
  final bool preferLowPower;
  
  NetworkPreferences({
    required this.preferWifi,
    required this.allowCellular,
    required this.preferLowLatency,
    required this.preferLowPower,
  });
}

/// Flutter plugin configuration
class FlutterPluginConfig {
  /// Enable native code integration
  final bool enableNativeIntegration;
  
  /// Enable platform channels
  final bool enablePlatformChannels;
  
  /// Enable method channel for custom communication
  final bool enableMethodChannel;
  
  /// Enable event channel for streaming events
  final bool enableEventChannel;
  
  FlutterPluginConfig({
    required this.enableNativeIntegration,
    required this.enablePlatformChannels,
    required this.enableMethodChannel,
    required this.enableEventChannel,
  });
  
  /// Creates default Flutter plugin configuration
  static FlutterPluginConfig defaultConfig() {
    return FlutterPluginConfig(
      enableNativeIntegration: true,
      enablePlatformChannels: true,
      enableMethodChannel: true,
      enableEventChannel: true,
    );
  }
  
  /// Creates minimal Flutter plugin configuration
  static FlutterPluginConfig minimalConfig() {
    return FlutterPluginConfig(
      enableNativeIntegration: true,
      enablePlatformChannels: false,
      enableMethodChannel: false,
      enableEventChannel: false,
    );
  }
}
