use anyhow::Result;
use std::env;
use std::time::Duration;

mod discovery;
use discovery::{
    Discovery, DiscoveryManager, discovery_selector,
    strategies::{udp::UdpDiscovery, mdns::MdnsDiscovery},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("auto");

    match mode {
        "discover" => {
            // Use the new enhanced discovery system
            let mut manager = DiscoveryManager::new();
            manager.add_strategy(Box::new(UdpDiscovery::new()));
            manager.add_strategy(Box::new(MdnsDiscovery::new()));
            
            println!("Starting discovery with enhanced system...");
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
            // Enhanced auto mode using DiscoveryManager
            let mut manager = DiscoveryManager::new();
            manager.add_strategy(Box::new(UdpDiscovery::new()));
            manager.add_strategy(Box::new(MdnsDiscovery::new()));
            
            println!("Auto-selecting best discovery strategy...");
            
            match manager.discover_peers(Duration::from_secs(5)).await {
                Ok(peers) => {
                    println!("(auto) Found {} peers", peers.len());
                    for peer in peers {
                        println!("- {} @ {:?} via {}", 
                            peer.peer_id, peer.addresses, peer.discovery_method);
                    }
                }
                Err(e) => {
                    eprintln!("(auto) Enhanced discovery failed: {}; falling back to legacy UDP", e);
                    let d = UdpDiscovery::new();
                    let peers = d.browse().await?;
                    println!("(auto) UDP fallback: found {} peers", peers.len());
                    for p in peers { println!("- {} @ {}:{}", p.id, p.addr, p.port); }
                }
            }
        }
    }

    Ok(())
}
