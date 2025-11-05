use crate::discovery::{Discovery, DiscoveryError, ServiceRecord};
use async_trait::async_trait;
use std::time::Duration;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::SystemTime;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use futures::StreamExt;

use libp2p::{
    kad::{Kademlia, KademliaEvent, QueryResult},
    mdns::{Mdns, MdnsEvent},
    swarm::{SwarmBuilder, SwarmEvent, NetworkBehaviour},
    identity, noise, yamux, tcp, relay, dcutr,
    Multiaddr, PeerId, Transport,
};
use libp2p_swarm::Swarm;

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Libp2pEvent")]
struct Libp2pBehaviour {
    kademlia: Kademlia<libp2p::kad::store::MemoryStore>,
    mdns: Mdns,
    relay: relay::Behaviour,
    dcutr: dcutr::Behaviour,
}

#[derive(Debug)]
enum Libp2pEvent {
    Kademlia(KademliaEvent),
    Mdns(MdnsEvent),
    Relay(relay::Event),
    Dcutr(dcutr::Event),
}

impl From<KademliaEvent> for Libp2pEvent {
    fn from(event: KademliaEvent) -> Self {
        Libp2pEvent::Kademlia(event)
    }
}

impl From<MdnsEvent> for Libp2pEvent {
    fn from(event: MdnsEvent) -> Self {
        Libp2pEvent::Mdns(event)
    }
}

impl From<relay::Event> for Libp2pEvent {
    fn from(event: relay::Event) -> Self {
        Libp2pEvent::Relay(event)
    }
}

impl From<dcutr::Event> for Libp2pEvent {
    fn from(event: dcutr::Event) -> Self {
        Libp2pEvent::Dcutr(event)
    }
}

pub struct Libp2pDiscovery {
    peer_id: PeerId,
    device_name: String,
    swarm: Option<Arc<RwLock<Swarm<Libp2pBehaviour>>>>,
    discovered_peers: Arc<RwLock<HashMap<String, ServiceRecord>>>,
    bootstrap_nodes: Vec<Multiaddr>,
    is_running: Arc<RwLock<bool>>,
}

impl Libp2pDiscovery {
    pub fn new() -> Result<Self, DiscoveryError> {
        let local_key = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(local_key.public());
        
        Ok(Self {
            peer_id,
            device_name: "Kizuna Device".to_string(),
            swarm: None,
            discovered_peers: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_nodes: Vec::new(),
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    pub fn with_config(device_name: String, bootstrap_nodes: Vec<Multiaddr>) -> Result<Self, DiscoveryError> {
        let local_key = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(local_key.public());
        
        Ok(Self {
            peer_id,
            device_name,
            swarm: None,
            discovered_peers: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_nodes,
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    async fn initialize_swarm(&mut self) -> Result<(), DiscoveryError> {
        let local_key = identity::Keypair::generate_ed25519();
        
        // Create transport with relay support
        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::Config::new(&local_key).map_err(|e| {
                DiscoveryError::Libp2p(format!("Failed to create noise config: {}", e))
            })?)
            .multiplex(yamux::Config::default())
            .boxed();

        // Create Kademlia store and behaviour with custom configuration
        let store = libp2p::kad::store::MemoryStore::new(self.peer_id);
        let mut kademlia_config = libp2p::kad::KademliaConfig::default();
        kademlia_config.set_query_timeout(Duration::from_secs(60));
        kademlia_config.set_replication_factor(std::num::NonZeroUsize::new(3).unwrap());
        
        let mut kademlia = Kademlia::with_config(self.peer_id, store, kademlia_config);
        
        // Configure Kademlia for Kizuna protocol
        kademlia.set_mode(Some(libp2p::kad::Mode::Server));
        
        // Add bootstrap nodes to Kademlia
        for addr in &self.bootstrap_nodes {
            if let Some(peer_id) = self.extract_peer_id_from_multiaddr(addr) {
                kademlia.add_address(&peer_id, addr.clone());
            }
        }

        // Create mDNS behaviour with custom configuration for Kizuna
        let mdns_config = libp2p::mdns::Config {
            ttl: Duration::from_secs(300), // 5 minutes TTL
            query_interval: Duration::from_secs(30), // Query every 30 seconds
            enable_ipv6: true,
        };
        let mdns = Mdns::new(mdns_config).await.map_err(|e| {
            DiscoveryError::Libp2p(format!("Failed to create mDNS: {}", e))
        })?;

        // Create relay behaviour for NAT traversal
        let relay_config = relay::Config::default();
        let relay = relay::Behaviour::new(self.peer_id, relay_config);
        
        // Create dcutr behaviour for direct connection upgrade
        let dcutr = dcutr::Behaviour::new(self.peer_id);

        // Create network behaviour
        let behaviour = Libp2pBehaviour {
            kademlia,
            mdns,
            relay,
            dcutr,
        };

        // Create swarm with connection limits
        let swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, self.peer_id)
            .connection_limits(
                libp2p::swarm::ConnectionLimits::default()
                    .with_max_pending_incoming(Some(10))
                    .with_max_pending_outgoing(Some(20))
                    .with_max_established_incoming(Some(50))
                    .with_max_established_outgoing(Some(100))
            )
            .build();

        self.swarm = Some(Arc::new(RwLock::new(swarm)));
        Ok(())
    }

    fn extract_peer_id_from_multiaddr(&self, addr: &Multiaddr) -> Option<PeerId> {
        addr.iter().find_map(|p| {
            if let libp2p::multiaddr::Protocol::P2p(hash) = p {
                PeerId::from_multihash(hash).ok()
            } else {
                None
            }
        })
    }

    async fn setup_bootstrap_connections(&self) -> Result<(), DiscoveryError> {
        if let Some(swarm_arc) = &self.swarm {
            let mut swarm = swarm_arc.write().await;
            
            // Connect to bootstrap nodes
            for addr in &self.bootstrap_nodes {
                if let Some(peer_id) = self.extract_peer_id_from_multiaddr(addr) {
                    // Add address to Kademlia
                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                    
                    // Attempt to dial the bootstrap node
                    if let Err(e) = swarm.dial(addr.clone()) {
                        eprintln!("Failed to dial bootstrap node {}: {}", addr, e);
                    }
                }
            }
            
            // Bootstrap the Kademlia DHT
            if !self.bootstrap_nodes.is_empty() {
                if let Err(e) = swarm.behaviour_mut().kademlia.bootstrap() {
                    return Err(DiscoveryError::Libp2p(format!("Failed to bootstrap Kademlia: {}", e)));
                }
            }
        }
        Ok(())
    }

    async fn manage_connection_lifecycle(&self) -> Result<(), DiscoveryError> {
        if let Some(swarm_arc) = &self.swarm {
            let mut swarm = swarm_arc.write().await;
            
            // Set up periodic maintenance tasks
            let kademlia = &mut swarm.behaviour_mut().kademlia;
            
            // Add ourselves to the DHT
            kademlia.get_closest_peers(self.peer_id);
            
            // Start periodic refresh of routing table
            tokio::spawn({
                let swarm_clone = swarm_arc.clone();
                let peer_id = self.peer_id;
                async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
                    loop {
                        interval.tick().await;
                        if let Ok(mut swarm) = swarm_clone.try_write() {
                            let _ = swarm.behaviour_mut().kademlia.get_closest_peers(peer_id);
                        }
                    }
                }
            });
        }
        Ok(())
    }

    async fn start_listening(&self) -> Result<(), DiscoveryError> {
        if let Some(swarm_arc) = &self.swarm {
            let mut swarm = swarm_arc.write().await;
            
            // Listen on multiple interfaces and protocols
            let listen_addresses = vec![
                "/ip4/0.0.0.0/tcp/0",
                "/ip6/::/tcp/0",
            ];
            
            for addr_str in listen_addresses {
                if let Ok(addr) = addr_str.parse() {
                    if let Err(e) = swarm.listen_on(addr) {
                        eprintln!("Failed to listen on {}: {}", addr_str, e);
                    }
                }
            }
            
            // Drop the write lock before calling other async methods
            drop(swarm);
        }
        
        // Set up bootstrap connections
        self.setup_bootstrap_connections().await?;
        
        // Initialize connection lifecycle management
        self.manage_connection_lifecycle().await?;
        
        Ok(())
    }

    async fn handle_swarm_events(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        if let Some(swarm_arc) = &self.swarm {
            let mut swarm = swarm_arc.write().await;
            let start_time = SystemTime::now();
            
            loop {
                if start_time.elapsed().unwrap_or(Duration::ZERO) >= timeout {
                    break;
                }

                tokio::select! {
                    event = swarm.select_next_some() => {
                        match event {
                            SwarmEvent::Behaviour(Libp2pEvent::Mdns(MdnsEvent::Discovered(list))) => {
                                println!("mDNS discovered {} peers", list.len());
                                for (peer_id, multiaddr) in list {
                                    self.add_discovered_peer(peer_id, multiaddr).await;
                                }
                            }
                            SwarmEvent::Behaviour(Libp2pEvent::Mdns(MdnsEvent::Expired(list))) => {
                                println!("mDNS expired {} peers", list.len());
                                for (peer_id, _) in list {
                                    let mut peers = self.discovered_peers.write().await;
                                    peers.remove(&peer_id.to_string());
                                }
                            }
                            SwarmEvent::Behaviour(Libp2pEvent::Kademlia(kad_event)) => {
                                self.handle_kademlia_event(kad_event).await;
                            }
                            SwarmEvent::Behaviour(Libp2pEvent::Relay(relay_event)) => {
                                println!("Relay event: {:?}", relay_event);
                            }
                            SwarmEvent::Behaviour(Libp2pEvent::Dcutr(dcutr_event)) => {
                                println!("DCUtR event: {:?}", dcutr_event);
                            }
                            SwarmEvent::NewListenAddr { address, .. } => {
                                println!("libp2p listening on: {}", address);
                            }
                            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                                println!("Connection established with peer: {} at {}", peer_id, endpoint.get_remote_address());
                                // Add connected peer to Kademlia routing table
                                swarm.behaviour_mut().kademlia.add_address(&peer_id, endpoint.get_remote_address().clone());
                            }
                            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                                println!("Connection closed with peer: {} (cause: {:?})", peer_id, cause);
                            }
                            SwarmEvent::IncomingConnection { .. } => {
                                println!("Incoming connection");
                            }
                            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                                if let Some(peer_id) = peer_id {
                                    println!("Outgoing connection error to {}: {}", peer_id, error);
                                }
                            }
                            _ => {}
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        // Continue polling
                    }
                }
            }
        }

        let peers = self.discovered_peers.read().await;
        Ok(peers.values().cloned().collect())
    }

    async fn handle_kademlia_event(&self, event: KademliaEvent) {
        match event {
            KademliaEvent::OutboundQueryProgressed { result, .. } => {
                match result {
                    QueryResult::GetClosestPeers(Ok(peers_result)) => {
                        println!("Kademlia found {} closest peers", peers_result.peers.len());
                        for peer in peers_result.peers {
                            let service_record = ServiceRecord {
                                peer_id: peer.to_string(),
                                name: format!("kademlia-peer-{}", &peer.to_string()[..8]),
                                addresses: vec![], // Will be populated when we connect
                                port: 0, // libp2p handles port management
                                discovery_method: "libp2p-kademlia".to_string(),
                                capabilities: {
                                    let mut caps = HashMap::new();
                                    caps.insert("protocol".to_string(), "kademlia".to_string());
                                    caps
                                },
                                last_seen: SystemTime::now(),
                            };
                            
                            let mut peers = self.discovered_peers.write().await;
                            peers.insert(peer.to_string(), service_record);
                        }
                    }
                    QueryResult::Bootstrap(Ok(bootstrap_result)) => {
                        println!("Kademlia bootstrap completed with {} peers", bootstrap_result.num_remaining);
                    }
                    QueryResult::GetRecord(Ok(record_result)) => {
                        println!("Kademlia record retrieved: {:?}", record_result.record.key);
                    }
                    _ => {}
                }
            }
            KademliaEvent::RoutingUpdated { peer, .. } => {
                println!("Kademlia routing table updated with peer: {}", peer);
            }
            KademliaEvent::UnroutablePeer { peer } => {
                println!("Kademlia unroutable peer: {}", peer);
            }
            KademliaEvent::RoutablePeer { peer, address } => {
                println!("Kademlia routable peer: {} at {}", peer, address);
            }
            _ => {}
        }
    }

    async fn add_discovered_peer(&self, peer_id: PeerId, multiaddr: Multiaddr) {
        // Extract IP address and port from multiaddr
        let mut ip_addr = None;
        let mut port = 0u16;
        
        for protocol in multiaddr.iter() {
            match protocol {
                libp2p::multiaddr::Protocol::Ip4(addr) => {
                    ip_addr = Some(std::net::IpAddr::V4(addr));
                }
                libp2p::multiaddr::Protocol::Ip6(addr) => {
                    ip_addr = Some(std::net::IpAddr::V6(addr));
                }
                libp2p::multiaddr::Protocol::Tcp(p) => {
                    port = p;
                }
                _ => {}
            }
        }

        let socket_addr = if let Some(ip) = ip_addr {
            vec![SocketAddr::new(ip, port)]
        } else {
            vec![]
        };

        let service_record = ServiceRecord {
            peer_id: peer_id.to_string(),
            name: format!("libp2p-peer-{}", &peer_id.to_string()[..8]),
            addresses: socket_addr,
            port,
            discovery_method: "libp2p".to_string(),
            capabilities: {
                let mut caps = HashMap::new();
                caps.insert("protocol".to_string(), "libp2p".to_string());
                caps.insert("multiaddr".to_string(), multiaddr.to_string());
                caps
            },
            last_seen: SystemTime::now(),
        };

        let mut peers = self.discovered_peers.write().await;
        peers.insert(peer_id.to_string(), service_record);
    }
}

#[async_trait]
impl Discovery for Libp2pDiscovery {
    async fn discover(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        // Initialize swarm if not already done
        if self.swarm.is_none() {
            return Err(DiscoveryError::StrategyUnavailable {
                strategy: "libp2p".to_string(),
            });
        }

        // Clear previous discoveries
        {
            let mut peers = self.discovered_peers.write().await;
            peers.clear();
        }

        // Start discovery process
        if let Some(swarm_arc) = &self.swarm {
            let mut swarm = swarm_arc.write().await;
            
            // Start Kademlia query for closest peers
            let _ = swarm.behaviour_mut().kademlia.get_closest_peers(self.peer_id);
            
            // Query for random keys to discover more peers in the network
            for _ in 0..3 {
                let random_key = libp2p::kad::RecordKey::new(&uuid::Uuid::new_v4().to_string());
                let _ = swarm.behaviour_mut().kademlia.get_record(random_key);
            }
            
            // Drop the write lock before handling events
            drop(swarm);
        }

        // Trigger additional DHT queries
        self.trigger_dht_query().await?;

        // Handle events and collect discovered peers
        self.handle_swarm_events(timeout).await
    }

    async fn announce(&self) -> Result<(), DiscoveryError> {
        // Check if swarm is initialized
        if self.swarm.is_none() {
            return Err(DiscoveryError::StrategyUnavailable {
                strategy: "libp2p - swarm not initialized".to_string(),
            });
        }

        // Start listening and announcing
        self.start_listening().await?;
        
        // Mark as running
        {
            let mut running = self.is_running.write().await;
            *running = true;
        }

        Ok(())
    }

    async fn stop_announce(&self) -> Result<(), DiscoveryError> {
        // Mark as not running
        {
            let mut running = self.is_running.write().await;
            *running = false;
        }

        // The swarm will be dropped when the struct is dropped
        Ok(())
    }

    fn strategy_name(&self) -> &'static str {
        "libp2p"
    }

    fn is_available(&self) -> bool {
        // libp2p should be available on most platforms
        // We could add more sophisticated checks here
        true
    }

    fn priority(&self) -> u8 {
        // Medium-high priority - good for global discovery
        60
    }
}

impl Libp2pDiscovery {
    pub async fn initialize(&mut self) -> Result<(), DiscoveryError> {
        if self.swarm.is_none() {
            self.initialize_swarm().await?;
        }
        Ok(())
    }

    pub fn add_bootstrap_node(&mut self, addr: Multiaddr) {
        self.bootstrap_nodes.push(addr);
    }

    pub fn set_bootstrap_nodes(&mut self, nodes: Vec<Multiaddr>) {
        self.bootstrap_nodes = nodes;
    }

    pub fn get_default_bootstrap_nodes() -> Vec<Multiaddr> {
        // These are example bootstrap nodes - in a real implementation,
        // you would use actual Kizuna bootstrap nodes
        vec![
            // Example bootstrap nodes (these would be real Kizuna nodes in production)
            "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ".parse().unwrap_or_else(|_| "/ip4/127.0.0.1/tcp/4001".parse().unwrap()),
        ]
    }

    pub async fn trigger_dht_query(&self) -> Result<(), DiscoveryError> {
        if let Some(swarm_arc) = &self.swarm {
            let mut swarm = swarm_arc.write().await;
            
            // Trigger a DHT query to find peers
            let _ = swarm.behaviour_mut().kademlia.get_closest_peers(self.peer_id);
            
            // Also query for a random key to discover more peers
            let random_key = libp2p::kad::RecordKey::new(&uuid::Uuid::new_v4().to_string());
            let _ = swarm.behaviour_mut().kademlia.get_record(random_key);
        }
        Ok(())
    }
}

impl Default for Libp2pDiscovery {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback implementation if new() fails
            let local_key = identity::Keypair::generate_ed25519();
            let peer_id = PeerId::from(local_key.public());
            
            Self {
                peer_id,
                device_name: "Kizuna Device".to_string(),
                swarm: None,
                discovered_peers: Arc::new(RwLock::new(HashMap::new())),
                bootstrap_nodes: Vec::new(),
                is_running: Arc::new(RwLock::new(false)),
            }
        })
    }
}
#[cfg(te
st)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    #[test]
    fn test_libp2p_discovery_creation() {
        let discovery = Libp2pDiscovery::new().unwrap();
        
        assert_eq!(discovery.strategy_name(), "libp2p");
        assert_eq!(discovery.device_name, "Kizuna Device");
        assert_eq!(discovery.priority(), 60);
        assert!(discovery.is_available());
        assert!(discovery.bootstrap_nodes.is_empty());
    }

    #[test]
    fn test_libp2p_discovery_with_config() {
        let bootstrap_nodes = vec![
            "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWTest".parse().unwrap(),
        ];
        let discovery = Libp2pDiscovery::with_config(
            "Test Libp2p Device".to_string(),
            bootstrap_nodes.clone(),
        ).unwrap();
        
        assert_eq!(discovery.device_name, "Test Libp2p Device");
        assert_eq!(discovery.bootstrap_nodes.len(), 1);
        assert_eq!(discovery.bootstrap_nodes[0], bootstrap_nodes[0]);
    }

    #[test]
    fn test_default_bootstrap_nodes() {
        let nodes = Libp2pDiscovery::get_default_bootstrap_nodes();
        assert!(!nodes.is_empty());
        
        // Verify the nodes are valid multiaddresses
        for node in nodes {
            assert!(node.to_string().contains("/ip4/"));
        }
    }

    #[test]
    fn test_add_bootstrap_node() {
        let mut discovery = Libp2pDiscovery::new().unwrap();
        let node: Multiaddr = "/ip4/192.168.1.100/tcp/4001/p2p/12D3KooWTest".parse().unwrap();
        
        discovery.add_bootstrap_node(node.clone());
        assert_eq!(discovery.bootstrap_nodes.len(), 1);
        assert_eq!(discovery.bootstrap_nodes[0], node);
    }

    #[test]
    fn test_set_bootstrap_nodes() {
        let mut discovery = Libp2pDiscovery::new().unwrap();
        let nodes = vec![
            "/ip4/192.168.1.100/tcp/4001/p2p/12D3KooWTest1".parse().unwrap(),
            "/ip4/192.168.1.101/tcp/4001/p2p/12D3KooWTest2".parse().unwrap(),
        ];
        
        discovery.set_bootstrap_nodes(nodes.clone());
        assert_eq!(discovery.bootstrap_nodes.len(), 2);
        assert_eq!(discovery.bootstrap_nodes, nodes);
    }

    #[test]
    fn test_extract_peer_id_from_multiaddr() {
        let discovery = Libp2pDiscovery::new().unwrap();
        
        // Test valid multiaddr with peer ID
        let addr_with_peer: Multiaddr = "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWTest".parse().unwrap();
        let peer_id = discovery.extract_peer_id_from_multiaddr(&addr_with_peer);
        assert!(peer_id.is_some());
        
        // Test multiaddr without peer ID
        let addr_without_peer: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().unwrap();
        let peer_id = discovery.extract_peer_id_from_multiaddr(&addr_without_peer);
        assert!(peer_id.is_none());
    }

    #[tokio::test]
    async fn test_libp2p_discovery_initialization() {
        let mut discovery = Libp2pDiscovery::new().unwrap();
        
        // Test initialization
        let result = discovery.initialize().await;
        assert!(result.is_ok());
        assert!(discovery.swarm.is_some());
    }

    #[tokio::test]
    async fn test_libp2p_discovery_announce_without_initialization() {
        let discovery = Libp2pDiscovery::new().unwrap();
        
        // Should fail if swarm is not initialized
        let result = discovery.announce().await;
        assert!(result.is_err());
        
        if let Err(DiscoveryError::StrategyUnavailable { strategy }) = result {
            assert!(strategy.contains("libp2p"));
        } else {
            panic!("Expected StrategyUnavailable error");
        }
    }

    #[tokio::test]
    async fn test_libp2p_discovery_discover_without_initialization() {
        let discovery = Libp2pDiscovery::new().unwrap();
        
        // Should fail if swarm is not initialized
        let result = discovery.discover(Duration::from_secs(1)).await;
        assert!(result.is_err());
        
        if let Err(DiscoveryError::StrategyUnavailable { strategy }) = result {
            assert_eq!(strategy, "libp2p");
        } else {
            panic!("Expected StrategyUnavailable error");
        }
    }

    #[tokio::test]
    async fn test_libp2p_discovery_stop_announce() {
        let discovery = Libp2pDiscovery::new().unwrap();
        
        // Should succeed even without initialization
        let result = discovery.stop_announce().await;
        assert!(result.is_ok());
        
        // Check that running flag is set to false
        let is_running = *discovery.is_running.read().await;
        assert!(!is_running);
    }

    #[tokio::test]
    async fn test_add_discovered_peer() {
        let discovery = Libp2pDiscovery::new().unwrap();
        let peer_id = PeerId::random();
        let multiaddr: Multiaddr = "/ip4/192.168.1.100/tcp/4001".parse().unwrap();
        
        // Add a discovered peer
        discovery.add_discovered_peer(peer_id, multiaddr.clone()).await;
        
        // Verify the peer was added
        let peers = discovery.discovered_peers.read().await;
        assert_eq!(peers.len(), 1);
        
        let service_record = peers.get(&peer_id.to_string()).unwrap();
        assert_eq!(service_record.peer_id, peer_id.to_string());
        assert_eq!(service_record.discovery_method, "libp2p");
        assert_eq!(service_record.port, 4001);
        assert!(!service_record.addresses.is_empty());
        assert!(service_record.capabilities.contains_key("protocol"));
        assert_eq!(service_record.capabilities.get("protocol"), Some(&"libp2p".to_string()));
        assert!(service_record.capabilities.contains_key("multiaddr"));
    }

    #[tokio::test]
    async fn test_trigger_dht_query_without_swarm() {
        let discovery = Libp2pDiscovery::new().unwrap();
        
        // Should not panic even without initialized swarm
        let result = discovery.trigger_dht_query().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_peer_id_generation() {
        let discovery1 = Libp2pDiscovery::new().unwrap();
        let discovery2 = Libp2pDiscovery::new().unwrap();
        
        // Each instance should have a unique peer ID
        assert_ne!(discovery1.peer_id, discovery2.peer_id);
    }

    #[test]
    fn test_default_implementation() {
        let discovery = Libp2pDiscovery::default();
        
        assert_eq!(discovery.strategy_name(), "libp2p");
        assert_eq!(discovery.device_name, "Kizuna Device");
        assert!(discovery.is_available());
    }

    #[tokio::test]
    async fn test_concurrent_peer_access() {
        let discovery = Arc::new(Libp2pDiscovery::new().unwrap());
        let peer_id = PeerId::random();
        let multiaddr: Multiaddr = "/ip4/192.168.1.100/tcp/4001".parse().unwrap();
        
        // Test concurrent access to discovered peers
        let discovery_clone = discovery.clone();
        let peer_id_clone = peer_id;
        let multiaddr_clone = multiaddr.clone();
        
        let handle = tokio::spawn(async move {
            discovery_clone.add_discovered_peer(peer_id_clone, multiaddr_clone).await;
        });
        
        // Add another peer concurrently
        let peer_id2 = PeerId::random();
        let multiaddr2: Multiaddr = "/ip4/192.168.1.101/tcp/4001".parse().unwrap();
        discovery.add_discovered_peer(peer_id2, multiaddr2).await;
        
        // Wait for the first task to complete
        handle.await.unwrap();
        
        // Verify both peers were added
        let peers = discovery.discovered_peers.read().await;
        assert_eq!(peers.len(), 2);
    }
}