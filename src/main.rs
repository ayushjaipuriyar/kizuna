use anyhow::Result;
use std::env;
use std::time::Duration;

// Use the library's discovery module instead of re-declaring it
use kizuna::discovery::{
    Discovery, DiscoveryManager, KizunaDiscovery, DiscoveryCli, ConfigManager,
    DiscoveryConfigFile, discovery_selector,
    strategies::{udp::UdpDiscovery, mdns::MdnsDiscovery},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match command {
        // New CLI commands
        "discover" => {
            let timeout = parse_arg(&args, "--timeout").and_then(|s| s.parse().ok());
            let strategies = parse_arg(&args, "--strategies")
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());
            let verbose = args.contains(&"--verbose".to_string());
            
            DiscoveryCli::discover_once(timeout, strategies, verbose).await.map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        "discover-continuous" => {
            let strategies = parse_arg(&args, "--strategies")
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());
            let verbose = args.contains(&"--verbose".to_string());
            
            DiscoveryCli::discover_continuous(strategies, verbose).await.map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        "announce" => {
            let name = parse_arg(&args, "--name").map(|s| s.to_string());
            let port = parse_arg(&args, "--port").and_then(|s| s.parse().ok());
            let strategies = parse_arg(&args, "--strategies")
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());
            let duration = parse_arg(&args, "--duration").and_then(|s| s.parse().ok());
            
            DiscoveryCli::announce(name, port, strategies, duration).await.map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        "test-strategy" => {
            let strategy = args.get(2).ok_or_else(|| anyhow::anyhow!("Strategy name required"))?.to_string();
            let timeout = parse_arg(&args, "--timeout").and_then(|s| s.parse().ok());
            let verbose = args.contains(&"--verbose".to_string());
            
            DiscoveryCli::test_strategy(strategy, timeout, verbose).await.map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        "benchmark" => {
            let iterations = parse_arg(&args, "--iterations").and_then(|s| s.parse().ok());
            let timeout = parse_arg(&args, "--timeout").and_then(|s| s.parse().ok());
            
            DiscoveryCli::benchmark(iterations, timeout).await.map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        "stats" => {
            let strategies = parse_arg(&args, "--strategies")
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect());
            
            DiscoveryCli::show_stats(strategies).await.map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        "config" => {
            let subcommand = args.get(2).map(|s| s.as_str()).unwrap_or("show");
            match subcommand {
                "init" => {
                    let force = args.contains(&"--force".to_string());
                    ConfigManager::init_config(force).map_err(|e| anyhow::anyhow!("{}", e))?;
                }
                "validate" => {
                    let path = args.get(3).map_or("kizuna-discovery.toml", |v| v);
                    ConfigManager::validate_config(path).map_err(|e| anyhow::anyhow!("{}", e))?;
                }
                "show" => {
                    let path = args.get(3).map_or("kizuna-discovery.toml", |v| v);
                    if std::path::Path::new(path).exists() {
                        ConfigManager::show_config(path).map_err(|e| anyhow::anyhow!("{}", e))?;
                    } else {
                        println!("Configuration file not found. Showing default configuration:");
                        println!("{}", ConfigManager::generate_sample());
                    }
                }
                "sample" => {
                    println!("{}", ConfigManager::generate_sample());
                }
                _ => {
                    println!("Unknown config subcommand. Available: init, validate, show, sample");
                }
            }
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        
        // Legacy commands for backward compatibility
        "discover-legacy" | "legacy-discover" => {
            // Use the new enhanced discovery system
            let mut manager = DiscoveryManager::new();
            manager.add_strategy(Box::new(UdpDiscovery::new()));
            manager.add_strategy(Box::new(MdnsDiscovery::new()));
            
            println!("Starting legacy discovery with enhanced system...");
            println!("Available strategies: {:?}", manager.get_available_strategies());
            
            match manager.discover_peers(Duration::from_secs(5)).await {
                Ok(peers) => {
                    println!("Discovered {} peers:", peers.len());
                    for peer in peers {
                        println!("- {} ({}) @ {:?} via {}", 
                            peer.name, peer.peer_id, peer.addresses, peer.discovery_method);
                    }
                }
                Err(e) => eprintln!("Discovery failed: {}", e),
            }
        }
        "udp" => {
            // Legacy UDP mode
            let d = UdpDiscovery::new();
            let peers = d.browse().await?;
            println!("Discovered {} peers via UDP", peers.len());
            for p in peers { println!("- {} @ {}:{}", p.id, p.addr, p.port); }
        }
        "mdns" => {
            // Legacy mDNS mode
            let d = MdnsDiscovery::new();
            match d.browse().await {
                Ok(peers) => {
                    println!("Discovered {} peers via mDNS", peers.len());
                    for p in peers { println!("- {} @ {}:{}", p.id, p.addr, p.port); }
                }
                Err(e) => eprintln!("mDNS browse failed: {}", e),
            }
        }
        "auto" | _ => {
            // Default behavior - show help for unknown commands
            if command != "auto" {
                println!("Unknown command: {}", command);
                println!();
            }
            print_help();
        }
    }

    Ok(())
}

/// Parse command line argument value
fn parse_arg<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|pos| args.get(pos + 1))
        .map(|s| s.as_str())
}

/// Print help information
fn print_help() {
    println!("Kizuna Discovery System");
    println!("A multi-strategy peer discovery system for local and global networks");
    println!();
    println!("USAGE:");
    println!("    kizuna <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    discover                 Discover peers once and exit");
    println!("    discover-continuous      Start continuous peer discovery");
    println!("    announce                 Announce this peer's presence");
    println!("    test-strategy <NAME>     Test a specific discovery strategy");
    println!("    benchmark               Benchmark all available strategies");
    println!("    stats                   Show discovery statistics");
    println!("    config <SUBCOMMAND>     Configuration management");
    println!("    help                    Show this help message");
    println!();
    println!("DISCOVERY OPTIONS:");
    println!("    --timeout SECS          Discovery timeout in seconds (default: 5)");
    println!("    --strategies LIST       Comma-separated list of strategies");
    println!("    --verbose               Show detailed output");
    println!();
    println!("ANNOUNCE OPTIONS:");
    println!("    --name NAME             Device name for announcements");
    println!("    --port PORT             Service port number");
    println!("    --duration SECS         Announce for specified seconds");
    println!();
    println!("CONFIG SUBCOMMANDS:");
    println!("    init                    Create default configuration file");
    println!("    validate [FILE]         Validate configuration file");
    println!("    show [FILE]             Show configuration");
    println!("    sample                  Generate sample configuration");
    println!();
    println!("AVAILABLE STRATEGIES:");
    println!("    mdns                    Multicast DNS (local network)");
    println!("    udp                     UDP broadcast (local network)");
    println!("    tcp                     TCP handshake beacon (local network)");
    println!("    bluetooth               Bluetooth LE (proximity)");
    println!();
    println!("EXAMPLES:");
    println!("    kizuna discover --strategies mdns,udp --timeout 10 --verbose");
    println!("    kizuna announce --name \"My Device\" --port 8080 --duration 60");
    println!("    kizuna test-strategy mdns --verbose");
    println!("    kizuna benchmark --iterations 5 --timeout 3");
    println!("    kizuna config init");
    println!("    kizuna config show");
    println!();
    println!("For more detailed configuration options, run:");
    println!("    kizuna config sample");
}
