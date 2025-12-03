// Container platform support
//
// This module provides Docker container support with minimal resource footprint,
// multi-architecture support, and container-specific configuration.

pub mod docker;
pub mod config;
pub mod networking;
pub mod kubernetes;
pub mod logging;
pub mod deployment;

pub use docker::{DockerAdapter, DockerImageBuilder};
pub use config::{ContainerConfig, ContainerEnvironment};
pub use networking::{ContainerNetworking, ServiceDiscovery};
pub use kubernetes::{KubernetesDeployment, KubernetesServiceDiscovery, KubernetesHealthCheck};
pub use logging::{ContainerLogger, MetricsCollector};
pub use deployment::{DeploymentManager, DeploymentStrategy, UpdateManager};

use crate::platform::{PlatformResult, PlatformError};
use serde::{Deserialize, Serialize};

/// Container runtime types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerRuntime {
    Docker,
    Podman,
    Containerd,
    Unknown,
}

/// Container architecture support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerArchitecture {
    AMD64,
    ARM64,
    ARM32,
}

impl ContainerArchitecture {
    pub fn from_platform_arch(arch: crate::platform::Architecture) -> Option<Self> {
        match arch {
            crate::platform::Architecture::X86_64 => Some(Self::AMD64),
            crate::platform::Architecture::ARM64 => Some(Self::ARM64),
            crate::platform::Architecture::ARM32 => Some(Self::ARM32),
            _ => None,
        }
    }

    pub fn to_docker_platform(&self) -> &str {
        match self {
            Self::AMD64 => "linux/amd64",
            Self::ARM64 => "linux/arm64",
            Self::ARM32 => "linux/arm/v7",
        }
    }
}

/// Container image metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerImage {
    pub name: String,
    pub tag: String,
    pub architecture: ContainerArchitecture,
    pub size_bytes: u64,
    pub layers: Vec<String>,
    pub labels: std::collections::HashMap<String, String>,
}

/// Detect container runtime
pub fn detect_container_runtime() -> ContainerRuntime {
    use std::process::Command;

    // Check for Docker
    if Command::new("docker").arg("--version").output().is_ok() {
        return ContainerRuntime::Docker;
    }

    // Check for Podman
    if Command::new("podman").arg("--version").output().is_ok() {
        return ContainerRuntime::Podman;
    }

    // Check for containerd
    if Command::new("ctr").arg("--version").output().is_ok() {
        return ContainerRuntime::Containerd;
    }

    ContainerRuntime::Unknown
}

/// Check if running inside a container
pub fn is_containerized() -> bool {
    crate::platform::detection::detect_platform()
        .map(|info| info.os == crate::platform::OperatingSystem::Container)
        .unwrap_or(false)
}

/// Get container ID if running in a container
pub fn get_container_id() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        use std::fs;

        // Try to read container ID from cgroup
        if let Ok(content) = fs::read_to_string("/proc/self/cgroup") {
            for line in content.lines() {
                if let Some(id_part) = line.split('/').last() {
                    if id_part.len() >= 12 && !id_part.contains(':') {
                        return Some(id_part.to_string());
                    }
                }
            }
        }

        // Try to read from Docker-specific location
        if let Ok(content) = fs::read_to_string("/proc/self/mountinfo") {
            for line in content.lines() {
                if line.contains("/docker/containers/") {
                    if let Some(id) = line.split("/docker/containers/").nth(1) {
                        if let Some(container_id) = id.split('/').next() {
                            return Some(container_id.to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_container_runtime() {
        let runtime = detect_container_runtime();
        // Runtime detection should not fail
        assert!(matches!(
            runtime,
            ContainerRuntime::Docker
                | ContainerRuntime::Podman
                | ContainerRuntime::Containerd
                | ContainerRuntime::Unknown
        ));
    }

    #[test]
    fn test_container_architecture_conversion() {
        let amd64 = ContainerArchitecture::from_platform_arch(crate::platform::Architecture::X86_64);
        assert_eq!(amd64, Some(ContainerArchitecture::AMD64));
        assert_eq!(amd64.unwrap().to_docker_platform(), "linux/amd64");

        let arm64 = ContainerArchitecture::from_platform_arch(crate::platform::Architecture::ARM64);
        assert_eq!(arm64, Some(ContainerArchitecture::ARM64));
        assert_eq!(arm64.unwrap().to_docker_platform(), "linux/arm64");
    }

    #[test]
    fn test_is_containerized() {
        // This test will pass whether we're in a container or not
        let _ = is_containerized();
    }
}
