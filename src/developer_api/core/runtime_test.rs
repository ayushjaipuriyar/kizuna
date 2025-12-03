/// Unit tests for async runtime integration and thread safety
#[cfg(test)]
mod tests {
    use super::super::runtime::{AsyncRuntime, RuntimeConfig, ThreadSafe, AsyncStreamBuilder};
    use std::time::Duration;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_runtime_creation() {
        let runtime = AsyncRuntime::new();
        assert!(runtime.is_ok(), "Runtime creation should succeed");
    }

    #[tokio::test]
    async fn test_runtime_with_custom_config() {
        let config = RuntimeConfig {
            worker_threads: Some(2),
            thread_name: "test-worker".to_string(),
            ..Default::default()
        };
        
        let runtime = AsyncRuntime::with_config(config);
        assert!(runtime.is_ok(), "Runtime creation with custom config should succeed");
        
        let runtime = runtime.unwrap();
        assert_eq!(runtime.config().worker_threads, Some(2));
        assert_eq!(runtime.config().thread_name, "test-worker");
    }

    #[tokio::test]
    async fn test_spawn_task() {
        let runtime = AsyncRuntime::new().unwrap();
        
        let handle = runtime.spawn(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            42
        });
        
        let result = handle.await.unwrap();
        assert_eq!(result, 42, "Spawned task should return correct value");
    }

    #[tokio::test]
    async fn test_spawn_with_timeout_success() {
        let runtime = AsyncRuntime::new().unwrap();
        
        let handle = runtime.spawn_with_timeout(
            async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                "completed"
            },
            Duration::from_millis(100),
        );
        
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Task should complete within timeout");
        assert_eq!(result.unwrap(), "completed");
    }

    #[tokio::test]
    async fn test_spawn_with_timeout_failure() {
        let runtime = AsyncRuntime::new().unwrap();
        
        let handle = runtime.spawn_with_timeout(
            async {
                tokio::time::sleep(Duration::from_millis(200)).await;
                "completed"
            },
            Duration::from_millis(50),
        );
        
        let result = handle.await.unwrap();
        assert!(result.is_err(), "Task should timeout");
    }

    #[tokio::test]
    async fn test_spawn_blocking() {
        let runtime = AsyncRuntime::new().unwrap();
        
        let handle = runtime.spawn_blocking(|| {
            std::thread::sleep(Duration::from_millis(10));
            "blocking task completed"
        });
        
        let result = handle.await.unwrap();
        assert_eq!(result, "blocking task completed");
    }

    #[tokio::test]
    async fn test_thread_safe_read_write() {
        let counter = ThreadSafe::new(0u32);
        
        // Write to the counter
        {
            let mut value = counter.write().await;
            *value = 42;
        }
        
        // Read from the counter
        {
            let value = counter.read().await;
            assert_eq!(*value, 42, "Counter should have correct value");
        }
    }

    #[tokio::test]
    async fn test_thread_safe_concurrent_access() {
        let counter = ThreadSafe::new(0u32);
        let mut handles = vec![];
        
        // Spawn 10 tasks that increment the counter
        for _ in 0..10 {
            let counter_clone = counter.clone();
            let handle = tokio::spawn(async move {
                let mut value = counter_clone.write().await;
                *value += 1;
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Check final value
        let final_value = counter.read().await;
        assert_eq!(*final_value, 10, "Counter should be incremented 10 times");
    }

    #[tokio::test]
    async fn test_thread_safe_try_read() {
        let counter = ThreadSafe::new(42u32);
        
        let result = counter.try_read();
        assert!(result.is_ok(), "Try read should succeed when not locked");
        assert_eq!(*result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_thread_safe_try_write() {
        let counter = ThreadSafe::new(0u32);
        
        let result = counter.try_write();
        assert!(result.is_ok(), "Try write should succeed when not locked");
        *result.unwrap() = 100;
        
        let value = counter.read().await;
        assert_eq!(*value, 100);
    }

    #[tokio::test]
    async fn test_async_stream_from_receiver() {
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        
        // Send some values
        tokio::spawn(async move {
            for i in 0..5 {
                let _ = tx.send(i).await;
            }
        });
        
        // Create stream and collect values
        let mut stream = AsyncStreamBuilder::from_receiver(rx);
        let mut values = vec![];
        
        while let Some(value) = stream.next().await {
            values.push(value);
            if values.len() == 5 {
                break;
            }
        }
        
        assert_eq!(values, vec![0, 1, 2, 3, 4], "Stream should yield correct values");
    }

    #[tokio::test]
    async fn test_async_stream_from_broadcast() {
        let (tx, rx) = tokio::sync::broadcast::channel(10);
        
        // Send some values
        for i in 0..3 {
            let _ = tx.send(i);
        }
        
        // Create stream and collect values
        let mut stream = AsyncStreamBuilder::from_broadcast(rx);
        let mut values = vec![];
        
        for _ in 0..3 {
            if let Some(value) = stream.next().await {
                values.push(value);
            }
        }
        
        assert_eq!(values, vec![0, 1, 2], "Broadcast stream should yield correct values");
    }

    #[tokio::test]
    async fn test_async_stream_from_watch() {
        let (tx, rx) = tokio::sync::watch::channel(0);
        
        // Spawn task to update values
        tokio::spawn(async move {
            for i in 1..=3 {
                tokio::time::sleep(Duration::from_millis(10)).await;
                let _ = tx.send(i);
            }
        });
        
        // Create stream and collect values
        let mut stream = AsyncStreamBuilder::from_watch(rx);
        let mut values = vec![];
        
        for _ in 0..3 {
            if let Some(value) = stream.next().await {
                values.push(value);
            }
        }
        
        assert_eq!(values.len(), 3, "Watch stream should yield 3 values");
    }

    #[tokio::test]
    async fn test_shutdown_signal() {
        let runtime = AsyncRuntime::new().unwrap();
        
        // Subscribe to shutdown
        let mut rx = runtime.subscribe_shutdown().await;
        
        // Spawn task to wait for shutdown
        let handle = tokio::spawn(async move {
            rx.recv().await.is_ok()
        });
        
        // Signal shutdown
        tokio::time::sleep(Duration::from_millis(10)).await;
        runtime.signal_shutdown().await;
        
        // Check that shutdown was received
        let received = handle.await.unwrap();
        assert!(received, "Shutdown signal should be received");
    }

    #[tokio::test]
    async fn test_operation_limiter() {
        let runtime = AsyncRuntime::new().unwrap();
        
        // Acquire a permit
        let permit = runtime.acquire_operation_permit().await;
        assert!(permit.is_ok(), "Should be able to acquire operation permit");
    }

    #[tokio::test]
    async fn test_runtime_clone() {
        let runtime1 = AsyncRuntime::new().unwrap();
        let runtime2 = runtime1.clone();
        
        // Both runtimes should be able to spawn tasks
        let handle1 = runtime1.spawn(async { 1 });
        let handle2 = runtime2.spawn(async { 2 });
        
        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();
        
        assert_eq!(result1, 1);
        assert_eq!(result2, 2);
    }
}
