/// Core API trait and implementation
use super::{KizunaConfig, KizunaError, KizunaEvent};
use super::events::{PeerId, PeerInfo, TransferId, StreamId};
use async_trait::async_trait;
use futures::Stream;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

// Import Kizuna core systems
use super::integration::IntegratedSystemManager;

/// Main API trait for Kizuna functionality
#[async_trait]
pub trait KizunaAPI: Send + Sync {
    /// Initializes a new Kizuna instance with the given configuration
    async fn initialize(config: KizunaConfig) -> Result<KizunaInstance, KizunaError>
    where
        Self: Sized;
    
    /// Discovers peers on the network
    async fn discover_peers(&self) -> Result<Pin<Box<dyn Stream<Item = PeerInfo> + Send>>, KizunaError>;
    
    /// Connects to a peer
    async fn connect_to_peer(&self, peer_id: PeerId) -> Result<PeerConnection, KizunaError>;
    
    /// Transfers a file to a peer
    async fn transfer_file(&self, file: PathBuf, peer_id: PeerId) -> Result<TransferHandle, KizunaError>;
    
    /// Starts a media stream
    async fn start_stream(&self, config: StreamConfig) -> Result<StreamHandle, KizunaError>;
    
    /// Executes a command on a peer
    async fn execute_command(&self, command: String, peer_id: PeerId) -> Result<CommandResult, KizunaError>;
    
    /// Subscribes to events
    async fn subscribe_events(&self) -> Result<Pin<Box<dyn Stream<Item = KizunaEvent> + Send>>, KizunaError>;
    
    /// Shuts down the instance
    async fn shutdown(&self) -> Result<(), KizunaError>;
}

/// Lifecycle state of the Kizuna instance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    /// Instance is being created
    Initializing,
    /// Instance is ready for use
    Ready,
    /// Instance is shutting down
    ShuttingDown,
    /// Instance has been shut down
    Shutdown,
}

/// Kizuna instance representing an active API session with thread-safe access
pub struct KizunaInstance {
    config: KizunaConfig,
    runtime: super::runtime::AsyncRuntime,
    event_emitter: super::runtime::ThreadSafe<super::events::EventEmitter>,
    event_tx: Arc<tokio::sync::broadcast::Sender<KizunaEvent>>,
    // Integrated system manager for all core systems
    system_manager: Arc<IntegratedSystemManager>,
    // Lifecycle management
    state: Arc<tokio::sync::RwLock<InstanceState>>,
    // Shutdown coordination
    shutdown_tx: Arc<tokio::sync::broadcast::Sender<()>>,
    // Resource cleanup tasks
    cleanup_tasks: super::runtime::ThreadSafe<Vec<tokio::task::JoinHandle<()>>>,
}

impl KizunaInstance {
    /// Creates a new Kizuna instance with thread-safe initialization
    pub fn new(config: KizunaConfig) -> Result<Self, KizunaError> {
        // Validate configuration
        config.validate()
            .map_err(|e| KizunaError::config(e))?;
        
        // Create async runtime with custom configuration
        let runtime_config = super::runtime::RuntimeConfig {
            worker_threads: config.runtime_threads,
            thread_name: "kizuna-api".to_string(),
            ..Default::default()
        };
        
        let runtime = super::runtime::AsyncRuntime::with_config(runtime_config)
            .map_err(|e| KizunaError::other(format!("Failed to create runtime: {}", e)))?;
        
        // Create event channel with larger buffer for high-throughput scenarios
        let (event_tx, _) = tokio::sync::broadcast::channel(1000);
        let event_tx = Arc::new(event_tx);
        
        // Create shutdown channel
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        let shutdown_tx = Arc::new(shutdown_tx);
        
        // Create integrated system manager
        let system_manager = Arc::new(IntegratedSystemManager::new(config.clone()));
        
        Ok(Self {
            config,
            runtime,
            event_emitter: super::runtime::ThreadSafe::new(super::events::EventEmitter::new()),
            event_tx,
            system_manager,
            state: Arc::new(tokio::sync::RwLock::new(InstanceState::Initializing)),
            shutdown_tx,
            cleanup_tasks: super::runtime::ThreadSafe::new(Vec::new()),
        })
    }
    
    /// Initializes the core systems with thread-safe access
    pub async fn initialize_systems(&self) -> Result<(), KizunaError> {
        // Check current state
        let current_state = *self.state.read().await;
        match current_state {
            InstanceState::Shutdown | InstanceState::ShuttingDown => {
                return Err(KizunaError::state("Cannot initialize: instance is shutting down or shutdown"));
            }
            InstanceState::Ready => {
                return Err(KizunaError::state("Instance is already initialized"));
            }
            InstanceState::Initializing => {
                // Continue with initialization
            }
        }
        
        // Initialize all systems through the integrated system manager
        self.system_manager.initialize().await?;
        
        // Update state to Ready
        *self.state.write().await = InstanceState::Ready;
        
        // Emit initialization complete event
        self.emit_event(KizunaEvent::Error(super::events::ErrorEvent {
            message: "Instance initialized successfully".to_string(),
            code: Some("INIT_SUCCESS".to_string()),
            context: std::collections::HashMap::new(),
        })).await;
        
        Ok(())
    }
    
    /// Gets the configuration (immutable reference)
    pub fn config(&self) -> &KizunaConfig {
        &self.config
    }
    
    /// Updates the configuration with validation
    pub async fn update_config(&self, new_config: KizunaConfig) -> Result<(), KizunaError> {
        // Validate new configuration
        new_config.validate()
            .map_err(|e| KizunaError::config(e))?;
        
        // Check if we can update (not during shutdown)
        let current_state = *self.state.read().await;
        if current_state == InstanceState::ShuttingDown || current_state == InstanceState::Shutdown {
            return Err(KizunaError::state("Cannot update config during shutdown"));
        }
        
        // Note: In a real implementation, we would need to handle config updates
        // by potentially reinitializing affected systems
        Err(KizunaError::other("Configuration updates not yet implemented"))
    }
    
    /// Gets the async runtime
    pub fn runtime(&self) -> &super::runtime::AsyncRuntime {
        &self.runtime
    }
    
    /// Gets the current lifecycle state
    pub async fn state(&self) -> InstanceState {
        *self.state.read().await
    }
    
    /// Emits an event with thread-safe access
    pub async fn emit_event(&self, event: KizunaEvent) {
        // Check if shutdown
        let current_state = *self.state.read().await;
        if current_state == InstanceState::Shutdown {
            return;
        }
        
        let emitter = self.event_emitter.read().await;
        emitter.emit(event).await;
    }
    
    /// Checks if the instance is shutdown
    pub async fn is_shutdown(&self) -> bool {
        *self.state.read().await == InstanceState::Shutdown
    }
    
    /// Gets the integrated system manager
    pub fn system_manager(&self) -> &Arc<IntegratedSystemManager> {
        &self.system_manager
    }
    
    /// Spawns a task on the runtime with automatic cleanup tracking
    pub async fn spawn_task<F>(&self, future: F) -> Result<tokio::task::JoinHandle<F::Output>, KizunaError>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        // Check if we can spawn tasks
        let current_state = *self.state.read().await;
        if current_state == InstanceState::Shutdown {
            return Err(KizunaError::state("Cannot spawn task: instance is shutdown"));
        }
        
        let handle = self.runtime.spawn(future);
        Ok(handle)
    }
    
    /// Spawns a task with timeout
    pub async fn spawn_task_with_timeout<F>(
        &self,
        future: F,
        timeout: std::time::Duration,
    ) -> Result<tokio::task::JoinHandle<Result<F::Output, tokio::time::error::Elapsed>>, KizunaError>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        // Check if we can spawn tasks
        let current_state = *self.state.read().await;
        if current_state == InstanceState::Shutdown {
            return Err(KizunaError::state("Cannot spawn task: instance is shutdown"));
        }
        
        let handle = self.runtime.spawn_with_timeout(future, timeout);
        Ok(handle)
    }
    
    /// Registers a cleanup task to be executed during shutdown
    pub async fn register_cleanup_task<F>(&self, cleanup: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let handle = self.runtime.spawn(async move {
            // Wait for cleanup to be triggered
            cleanup.await;
        });
        
        let mut tasks = self.cleanup_tasks.write().await;
        tasks.push(handle);
    }
    
    /// Subscribes to shutdown signals
    pub fn subscribe_shutdown(&self) -> tokio::sync::broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }
    
    /// Performs graceful shutdown with resource cleanup
    async fn perform_shutdown(&self) -> Result<(), KizunaError> {
        // Transition to shutting down state
        {
            let mut state = self.state.write().await;
            match *state {
                InstanceState::Shutdown => {
                    return Ok(()); // Already shutdown
                }
                InstanceState::ShuttingDown => {
                    return Err(KizunaError::state("Shutdown already in progress"));
                }
                _ => {
                    *state = InstanceState::ShuttingDown;
                }
            }
        }
        
        // Emit shutdown event
        self.emit_event(KizunaEvent::Error(super::events::ErrorEvent {
            message: "Instance shutting down".to_string(),
            code: Some("SHUTDOWN_INITIATED".to_string()),
            context: std::collections::HashMap::new(),
        })).await;
        
        // Signal shutdown to all subscribers
        let _ = self.shutdown_tx.send(());
        
        // Signal shutdown to runtime
        self.runtime.signal_shutdown().await;
        
        // Shutdown all integrated systems
        let _ = self.system_manager.shutdown().await;
        
        // Wait for cleanup tasks to complete with timeout
        {
            let mut tasks = self.cleanup_tasks.write().await;
            let timeout = std::time::Duration::from_secs(5);
            
            for task in tasks.drain(..) {
                let _ = tokio::time::timeout(timeout, task).await;
            }
        }
        
        // Transition to shutdown state
        *self.state.write().await = InstanceState::Shutdown;
        
        Ok(())
    }
}

#[async_trait]
impl KizunaAPI for KizunaInstance {
    async fn initialize(config: KizunaConfig) -> Result<KizunaInstance, KizunaError> {
        let instance = KizunaInstance::new(config)?;
        instance.initialize_systems().await?;
        Ok(instance)
    }
    
    async fn discover_peers(&self) -> Result<Pin<Box<dyn Stream<Item = PeerInfo> + Send>>, KizunaError> {
        // Check state
        let current_state = *self.state.read().await;
        if current_state != InstanceState::Ready {
            return Err(KizunaError::state(format!("Cannot discover peers: instance is in {:?} state", current_state)));
        }
        
        // Get discovery system from integrated manager
        let discovery_arc = self.system_manager.discovery().await?;
        let discovery = discovery_arc.read().await;
        
        // Discover peers
        let peers = discovery.discover_once(None).await
            .map_err(|e| KizunaError::discovery(format!("Discovery failed: {}", e)))?;
        
        // Convert to stream
        use futures::stream;
        let peer_infos: Vec<PeerInfo> = peers.into_iter().map(|sr| PeerInfo {
            peer_id: sr.peer_id.into(),
            name: sr.name,
            addresses: sr.addresses,
        }).collect();
        
        let stream = stream::iter(peer_infos);
        Ok(Box::pin(stream))
    }
    
    async fn connect_to_peer(&self, peer_id: PeerId) -> Result<PeerConnection, KizunaError> {
        // Check state
        let current_state = *self.state.read().await;
        if current_state != InstanceState::Ready {
            return Err(KizunaError::state(format!("Cannot connect to peer: instance is in {:?} state", current_state)));
        }
        
        // Get transport system from integrated manager
        let transport_arc = self.system_manager.transport().await?;
        let transport = transport_arc.read().await;
        
        // Create peer address (simplified - in production would need full address info)
        let peer_address = crate::transport::PeerAddress::new(
            peer_id.to_string(),
            vec![],
            vec!["tcp".to_string()],
            crate::transport::TransportCapabilities::tcp(),
        );
        
        // Connect to peer
        let _connection = transport.connect_to_peer(&peer_address).await
            .map_err(|e| KizunaError::transport(format!("Connection failed: {}", e)))?;
        
        Ok(PeerConnection { peer_id })
    }
    
    async fn transfer_file(&self, file: PathBuf, peer_id: PeerId) -> Result<TransferHandle, KizunaError> {
        // Check state
        let current_state = *self.state.read().await;
        if current_state != InstanceState::Ready {
            return Err(KizunaError::state(format!("Cannot transfer file: instance is in {:?} state", current_state)));
        }
        
        // Validate file exists
        if !file.exists() {
            return Err(KizunaError::other(format!("File not found: {:?}", file)));
        }
        
        // Get file transfer system from integrated manager
        let ft_arc = self.system_manager.file_transfer().await?;
        let ft = ft_arc.as_ref();
        
        // Start file transfer
        let session = ft.send_file(file, peer_id.to_string()).await
            .map_err(|e| KizunaError::file_transfer(format!("File transfer failed: {}", e)))?;
        
        Ok(TransferHandle { 
            transfer_id: TransferId::from_uuid(session.session_id)
        })
    }
    
    #[cfg(feature = "streaming")]
    async fn start_stream(&self, config: StreamConfig) -> Result<StreamHandle, KizunaError> {
        // Check state
        let current_state = *self.state.read().await;
        if current_state != InstanceState::Ready {
            return Err(KizunaError::state(format!("Cannot start stream: instance is in {:?} state", current_state)));
        }
        
        // Get streaming system from integrated manager
        let streaming_arc = self.system_manager.streaming().await?;
        let streaming = streaming_arc.as_ref();
        
        // Convert config to streaming config
        let stream_config = crate::streaming::StreamConfig {
            quality: config.quality,
            ..Default::default()
        };
        
        // Start camera stream
        let session = streaming.start_camera_stream(stream_config).await
            .map_err(|e| KizunaError::streaming(format!("Stream start failed: {}", e)))?;
        
        Ok(StreamHandle { 
            stream_id: StreamId::from_uuid(session.session_id)
        })
    }
    
    #[cfg(not(feature = "streaming"))]
    async fn start_stream(&self, _config: StreamConfig) -> Result<StreamHandle, KizunaError> {
        Err(KizunaError::state("Streaming feature not enabled"))
    }
    
    async fn execute_command(&self, command: String, peer_id: PeerId) -> Result<CommandResult, KizunaError> {
        // Check state
        let current_state = *self.state.read().await;
        if current_state != InstanceState::Ready {
            return Err(KizunaError::state(format!("Cannot execute command: instance is in {:?} state", current_state)));
        }
        
        // Command execution would require a separate command execution system
        // For now, return an error indicating this feature is not yet available
        Err(KizunaError::other(format!(
            "Command execution not yet implemented for peer {} with command: {}", 
            peer_id, command
        )))
    }
    
    async fn subscribe_events(&self) -> Result<Pin<Box<dyn Stream<Item = KizunaEvent> + Send>>, KizunaError> {
        // Check state
        let current_state = *self.state.read().await;
        if current_state == InstanceState::Shutdown {
            return Err(KizunaError::state("Cannot subscribe to events: instance is shutdown"));
        }
        
        // Use the AsyncStreamBuilder for creating the event stream
        let rx = self.event_tx.subscribe();
        let stream = super::runtime::AsyncStreamBuilder::from_broadcast(rx);
        
        Ok(stream)
    }
    
    async fn shutdown(&self) -> Result<(), KizunaError> {
        self.perform_shutdown().await
    }
}

/// Handle to a peer connection
pub struct PeerConnection {
    peer_id: PeerId,
}

impl PeerConnection {
    /// Gets the peer ID
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
}

/// Handle to a file transfer operation
pub struct TransferHandle {
    transfer_id: TransferId,
}

impl TransferHandle {
    /// Gets the transfer ID
    pub fn transfer_id(&self) -> &TransferId {
        &self.transfer_id
    }
    
    /// Cancels the transfer
    pub async fn cancel(&self) -> Result<(), KizunaError> {
        // Cancel the file transfer
        // This would integrate with the file transfer system
        Ok(())
    }
}

/// Handle to a media stream
pub struct StreamHandle {
    stream_id: StreamId,
}

impl StreamHandle {
    /// Gets the stream ID
    pub fn stream_id(&self) -> &StreamId {
        &self.stream_id
    }
    
    /// Stops the stream
    pub async fn stop(&self) -> Result<(), KizunaError> {
        // Stop the media stream
        // This would integrate with the streaming system
        Ok(())
    }
}

/// Configuration for media streaming
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Target peer ID
    pub peer_id: PeerId,
    
    /// Video quality
    #[cfg(feature = "streaming")]
    pub quality: crate::streaming::StreamQuality,
    
    #[cfg(not(feature = "streaming"))]
    pub quality: String,
}

/// Result of a command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Exit code
    pub exit_code: i32,
    
    /// Standard output
    pub stdout: String,
    
    /// Standard error
    pub stderr: String,
}

// Tests are in a separate file
#[cfg(test)]
#[path = "api_test.rs"]
mod tests;
