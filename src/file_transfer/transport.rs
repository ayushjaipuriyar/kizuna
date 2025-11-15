// Transport Negotiation Module
//
// Handles transport protocol selection and capability exchange

use crate::file_transfer::{
    error::{FileTransferError, Result},
    types::*,
    TransportNegotiator,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cache entry for peer capabilities
#[derive(Debug, Clone)]
struct CapabilityCache {
    capabilities: TransportCapabilities,
    cached_at: Instant,
    performance_metrics: Option<HashMap<TransportProtocol, PerformanceMetrics>>,
}

impl CapabilityCache {
    fn new(capabilities: TransportCapabilities) -> Self {
        Self {
            capabilities,
            cached_at: Instant::now(),
            performance_metrics: Some(HashMap::new()),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.cached_at.elapsed() > ttl
    }
}

/// Transport performance degradation detector
#[derive(Debug, Clone)]
struct PerformanceDegradation {
    protocol: TransportProtocol,
    detected_at: Instant,
    baseline_throughput: u64,
    current_throughput: u64,
    degradation_percentage: f64,
}

impl PerformanceDegradation {
    fn new(
        protocol: TransportProtocol,
        baseline_throughput: u64,
        current_throughput: u64,
    ) -> Self {
        let degradation_percentage = if baseline_throughput > 0 {
            ((baseline_throughput - current_throughput) as f64 / baseline_throughput as f64) * 100.0
        } else {
            0.0
        };

        Self {
            protocol,
            detected_at: Instant::now(),
            baseline_throughput,
            current_throughput,
            degradation_percentage,
        }
    }

    fn is_severe(&self) -> bool {
        // Consider degradation severe if throughput drops by more than 50%
        self.degradation_percentage > 50.0
    }
}

/// Transport negotiator implementation
pub struct TransportNegotiatorImpl {
    /// Cache of peer capabilities to avoid repeated negotiations
    capability_cache: Arc<RwLock<HashMap<PeerId, CapabilityCache>>>,
    /// Cache TTL (time-to-live) in seconds
    cache_ttl: Duration,
    /// Performance benchmark timeout
    benchmark_timeout: Duration,
    /// Track performance degradation per peer and protocol
    degradation_tracker: Arc<RwLock<HashMap<(PeerId, TransportProtocol), PerformanceDegradation>>>,
}

impl TransportNegotiatorImpl {
    /// Create a new transport negotiator with default settings
    pub fn new() -> Self {
        Self {
            capability_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(300), // 5 minutes
            benchmark_timeout: Duration::from_secs(5),
            degradation_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new transport negotiator with custom cache TTL
    pub fn with_cache_ttl(cache_ttl: Duration) -> Self {
        Self {
            capability_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl,
            benchmark_timeout: Duration::from_secs(5),
            degradation_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Clear expired cache entries
    async fn cleanup_cache(&self) {
        let mut cache = self.capability_cache.write().await;
        cache.retain(|_, entry| !entry.is_expired(self.cache_ttl));
    }

    /// Get cached capabilities if available and not expired
    async fn get_cached_capabilities(&self, peer_id: &PeerId) -> Option<TransportCapabilities> {
        let cache = self.capability_cache.read().await;
        cache.get(peer_id).and_then(|entry| {
            if entry.is_expired(self.cache_ttl) {
                None
            } else {
                Some(entry.capabilities.clone())
            }
        })
    }

    /// Cache peer capabilities
    async fn cache_capabilities(&self, peer_id: PeerId, capabilities: TransportCapabilities) {
        let mut cache = self.capability_cache.write().await;
        cache.insert(peer_id, CapabilityCache::new(capabilities));
    }

    /// Cache performance metrics for a specific protocol
    async fn cache_performance_metrics(
        &self,
        peer_id: &PeerId,
        protocol: TransportProtocol,
        metrics: PerformanceMetrics,
    ) {
        let mut cache = self.capability_cache.write().await;
        if let Some(entry) = cache.get_mut(peer_id) {
            if let Some(ref mut perf_metrics) = entry.performance_metrics {
                perf_metrics.insert(protocol, metrics);
            }
        }
    }

    /// Get cached performance metrics for a protocol
    async fn get_cached_performance_metrics(
        &self,
        peer_id: &PeerId,
        protocol: TransportProtocol,
    ) -> Option<PerformanceMetrics> {
        let cache = self.capability_cache.read().await;
        cache.get(peer_id).and_then(|entry| {
            if entry.is_expired(self.cache_ttl) {
                None
            } else {
                entry
                    .performance_metrics
                    .as_ref()
                    .and_then(|metrics| metrics.get(&protocol).cloned())
            }
        })
    }

    /// Discover peer capabilities by querying the peer
    /// In a real implementation, this would send a capability exchange message
    async fn discover_peer_capabilities(&self, peer_id: &PeerId) -> Result<TransportCapabilities> {
        // TODO: Implement actual capability exchange protocol
        // For now, return default capabilities
        // In production, this would:
        // 1. Send a capability request message to the peer
        // 2. Wait for capability response with timeout
        // 3. Parse and validate the response
        
        // Simulate network delay
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        Ok(TransportCapabilities::default())
    }

    /// Select best transport protocol based on file size and capabilities
    /// 
    /// Selection strategy:
    /// - QUIC: Prioritized for large files (>10MB) due to resumability and performance
    /// - TCP: Fallback for peers without QUIC support, maximum compatibility
    /// - WebRTC: For browser-based peers and NAT traversal scenarios
    fn select_protocol(
        &self,
        file_size: u64,
        capabilities: &TransportCapabilities,
        performance_metrics: Option<&HashMap<TransportProtocol, PerformanceMetrics>>,
    ) -> TransportProtocol {
        // Large file threshold: 10MB
        const LARGE_FILE_THRESHOLD: u64 = 10 * 1024 * 1024;
        // Performance threshold: QUIC should be at least 80% as fast as TCP
        const QUIC_PERFORMANCE_THRESHOLD: u64 = 80;

        // Strategy 1: For large files, prioritize QUIC for resumability
        if file_size >= LARGE_FILE_THRESHOLD && capabilities.supports_quic {
            // Validate QUIC performance if metrics available
            if let Some(metrics) = performance_metrics {
                if let (Some(quic_metrics), Some(tcp_metrics)) = 
                    (metrics.get(&TransportProtocol::Quic), metrics.get(&TransportProtocol::Tcp)) {
                    // Only use QUIC if performance is acceptable
                    if quic_metrics.throughput_bps >= (tcp_metrics.throughput_bps * QUIC_PERFORMANCE_THRESHOLD / 100) {
                        return TransportProtocol::Quic;
                    }
                    // If QUIC performance is poor, fall through to TCP
                } else {
                    // No comparative metrics, trust QUIC for large files
                    return TransportProtocol::Quic;
                }
            } else {
                // No metrics available, use QUIC for large files
                return TransportProtocol::Quic;
            }
        }

        // Strategy 2: For browser-based peers (WebRTC only), use WebRTC
        if capabilities.supports_webrtc && !capabilities.supports_quic && !capabilities.supports_tcp {
            return TransportProtocol::WebRtc;
        }

        // Strategy 3: TCP fallback for maximum compatibility
        if capabilities.supports_tcp {
            return TransportProtocol::Tcp;
        }

        // Strategy 4: QUIC as secondary option if TCP not available
        if capabilities.supports_quic {
            return TransportProtocol::Quic;
        }

        // Strategy 5: WebRTC as last resort
        TransportProtocol::WebRtc
    }

    /// Select protocol with explicit preference consideration
    fn select_protocol_with_preference(
        &self,
        file_size: u64,
        capabilities: &TransportCapabilities,
        performance_metrics: Option<&HashMap<TransportProtocol, PerformanceMetrics>>,
        preference: Option<TransportProtocol>,
    ) -> TransportProtocol {
        // If user has a preference and peer supports it, honor it
        if let Some(preferred) = preference {
            let supported = match preferred {
                TransportProtocol::Quic => capabilities.supports_quic,
                TransportProtocol::Tcp => capabilities.supports_tcp,
                TransportProtocol::WebRtc => capabilities.supports_webrtc,
            };
            
            if supported {
                return preferred;
            }
        }

        // Otherwise use intelligent selection
        self.select_protocol(file_size, capabilities, performance_metrics)
    }

    /// Perform basic performance benchmark for a transport protocol
    async fn perform_benchmark(
        &self,
        protocol: TransportProtocol,
        _peer_id: &PeerId,
    ) -> Result<PerformanceMetrics> {
        // TODO: Implement actual benchmarking
        // In production, this would:
        // 1. Establish a test connection using the protocol
        // 2. Send test data packets
        // 3. Measure latency, throughput, packet loss
        // 4. Calculate jitter
        
        // Simulate benchmark with timeout
        tokio::time::timeout(self.benchmark_timeout, async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Return simulated metrics based on protocol characteristics
            Ok(match protocol {
                TransportProtocol::Quic => PerformanceMetrics {
                    latency_ms: 20,
                    throughput_bps: 100_000_000, // 100 Mbps
                    packet_loss: 0.01,
                    jitter_ms: 5,
                },
                TransportProtocol::Tcp => PerformanceMetrics {
                    latency_ms: 25,
                    throughput_bps: 95_000_000, // 95 Mbps
                    packet_loss: 0.005,
                    jitter_ms: 3,
                },
                TransportProtocol::WebRtc => PerformanceMetrics {
                    latency_ms: 30,
                    throughput_bps: 80_000_000, // 80 Mbps
                    packet_loss: 0.02,
                    jitter_ms: 8,
                },
            })
        })
        .await
        .map_err(|_| FileTransferError::TransportError("Benchmark timeout".to_string()))?
    }

    /// Monitor transport performance and detect degradation
    pub async fn monitor_performance(
        &self,
        peer_id: &PeerId,
        protocol: TransportProtocol,
        current_throughput: u64,
    ) -> Result<Option<PerformanceDegradation>> {
        // Get baseline performance metrics
        let baseline_metrics = self.get_cached_performance_metrics(peer_id, protocol).await;
        
        if let Some(baseline) = baseline_metrics {
            let baseline_throughput = baseline.throughput_bps;
            
            // Check if performance has degraded significantly
            if current_throughput < baseline_throughput {
                let degradation = PerformanceDegradation::new(
                    protocol,
                    baseline_throughput,
                    current_throughput,
                );
                
                // Track the degradation
                let mut tracker = self.degradation_tracker.write().await;
                tracker.insert((peer_id.clone(), protocol), degradation.clone());
                
                return Ok(Some(degradation));
            }
        }
        
        Ok(None)
    }

    /// Check if a protocol should be avoided due to recent failures or degradation
    pub async fn should_avoid_protocol(
        &self,
        peer_id: &PeerId,
        protocol: TransportProtocol,
    ) -> bool {
        let tracker = self.degradation_tracker.read().await;
        
        if let Some(degradation) = tracker.get(&(peer_id.clone(), protocol)) {
            // Avoid protocol if degradation is severe and recent (within last 5 minutes)
            degradation.is_severe() && degradation.detected_at.elapsed() < Duration::from_secs(300)
        } else {
            false
        }
    }

    /// Get fallback protocol considering peer capabilities and recent failures
    pub async fn get_fallback_protocol(
        &self,
        peer_id: &PeerId,
        current: TransportProtocol,
        capabilities: &TransportCapabilities,
    ) -> Result<Option<TransportProtocol>> {
        // Get the standard fallback chain
        let fallback = match current {
            TransportProtocol::Quic => Some(TransportProtocol::Tcp),
            TransportProtocol::Tcp => Some(TransportProtocol::WebRtc),
            TransportProtocol::WebRtc => None,
        };

        // Check if fallback is supported and not recently failed
        if let Some(fb_protocol) = fallback {
            let supported = match fb_protocol {
                TransportProtocol::Quic => capabilities.supports_quic,
                TransportProtocol::Tcp => capabilities.supports_tcp,
                TransportProtocol::WebRtc => capabilities.supports_webrtc,
            };

            if supported && !self.should_avoid_protocol(peer_id, fb_protocol).await {
                return Ok(Some(fb_protocol));
            }

            // If first fallback is not viable, try next in chain
            if let Ok(Some(next_fallback)) = self.fallback_transport(fb_protocol).await {
                let next_supported = match next_fallback {
                    TransportProtocol::Quic => capabilities.supports_quic,
                    TransportProtocol::Tcp => capabilities.supports_tcp,
                    TransportProtocol::WebRtc => capabilities.supports_webrtc,
                };

                if next_supported && !self.should_avoid_protocol(peer_id, next_fallback).await {
                    return Ok(Some(next_fallback));
                }
            }
        }

        Ok(None)
    }

    /// Attempt automatic transport switching on connection failure
    pub async fn handle_connection_failure(
        &self,
        peer_id: &PeerId,
        failed_protocol: TransportProtocol,
        file_size: u64,
    ) -> Result<Option<TransportProtocol>> {
        // Mark the protocol as degraded
        let degradation = PerformanceDegradation::new(failed_protocol, 100_000_000, 0);
        let mut tracker = self.degradation_tracker.write().await;
        tracker.insert((peer_id.clone(), failed_protocol), degradation);
        drop(tracker);

        // Get peer capabilities
        let capabilities = self.get_peer_capabilities(peer_id.clone()).await?;

        // Try to find a suitable fallback
        self.get_fallback_protocol(peer_id, failed_protocol, &capabilities).await
    }

    /// Clear degradation tracking for a peer (e.g., after successful connection)
    pub async fn clear_degradation(&self, peer_id: &PeerId, protocol: TransportProtocol) {
        let mut tracker = self.degradation_tracker.write().await;
        tracker.remove(&(peer_id.clone(), protocol));
    }

    /// Clear all degradation tracking for a peer
    pub async fn clear_all_degradation(&self, peer_id: &PeerId) {
        let mut tracker = self.degradation_tracker.write().await;
        tracker.retain(|(pid, _), _| pid != peer_id);
    }
}

impl Default for TransportNegotiatorImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportNegotiator for TransportNegotiatorImpl {
    async fn negotiate_transport(
        &self,
        peer_id: PeerId,
        file_size: u64,
    ) -> Result<TransportProtocol> {
        // Cleanup expired cache entries periodically
        self.cleanup_cache().await;

        // Get peer capabilities (from cache or discover)
        let capabilities = if let Some(cached) = self.get_cached_capabilities(&peer_id).await {
            cached
        } else {
            let caps = self.discover_peer_capabilities(&peer_id).await?;
            self.cache_capabilities(peer_id.clone(), caps.clone()).await;
            caps
        };

        // Get cached performance metrics if available
        let performance_metrics = {
            let cache = self.capability_cache.read().await;
            cache.get(&peer_id).and_then(|entry| {
                if entry.is_expired(self.cache_ttl) {
                    None
                } else {
                    entry.performance_metrics.as_ref().cloned()
                }
            })
        };

        // Select best protocol based on file size, capabilities, and performance
        let protocol = self.select_protocol_with_preference(
            file_size,
            &capabilities,
            performance_metrics.as_ref(),
            None,
        );

        Ok(protocol)
    }

    async fn get_peer_capabilities(&self, peer_id: PeerId) -> Result<TransportCapabilities> {
        // Check cache first
        if let Some(cached) = self.get_cached_capabilities(&peer_id).await {
            return Ok(cached);
        }

        // Discover and cache capabilities
        let capabilities = self.discover_peer_capabilities(&peer_id).await?;
        self.cache_capabilities(peer_id, capabilities.clone()).await;

        Ok(capabilities)
    }

    async fn benchmark_transport(
        &self,
        protocol: TransportProtocol,
        peer_id: PeerId,
    ) -> Result<PerformanceMetrics> {
        // Check if we have cached metrics
        if let Some(cached) = self.get_cached_performance_metrics(&peer_id, protocol).await {
            return Ok(cached);
        }

        // Perform benchmark
        let metrics = self.perform_benchmark(protocol, &peer_id).await?;

        // Cache the results
        self.cache_performance_metrics(&peer_id, protocol, metrics.clone()).await;

        Ok(metrics)
    }

    async fn fallback_transport(
        &self,
        current: TransportProtocol,
    ) -> Result<Option<TransportProtocol>> {
        // Define fallback chain: QUIC -> TCP -> WebRTC
        let fallback = match current {
            TransportProtocol::Quic => Some(TransportProtocol::Tcp),
            TransportProtocol::Tcp => Some(TransportProtocol::WebRtc),
            TransportProtocol::WebRtc => None, // No fallback from WebRTC
        };

        Ok(fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_negotiator_creation() {
        let negotiator = TransportNegotiatorImpl::new();
        assert_eq!(negotiator.cache_ttl, Duration::from_secs(300));
    }

    #[tokio::test]
    async fn test_capability_caching() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();
        let capabilities = TransportCapabilities::default();

        // Cache capabilities
        negotiator.cache_capabilities(peer_id.clone(), capabilities.clone()).await;

        // Retrieve from cache
        let cached = negotiator.get_cached_capabilities(&peer_id).await;
        assert!(cached.is_some());
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let negotiator = TransportNegotiatorImpl::with_cache_ttl(Duration::from_millis(100));
        let peer_id = "test-peer".to_string();
        let capabilities = TransportCapabilities::default();

        // Cache capabilities
        negotiator.cache_capabilities(peer_id.clone(), capabilities).await;

        // Should be in cache
        assert!(negotiator.get_cached_capabilities(&peer_id).await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be expired
        assert!(negotiator.get_cached_capabilities(&peer_id).await.is_none());
    }

    #[tokio::test]
    async fn test_protocol_selection_large_file() {
        let negotiator = TransportNegotiatorImpl::new();
        let capabilities = TransportCapabilities {
            supports_quic: true,
            supports_tcp: true,
            supports_webrtc: false,
            max_parallel_streams: 4,
            max_bandwidth: None,
        };

        // Large file should prefer QUIC
        let protocol = negotiator.select_protocol(100_000_000, &capabilities, None);
        assert_eq!(protocol, TransportProtocol::Quic);
    }

    #[tokio::test]
    async fn test_protocol_selection_small_file() {
        let negotiator = TransportNegotiatorImpl::new();
        let capabilities = TransportCapabilities {
            supports_quic: true,
            supports_tcp: true,
            supports_webrtc: false,
            max_parallel_streams: 4,
            max_bandwidth: None,
        };

        // Small file should prefer TCP for compatibility
        let protocol = negotiator.select_protocol(1_000_000, &capabilities, None);
        assert_eq!(protocol, TransportProtocol::Tcp);
    }

    #[tokio::test]
    async fn test_protocol_selection_webrtc_only() {
        let negotiator = TransportNegotiatorImpl::new();
        let capabilities = TransportCapabilities {
            supports_quic: false,
            supports_tcp: false,
            supports_webrtc: true,
            max_parallel_streams: 4,
            max_bandwidth: None,
        };

        // Should select WebRTC when it's the only option
        let protocol = negotiator.select_protocol(1_000_000, &capabilities, None);
        assert_eq!(protocol, TransportProtocol::WebRtc);
    }

    #[tokio::test]
    async fn test_fallback_chain() {
        let negotiator = TransportNegotiatorImpl::new();

        // QUIC falls back to TCP
        let fallback = negotiator.fallback_transport(TransportProtocol::Quic).await.unwrap();
        assert_eq!(fallback, Some(TransportProtocol::Tcp));

        // TCP falls back to WebRTC
        let fallback = negotiator.fallback_transport(TransportProtocol::Tcp).await.unwrap();
        assert_eq!(fallback, Some(TransportProtocol::WebRtc));

        // WebRTC has no fallback
        let fallback = negotiator.fallback_transport(TransportProtocol::WebRtc).await.unwrap();
        assert_eq!(fallback, None);
    }

    #[tokio::test]
    async fn test_negotiate_transport() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();

        // Negotiate for large file
        let protocol = negotiator.negotiate_transport(peer_id.clone(), 100_000_000).await.unwrap();
        assert_eq!(protocol, TransportProtocol::Quic);

        // Capabilities should be cached
        let cached = negotiator.get_cached_capabilities(&peer_id).await;
        assert!(cached.is_some());
    }

    #[tokio::test]
    async fn test_get_peer_capabilities() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();

        // Get capabilities (should discover and cache)
        let capabilities = negotiator.get_peer_capabilities(peer_id.clone()).await.unwrap();
        assert!(capabilities.supports_tcp);

        // Second call should use cache
        let capabilities2 = negotiator.get_peer_capabilities(peer_id).await.unwrap();
        assert_eq!(capabilities.supports_tcp, capabilities2.supports_tcp);
    }

    #[tokio::test]
    async fn test_benchmark_transport() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();

        // Benchmark QUIC
        let metrics = negotiator.benchmark_transport(TransportProtocol::Quic, peer_id.clone()).await.unwrap();
        assert!(metrics.throughput_bps > 0);
        assert!(metrics.latency_ms > 0);

        // Second call should use cache
        let metrics2 = negotiator.benchmark_transport(TransportProtocol::Quic, peer_id).await.unwrap();
        assert_eq!(metrics.throughput_bps, metrics2.throughput_bps);
    }

    #[tokio::test]
    async fn test_performance_metrics_caching() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();
        let metrics = PerformanceMetrics {
            latency_ms: 10,
            throughput_bps: 100_000_000,
            packet_loss: 0.01,
            jitter_ms: 5,
        };

        // Cache metrics
        negotiator.cache_performance_metrics(&peer_id, TransportProtocol::Quic, metrics.clone()).await;

        // Retrieve from cache
        let cached = negotiator.get_cached_performance_metrics(&peer_id, TransportProtocol::Quic).await;
        assert!(cached.is_some());
        let cached_metrics = cached.unwrap();
        assert_eq!(cached_metrics.latency_ms, metrics.latency_ms);
    }

    #[tokio::test]
    async fn test_protocol_selection_with_performance_metrics() {
        let negotiator = TransportNegotiatorImpl::new();
        let capabilities = TransportCapabilities {
            supports_quic: true,
            supports_tcp: true,
            supports_webrtc: false,
            max_parallel_streams: 4,
            max_bandwidth: None,
        };

        let mut metrics = HashMap::new();
        metrics.insert(
            TransportProtocol::Quic,
            PerformanceMetrics {
                latency_ms: 20,
                throughput_bps: 90_000_000, // 90 Mbps
                packet_loss: 0.01,
                jitter_ms: 5,
            },
        );
        metrics.insert(
            TransportProtocol::Tcp,
            PerformanceMetrics {
                latency_ms: 25,
                throughput_bps: 100_000_000, // 100 Mbps
                packet_loss: 0.005,
                jitter_ms: 3,
            },
        );

        // Large file with good QUIC performance (90% of TCP) should use QUIC
        let protocol = negotiator.select_protocol(100_000_000, &capabilities, Some(&metrics));
        assert_eq!(protocol, TransportProtocol::Quic);
    }

    #[tokio::test]
    async fn test_protocol_selection_poor_quic_performance() {
        let negotiator = TransportNegotiatorImpl::new();
        let capabilities = TransportCapabilities {
            supports_quic: true,
            supports_tcp: true,
            supports_webrtc: false,
            max_parallel_streams: 4,
            max_bandwidth: None,
        };

        let mut metrics = HashMap::new();
        metrics.insert(
            TransportProtocol::Quic,
            PerformanceMetrics {
                latency_ms: 50,
                throughput_bps: 50_000_000, // 50 Mbps (only 50% of TCP)
                packet_loss: 0.05,
                jitter_ms: 15,
            },
        );
        metrics.insert(
            TransportProtocol::Tcp,
            PerformanceMetrics {
                latency_ms: 25,
                throughput_bps: 100_000_000, // 100 Mbps
                packet_loss: 0.005,
                jitter_ms: 3,
            },
        );

        // Large file with poor QUIC performance should fall back to TCP
        let protocol = negotiator.select_protocol(100_000_000, &capabilities, Some(&metrics));
        assert_eq!(protocol, TransportProtocol::Tcp);
    }

    #[tokio::test]
    async fn test_protocol_selection_with_preference() {
        let negotiator = TransportNegotiatorImpl::new();
        let capabilities = TransportCapabilities {
            supports_quic: true,
            supports_tcp: true,
            supports_webrtc: true,
            max_parallel_streams: 4,
            max_bandwidth: None,
        };

        // User prefers WebRTC and peer supports it
        let protocol = negotiator.select_protocol_with_preference(
            1_000_000,
            &capabilities,
            None,
            Some(TransportProtocol::WebRtc),
        );
        assert_eq!(protocol, TransportProtocol::WebRtc);

        // User prefers QUIC but peer doesn't support it
        let capabilities_no_quic = TransportCapabilities {
            supports_quic: false,
            supports_tcp: true,
            supports_webrtc: true,
            max_parallel_streams: 4,
            max_bandwidth: None,
        };
        let protocol = negotiator.select_protocol_with_preference(
            1_000_000,
            &capabilities_no_quic,
            None,
            Some(TransportProtocol::Quic),
        );
        // Should fall back to TCP
        assert_eq!(protocol, TransportProtocol::Tcp);
    }

    #[tokio::test]
    async fn test_tcp_fallback_for_no_quic() {
        let negotiator = TransportNegotiatorImpl::new();
        let capabilities = TransportCapabilities {
            supports_quic: false,
            supports_tcp: true,
            supports_webrtc: false,
            max_parallel_streams: 4,
            max_bandwidth: None,
        };

        // Even for large files, should use TCP if QUIC not available
        let protocol = negotiator.select_protocol(100_000_000, &capabilities, None);
        assert_eq!(protocol, TransportProtocol::Tcp);
    }

    #[tokio::test]
    async fn test_webrtc_for_browser_peers() {
        let negotiator = TransportNegotiatorImpl::new();
        let capabilities = TransportCapabilities {
            supports_quic: false,
            supports_tcp: false,
            supports_webrtc: true,
            max_parallel_streams: 4,
            max_bandwidth: None,
        };

        // Browser-based peer should use WebRTC
        let protocol = negotiator.select_protocol(1_000_000, &capabilities, None);
        assert_eq!(protocol, TransportProtocol::WebRtc);
    }

    #[tokio::test]
    async fn test_performance_degradation_detection() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();
        
        // Set baseline performance
        let baseline_metrics = PerformanceMetrics {
            latency_ms: 20,
            throughput_bps: 100_000_000,
            packet_loss: 0.01,
            jitter_ms: 5,
        };
        negotiator.cache_performance_metrics(&peer_id, TransportProtocol::Quic, baseline_metrics).await;

        // Monitor with degraded performance
        let degradation = negotiator.monitor_performance(
            &peer_id,
            TransportProtocol::Quic,
            40_000_000, // 40% of baseline
        ).await.unwrap();

        assert!(degradation.is_some());
        let deg = degradation.unwrap();
        assert!(deg.is_severe());
        assert_eq!(deg.protocol, TransportProtocol::Quic);
    }

    #[tokio::test]
    async fn test_should_avoid_protocol() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();

        // Initially should not avoid
        assert!(!negotiator.should_avoid_protocol(&peer_id, TransportProtocol::Quic).await);

        // Simulate severe degradation
        let degradation = PerformanceDegradation::new(TransportProtocol::Quic, 100_000_000, 30_000_000);
        let mut tracker = negotiator.degradation_tracker.write().await;
        tracker.insert((peer_id.clone(), TransportProtocol::Quic), degradation);
        drop(tracker);

        // Should now avoid the protocol
        assert!(negotiator.should_avoid_protocol(&peer_id, TransportProtocol::Quic).await);
    }

    #[tokio::test]
    async fn test_get_fallback_protocol() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();
        let capabilities = TransportCapabilities {
            supports_quic: true,
            supports_tcp: true,
            supports_webrtc: true,
            max_parallel_streams: 4,
            max_bandwidth: None,
        };

        // QUIC should fall back to TCP
        let fallback = negotiator.get_fallback_protocol(
            &peer_id,
            TransportProtocol::Quic,
            &capabilities,
        ).await.unwrap();
        assert_eq!(fallback, Some(TransportProtocol::Tcp));

        // TCP should fall back to WebRTC
        let fallback = negotiator.get_fallback_protocol(
            &peer_id,
            TransportProtocol::Tcp,
            &capabilities,
        ).await.unwrap();
        assert_eq!(fallback, Some(TransportProtocol::WebRtc));

        // WebRTC has no fallback
        let fallback = negotiator.get_fallback_protocol(
            &peer_id,
            TransportProtocol::WebRtc,
            &capabilities,
        ).await.unwrap();
        assert_eq!(fallback, None);
    }

    #[tokio::test]
    async fn test_fallback_skips_degraded_protocol() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();
        let capabilities = TransportCapabilities {
            supports_quic: true,
            supports_tcp: true,
            supports_webrtc: true,
            max_parallel_streams: 4,
            max_bandwidth: None,
        };

        // Mark TCP as degraded
        let degradation = PerformanceDegradation::new(TransportProtocol::Tcp, 100_000_000, 20_000_000);
        let mut tracker = negotiator.degradation_tracker.write().await;
        tracker.insert((peer_id.clone(), TransportProtocol::Tcp), degradation);
        drop(tracker);

        // QUIC should skip TCP and fall back to WebRTC
        let fallback = negotiator.get_fallback_protocol(
            &peer_id,
            TransportProtocol::Quic,
            &capabilities,
        ).await.unwrap();
        assert_eq!(fallback, Some(TransportProtocol::WebRtc));
    }

    #[tokio::test]
    async fn test_handle_connection_failure() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();

        // Handle QUIC connection failure
        let fallback = negotiator.handle_connection_failure(
            &peer_id,
            TransportProtocol::Quic,
            100_000_000,
        ).await.unwrap();

        // Should suggest TCP as fallback
        assert_eq!(fallback, Some(TransportProtocol::Tcp));

        // QUIC should now be marked as degraded
        assert!(negotiator.should_avoid_protocol(&peer_id, TransportProtocol::Quic).await);
    }

    #[tokio::test]
    async fn test_clear_degradation() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();

        // Mark protocol as degraded
        let degradation = PerformanceDegradation::new(TransportProtocol::Quic, 100_000_000, 30_000_000);
        let mut tracker = negotiator.degradation_tracker.write().await;
        tracker.insert((peer_id.clone(), TransportProtocol::Quic), degradation);
        drop(tracker);

        // Verify it's marked
        assert!(negotiator.should_avoid_protocol(&peer_id, TransportProtocol::Quic).await);

        // Clear degradation
        negotiator.clear_degradation(&peer_id, TransportProtocol::Quic).await;

        // Should no longer be avoided
        assert!(!negotiator.should_avoid_protocol(&peer_id, TransportProtocol::Quic).await);
    }

    #[tokio::test]
    async fn test_clear_all_degradation() {
        let negotiator = TransportNegotiatorImpl::new();
        let peer_id = "test-peer".to_string();

        // Mark multiple protocols as degraded
        let mut tracker = negotiator.degradation_tracker.write().await;
        tracker.insert(
            (peer_id.clone(), TransportProtocol::Quic),
            PerformanceDegradation::new(TransportProtocol::Quic, 100_000_000, 30_000_000),
        );
        tracker.insert(
            (peer_id.clone(), TransportProtocol::Tcp),
            PerformanceDegradation::new(TransportProtocol::Tcp, 100_000_000, 40_000_000),
        );
        drop(tracker);

        // Clear all degradation for peer
        negotiator.clear_all_degradation(&peer_id).await;

        // Neither should be avoided
        assert!(!negotiator.should_avoid_protocol(&peer_id, TransportProtocol::Quic).await);
        assert!(!negotiator.should_avoid_protocol(&peer_id, TransportProtocol::Tcp).await);
    }
}
  
