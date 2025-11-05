use crate::discovery::{KizunaDiscovery, DiscoveryBuilder, DiscoveryEvent};
use std::time::Duration;
use tokio::time::timeout;

/// CLI commands for discovery testing and debugging
pub struct DiscoveryCli;

impl DiscoveryCli {
    /// Run discovery once and display results
    pub async fn discover_once(
        timeout_secs: Option<u64>,
        strategies: Option<Vec<String>>,
        verbose: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let timeout_duration = Duration::from_secs(timeout_secs.unwrap_or(5));
        
        let mut discovery = if let Some(strategies) = strategies {
            DiscoveryBuilder::new()
                .strategies(strategies)
                .timeout(timeout_duration)
                .build()
        } else {
            KizunaDiscovery::new()
        };

        discovery.initialize().await?;

        if verbose {
            println!("Starting discovery with timeout: {:?}", timeout_duration);
            println!("Available strategies: {:?}", discovery.get_available_strategies());
        }

        let peers = discovery.discover_once(Some(timeout_duration)).await?;

        if peers.is_empty() {
            println!("No peers discovered");
        } else {
            println!("Discovered {} peer(s):", peers.len());
            for (i, peer) in peers.iter().enumerate() {
                println!("  {}. {} ({})", i + 1, peer.name, peer.peer_id);
                if verbose {
                    println!("     Addresses: {:?}", peer.addresses);
                    println!("     Port: {}", peer.port);
                    println!("     Method: {}", peer.discovery_method);
                    if !peer.capabilities.is_empty() {
                        println!("     Capabilities: {:?}", peer.capabilities);
                    }
                    println!("     Last seen: {:?}", peer.last_seen);
                }
            }
        }

        discovery.shutdown().await?;
        Ok(())
    }

    /// Start continuous discovery and display events
    pub async fn discover_continuous(
        strategies: Option<Vec<String>>,
        verbose: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut discovery = if let Some(strategies) = strategies {
            DiscoveryBuilder::new()
                .strategies(strategies)
                .build()
        } else {
            KizunaDiscovery::new()
        };

        discovery.initialize().await?;

        if verbose {
            println!("Available strategies: {:?}", discovery.get_available_strategies());
        }

        println!("Starting continuous discovery... (Press Ctrl+C to stop)");
        
        let mut event_receiver = discovery.start_discovery().await?;

        // Handle Ctrl+C gracefully
        let ctrl_c = tokio::signal::ctrl_c();
        tokio::pin!(ctrl_c);

        loop {
            tokio::select! {
                event = event_receiver.recv() => {
                    match event {
                        Some(DiscoveryEvent::PeerDiscovered(peer)) => {
                            println!("[DISCOVERED] {} ({}) via {}", 
                                peer.name, peer.peer_id, peer.discovery_method);
                            if verbose {
                                println!("             Addresses: {:?}", peer.addresses);
                                if !peer.capabilities.is_empty() {
                                    println!("             Capabilities: {:?}", peer.capabilities);
                                }
                            }
                        }
                        Some(DiscoveryEvent::PeerLost(peer_id)) => {
                            println!("[LOST] Peer {} expired from cache", peer_id);
                        }
                        Some(DiscoveryEvent::StrategyChanged(strategy)) => {
                            println!("[STRATEGY] Switched to {}", strategy);
                        }
                        Some(DiscoveryEvent::Error(error)) => {
                            eprintln!("[ERROR] Discovery error: {}", error);
                        }
                        None => {
                            println!("Discovery event stream ended");
                            break;
                        }
                    }
                }
                _ = &mut ctrl_c => {
                    println!("\nShutting down discovery...");
                    break;
                }
            }
        }

        discovery.shutdown().await?;
        Ok(())
    }

    /// Announce this peer's presence
    pub async fn announce(
        peer_name: Option<String>,
        _port: Option<u16>,
        strategies: Option<Vec<String>>,
        duration_secs: Option<u64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut discovery = if let Some(strategies) = strategies {
            DiscoveryBuilder::new()
                .strategies(strategies)
                .build()
        } else {
            KizunaDiscovery::new()
        };

        discovery.initialize().await?;

        let peer_name = peer_name.unwrap_or_else(|| {
            hostname::get()
                .unwrap_or_else(|_| "Unknown".into())
                .to_string_lossy()
                .to_string()
        });

        println!("Announcing presence as '{}' on available strategies...", peer_name);
        println!("Available strategies: {:?}", discovery.get_available_strategies());

        discovery.announce().await?;

        if let Some(duration) = duration_secs {
            println!("Announcing for {} seconds...", duration);
            tokio::time::sleep(Duration::from_secs(duration)).await;
        } else {
            println!("Announcing indefinitely... (Press Ctrl+C to stop)");
            tokio::signal::ctrl_c().await?;
        }

        println!("Stopping announcement...");
        discovery.stop_announce().await?;
        discovery.shutdown().await?;

        Ok(())
    }

    /// Test specific discovery strategy
    pub async fn test_strategy(
        strategy_name: String,
        timeout_secs: Option<u64>,
        verbose: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let timeout_duration = Duration::from_secs(timeout_secs.unwrap_or(5));
        
        let mut discovery = DiscoveryBuilder::new()
            .strategies(vec![strategy_name.clone()])
            .timeout(timeout_duration)
            .auto_select(false)
            .build();

        discovery.initialize().await?;

        if !discovery.get_available_strategies().contains(&strategy_name) {
            return Err(format!("Strategy '{}' is not available on this platform", strategy_name).into());
        }

        discovery.set_active_strategy(strategy_name.clone())?;

        if verbose {
            println!("Testing strategy: {}", strategy_name);
            println!("Timeout: {:?}", timeout_duration);
        }

        let start_time = std::time::Instant::now();
        
        match timeout(timeout_duration, discovery.discover_once(Some(timeout_duration))).await {
            Ok(Ok(peers)) => {
                let elapsed = start_time.elapsed();
                println!("Strategy '{}' completed in {:?}", strategy_name, elapsed);
                println!("Discovered {} peer(s):", peers.len());
                
                for (i, peer) in peers.iter().enumerate() {
                    println!("  {}. {} ({})", i + 1, peer.name, peer.peer_id);
                    if verbose {
                        println!("     Addresses: {:?}", peer.addresses);
                        println!("     Capabilities: {:?}", peer.capabilities);
                    }
                }
            }
            Ok(Err(e)) => {
                let elapsed = start_time.elapsed();
                println!("Strategy '{}' failed after {:?}: {}", strategy_name, elapsed, e);
            }
            Err(_) => {
                println!("Strategy '{}' timed out after {:?}", strategy_name, timeout_duration);
            }
        }

        discovery.shutdown().await?;
        Ok(())
    }

    /// Show discovery statistics and performance metrics
    pub async fn show_stats(
        strategies: Option<Vec<String>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut discovery = if let Some(strategies) = strategies {
            DiscoveryBuilder::new()
                .strategies(strategies)
                .build()
        } else {
            KizunaDiscovery::new()
        };

        discovery.initialize().await?;

        // Perform a quick discovery to generate some stats
        println!("Performing discovery to generate statistics...");
        let _ = discovery.discover_once(Some(Duration::from_secs(3))).await;

        let cached_peers = discovery.get_cached_peers();
        println!("\n=== Discovery Statistics ===");
        println!("Available strategies: {:?}", discovery.get_available_strategies());
        println!("Cached peers: {}", cached_peers.len());

        if !cached_peers.is_empty() {
            println!("\nCached Peers:");
            for (i, peer) in cached_peers.iter().enumerate() {
                println!("  {}. {} ({}) via {} - {} addresses", 
                    i + 1, peer.name, peer.peer_id, peer.discovery_method, peer.addresses.len());
            }
        }

        let config = discovery.get_config();
        println!("\n=== Configuration ===");
        println!("Auto-select: {}", config.auto_select);
        println!("Default timeout: {:?}", config.default_timeout);
        println!("Peer cache TTL: {:?}", config.peer_cache_ttl);
        println!("Max concurrent discoveries: {}", config.max_concurrent_discoveries);
        println!("Enabled strategies: {:?}", config.enabled_strategies);

        discovery.shutdown().await?;
        Ok(())
    }

    /// Benchmark discovery strategies
    pub async fn benchmark(
        iterations: Option<usize>,
        timeout_secs: Option<u64>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let iterations = iterations.unwrap_or(5);
        let timeout_duration = Duration::from_secs(timeout_secs.unwrap_or(3));

        let mut discovery = KizunaDiscovery::new();
        discovery.initialize().await?;

        let strategies = discovery.get_available_strategies();
        
        if strategies.is_empty() {
            return Err("No discovery strategies available for benchmarking".into());
        }

        println!("Benchmarking {} strategies with {} iterations each", strategies.len(), iterations);
        println!("Timeout per iteration: {:?}\n", timeout_duration);

        for strategy in &strategies {
            println!("Benchmarking strategy: {}", strategy);
            
            let mut total_time = Duration::ZERO;
            let mut successful_runs = 0;
            let mut total_peers = 0;

            for i in 1..=iterations {
                print!("  Run {}/{}: ", i, iterations);
                
                let mut strategy_discovery = DiscoveryBuilder::new()
                    .strategies(vec![strategy.clone()])
                    .timeout(timeout_duration)
                    .auto_select(false)
                    .build();

                strategy_discovery.initialize().await?;
                strategy_discovery.set_active_strategy(strategy.clone())?;

                let start_time = std::time::Instant::now();
                
                match timeout(timeout_duration, strategy_discovery.discover_once(Some(timeout_duration))).await {
                    Ok(Ok(peers)) => {
                        let elapsed = start_time.elapsed();
                        total_time += elapsed;
                        successful_runs += 1;
                        total_peers += peers.len();
                        println!("{:?} - {} peers", elapsed, peers.len());
                    }
                    Ok(Err(e)) => {
                        println!("Failed: {}", e);
                    }
                    Err(_) => {
                        println!("Timed out");
                    }
                }

                strategy_discovery.shutdown().await?;
            }

            if successful_runs > 0 {
                let avg_time = total_time / successful_runs as u32;
                let success_rate = (successful_runs as f64 / iterations as f64) * 100.0;
                let avg_peers = total_peers as f64 / successful_runs as f64;
                
                println!("  Results: {:.1}% success rate, avg {:?}, avg {:.1} peers\n", 
                    success_rate, avg_time, avg_peers);
            } else {
                println!("  Results: 0% success rate\n");
            }
        }

        discovery.shutdown().await?;
        Ok(())
    }

    /// Show detailed configuration options
    pub fn show_config_help() {
        println!("=== Discovery Configuration Options ===\n");
        
        println!("Available Strategies:");
        println!("  mdns      - Multicast DNS discovery (local network)");
        println!("  udp       - UDP broadcast discovery (local network)");
        println!("  tcp       - TCP handshake beacon (local network)");
        println!("  bluetooth - Bluetooth LE discovery (proximity)");
        println!("  libp2p    - libp2p hybrid discovery (global)\n");
        
        println!("Configuration Parameters:");
        println!("  --timeout SECS        Discovery timeout in seconds (default: 5)");
        println!("  --strategies LIST     Comma-separated list of strategies to use");
        println!("  --verbose             Show detailed output");
        println!("  --auto-select         Enable automatic strategy selection");
        println!("  --concurrent          Enable concurrent discovery across strategies\n");
        
        println!("Examples:");
        println!("  kizuna discover --strategies mdns,udp --timeout 10");
        println!("  kizuna announce --name \"My Device\" --port 8080");
        println!("  kizuna test-strategy mdns --verbose");
        println!("  kizuna benchmark --iterations 10");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cli_discover_once() {
        // This test would require actual network setup to work properly
        // For now, we just test that the function doesn't panic
        let result = DiscoveryCli::discover_once(
            Some(1), // Very short timeout
            Some(vec!["mdns".to_string()]),
            false,
        ).await;
        
        // Should either succeed or fail gracefully
        match result {
            Ok(_) => println!("Discovery succeeded"),
            Err(e) => println!("Discovery failed as expected: {}", e),
        }
    }

    #[tokio::test]
    async fn test_cli_test_strategy() {
        // Test with a very short timeout to avoid long waits
        let result = DiscoveryCli::test_strategy(
            "mdns".to_string(),
            Some(1),
            false,
        ).await;
        
        // Should either succeed or fail gracefully
        match result {
            Ok(_) => println!("Strategy test succeeded"),
            Err(e) => println!("Strategy test failed as expected: {}", e),
        }
    }

    #[test]
    fn test_show_config_help() {
        // This should not panic
        DiscoveryCli::show_config_help();
    }
}