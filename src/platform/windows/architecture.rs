// Windows architecture support for x64 and ARM64

use crate::platform::{PlatformResult, PlatformError};

#[cfg(windows)]
use winapi::um::{
    sysinfoapi::{GetSystemInfo, SYSTEM_INFO},
    winnt::{PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM64, PROCESSOR_ARCHITECTURE_INTEL},
};

/// Windows architecture information
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowsArchitecture {
    X64,
    ARM64,
    X86,
    Unknown,
}

impl WindowsArchitecture {
    /// Detect current Windows architecture
    pub fn detect() -> PlatformResult<Self> {
        #[cfg(windows)]
        {
            unsafe {
                let mut system_info: SYSTEM_INFO = std::mem::zeroed();
                GetSystemInfo(&mut system_info);
                
                let arch = match system_info.u.s().wProcessorArchitecture {
                    PROCESSOR_ARCHITECTURE_AMD64 => WindowsArchitecture::X64,
                    PROCESSOR_ARCHITECTURE_ARM64 => WindowsArchitecture::ARM64,
                    PROCESSOR_ARCHITECTURE_INTEL => WindowsArchitecture::X86,
                    _ => WindowsArchitecture::Unknown,
                };
                
                Ok(arch)
            }
        }
        
        #[cfg(not(windows))]
        {
            Err(PlatformError::UnsupportedPlatform("Not on Windows".to_string()))
        }
    }

    /// Get architecture name as string
    pub fn as_str(&self) -> &str {
        match self {
            WindowsArchitecture::X64 => "x64",
            WindowsArchitecture::ARM64 => "ARM64",
            WindowsArchitecture::X86 => "x86",
            WindowsArchitecture::Unknown => "unknown",
        }
    }

    /// Check if architecture supports specific features
    pub fn supports_feature(&self, feature: ArchitectureFeature) -> bool {
        match feature {
            ArchitectureFeature::SIMD => matches!(self, WindowsArchitecture::X64 | WindowsArchitecture::ARM64),
            ArchitectureFeature::AVX => matches!(self, WindowsArchitecture::X64),
            ArchitectureFeature::NEON => matches!(self, WindowsArchitecture::ARM64),
            ArchitectureFeature::HardwareAES => matches!(self, WindowsArchitecture::X64 | WindowsArchitecture::ARM64),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchitectureFeature {
    SIMD,
    AVX,
    NEON,
    HardwareAES,
}

/// Architecture-specific optimization manager
pub struct ArchitectureOptimizer {
    architecture: WindowsArchitecture,
}

impl ArchitectureOptimizer {
    pub fn new() -> PlatformResult<Self> {
        let architecture = WindowsArchitecture::detect()?;
        Ok(Self { architecture })
    }

    /// Get current architecture
    pub fn architecture(&self) -> &WindowsArchitecture {
        &self.architecture
    }

    /// Apply architecture-specific optimizations
    pub fn apply_optimizations(&self) -> PlatformResult<OptimizationConfig> {
        let mut config = OptimizationConfig::default();
        
        match self.architecture {
            WindowsArchitecture::X64 => {
                config.use_simd = true;
                config.use_avx = true;
                config.use_hardware_aes = true;
                config.thread_pool_size = num_cpus::get();
            }
            WindowsArchitecture::ARM64 => {
                config.use_simd = true;
                config.use_neon = true;
                config.use_hardware_aes = true;
                config.thread_pool_size = num_cpus::get();
                config.power_efficient = true;
            }
            WindowsArchitecture::X86 => {
                config.use_simd = false;
                config.thread_pool_size = num_cpus::get().min(4);
            }
            WindowsArchitecture::Unknown => {
                config.thread_pool_size = 2;
            }
        }
        
        Ok(config)
    }

    /// Get recommended buffer sizes for architecture
    pub fn get_buffer_sizes(&self) -> BufferSizes {
        match self.architecture {
            WindowsArchitecture::X64 => BufferSizes {
                network_buffer: 65536,
                file_buffer: 131072,
                crypto_buffer: 16384,
            },
            WindowsArchitecture::ARM64 => BufferSizes {
                network_buffer: 32768,
                file_buffer: 65536,
                crypto_buffer: 8192,
            },
            _ => BufferSizes {
                network_buffer: 16384,
                file_buffer: 32768,
                crypto_buffer: 4096,
            },
        }
    }

    /// Get CPU feature flags
    #[cfg(windows)]
    pub fn get_cpu_features(&self) -> Vec<String> {
        let mut features = Vec::new();
        
        if self.architecture.supports_feature(ArchitectureFeature::SIMD) {
            features.push("SIMD".to_string());
        }
        if self.architecture.supports_feature(ArchitectureFeature::AVX) {
            features.push("AVX".to_string());
        }
        if self.architecture.supports_feature(ArchitectureFeature::NEON) {
            features.push("NEON".to_string());
        }
        if self.architecture.supports_feature(ArchitectureFeature::HardwareAES) {
            features.push("AES-NI".to_string());
        }
        
        features
    }
}

impl Default for ArchitectureOptimizer {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            architecture: WindowsArchitecture::Unknown,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    pub use_simd: bool,
    pub use_avx: bool,
    pub use_neon: bool,
    pub use_hardware_aes: bool,
    pub thread_pool_size: usize,
    pub power_efficient: bool,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            use_simd: false,
            use_avx: false,
            use_neon: false,
            use_hardware_aes: false,
            thread_pool_size: 2,
            power_efficient: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BufferSizes {
    pub network_buffer: usize,
    pub file_buffer: usize,
    pub crypto_buffer: usize,
}

// Helper function to get number of CPUs
mod num_cpus {
    pub fn get() -> usize {
        #[cfg(windows)]
        {
            unsafe {
                use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};
                let mut system_info: SYSTEM_INFO = std::mem::zeroed();
                GetSystemInfo(&mut system_info);
                system_info.dwNumberOfProcessors as usize
            }
        }
        
        #[cfg(not(windows))]
        {
            4 // Default fallback
        }
    }
}
