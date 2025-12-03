# Container and Deployment Support

This document describes the container and deployment support features in Kizuna.

## Overview

Kizuna provides comprehensive container support including:
- Docker container images with minimal resource footprint
- Multi-architecture support (x64, ARM64)
- Kubernetes orchestration
- Container-specific configuration management
- Logging and monitoring integration
- Deployment strategies and update mechanisms

## Docker Support

### Building Container Images

```rust
use kizuna::platform::container::{DockerImageBuilder, OptimizationLevel};

let builder = DockerImageBuilder::new()
    .with_base_image("alpine:latest".to_string())
    .with_architectures(vec![
        ContainerArchitecture::AMD64,
        ContainerArchitecture::ARM64,
    ])
    .with_optimization(OptimizationLevel::Minimal)
    .with_label("version".to_string(), "1.0.0".to_string());

// Generate Dockerfile
let dockerfile = builder.generate_dockerfile();

// Generate build script for multi-architecture builds
let build_script = builder.generate_build_script();

// Generate .dockerignore
let dockerignore = builder.generate_dockerignore();
```

### Docker Compose

```rust
use kizuna::platform::container::docker::DockerComposeGenerator;

let compose = DockerComposeGenerator::new(
    "kizuna".to_string(),
    "kizuna:latest".to_string(),
)
.with_port(8080, 8080)
.with_env("LOG_LEVEL".to_string(), "info".to_string())
.with_volume("./data".to_string(), "/app/data".to_string());

let compose_yaml = compose.generate();
```

## Kubernetes Support

### Deployment Configuration

```rust
use kizuna::platform::container::{KubernetesDeployment, Protocol};

let deployment = KubernetesDeployment::new(
    "kizuna".to_string(),
    "kizuna:latest".to_string(),
)
.with_namespace("production".to_string())
.with_replicas(3)
.with_port("http".to_string(), 8080, Protocol::TCP)
.with_env("LOG_LEVEL".to_string(), "info".to_string());

// Generate deployment YAML
let deployment_yaml = deployment.generate_deployment_yaml();

// Generate service YAML
let service_yaml = deployment.generate_service_yaml();

// Generate ConfigMap
let mut config_data = HashMap::new();
config_data.insert("config.yaml".to_string(), "key: value".to_string());
let configmap_yaml = deployment.generate_configmap_yaml(config_data);
```

### Service Discovery

```rust
use kizuna::platform::container::KubernetesServiceDiscovery;

let discovery = KubernetesServiceDiscovery::new("default".to_string());

// Check if running in Kubernetes
if KubernetesServiceDiscovery::is_kubernetes_environment() {
    // Get service endpoint from environment
    if let Some(endpoint) = discovery.get_service_endpoint("database") {
        println!("Database endpoint: {}", endpoint);
    }

    // Get DNS name for service
    let dns = discovery.get_service_dns("api");
    println!("API DNS: {}", dns);
}
```

### Health Checks

```rust
use kizuna::platform::container::KubernetesHealthCheck;

let mut health = KubernetesHealthCheck::new();

// Add liveness check
health.add_liveness_check(|| {
    // Check if service is alive
    true
});

// Add readiness check
health.add_readiness_check(|| {
    // Check if service is ready to accept traffic
    true
});

// Check health status
if health.is_alive() && health.is_ready() {
    println!("Service is healthy and ready");
}
```

### Horizontal Pod Autoscaler

```rust
use kizuna::platform::container::kubernetes::HorizontalPodAutoscaler;

let hpa = HorizontalPodAutoscaler::new(
    "kizuna-hpa".to_string(),
    "kizuna".to_string(),
)
.with_replicas(2, 10)
.with_cpu_target(75);

let hpa_yaml = hpa.generate_yaml();
```

## Configuration Management

### Container Configuration

```rust
use kizuna::platform::container::{ContainerConfig, ContainerConfigBuilder};

// Load from environment
let config = ContainerConfig::from_env()?;

// Or build programmatically
let config = ContainerConfigBuilder::new()
    .with_cpu_limit(2.0)
    .with_memory_limit(1024)
    .with_port(9090)
    .with_log_level("debug".to_string())
    .with_env("DATABASE_URL".to_string(), "postgres://...".to_string())
    .build()?;

// Apply configuration
config.apply()?;
```

### Environment Variables

```rust
use kizuna::platform::container::ContainerEnvironment;

let mut env = ContainerEnvironment::from_env()?;

// Set environment variable
env.set("API_KEY".to_string(), "secret".to_string());

// Get environment variable
if let Some(value) = env.get("API_KEY") {
    println!("API Key: {}", value);
}

// Manage secrets
env.set_secret("DB_PASSWORD".to_string(), "password".to_string());
```

## Logging and Monitoring

### Structured Logging

```rust
use kizuna::platform::container::{ContainerLogger, LogConfig, LogFormat, LogOutput};

let config = LogConfig {
    level: "info".to_string(),
    format: LogFormat::Json,
    output: LogOutput::Stdout,
};

let logger = ContainerLogger::new(config);
logger.initialize()?;

// Log messages
logger.info("Application started");
logger.warn("High memory usage detected");
logger.error("Failed to connect to database");

// Log with structured data
let mut fields = HashMap::new();
fields.insert("user_id".to_string(), "123".to_string());
logger.log(LogLevel::Info, "User logged in", fields);
```

### Metrics Collection

```rust
use kizuna::platform::container::MetricsCollector;

let mut metrics = MetricsCollector::new();

// Record metrics
metrics.counter("requests_total".to_string(), 100.0, HashMap::new());
metrics.gauge("memory_usage_mb".to_string(), 512.0, HashMap::new());
metrics.histogram("request_duration_ms".to_string(), 45.0, HashMap::new());

// Export in Prometheus format
let prometheus_metrics = metrics.export_prometheus();
```

## Deployment Strategies

### Rolling Update

```rust
use kizuna::platform::container::{DeploymentManager, DeploymentStrategy, Deployment};

let manager = DeploymentManager::new(
    DeploymentStrategy::RollingUpdate,
    DeploymentConfig::default(),
);

let deployment = Deployment {
    name: "kizuna".to_string(),
    version: "1.1.0".to_string(),
    image: "kizuna:1.1.0".to_string(),
    replicas: 3,
    environment: HashMap::new(),
};

let result = manager.deploy(&deployment).await?;
```

### Blue-Green Deployment

```rust
let manager = DeploymentManager::new(
    DeploymentStrategy::BlueGreen,
    DeploymentConfig::default(),
);

let result = manager.deploy(&deployment).await?;

// Rollback if needed
if !result.success {
    manager.rollback("1.0.0".to_string()).await?;
}
```

### Canary Deployment

```rust
let config = DeploymentConfig {
    canary_percentage: 10,
    ..Default::default()
};

let manager = DeploymentManager::new(
    DeploymentStrategy::Canary,
    config,
);

let result = manager.deploy(&deployment).await?;
```

## Update Management

### Checking for Updates

```rust
use kizuna::platform::container::{UpdateManager, UpdateChannel};

let mut manager = UpdateManager::new(
    "1.0.0".to_string(),
    UpdateChannel::Stable,
);

// Check for updates
if let Some(update) = manager.check_for_updates().await? {
    println!("Update available: {}", update.version);
    println!("Release notes: {}", update.release_notes);

    // Apply update
    manager.apply_update(update).await?;
}
```

## Networking

### Container Networking

```rust
use kizuna::platform::container::ContainerNetworking;

let networking = ContainerNetworking::default();

// Setup networking
networking.setup().await?;

// Get container IP
let container_ip = networking.get_container_ip()?;
println!("Container IP: {}", container_ip);

// Get host IP
let host_ip = networking.get_host_ip()?;
println!("Host IP: {}", host_ip);
```

### Service Discovery

```rust
use kizuna::platform::container::{ServiceDiscovery, ServiceInfo, ServiceProtocol};

let mut discovery = ServiceDiscovery::new();

// Initialize service discovery
discovery.initialize().await?;

// Register a service
let service = ServiceInfo {
    name: "api".to_string(),
    address: "127.0.0.1:8080".parse()?,
    protocol: ServiceProtocol::Http,
    health_check: Some(HealthCheck::default()),
    metadata: HashMap::new(),
};

discovery.register_service("api".to_string(), service);

// Discover a service
if let Some(service) = discovery.discover_service("api") {
    println!("Service address: {}", service.address);
}
```

## Example Dockerfile

The generated Dockerfile for minimal footprint:

```dockerfile
FROM alpine:latest

# Metadata
LABEL version="1.0.0"

# Install dependencies
RUN apk add --no-cache ca-certificates

# Create non-root user
RUN addgroup -g 1000 kizuna && \
    adduser -D -u 1000 -G kizuna kizuna

WORKDIR /app

# Copy application binary
COPY --chown=kizuna:kizuna kizuna /app/kizuna
RUN chmod +x /app/kizuna

USER kizuna

# Expose default ports
EXPOSE 8080
EXPOSE 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["./kizuna", "health"]

# Entry point
ENTRYPOINT ["/app/kizuna"]
CMD ["serve"]
```

## Example Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kizuna
  namespace: default
  labels:
    app: kizuna
spec:
  replicas: 3
  selector:
    matchLabels:
      app: kizuna
  template:
    metadata:
      labels:
        app: kizuna
    spec:
      containers:
      - name: kizuna
        image: kizuna:latest
        imagePullPolicy: IfNotPresent
        ports:
        - name: http
          containerPort: 8080
          protocol: TCP
        env:
        - name: LOG_LEVEL
          value: "info"
        resources:
          requests:
            cpu: 250m
            memory: 128Mi
          limits:
            cpu: 1000m
            memory: 512Mi
        livenessProbe:
          httpGet:
            path: /health/live
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
          timeoutSeconds: 3
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
          timeoutSeconds: 3
```

## Best Practices

1. **Minimal Images**: Use Alpine Linux as base for smallest footprint
2. **Multi-Architecture**: Build for both AMD64 and ARM64
3. **Non-Root User**: Always run containers as non-root user
4. **Health Checks**: Implement both liveness and readiness probes
5. **Resource Limits**: Set appropriate CPU and memory limits
6. **Structured Logging**: Use JSON format for log aggregation
7. **Metrics**: Export metrics in Prometheus format
8. **Configuration**: Use environment variables for configuration
9. **Secrets**: Never hardcode secrets, use environment or secret management
10. **Updates**: Implement rolling updates with health checks

## Security Considerations

1. **Image Scanning**: Scan images for vulnerabilities
2. **Minimal Dependencies**: Only include necessary dependencies
3. **Non-Root**: Run as non-root user
4. **Read-Only Filesystem**: Use read-only root filesystem where possible
5. **Network Policies**: Implement Kubernetes network policies
6. **Secret Management**: Use Kubernetes secrets or external secret managers
7. **RBAC**: Implement proper role-based access control
8. **Pod Security**: Use pod security policies/standards
