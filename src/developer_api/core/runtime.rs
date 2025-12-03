/// Async runtime integration for the Developer API
use tokio::runtime::{Builder, Runtime, Handle};
use tokio::sync::{RwLock, Mutex, Semaphore};
use std::sync::Arc;
use std::time::Duration;
use futures::Stream;
use std::pin::Pin;

/// Configuration for the async runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Number of worker threads (None = default based on CPU cores)
    pub worker_threads: Option<usize>,
    
    /// Thread stack size in bytes
    pub thread_stack_size: Option<usize>,
    
    /// Thread name prefix
    pub thread_name: String,
    
    /// Maximum blocking threads
    pub max_blocking_threads: Option<usize>,
    
    /// Enable IO driver
    pub enable_io: bool,
    
    /// Enable time driver
    pub enable_time: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: None,
            thread_stack_size: None,
            thread_name: "kizuna-worker".to_string(),
            max_blocking_threads: None,
            enable_io: true,
            enable_time: true,
        }
    }
}

/// Async runtime wrapper for Kizuna API with thread-safe access
pub struct AsyncRuntime {
    /// The underlying tokio runtime
    runtime: Arc<Runtime>,
    
    /// Runtime configuration
    config: RuntimeConfig,
    
    /// Semaphore for limiting concurrent operations
    operation_limiter: Arc<Semaphore>,
    
    /// Shutdown signal
    shutdown_tx: Arc<Mutex<Option<tokio::sync::broadcast::Sender<()>>>>,
}

impl AsyncRuntime {
    /// Creates a new async runtime with default configuration
    pub fn new() -> Result<Self, std::io::Error> {
        Self::with_config(RuntimeConfig::default())
    }
    
    /// Creates a new async runtime with custom configuration
    pub fn with_config(config: RuntimeConfig) -> Result<Self, std::io::Error> {
        let mut builder = Builder::new_multi_thread();
        
        // Configure worker threads
        if let Some(threads) = config.worker_threads {
            builder.worker_threads(threads);
        }
        
        // Configure thread stack size
        if let Some(stack_size) = config.thread_stack_size {
            builder.thread_stack_size(stack_size);
        }
        
        // Configure thread name
        builder.thread_name(&config.thread_name);
        
        // Configure max blocking threads
        if let Some(max_blocking) = config.max_blocking_threads {
            builder.max_blocking_threads(max_blocking);
        }
        
        // Enable drivers
        if config.enable_io && config.enable_time {
            builder.enable_all();
        } else {
            if config.enable_io {
                builder.enable_io();
            }
            if config.enable_time {
                builder.enable_time();
            }
        }
        
        let runtime = builder.build()?;
        
        // Create shutdown channel
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        
        Ok(Self {
            runtime: Arc::new(runtime),
            config,
            operation_limiter: Arc::new(Semaphore::new(1000)), // Limit to 1000 concurrent operations
            shutdown_tx: Arc::new(Mutex::new(Some(shutdown_tx))),
        })
    }
    
    /// Spawns a future on the runtime with thread-safe access
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }
    
    /// Spawns a future with a timeout
    pub fn spawn_with_timeout<F>(
        &self,
        future: F,
        timeout: Duration,
    ) -> tokio::task::JoinHandle<Result<F::Output, tokio::time::error::Elapsed>>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(async move {
            tokio::time::timeout(timeout, future).await
        })
    }
    
    /// Spawns a blocking task on a dedicated thread pool
    pub fn spawn_blocking<F, R>(&self, f: F) -> tokio::task::JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.runtime.spawn_blocking(f)
    }
    
    /// Blocks on a future until it completes
    pub fn block_on<F>(&self, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.runtime.block_on(future)
    }
    
    /// Gets a handle to the runtime for spawning tasks from other threads
    pub fn handle(&self) -> Handle {
        self.runtime.handle().clone()
    }
    
    /// Gets the runtime configuration
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }
    
    /// Acquires a permit for operation limiting
    pub async fn acquire_operation_permit(&self) -> Result<tokio::sync::SemaphorePermit<'_>, tokio::sync::AcquireError> {
        self.operation_limiter.acquire().await
    }
    
    /// Subscribes to shutdown signals
    pub async fn subscribe_shutdown(&self) -> tokio::sync::broadcast::Receiver<()> {
        let guard = self.shutdown_tx.lock().await;
        guard.as_ref()
            .expect("Shutdown channel not initialized")
            .subscribe()
    }
    
    /// Signals shutdown to all subscribers
    pub async fn signal_shutdown(&self) {
        let guard = self.shutdown_tx.lock().await;
        if let Some(tx) = guard.as_ref() {
            let _ = tx.send(());
        }
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
            config: self.config.clone(),
            operation_limiter: Arc::clone(&self.operation_limiter),
            shutdown_tx: Arc::clone(&self.shutdown_tx),
        }
    }
}

/// Thread-safe wrapper for shared state
pub struct ThreadSafe<T> {
    inner: Arc<RwLock<T>>,
}

impl<T> ThreadSafe<T> {
    /// Creates a new thread-safe wrapper
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(value)),
        }
    }
    
    /// Gets a read lock on the value
    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, T> {
        self.inner.read().await
    }
    
    /// Gets a write lock on the value
    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, T> {
        self.inner.write().await
    }
    
    /// Tries to get a read lock without blocking
    pub fn try_read(&self) -> Result<tokio::sync::RwLockReadGuard<'_, T>, tokio::sync::TryLockError> {
        self.inner.try_read()
    }
    
    /// Tries to get a write lock without blocking
    pub fn try_write(&self) -> Result<tokio::sync::RwLockWriteGuard<'_, T>, tokio::sync::TryLockError> {
        self.inner.try_write()
    }
}

impl<T> Clone for ThreadSafe<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

/// Async stream utilities for real-time events
pub struct AsyncStreamBuilder;

impl AsyncStreamBuilder {
    /// Creates a stream from a channel receiver
    pub fn from_receiver<T>(
        mut rx: tokio::sync::mpsc::Receiver<T>,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
        T: Send + 'static,
    {
        Box::pin(async_stream::stream! {
            while let Some(item) = rx.recv().await {
                yield item;
            }
        })
    }
    
    /// Creates a stream from a broadcast receiver
    pub fn from_broadcast<T>(
        mut rx: tokio::sync::broadcast::Receiver<T>,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
        T: Clone + Send + 'static,
    {
        Box::pin(async_stream::stream! {
            while let Ok(item) = rx.recv().await {
                yield item;
            }
        })
    }
    
    /// Creates a stream from a watch receiver
    pub fn from_watch<T>(
        mut rx: tokio::sync::watch::Receiver<T>,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
        T: Clone + Send + 'static,
    {
        Box::pin(async_stream::stream! {
            while rx.changed().await.is_ok() {
                yield rx.borrow().clone();
            }
        })
    }
    
    /// Creates a merged stream from multiple streams
    pub fn merge<T>(
        streams: Vec<Pin<Box<dyn Stream<Item = T> + Send>>>,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
        T: Send + 'static,
    {
        use futures::stream::StreamExt;
        
        Box::pin(async_stream::stream! {
            let mut streams = futures::stream::select_all(streams);
            while let Some(item) = streams.next().await {
                yield item;
            }
        })
    }
    
    /// Creates a filtered stream
    pub fn filter<T, F>(
        stream: Pin<Box<dyn Stream<Item = T> + Send>>,
        predicate: F,
    ) -> Pin<Box<dyn Stream<Item = T> + Send>>
    where
        T: Send + 'static,
        F: Fn(&T) -> bool + Send + 'static,
    {
        use futures::stream::StreamExt;
        
        Box::pin(stream.filter(move |item| {
            let result = predicate(item);
            async move { result }
        }))
    }
    
    /// Creates a mapped stream
    pub fn map<T, U, F>(
        stream: Pin<Box<dyn Stream<Item = T> + Send>>,
        mapper: F,
    ) -> Pin<Box<dyn Stream<Item = U> + Send>>
    where
        T: Send + 'static,
        U: Send + 'static,
        F: Fn(T) -> U + Send + 'static,
    {
        use futures::stream::StreamExt;
        
        Box::pin(stream.map(mapper))
    }
}

/// Async mutex wrapper for exclusive access
pub struct AsyncMutex<T> {
    inner: Arc<Mutex<T>>,
}

impl<T> AsyncMutex<T> {
    /// Creates a new async mutex
    pub fn new(value: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
        }
    }
    
    /// Locks the mutex
    pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, T> {
        self.inner.lock().await
    }
    
    /// Tries to lock the mutex without blocking
    pub fn try_lock(&self) -> Result<tokio::sync::MutexGuard<'_, T>, tokio::sync::TryLockError> {
        self.inner.try_lock()
    }
}

impl<T> Clone for AsyncMutex<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod runtime_test;
