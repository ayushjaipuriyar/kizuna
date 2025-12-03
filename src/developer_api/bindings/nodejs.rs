/// Node.js bindings using NAPI
/// Provides Promise-based JavaScript API for Kizuna functionality
/// 
/// This module provides comprehensive Node.js bindings with:
/// - Promise-based async API
/// - Event loop integration via NAPI
/// - Callback management for real-time events
/// - Thread-safe access to Kizuna instance

#[cfg(feature = "nodejs")]
pub mod napi_bindings {
    use napi::bindgen_prelude::*;
    use napi::threadsafe_function::{ThreadsafeFunction, ThreadSafeFunctionCallMode, ErrorStrategy};
    use napi_derive::napi;
    use std::sync::Arc;
    use tokio::sync::{RwLock, Mutex};
    use futures::StreamExt;
    
    use crate::developer_api::core::{
        KizunaAPI, KizunaConfig, KizunaError, KizunaEvent, KizunaInstance,
        api::{StreamConfig, CommandResult as CoreCommandResult, PeerConnection as CorePeerConnection},
        events::{PeerId, PeerInfo, TransferId, StreamId, TransferProgress, TransferResult, StreamType, TransferDirection},
    };
    
    /// Main Kizuna API class for Node.js
    /// 
    /// Provides thread-safe access to Kizuna functionality with Promise-based API
    /// and event callback support for real-time notifications.
    #[napi]
    pub struct Kizuna {
        instance: Arc<RwLock<Option<KizunaInstance>>>,
        event_callback: Arc<Mutex<Option<ThreadsafeFunction<Event, ErrorStrategy::Fatal>>>>,
        event_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    }
    
    /// Configuration for Kizuna initialization
    #[napi(object)]
    pub struct Config {
        /// Device name
        pub device_name: Option<String>,
        /// User name
        pub user_name: Option<String>,
        /// Enable mDNS discovery
        pub enable_mdns: Option<bool>,
        /// Enable UDP discovery
        pub enable_udp: Option<bool>,
        /// Enable Bluetooth discovery
        pub enable_bluetooth: Option<bool>,
        /// Enable encryption
        pub enable_encryption: Option<bool>,
        /// Require authentication
        pub require_authentication: Option<bool>,
        /// Listen port
        pub listen_port: Option<u16>,
    }
    
    /// Peer information
    #[napi(object)]
    pub struct Peer {
        /// Peer ID
        pub id: String,
        /// Peer name
        pub name: String,
        /// Peer addresses
        pub addresses: Vec<String>,
        /// Peer capabilities
        pub capabilities: Vec<String>,
        /// Discovery method
        pub discovery_method: String,
    }
    
    /// Transfer information
    #[napi(object)]
    pub struct Transfer {
        /// Transfer ID
        pub id: String,
        /// File name
        pub file_name: String,
        /// File size in bytes
        pub file_size: f64,
        /// Peer ID
        pub peer_id: String,
        /// Transfer direction ("send" or "receive")
        pub direction: String,
    }
    
    /// Transfer progress
    #[napi(object)]
    pub struct Progress {
        /// Transfer ID
        pub id: String,
        /// Bytes transferred
        pub bytes_transferred: f64,
        /// Total bytes
        pub total_bytes: f64,
        /// Transfer speed in bytes per second
        pub speed_bps: f64,
        /// Progress percentage
        pub percentage: f64,
    }
    
    /// Transfer result
    #[napi(object)]
    pub struct TransferResultJs {
        /// Transfer ID
        pub id: String,
        /// Success status
        pub success: bool,
        /// Error message if failed
        pub error: Option<String>,
        /// Bytes transferred
        pub bytes_transferred: f64,
        /// Duration in milliseconds
        pub duration_ms: f64,
    }
    
    /// Stream configuration
    #[napi(object)]
    pub struct StreamConfigJs {
        /// Stream type ("camera", "screen", or "audio")
        pub stream_type: String,
        /// Target peer ID
        pub peer_id: String,
        /// Video quality (0-100)
        pub quality: u8,
    }
    
    /// Stream information
    #[napi(object)]
    pub struct Stream {
        /// Stream ID
        pub id: String,
        /// Stream type
        pub stream_type: String,
        /// Peer ID
        pub peer_id: String,
    }
    
    /// Command execution result
    #[napi(object)]
    pub struct CommandResult {
        /// Command that was executed
        pub command: String,
        /// Peer ID
        pub peer_id: String,
        /// Exit code
        pub exit_code: i32,
        /// Standard output
        pub stdout: String,
        /// Standard error
        pub stderr: String,
    }
    
    /// Event emitted by Kizuna
    #[napi(object)]
    pub struct Event {
        /// Event type
        pub event_type: String,
        /// Event data as JSON string
        pub data: String,
    }
    
    /// Peer connection handle
    #[napi]
    pub struct PeerConnectionHandle {
        peer_id: String,
    }
    
    #[napi]
    impl PeerConnectionHandle {
        /// Gets the peer ID
        #[napi]
        pub fn peer_id(&self) -> String {
            self.peer_id.clone()
        }
    }
    
    /// Transfer handle for managing file transfers
    #[napi]
    pub struct TransferHandle {
        transfer_id: String,
    }
    
    #[napi]
    impl TransferHandle {
        /// Gets the transfer ID
        #[napi]
        pub fn transfer_id(&self) -> String {
            self.transfer_id.clone()
        }
        
        /// Cancels the transfer
        #[napi]
        pub async fn cancel(&self) -> Result<()> {
            // TODO: Implement transfer cancellation
            Ok(())
        }
    }
    
    /// Stream handle for managing media streams
    #[napi]
    pub struct StreamHandle {
        stream_id: String,
    }
    
    #[napi]
    impl StreamHandle {
        /// Gets the stream ID
        #[napi]
        pub fn stream_id(&self) -> String {
            self.stream_id.clone()
        }
        
        /// Stops the stream
        #[napi]
        pub async fn stop(&self) -> Result<()> {
            // TODO: Implement stream stopping
            Ok(())
        }
    }
    
    #[napi]
    impl Kizuna {
        /// Creates a new Kizuna instance
        /// 
        /// This constructor initializes the Kizuna API wrapper but does not
        /// start any services. Call `initialize()` to start Kizuna.
        #[napi(constructor)]
        pub fn new() -> Self {
            Self {
                instance: Arc::new(RwLock::new(None)),
                event_callback: Arc::new(Mutex::new(None)),
                event_task: Arc::new(Mutex::new(None)),
            }
        }
        
        /// Initializes Kizuna with the given configuration
        /// 
        /// Returns a Promise that resolves when initialization is complete.
        /// This method integrates with the Node.js event loop for proper async handling.
        /// 
        /// # Arguments
        /// * `config` - Configuration object with optional fields for customization
        /// 
        /// # Example
        /// ```javascript
        /// const kizuna = new Kizuna();
        /// await kizuna.initialize({
        ///   deviceName: 'My Device',
        ///   enableMdns: true,
        ///   enableEncryption: true
        /// });
        /// ```
        #[napi]
        pub async fn initialize(&self, config: Config) -> Result<()> {
            // Check if already initialized
            {
                let instance = self.instance.read().await;
                if instance.is_some() {
                    return Err(Error::from_reason("Kizuna is already initialized"));
                }
            }
            
            let kizuna_config = config_from_js(config)?;
            
            let instance = KizunaInstance::new(kizuna_config)
                .map_err(|e| Error::from_reason(format!("Failed to initialize: {}", e)))?;
            
            instance.initialize_systems().await
                .map_err(|e| Error::from_reason(format!("Failed to initialize systems: {}", e)))?;
            
            *self.instance.write().await = Some(instance);
            Ok(())
        }
        
        /// Registers an event callback for real-time event notifications
        /// 
        /// The callback will be invoked on the Node.js event loop whenever
        /// a Kizuna event occurs (peer discovered, transfer progress, etc.)
        /// 
        /// # Arguments
        /// * `callback` - JavaScript function to call with event data
        /// 
        /// # Example
        /// ```javascript
        /// kizuna.onEvent((event) => {
        ///   console.log('Event:', event.eventType, event.data);
        /// });
        /// ```
        #[napi]
        pub async fn on_event(&self, callback: JsFunction) -> Result<()> {
            let instance = self.instance.read().await;
            let instance = instance.as_ref()
                .ok_or_else(|| Error::from_reason("Kizuna not initialized"))?;
            
            // Create threadsafe function for callback
            let tsfn: ThreadsafeFunction<Event, ErrorStrategy::Fatal> = callback
                .create_threadsafe_function(0, |ctx| {
                    Ok(vec![ctx.value])
                })?;
            
            // Store the callback
            *self.event_callback.lock().await = Some(tsfn.clone());
            
            // Subscribe to events and spawn task to forward them to callback
            let event_stream = instance.subscribe_events().await
                .map_err(|e| Error::from_reason(format!("Failed to subscribe to events: {}", e)))?;
            
            let task = tokio::spawn(async move {
                let mut stream = event_stream;
                while let Some(event) = stream.next().await {
                    let js_event = event_to_js(event);
                    // Call the JavaScript callback on the Node.js event loop
                    let _ = tsfn.call(js_event, ThreadSafeFunctionCallMode::NonBlocking);
                }
            });
            
            *self.event_task.lock().await = Some(task);
            
            Ok(())
        }
        
        /// Discovers peers on the network
        /// 
        /// Returns a Promise that resolves with an array of discovered peers.
        /// For real-time peer discovery, use the `onEvent` callback to receive
        /// PeerDiscovered events as they occur.
        /// 
        /// # Example
        /// ```javascript
        /// const peers = await kizuna.discoverPeers();
        /// peers.forEach(peer => {
        ///   console.log('Found peer:', peer.name, peer.id);
        /// });
        /// ```
        #[napi]
        pub async fn discover_peers(&self) -> Result<Vec<Peer>> {
            let instance = self.instance.read().await;
            let instance = instance.as_ref()
                .ok_or_else(|| Error::from_reason("Kizuna not initialized"))?;
            
            // Get peer stream from discovery
            let mut peer_stream = instance.discover_peers().await
                .map_err(|e| Error::from_reason(format!("Failed to discover peers: {}", e)))?;
            
            // Collect peers with timeout to avoid blocking indefinitely
            let mut peers = Vec::new();
            let timeout = std::time::Duration::from_secs(5);
            let deadline = tokio::time::Instant::now() + timeout;
            
            while tokio::time::Instant::now() < deadline {
                match tokio::time::timeout_at(deadline, peer_stream.next()).await {
                    Ok(Some(peer_info)) => {
                        peers.push(peer_to_js(peer_info));
                    }
                    Ok(None) => break, // Stream ended
                    Err(_) => break,   // Timeout
                }
            }
            
            Ok(peers)
        }
        
        /// Connects to a peer
        #[napi]
        pub async fn connect_to_peer(&self, peer_id: String) -> Result<PeerConnectionHandle> {
            let instance = self.instance.read().await;
            let instance = instance.as_ref()
                .ok_or_else(|| Error::from_reason("Kizuna not initialized"))?;
            
            let peer_id_obj = PeerId::from(peer_id.clone());
            let _connection = instance.connect_to_peer(peer_id_obj).await
                .map_err(|e| Error::from_reason(format!("Failed to connect: {}", e)))?;
            
            Ok(PeerConnectionHandle { peer_id })
        }
        
        /// Transfers a file to a peer
        #[napi]
        pub async fn transfer_file(&self, file_path: String, peer_id: String) -> Result<TransferHandle> {
            let instance = self.instance.read().await;
            let instance = instance.as_ref()
                .ok_or_else(|| Error::from_reason("Kizuna not initialized"))?;
            
            let path = std::path::PathBuf::from(file_path);
            let peer_id_obj = PeerId::from(peer_id);
            
            let handle = instance.transfer_file(path, peer_id_obj).await
                .map_err(|e| Error::from_reason(format!("Failed to transfer file: {}", e)))?;
            
            Ok(TransferHandle {
                transfer_id: handle.transfer_id().to_string(),
            })
        }
        
        /// Starts a media stream
        #[napi]
        pub async fn start_stream(&self, config: StreamConfigJs) -> Result<StreamHandle> {
            let instance = self.instance.read().await;
            let instance = instance.as_ref()
                .ok_or_else(|| Error::from_reason("Kizuna not initialized"))?;
            
            let stream_type = match config.stream_type.as_str() {
                "camera" => StreamType::Camera,
                "screen" => StreamType::Screen,
                "audio" => StreamType::Audio,
                _ => return Err(Error::from_reason("Invalid stream type")),
            };
            
            let stream_config = StreamConfig {
                stream_type,
                peer_id: PeerId::from(config.peer_id),
                quality: config.quality,
            };
            
            let handle = instance.start_stream(stream_config).await
                .map_err(|e| Error::from_reason(format!("Failed to start stream: {}", e)))?;
            
            Ok(StreamHandle {
                stream_id: handle.stream_id().to_string(),
            })
        }
        
        /// Executes a command on a peer
        #[napi]
        pub async fn execute_command(&self, command: String, peer_id: String) -> Result<CommandResult> {
            let instance = self.instance.read().await;
            let instance = instance.as_ref()
                .ok_or_else(|| Error::from_reason("Kizuna not initialized"))?;
            
            let peer_id_obj = PeerId::from(peer_id.clone());
            let result = instance.execute_command(command.clone(), peer_id_obj).await
                .map_err(|e| Error::from_reason(format!("Failed to execute command: {}", e)))?;
            
            Ok(CommandResult {
                command,
                peer_id,
                exit_code: result.exit_code,
                stdout: result.stdout,
                stderr: result.stderr,
            })
        }
        
        /// Subscribes to events (returns event data as they occur)
        #[napi]
        pub async fn get_next_event(&self) -> Result<Option<Event>> {
            let instance = self.instance.read().await;
            let _instance = instance.as_ref()
                .ok_or_else(|| Error::from_reason("Kizuna not initialized"))?;
            
            // TODO: Implement event streaming
            // For now, return None to indicate no events
            Ok(None)
        }
        
        /// Shuts down the Kizuna instance
        /// 
        /// Performs graceful shutdown of all Kizuna systems and cleans up resources.
        /// This includes stopping event callbacks and canceling background tasks.
        /// 
        /// # Example
        /// ```javascript
        /// await kizuna.shutdown();
        /// ```
        #[napi]
        pub async fn shutdown(&self) -> Result<()> {
            // Stop event task first
            {
                let mut event_task = self.event_task.lock().await;
                if let Some(task) = event_task.take() {
                    task.abort();
                }
            }
            
            // Clear event callback
            {
                let mut callback = self.event_callback.lock().await;
                *callback = None;
            }
            
            // Shutdown the instance
            let instance = self.instance.read().await;
            if let Some(instance) = instance.as_ref() {
                instance.shutdown().await
                    .map_err(|e| Error::from_reason(format!("Failed to shutdown: {}", e)))?;
            }
            
            drop(instance);
            *self.instance.write().await = None;
            Ok(())
        }
        
        /// Checks if Kizuna is initialized and ready
        /// 
        /// Returns true if the instance is initialized and ready for use.
        #[napi]
        pub async fn is_initialized(&self) -> Result<bool> {
            let instance = self.instance.read().await;
            Ok(instance.is_some())
        }
    }
    
    /// Converts JavaScript config to Rust config
    fn config_from_js(config: Config) -> Result<KizunaConfig> {
        use crate::developer_api::core::config::{
            IdentityConfig, DiscoveryConfig, SecurityConfig, NetworkConfig,
        };
        
        let mut kizuna_config = KizunaConfig::default();
        
        // Set identity if provided
        if let Some(device_name) = config.device_name {
            kizuna_config.identity = Some(IdentityConfig {
                device_name,
                user_name: config.user_name,
                identity_path: None,
            });
        }
        
        // Set discovery config
        if let Some(enable_mdns) = config.enable_mdns {
            kizuna_config.discovery.enable_mdns = enable_mdns;
        }
        if let Some(enable_udp) = config.enable_udp {
            kizuna_config.discovery.enable_udp = enable_udp;
        }
        if let Some(enable_bluetooth) = config.enable_bluetooth {
            kizuna_config.discovery.enable_bluetooth = enable_bluetooth;
        }
        
        // Set security config
        if let Some(enable_encryption) = config.enable_encryption {
            kizuna_config.security.enable_encryption = enable_encryption;
        }
        if let Some(require_authentication) = config.require_authentication {
            kizuna_config.security.require_authentication = require_authentication;
        }
        
        // Set networking config
        if let Some(listen_port) = config.listen_port {
            kizuna_config.networking.listen_port = Some(listen_port);
        }
        
        Ok(kizuna_config)
    }
    
    /// Converts Rust PeerInfo to JavaScript Peer
    fn peer_to_js(peer: PeerInfo) -> Peer {
        Peer {
            id: peer.id.0,
            name: peer.name,
            addresses: peer.addresses,
            capabilities: peer.capabilities,
            discovery_method: peer.discovery_method,
        }
    }
    
    /// Converts Rust KizunaEvent to JavaScript Event
    fn event_to_js(event: KizunaEvent) -> Event {
        match event {
            KizunaEvent::PeerDiscovered(peer) => {
                let data = serde_json::to_string(&peer_to_js(peer))
                    .unwrap_or_else(|_| "{}".to_string());
                Event {
                    event_type: "peer_discovered".to_string(),
                    data,
                }
            }
            KizunaEvent::PeerConnected(peer_id) => {
                let data = serde_json::json!({ "peerId": peer_id.0 }).to_string();
                Event {
                    event_type: "peer_connected".to_string(),
                    data,
                }
            }
            KizunaEvent::PeerDisconnected(peer_id) => {
                let data = serde_json::json!({ "peerId": peer_id.0 }).to_string();
                Event {
                    event_type: "peer_disconnected".to_string(),
                    data,
                }
            }
            KizunaEvent::TransferStarted(info) => {
                let transfer = Transfer {
                    id: info.id.0.to_string(),
                    file_name: info.file_name,
                    file_size: info.file_size as f64,
                    peer_id: info.peer_id.0,
                    direction: match info.direction {
                        TransferDirection::Send => "send".to_string(),
                        TransferDirection::Receive => "receive".to_string(),
                    },
                };
                let data = serde_json::to_string(&transfer)
                    .unwrap_or_else(|_| "{}".to_string());
                Event {
                    event_type: "transfer_started".to_string(),
                    data,
                }
            }
            KizunaEvent::TransferProgress(progress) => {
                let prog = Progress {
                    id: progress.id.0.to_string(),
                    bytes_transferred: progress.bytes_transferred as f64,
                    total_bytes: progress.total_bytes as f64,
                    speed_bps: progress.speed_bps as f64,
                    percentage: progress.percentage(),
                };
                let data = serde_json::to_string(&prog)
                    .unwrap_or_else(|_| "{}".to_string());
                Event {
                    event_type: "transfer_progress".to_string(),
                    data,
                }
            }
            KizunaEvent::TransferCompleted(result) => {
                let res = TransferResultJs {
                    id: result.id.0.to_string(),
                    success: result.success,
                    error: result.error,
                    bytes_transferred: result.bytes_transferred as f64,
                    duration_ms: result.duration_ms as f64,
                };
                let data = serde_json::to_string(&res)
                    .unwrap_or_else(|_| "{}".to_string());
                Event {
                    event_type: "transfer_completed".to_string(),
                    data,
                }
            }
            KizunaEvent::StreamStarted(info) => {
                let stream = Stream {
                    id: info.id.0.to_string(),
                    stream_type: match info.stream_type {
                        StreamType::Camera => "camera".to_string(),
                        StreamType::Screen => "screen".to_string(),
                        StreamType::Audio => "audio".to_string(),
                    },
                    peer_id: info.peer_id.0,
                };
                let data = serde_json::to_string(&stream)
                    .unwrap_or_else(|_| "{}".to_string());
                Event {
                    event_type: "stream_started".to_string(),
                    data,
                }
            }
            KizunaEvent::StreamEnded(stream_id) => {
                let data = serde_json::json!({ "streamId": stream_id.0.to_string() }).to_string();
                Event {
                    event_type: "stream_ended".to_string(),
                    data,
                }
            }
            KizunaEvent::CommandExecuted(result) => {
                let cmd_result = CommandResult {
                    command: result.command,
                    peer_id: result.peer_id.0,
                    exit_code: result.exit_code,
                    stdout: result.stdout,
                    stderr: result.stderr,
                };
                let data = serde_json::to_string(&cmd_result)
                    .unwrap_or_else(|_| "{}".to_string());
                Event {
                    event_type: "command_executed".to_string(),
                    data,
                }
            }
            KizunaEvent::Error(error) => {
                let data = serde_json::to_string(&error)
                    .unwrap_or_else(|_| "{}".to_string());
                Event {
                    event_type: "error".to_string(),
                    data,
                }
            }
        }
    }
}

#[cfg(not(feature = "nodejs"))]
pub mod napi_bindings {
    // Placeholder when nodejs feature is not enabled
}
