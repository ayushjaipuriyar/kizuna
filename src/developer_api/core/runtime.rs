/// Async runtime integration for the Developer API
use tokio::runtime::{Builder, Runtime};
use std::sync::Arc;

/// Async runtime wrapper for Kizuna API
pub struct AsyncRuntime {
    runtime: Arc<Runtime>,
}

impl AsyncRuntime {
    /// Creates a new async runtime with default configuration
    pub fn new() -> Result<Self, std::io::Error> {
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .thread_name("kizuna-worker")
            .build()?;
        
        Ok(Self {
            runtime: Arc::new(runtime),
        })
    }
    
    /// Creates a new async runtime with custom configuration
    pub fn with_config(worker_threads: usize) -> Result<Self, std::io::Error> {
        let runtime = Builder::new_multi_thread()
            .worker_threads(worker_threads)
            .enable_all()
            .thread_name("kizuna-worker")
            .build()?;
        
        Ok(Self {
            runtime: Arc::new(runtime),
        })
    }
    
    /// Spawns a future on the runtime
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }
    
    /// Blocks on a future until it completes
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.runtime.block_on(future)
    }
    
    /// Gets a handle to the runtime
    pub fn handle(&self) -> tokio::runtime::Handle {
        self.runtime.handle().clone()
    }
}

impl Default for AsyncRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create async runtime")
    }
}

impl Clone for AsyncRuntime {
    fn clone(&self) -> Self {
        Self {
            runtime: Arc::clone(&self.runtime),
        }
    }
}
