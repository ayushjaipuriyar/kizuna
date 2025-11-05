use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
    
    #[error("Strategy not available: {strategy}")]
    StrategyUnavailable { strategy: String },
    
    #[error("Discovery timeout after {timeout:?}")]
    Timeout { timeout: Duration },
    
    #[error("Invalid service record: {reason}")]
    InvalidServiceRecord { reason: String },
    
    #[error("Bluetooth error: {0}")]
    Bluetooth(String),
    
    #[error("libp2p error: {0}")]
    Libp2p(String),
    
    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
}

impl From<anyhow::Error> for DiscoveryError {
    fn from(err: anyhow::Error) -> Self {
        DiscoveryError::Network(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
        ))
    }
}