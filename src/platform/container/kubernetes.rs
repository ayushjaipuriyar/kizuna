// Kubernetes orchestration support
//
// Provides Kubernetes deployment configurations, manifests, service discovery,
// and health check integration.

use crate::platform::{PlatformResult, PlatformError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kubernetes deployment generator
pub struct KubernetesDeployment {
    name: String,
    namespace: String,
    replicas: u32,
    image: String,
    image_pull_policy: ImagePullPolicy,
    ports: Vec<ContainerPort>,
    environment: HashMap<String, String>,
    resources: ResourceRequirements,
    health_checks: HealthChecks,
    labels: HashMap<String, String>,
    annotations: HashMap<String, String>,
}

impl KubernetesDeployment {
    pub fn new(name: String, image: String) -> Self {
        Self {
            name: name.clone(),
            namespace: "default".to_string(),
            replicas: 1,
            image,
            image_pull_policy: ImagePullPolicy::IfNotPresent,
            ports: vec![],
            environment: HashMap::new(),
            resources: ResourceRequirements::default(),
            health_checks: HealthChecks::default(),
            labels: HashMap::from([("app".to_string(), name)]),
            annotations: HashMap::new(),
        }
    }

    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = namespace;
        self
    }

    pub fn with_replicas(mut self, replicas: u32) -> Self {
        self.replicas = replicas;
        self
    }

    pub fn with_port(mut self, name: String, port: u16, protocol: Protocol) -> Self {
        self.ports.push(ContainerPort {
            name,
            container_port: port,
            protocol,
        });
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.environment.insert(key, value);
        self
    }

    pub fn with_resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = resources;
        self
    }

    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }

    /// Generate Kubernetes deployment YAML
    pub fn generate_deployment_yaml(&self) -> String {
        let mut yaml = String::new();

        yaml.push_str("apiVersion: apps/v1\n");
        yaml.push_str("kind: Deployment\n");
        yaml.push_str("metadata:\n");
        yaml.push_str(&format!("  name: {}\n", self.name));
        yaml.push_str(&format!("  namespace: {}\n", self.namespace));

        // Labels
        if !self.labels.is_empty() {
            yaml.push_str("  labels:\n");
            for (key, value) in &self.labels {
                yaml.push_str(&format!("    {}: {}\n", key, value));
            }
        }

        // Annotations
        if !self.annotations.is_empty() {
            yaml.push_str("  annotations:\n");
            for (key, value) in &self.annotations {
                yaml.push_str(&format!("    {}: {}\n", key, value));
            }
        }

        yaml.push_str("spec:\n");
        yaml.push_str(&format!("  replicas: {}\n", self.replicas));
        yaml.push_str("  selector:\n");
        yaml.push_str("    matchLabels:\n");
        for (key, value) in &self.labels {
            yaml.push_str(&format!("      {}: {}\n", key, value));
        }

        yaml.push_str("  template:\n");
        yaml.push_str("    metadata:\n");
        yaml.push_str("      labels:\n");
        for (key, value) in &self.labels {
            yaml.push_str(&format!("        {}: {}\n", key, value));
        }

        yaml.push_str("    spec:\n");
        yaml.push_str("      containers:\n");
        yaml.push_str(&format!("      - name: {}\n", self.name));
        yaml.push_str(&format!("        image: {}\n", self.image));
        yaml.push_str(&format!("        imagePullPolicy: {}\n", self.image_pull_policy.as_str()));

        // Ports
        if !self.ports.is_empty() {
            yaml.push_str("        ports:\n");
            for port in &self.ports {
                yaml.push_str(&format!("        - name: {}\n", port.name));
                yaml.push_str(&format!("          containerPort: {}\n", port.container_port));
                yaml.push_str(&format!("          protocol: {}\n", port.protocol.as_str()));
            }
        }

        // Environment variables
        if !self.environment.is_empty() {
            yaml.push_str("        env:\n");
            for (key, value) in &self.environment {
                yaml.push_str(&format!("        - name: {}\n", key));
                yaml.push_str(&format!("          value: \"{}\"\n", value));
            }
        }

        // Resources
        yaml.push_str("        resources:\n");
        yaml.push_str("          requests:\n");
        yaml.push_str(&format!("            cpu: {}\n", self.resources.requests.cpu));
        yaml.push_str(&format!("            memory: {}\n", self.resources.requests.memory));
        yaml.push_str("          limits:\n");
        yaml.push_str(&format!("            cpu: {}\n", self.resources.limits.cpu));
        yaml.push_str(&format!("            memory: {}\n", self.resources.limits.memory));

        // Liveness probe
        yaml.push_str("        livenessProbe:\n");
        yaml.push_str(&format!("          httpGet:\n"));
        yaml.push_str(&format!("            path: {}\n", self.health_checks.liveness_path));
        yaml.push_str(&format!("            port: {}\n", self.health_checks.port));
        yaml.push_str(&format!("          initialDelaySeconds: {}\n", self.health_checks.initial_delay_seconds));
        yaml.push_str(&format!("          periodSeconds: {}\n", self.health_checks.period_seconds));
        yaml.push_str(&format!("          timeoutSeconds: {}\n", self.health_checks.timeout_seconds));
        yaml.push_str(&format!("          failureThreshold: {}\n", self.health_checks.failure_threshold));

        // Readiness probe
        yaml.push_str("        readinessProbe:\n");
        yaml.push_str(&format!("          httpGet:\n"));
        yaml.push_str(&format!("            path: {}\n", self.health_checks.readiness_path));
        yaml.push_str(&format!("            port: {}\n", self.health_checks.port));
        yaml.push_str(&format!("          initialDelaySeconds: {}\n", self.health_checks.initial_delay_seconds));
        yaml.push_str(&format!("          periodSeconds: {}\n", self.health_checks.period_seconds));
        yaml.push_str(&format!("          timeoutSeconds: {}\n", self.health_checks.timeout_seconds));

        yaml
    }

    /// Generate Kubernetes service YAML
    pub fn generate_service_yaml(&self) -> String {
        let mut yaml = String::new();

        yaml.push_str("apiVersion: v1\n");
        yaml.push_str("kind: Service\n");
        yaml.push_str("metadata:\n");
        yaml.push_str(&format!("  name: {}\n", self.name));
        yaml.push_str(&format!("  namespace: {}\n", self.namespace));
        yaml.push_str("  labels:\n");
        for (key, value) in &self.labels {
            yaml.push_str(&format!("    {}: {}\n", key, value));
        }

        yaml.push_str("spec:\n");
        yaml.push_str("  type: ClusterIP\n");
        yaml.push_str("  selector:\n");
        for (key, value) in &self.labels {
            yaml.push_str(&format!("    {}: {}\n", key, value));
        }

        if !self.ports.is_empty() {
            yaml.push_str("  ports:\n");
            for port in &self.ports {
                yaml.push_str(&format!("  - name: {}\n", port.name));
                yaml.push_str(&format!("    port: {}\n", port.container_port));
                yaml.push_str(&format!("    targetPort: {}\n", port.container_port));
                yaml.push_str(&format!("    protocol: {}\n", port.protocol.as_str()));
            }
        }

        yaml
    }

    /// Generate ConfigMap YAML for configuration
    pub fn generate_configmap_yaml(&self, config_data: HashMap<String, String>) -> String {
        let mut yaml = String::new();

        yaml.push_str("apiVersion: v1\n");
        yaml.push_str("kind: ConfigMap\n");
        yaml.push_str("metadata:\n");
        yaml.push_str(&format!("  name: {}-config\n", self.name));
        yaml.push_str(&format!("  namespace: {}\n", self.namespace));

        yaml.push_str("data:\n");
        for (key, value) in config_data {
            yaml.push_str(&format!("  {}: |\n", key));
            for line in value.lines() {
                yaml.push_str(&format!("    {}\n", line));
            }
        }

        yaml
    }
}

/// Image pull policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImagePullPolicy {
    Always,
    IfNotPresent,
    Never,
}

impl ImagePullPolicy {
    fn as_str(&self) -> &str {
        match self {
            Self::Always => "Always",
            Self::IfNotPresent => "IfNotPresent",
            Self::Never => "Never",
        }
    }
}

/// Container port configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerPort {
    pub name: String,
    pub container_port: u16,
    pub protocol: Protocol,
}

/// Network protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    TCP,
    UDP,
}

impl Protocol {
    fn as_str(&self) -> &str {
        match self {
            Self::TCP => "TCP",
            Self::UDP => "UDP",
        }
    }
}

/// Resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub requests: ResourceSpec,
    pub limits: ResourceSpec,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            requests: ResourceSpec {
                cpu: "250m".to_string(),
                memory: "128Mi".to_string(),
            },
            limits: ResourceSpec {
                cpu: "1000m".to_string(),
                memory: "512Mi".to_string(),
            },
        }
    }
}

/// Resource specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSpec {
    pub cpu: String,
    pub memory: String,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthChecks {
    pub liveness_path: String,
    pub readiness_path: String,
    pub port: u16,
    pub initial_delay_seconds: u32,
    pub period_seconds: u32,
    pub timeout_seconds: u32,
    pub failure_threshold: u32,
}

impl Default for HealthChecks {
    fn default() -> Self {
        Self {
            liveness_path: "/health/live".to_string(),
            readiness_path: "/health/ready".to_string(),
            port: 8080,
            initial_delay_seconds: 10,
            period_seconds: 30,
            timeout_seconds: 3,
            failure_threshold: 3,
        }
    }
}

/// Kubernetes service discovery client
pub struct KubernetesServiceDiscovery {
    namespace: String,
}

impl KubernetesServiceDiscovery {
    pub fn new(namespace: String) -> Self {
        Self { namespace }
    }

    pub fn default() -> Self {
        Self::new("default".to_string())
    }

    /// Check if running in Kubernetes
    pub fn is_kubernetes_environment() -> bool {
        std::env::var("KUBERNETES_SERVICE_HOST").is_ok()
    }

    /// Get service endpoint from environment
    pub fn get_service_endpoint(&self, service_name: &str) -> Option<String> {
        // Kubernetes exposes services via environment variables
        let host_var = format!("{}_SERVICE_HOST", service_name.to_uppercase().replace('-', "_"));
        let port_var = format!("{}_SERVICE_PORT", service_name.to_uppercase().replace('-', "_"));

        let host = std::env::var(&host_var).ok()?;
        let port = std::env::var(&port_var).ok()?;

        Some(format!("{}:{}", host, port))
    }

    /// Get DNS name for service
    pub fn get_service_dns(&self, service_name: &str) -> String {
        format!("{}.{}.svc.cluster.local", service_name, self.namespace)
    }
}

/// Kubernetes health check handler
pub struct KubernetesHealthCheck {
    liveness_checks: Vec<Box<dyn Fn() -> bool + Send + Sync>>,
    readiness_checks: Vec<Box<dyn Fn() -> bool + Send + Sync>>,
}

impl KubernetesHealthCheck {
    pub fn new() -> Self {
        Self {
            liveness_checks: vec![],
            readiness_checks: vec![],
        }
    }

    /// Add liveness check
    pub fn add_liveness_check<F>(&mut self, check: F)
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.liveness_checks.push(Box::new(check));
    }

    /// Add readiness check
    pub fn add_readiness_check<F>(&mut self, check: F)
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.readiness_checks.push(Box::new(check));
    }

    /// Check if service is alive
    pub fn is_alive(&self) -> bool {
        self.liveness_checks.iter().all(|check| check())
    }

    /// Check if service is ready
    pub fn is_ready(&self) -> bool {
        self.readiness_checks.iter().all(|check| check())
    }
}

impl Default for KubernetesHealthCheck {
    fn default() -> Self {
        Self::new()
    }
}

/// Horizontal Pod Autoscaler configuration
pub struct HorizontalPodAutoscaler {
    name: String,
    namespace: String,
    target_deployment: String,
    min_replicas: u32,
    max_replicas: u32,
    target_cpu_utilization: u32,
    target_memory_utilization: Option<u32>,
}

impl HorizontalPodAutoscaler {
    pub fn new(name: String, target_deployment: String) -> Self {
        Self {
            name,
            namespace: "default".to_string(),
            target_deployment,
            min_replicas: 1,
            max_replicas: 10,
            target_cpu_utilization: 80,
            target_memory_utilization: None,
        }
    }

    pub fn with_replicas(mut self, min: u32, max: u32) -> Self {
        self.min_replicas = min;
        self.max_replicas = max;
        self
    }

    pub fn with_cpu_target(mut self, target: u32) -> Self {
        self.target_cpu_utilization = target;
        self
    }

    /// Generate HPA YAML
    pub fn generate_yaml(&self) -> String {
        let mut yaml = String::new();

        yaml.push_str("apiVersion: autoscaling/v2\n");
        yaml.push_str("kind: HorizontalPodAutoscaler\n");
        yaml.push_str("metadata:\n");
        yaml.push_str(&format!("  name: {}\n", self.name));
        yaml.push_str(&format!("  namespace: {}\n", self.namespace));

        yaml.push_str("spec:\n");
        yaml.push_str("  scaleTargetRef:\n");
        yaml.push_str("    apiVersion: apps/v1\n");
        yaml.push_str("    kind: Deployment\n");
        yaml.push_str(&format!("    name: {}\n", self.target_deployment));

        yaml.push_str(&format!("  minReplicas: {}\n", self.min_replicas));
        yaml.push_str(&format!("  maxReplicas: {}\n", self.max_replicas));

        yaml.push_str("  metrics:\n");
        yaml.push_str("  - type: Resource\n");
        yaml.push_str("    resource:\n");
        yaml.push_str("      name: cpu\n");
        yaml.push_str("      target:\n");
        yaml.push_str("        type: Utilization\n");
        yaml.push_str(&format!("        averageUtilization: {}\n", self.target_cpu_utilization));

        if let Some(memory_target) = self.target_memory_utilization {
            yaml.push_str("  - type: Resource\n");
            yaml.push_str("    resource:\n");
            yaml.push_str("      name: memory\n");
            yaml.push_str("      target:\n");
            yaml.push_str("        type: Utilization\n");
            yaml.push_str(&format!("        averageUtilization: {}\n", memory_target));
        }

        yaml
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kubernetes_deployment_creation() {
        let deployment = KubernetesDeployment::new(
            "kizuna".to_string(),
            "kizuna:latest".to_string(),
        );

        assert_eq!(deployment.name, "kizuna");
        assert_eq!(deployment.replicas, 1);
    }

    #[test]
    fn test_deployment_yaml_generation() {
        let deployment = KubernetesDeployment::new(
            "kizuna".to_string(),
            "kizuna:latest".to_string(),
        )
        .with_replicas(3)
        .with_port("http".to_string(), 8080, Protocol::TCP)
        .with_env("LOG_LEVEL".to_string(), "info".to_string());

        let yaml = deployment.generate_deployment_yaml();
        assert!(yaml.contains("kind: Deployment"));
        assert!(yaml.contains("replicas: 3"));
        assert!(yaml.contains("image: kizuna:latest"));
        assert!(yaml.contains("containerPort: 8080"));
        assert!(yaml.contains("LOG_LEVEL"));
    }

    #[test]
    fn test_service_yaml_generation() {
        let deployment = KubernetesDeployment::new(
            "kizuna".to_string(),
            "kizuna:latest".to_string(),
        )
        .with_port("http".to_string(), 8080, Protocol::TCP);

        let yaml = deployment.generate_service_yaml();
        assert!(yaml.contains("kind: Service"));
        assert!(yaml.contains("type: ClusterIP"));
        assert!(yaml.contains("port: 8080"));
    }

    #[test]
    fn test_configmap_generation() {
        let deployment = KubernetesDeployment::new(
            "kizuna".to_string(),
            "kizuna:latest".to_string(),
        );

        let mut config_data = HashMap::new();
        config_data.insert("config.yaml".to_string(), "key: value".to_string());

        let yaml = deployment.generate_configmap_yaml(config_data);
        assert!(yaml.contains("kind: ConfigMap"));
        assert!(yaml.contains("config.yaml"));
    }

    #[test]
    fn test_service_discovery() {
        let discovery = KubernetesServiceDiscovery::new("default".to_string());
        let dns = discovery.get_service_dns("kizuna");
        assert_eq!(dns, "kizuna.default.svc.cluster.local");
    }

    #[test]
    fn test_health_check() {
        let mut health = KubernetesHealthCheck::new();
        
        health.add_liveness_check(|| true);
        health.add_readiness_check(|| true);

        assert!(health.is_alive());
        assert!(health.is_ready());
    }

    #[test]
    fn test_hpa_generation() {
        let hpa = HorizontalPodAutoscaler::new(
            "kizuna-hpa".to_string(),
            "kizuna".to_string(),
        )
        .with_replicas(2, 10)
        .with_cpu_target(75);

        let yaml = hpa.generate_yaml();
        assert!(yaml.contains("kind: HorizontalPodAutoscaler"));
        assert!(yaml.contains("minReplicas: 2"));
        assert!(yaml.contains("maxReplicas: 10"));
        assert!(yaml.contains("averageUtilization: 75"));
    }
}
