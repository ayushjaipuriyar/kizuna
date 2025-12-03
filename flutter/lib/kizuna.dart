/// Kizuna Flutter Plugin
/// 
/// A comprehensive peer-to-peer connectivity solution for Flutter applications.
/// Provides seamless file sharing, screen streaming, and device communication
/// across multiple platforms.
library kizuna;

// Export all public APIs
export 'src/kizuna_api.dart';
export 'src/models.dart';
export 'src/platform_info.dart';
export 'src/platform_optimizations.dart';

/// Version information
const String kizunaVersion = '0.1.0';

/// Supported platforms
const List<String> supportedPlatforms = [
  'android',
  'ios',
  'windows',
  'macos',
  'linux',
];

/// Feature flags
class KizunaFeatures {
  /// Whether screen streaming is supported on this platform
  static bool get supportsScreenStreaming {
    // Screen streaming is only supported on desktop platforms
    return ['windows', 'macos', 'linux'].contains(_currentPlatform);
  }
  
  /// Whether camera streaming is supported on this platform
  static bool get supportsCameraStreaming {
    // Camera streaming is supported on all platforms
    return true;
  }
  
  /// Whether Bluetooth discovery is supported on this platform
  static bool get supportsBluetoothDiscovery {
    // Bluetooth is supported on all platforms
    return true;
  }
  
  /// Whether mDNS discovery is supported on this platform
  static bool get supportsMdnsDiscovery {
    // mDNS is supported on all platforms
    return true;
  }
  
  /// Whether background execution is supported on this platform
  static bool get supportsBackgroundExecution {
    // Background execution is supported on mobile and desktop
    return ['android', 'ios', 'windows', 'macos', 'linux'].contains(_currentPlatform);
  }
  
  static String get _currentPlatform {
    // This would be determined at runtime
    return 'unknown';
  }
}
