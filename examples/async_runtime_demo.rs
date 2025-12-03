/// Example demonstrating async runtime integration and thread safety
/// in the Kizuna Developer API
use kizuna::developer_api::core::{
    KizunaAPI, KizunaConfig, KizunaInstance, KizunaEvent,
    runtime::{AsyncRuntime, RuntimeConfig, ThreadSafe, AsyncStreamBuilder},
};
use futures::StreamExt;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Kizuna Async Runtime Integration Demo ===\n");
    
    // Demo 1: Custom Runtime Configuration
    demo_custom_runtime().await?;
    
    // Demo 2: Thread-Safe State Management
    demo_thread_safe_state().await?;
    
    // Demo 3: Async Stream Interfaces
    demo_async_streams().await?;
    
    // Demo 4: Concurrent Task Execution
    demo_concurrent_tasks().await?;
    
    // Demo 5: Shutdown Coordination
    demo_shutdown_coordination().await?;
    
    println!("\n=== Demo Complete ===");
    Ok(())
}

/// Demonstrates custom runtime configuration
async fn demo_custom_runtime() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 1: Custom Runtime Configuration ---");
    
    // Create runtime with custom configuration
    let runtime_config = RuntimeConfig {
        worker_threads: Some(4),
        thread_name: "demo-worker".to_string(),
        max_blocking_threads: Some(8),
        ..Default::default()
    };
    
    let runtime = AsyncRuntime::with_config(runtime_config)?;
    println!("✓ Created runtime with 4 worker threads");
    
    // Spawn tasks on the runtime
    let handle1 = runtime.spawn(async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        "Task 1 completed"
    });
    
    let handle2 = runtime.spawn(async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        "Task 2 completed"
    });
    
    // Wait for tasks to complete
    let result1 = handle1.await?;
    let result2 = handle2.await?;
    
    println!("✓ {}", result1);
    println!("✓ {}", result2);
    
    // Demonstrate spawn_with_timeout
    let timeout_handle = runtime.spawn_with_timeout(
        async {
            tokio::time::sleep(Duration::from_secs(10)).await;
            "This should timeout"
        },
        Duration::from_millis(100),
    );
    
    match timeout_handle.await? {
        Ok(_) => println!("✗ Task completed (unexpected)"),
        Err(_) => println!("✓ Task timed out as expected"),
    }
    
    println!();
    Ok(())
}

/// Demonstrates thread-safe state management
async fn demo_thread_safe_state() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 2: Thread-Safe State Management ---");
    
    // Create thread-safe counter
    let counter = ThreadSafe::new(0u32);
    
    // Spawn multiple tasks that increment the counter
    let mut handles = vec![];
    for i in 0..10 {
        let counter_clone = counter.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(i * 10)).await;
            let mut value = counter_clone.write().await;
            *value += 1;
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await?;
    }
    
    // Read final value
    let final_value = counter.read().await;
    println!("✓ Counter value after 10 concurrent increments: {}", *final_value);
    
    // Demonstrate try_read and try_write
    match counter.try_read() {
        Ok(value) => println!("✓ Non-blocking read successful: {}", *value),
        Err(_) => println!("✗ Non-blocking read failed"),
    }
    
    println!();
    Ok(())
}

/// Demonstrates async stream interfaces
async fn demo_async_streams() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 3: Async Stream Interfaces ---");
    
    // Create a channel-based stream
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    
    // Spawn a task to send events
    tokio::spawn(async move {
        for i in 0..5 {
            let _ = tx.send(format!("Event {}", i)).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });
    
    // Create stream from receiver
    let mut stream = AsyncStreamBuilder::from_receiver(rx);
    
    println!("✓ Receiving events from stream:");
    while let Some(event) = stream.next().await {
        println!("  - {}", event);
    }
    
    // Demonstrate broadcast stream
    let (broadcast_tx, broadcast_rx) = tokio::sync::broadcast::channel(10);
    
    // Send some events
    for i in 0..3 {
        let _ = broadcast_tx.send(format!("Broadcast {}", i));
    }
    
    let mut broadcast_stream = AsyncStreamBuilder::from_broadcast(broadcast_rx);
    
    println!("✓ Receiving events from broadcast stream:");
    for _ in 0..3 {
        if let Some(event) = broadcast_stream.next().await {
            println!("  - {}", event);
        }
    }
    
    println!();
    Ok(())
}

/// Demonstrates concurrent task execution with operation limiting
async fn demo_concurrent_tasks() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 4: Concurrent Task Execution ---");
    
    let config = KizunaConfig {
        runtime_threads: Some(2),
        ..Default::default()
    };
    
    let instance = KizunaInstance::new(config)?;
    
    // Spawn multiple tasks concurrently
    let mut handles = vec![];
    for i in 0..5 {
        let handle = instance.spawn_task(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            format!("Concurrent task {} completed", i)
        });
        handles.push(handle);
    }
    
    println!("✓ Spawned 5 concurrent tasks");
    
    // Wait for all tasks and collect results
    for handle in handles {
        let result = handle.await?;
        println!("  - {}", result);
    }
    
    // Demonstrate task with timeout
    let timeout_result = instance.spawn_task_with_timeout(
        async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            "Fast task"
        },
        Duration::from_millis(200),
    ).await?;
    
    match timeout_result {
        Ok(result) => println!("✓ Task completed within timeout: {}", result),
        Err(_) => println!("✗ Task timed out"),
    }
    
    println!();
    Ok(())
}

/// Demonstrates shutdown coordination
async fn demo_shutdown_coordination() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 5: Shutdown Coordination ---");
    
    let config = KizunaConfig::default();
    let instance = KizunaInstance::new(config)?;
    
    // Initialize systems
    instance.initialize_systems().await?;
    println!("✓ Systems initialized");
    
    // Check shutdown status
    let is_shutdown = instance.is_shutdown().await;
    println!("✓ Shutdown status before shutdown: {}", is_shutdown);
    
    // Subscribe to shutdown signal
    let mut shutdown_rx = instance.runtime().subscribe_shutdown().await;
    
    // Spawn a task that waits for shutdown
    let shutdown_handle = tokio::spawn(async move {
        match shutdown_rx.recv().await {
            Ok(_) => println!("  - Received shutdown signal"),
            Err(_) => println!("  - Shutdown channel closed"),
        }
    });
    
    // Perform shutdown
    tokio::time::sleep(Duration::from_millis(100)).await;
    instance.shutdown().await?;
    println!("✓ Shutdown initiated");
    
    // Wait for shutdown task
    shutdown_handle.await?;
    
    // Check shutdown status after shutdown
    let is_shutdown = instance.is_shutdown().await;
    println!("✓ Shutdown status after shutdown: {}", is_shutdown);
    
    // Try to use API after shutdown (should fail)
    match instance.discover_peers().await {
        Ok(_) => println!("✗ API call succeeded after shutdown (unexpected)"),
        Err(e) => println!("✓ API call failed after shutdown as expected: {}", e),
    }
    
    println!();
    Ok(())
}
