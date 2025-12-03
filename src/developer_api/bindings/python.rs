/// Python bindings using PyO3
/// This module provides Python-compatible bindings for the Kizuna API

#[cfg(feature = "python")]
pub mod pyo3_bindings {
    use pyo3::prelude::*;
    use pyo3::exceptions::{PyException, PyRuntimeError, PyValueError};
    use pyo3::types::{PyDict, PyList};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    use crate::developer_api::core::KizunaAPI;
    use crate::developer_api::core::config::{
        KizunaConfig, IdentityConfig, DiscoveryConfig, SecurityConfig, NetworkConfig, PluginConfig, TrustMode,
    };
    use crate::developer_api::core::error::KizunaError;
    use crate::developer_api::core::events::{
        KizunaEvent, PeerId, PeerInfo, TransferId, TransferInfo, TransferProgress, TransferResult, TransferDirection,
        StreamId, StreamInfo, StreamType, CommandResult as CoreCommandResult, ErrorEvent,
    };
    use crate::developer_api::core::api::{KizunaInstance, StreamConfig};
    
    /// Python wrapper for KizunaInstance
    #[pyclass(name = "Kizuna")]
    pub struct PyKizuna {
        instance: Arc<Mutex<KizunaInstance>>,
        runtime: Arc<tokio::runtime::Runtime>,
    }
    
    #[pymethods]
    impl PyKizuna {
        /// Initialize a new Kizuna instance
        #[new]
        #[pyo3(signature = (config=None))]
        fn new(config: Option<&PyDict>) -> PyResult<Self> {
            let config = if let Some(config_dict) = config {
                parse_config(config_dict)?
            } else {
                KizunaConfig::default()
            };
            
            let runtime = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
            
            let instance = runtime.block_on(async {
                KizunaInstance::initialize(config).await
            }).map_err(to_py_err)?;
            
            Ok(Self {
                instance: Arc::new(Mutex::new(instance)),
                runtime: Arc::new(runtime),
            })
        }
        
        /// Discover peers on the network
        /// Returns a list of discovered peers
        fn discover_peers<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
            let instance = Arc::clone(&self.instance);
            
            pyo3_asyncio::tokio::future_into_py(py, async move {
                let inst = instance.lock().await;
                let mut stream = inst.discover_peers().await.map_err(to_py_err)?;
                
                let mut peers = Vec::new();
                use futures::StreamExt;
                while let Some(peer) = stream.next().await {
                    peers.push(PyPeerInfo::from(peer));
                }
                
                Ok(Python::with_gil(|py| peers.into_py(py)))
            })
        }
        
        /// Connect to a peer
        #[pyo3(signature = (peer_id))]
        fn connect_to_peer<'py>(&self, py: Python<'py>, peer_id: String) -> PyResult<&'py PyAny> {
            let instance = Arc::clone(&self.instance);
            
            pyo3_asyncio::tokio::future_into_py(py, async move {
                let inst = instance.lock().await;
                let peer_id = PeerId::from(peer_id);
                let _connection = inst.connect_to_peer(peer_id.clone()).await.map_err(to_py_err)?;
                
                Ok(Python::with_gil(|py| PyPeerConnection { peer_id: peer_id.0 }.into_py(py)))
            })
        }
        
        /// Transfer a file to a peer
        #[pyo3(signature = (file_path, peer_id))]
        fn transfer_file<'py>(&self, py: Python<'py>, file_path: String, peer_id: String) -> PyResult<&'py PyAny> {
            let instance = Arc::clone(&self.instance);
            
            pyo3_asyncio::tokio::future_into_py(py, async move {
                let inst = instance.lock().await;
                let path = PathBuf::from(file_path);
                let peer_id = PeerId::from(peer_id);
                let handle = inst.transfer_file(path, peer_id).await.map_err(to_py_err)?;
                
                Ok(Python::with_gil(|py| PyTransferHandle {
                    transfer_id: handle.transfer_id().0.to_string(),
                }.into_py(py)))
            })
        }
        
        /// Start a media stream
        #[pyo3(signature = (stream_type, peer_id, quality=80))]
        fn start_stream<'py>(&self, py: Python<'py>, stream_type: String, peer_id: String, quality: u8) -> PyResult<&'py PyAny> {
            let instance = Arc::clone(&self.instance);
            
            pyo3_asyncio::tokio::future_into_py(py, async move {
                let inst = instance.lock().await;
                let stream_type = match stream_type.as_str() {
                    "camera" => StreamType::Camera,
                    "screen" => StreamType::Screen,
                    "audio" => StreamType::Audio,
                    _ => return Err(PyValueError::new_err("Invalid stream type")),
                };
                
                let config = StreamConfig {
                    stream_type,
                    peer_id: PeerId::from(peer_id),
                    quality,
                };
                
                let handle = inst.start_stream(config).await.map_err(to_py_err)?;
                
                Ok(Python::with_gil(|py| PyStreamHandle {
                    stream_id: handle.stream_id().0.to_string(),
                }.into_py(py)))
            })
        }
        
        /// Execute a command on a peer
        #[pyo3(signature = (command, peer_id))]
        fn execute_command<'py>(&self, py: Python<'py>, command: String, peer_id: String) -> PyResult<&'py PyAny> {
            let instance = Arc::clone(&self.instance);
            
            pyo3_asyncio::tokio::future_into_py(py, async move {
                let inst = instance.lock().await;
                let peer_id = PeerId::from(peer_id);
                let result = inst.execute_command(command, peer_id).await.map_err(to_py_err)?;
                
                Ok(Python::with_gil(|py| PyCommandResult {
                    exit_code: result.exit_code,
                    stdout: result.stdout,
                    stderr: result.stderr,
                }.into_py(py)))
            })
        }
        
        /// Subscribe to events
        /// Returns an async iterator of events
        fn subscribe_events<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
            let instance = Arc::clone(&self.instance);
            
            pyo3_asyncio::tokio::future_into_py(py, async move {
                let inst = instance.lock().await;
                let mut stream = inst.subscribe_events().await.map_err(to_py_err)?;
                
                let mut events = Vec::new();
                use futures::StreamExt;
                while let Some(event) = stream.next().await {
                    events.push(PyKizunaEvent::from(event));
                }
                
                Ok(Python::with_gil(|py| events.into_py(py)))
            })
        }
        
        /// Shutdown the Kizuna instance
        fn shutdown<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
            let instance = Arc::clone(&self.instance);
            
            pyo3_asyncio::tokio::future_into_py(py, async move {
                let inst = instance.lock().await;
                inst.shutdown().await.map_err(to_py_err)?;
                Ok(Python::with_gil(|py| py.None()))
            })
        }
    }
    
    /// Python wrapper for PeerInfo
    #[pyclass(name = "PeerInfo")]
    #[derive(Clone)]
    pub struct PyPeerInfo {
        #[pyo3(get)]
        pub id: String,
        #[pyo3(get)]
        pub name: String,
        #[pyo3(get)]
        pub addresses: Vec<String>,
        #[pyo3(get)]
        pub capabilities: Vec<String>,
        #[pyo3(get)]
        pub discovery_method: String,
    }
    
    impl From<PeerInfo> for PyPeerInfo {
        fn from(info: PeerInfo) -> Self {
            Self {
                id: info.id.0,
                name: info.name,
                addresses: info.addresses,
                capabilities: info.capabilities,
                discovery_method: info.discovery_method,
            }
        }
    }
    
    #[pymethods]
    impl PyPeerInfo {
        fn __repr__(&self) -> String {
            format!("PeerInfo(id='{}', name='{}')", self.id, self.name)
        }
    }
    
    /// Python wrapper for PeerConnection
    #[pyclass(name = "PeerConnection")]
    pub struct PyPeerConnection {
        #[pyo3(get)]
        pub peer_id: String,
    }
    
    #[pymethods]
    impl PyPeerConnection {
        fn __repr__(&self) -> String {
            format!("PeerConnection(peer_id='{}')", self.peer_id)
        }
    }
    
    /// Python wrapper for TransferHandle
    #[pyclass(name = "TransferHandle")]
    pub struct PyTransferHandle {
        #[pyo3(get)]
        pub transfer_id: String,
    }
    
    #[pymethods]
    impl PyTransferHandle {
        fn __repr__(&self) -> String {
            format!("TransferHandle(transfer_id='{}')", self.transfer_id)
        }
    }
    
    /// Python wrapper for StreamHandle
    #[pyclass(name = "StreamHandle")]
    pub struct PyStreamHandle {
        #[pyo3(get)]
        pub stream_id: String,
    }
    
    #[pymethods]
    impl PyStreamHandle {
        fn __repr__(&self) -> String {
            format!("StreamHandle(stream_id='{}')", self.stream_id)
        }
    }
    
    /// Python wrapper for CommandResult
    #[pyclass(name = "CommandResult")]
    pub struct PyCommandResult {
        #[pyo3(get)]
        pub exit_code: i32,
        #[pyo3(get)]
        pub stdout: String,
        #[pyo3(get)]
        pub stderr: String,
    }
    
    #[pymethods]
    impl PyCommandResult {
        fn __repr__(&self) -> String {
            format!("CommandResult(exit_code={})", self.exit_code)
        }
    }
    
    /// Python wrapper for KizunaEvent
    #[pyclass(name = "KizunaEvent")]
    #[derive(Clone)]
    pub struct PyKizunaEvent {
        #[pyo3(get)]
        pub event_type: String,
        #[pyo3(get)]
        pub data: String,
    }
    
    impl From<KizunaEvent> for PyKizunaEvent {
        fn from(event: KizunaEvent) -> Self {
            let (event_type, data) = match event {
                KizunaEvent::PeerDiscovered(info) => {
                    ("peer_discovered".to_string(), serde_json::to_string(&info).unwrap_or_default())
                }
                KizunaEvent::PeerConnected(peer_id) => {
                    ("peer_connected".to_string(), peer_id.0)
                }
                KizunaEvent::PeerDisconnected(peer_id) => {
                    ("peer_disconnected".to_string(), peer_id.0)
                }
                KizunaEvent::TransferStarted(info) => {
                    ("transfer_started".to_string(), serde_json::to_string(&info).unwrap_or_default())
                }
                KizunaEvent::TransferProgress(progress) => {
                    ("transfer_progress".to_string(), serde_json::to_string(&progress).unwrap_or_default())
                }
                KizunaEvent::TransferCompleted(result) => {
                    ("transfer_completed".to_string(), serde_json::to_string(&result).unwrap_or_default())
                }
                KizunaEvent::StreamStarted(info) => {
                    ("stream_started".to_string(), serde_json::to_string(&info).unwrap_or_default())
                }
                KizunaEvent::StreamEnded(stream_id) => {
                    ("stream_ended".to_string(), stream_id.0.to_string())
                }
                KizunaEvent::CommandExecuted(result) => {
                    ("command_executed".to_string(), serde_json::to_string(&result).unwrap_or_default())
                }
                KizunaEvent::Error(error) => {
                    ("error".to_string(), serde_json::to_string(&error).unwrap_or_default())
                }
            };
            
            Self { event_type, data }
        }
    }
    
    #[pymethods]
    impl PyKizunaEvent {
        fn __repr__(&self) -> String {
            format!("KizunaEvent(type='{}')", self.event_type)
        }
    }
    
    /// Python wrapper for TransferProgress
    #[pyclass(name = "TransferProgress")]
    #[derive(Clone)]
    pub struct PyTransferProgress {
        #[pyo3(get)]
        pub transfer_id: String,
        #[pyo3(get)]
        pub bytes_transferred: u64,
        #[pyo3(get)]
        pub total_bytes: u64,
        #[pyo3(get)]
        pub speed_bps: u64,
    }
    
    impl From<TransferProgress> for PyTransferProgress {
        fn from(progress: TransferProgress) -> Self {
            Self {
                transfer_id: progress.id.0.to_string(),
                bytes_transferred: progress.bytes_transferred,
                total_bytes: progress.total_bytes,
                speed_bps: progress.speed_bps,
            }
        }
    }
    
    #[pymethods]
    impl PyTransferProgress {
        fn percentage(&self) -> f64 {
            if self.total_bytes == 0 {
                0.0
            } else {
                (self.bytes_transferred as f64 / self.total_bytes as f64) * 100.0
            }
        }
        
        fn __repr__(&self) -> String {
            format!("TransferProgress({}%)", self.percentage())
        }
    }
    
    /// Helper function to parse Python dict into KizunaConfig
    fn parse_config(config_dict: &PyDict) -> PyResult<KizunaConfig> {
        let mut config = KizunaConfig::default();
        
        // Parse identity config
        if let Some(identity) = config_dict.get_item("identity")? {
            let identity_dict = identity.downcast::<PyDict>()?;
            let device_name = identity_dict.get_item("device_name")?
                .ok_or_else(|| PyValueError::new_err("device_name is required"))?
                .extract::<String>()?;
            
            let user_name = identity_dict.get_item("user_name")?
                .map(|v| v.extract::<String>())
                .transpose()?;
            
            let identity_path = identity_dict.get_item("identity_path")?
                .map(|v| v.extract::<String>().map(PathBuf::from))
                .transpose()?;
            
            config.identity = Some(IdentityConfig {
                device_name,
                user_name,
                identity_path,
            });
        }
        
        // Parse discovery config
        if let Some(discovery) = config_dict.get_item("discovery")? {
            let discovery_dict = discovery.downcast::<PyDict>()?;
            config.discovery = DiscoveryConfig {
                enable_mdns: discovery_dict.get_item("enable_mdns")?
                    .map(|v| v.extract::<bool>())
                    .transpose()?
                    .unwrap_or(true),
                enable_udp: discovery_dict.get_item("enable_udp")?
                    .map(|v| v.extract::<bool>())
                    .transpose()?
                    .unwrap_or(true),
                enable_bluetooth: discovery_dict.get_item("enable_bluetooth")?
                    .map(|v| v.extract::<bool>())
                    .transpose()?
                    .unwrap_or(false),
                interval_secs: discovery_dict.get_item("interval_secs")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?
                    .unwrap_or(5),
                timeout_secs: discovery_dict.get_item("timeout_secs")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?
                    .unwrap_or(30),
            };
        }
        
        // Parse security config
        if let Some(security) = config_dict.get_item("security")? {
            let security_dict = security.downcast::<PyDict>()?;
            let trust_mode = security_dict.get_item("trust_mode")?
                .map(|v| {
                    let mode_str = v.extract::<String>()?;
                    match mode_str.as_str() {
                        "trust_all" => Ok(TrustMode::TrustAll),
                        "manual" => Ok(TrustMode::Manual),
                        "allowlist_only" => Ok(TrustMode::AllowlistOnly),
                        _ => Err(PyValueError::new_err("Invalid trust mode")),
                    }
                })
                .transpose()?
                .unwrap_or(TrustMode::Manual);
            
            config.security = SecurityConfig {
                enable_encryption: security_dict.get_item("enable_encryption")?
                    .map(|v| v.extract::<bool>())
                    .transpose()?
                    .unwrap_or(true),
                require_authentication: security_dict.get_item("require_authentication")?
                    .map(|v| v.extract::<bool>())
                    .transpose()?
                    .unwrap_or(true),
                trust_mode,
                key_storage_path: security_dict.get_item("key_storage_path")?
                    .map(|v| v.extract::<String>().map(PathBuf::from))
                    .transpose()?,
            };
        }
        
        // Parse networking config
        if let Some(networking) = config_dict.get_item("networking")? {
            let networking_dict = networking.downcast::<PyDict>()?;
            config.networking = NetworkConfig {
                listen_port: networking_dict.get_item("listen_port")?
                    .map(|v| v.extract::<u16>())
                    .transpose()?,
                enable_ipv6: networking_dict.get_item("enable_ipv6")?
                    .map(|v| v.extract::<bool>())
                    .transpose()?
                    .unwrap_or(true),
                enable_quic: networking_dict.get_item("enable_quic")?
                    .map(|v| v.extract::<bool>())
                    .transpose()?
                    .unwrap_or(true),
                enable_webrtc: networking_dict.get_item("enable_webrtc")?
                    .map(|v| v.extract::<bool>())
                    .transpose()?
                    .unwrap_or(true),
                enable_websocket: networking_dict.get_item("enable_websocket")?
                    .map(|v| v.extract::<bool>())
                    .transpose()?
                    .unwrap_or(true),
                connection_timeout_secs: networking_dict.get_item("connection_timeout_secs")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?
                    .unwrap_or(30),
            };
        }
        
        Ok(config)
    }
    
    /// Helper function to convert KizunaError to PyErr
    fn to_py_err(error: KizunaError) -> PyErr {
        PyRuntimeError::new_err(error.to_string())
    }
    
    /// Python module initialization
    #[pymodule]
    fn kizuna(_py: Python, m: &PyModule) -> PyResult<()> {
        m.add_class::<PyKizuna>()?;
        m.add_class::<PyPeerInfo>()?;
        m.add_class::<PyPeerConnection>()?;
        m.add_class::<PyTransferHandle>()?;
        m.add_class::<PyStreamHandle>()?;
        m.add_class::<PyCommandResult>()?;
        m.add_class::<PyKizunaEvent>()?;
        m.add_class::<PyTransferProgress>()?;
        Ok(())
    }
}

#[cfg(not(feature = "python"))]
pub mod pyo3_bindings {
    // Placeholder when python feature is not enabled
}
