// System Information Query Demo
//
// This example demonstrates the system information query capabilities
// including hardware info, system metrics, software info, and network info.

use kizuna::command_execution::SystemInfoProvider;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== System Information Query Demo ===\n");

    // Create a system information provider
    let provider = SystemInfoProvider::new();

    // Get complete system information
    println!("Fetching complete system information...\n");
    let system_info = provider.get_system_info(None).await?;

    // Display hardware information
    println!("--- Hardware Information ---");
    println!("CPU Model: {}", system_info.hardware.cpu.model);
    println!("CPU Cores: {}", system_info.hardware.cpu.cores);
    println!("CPU Frequency: {} MHz", system_info.hardware.cpu.frequency_mhz);
    println!("Total Memory: {} MB", system_info.hardware.memory.total_mb);
    println!("Available Memory: {} MB", system_info.hardware.memory.available_mb);
    
    println!("\nStorage Devices:");
    for storage in &system_info.hardware.storage {
        println!("  - {}: {} GB total, {} GB available ({})", 
            storage.name,
            storage.total_gb,
            storage.available_gb,
            storage.mount_point.display()
        );
    }

    if let Some(battery) = &system_info.hardware.battery {
        println!("\nBattery:");
        println!("  - Charge: {:.1}%", battery.percentage);
        println!("  - Charging: {}", if battery.is_charging { "Yes" } else { "No" });
    }

    // Display system metrics
    println!("\n--- System Metrics ---");
    println!("CPU Usage: {:.1}%", system_info.system.cpu_usage);
    println!("Memory Usage: {} MB / {} MB ({:.1}%)", 
        system_info.system.memory_usage.used_mb,
        system_info.system.memory_usage.total_mb,
        system_info.system.memory_usage.percentage
    );
    println!("Uptime: {} seconds", system_info.system.uptime.as_secs());
    
    if let Some(load_avg) = system_info.system.load_average {
        println!("Load Average: {:.2}, {:.2}, {:.2}", load_avg[0], load_avg[1], load_avg[2]);
    }

    println!("\nDisk Usage:");
    for disk in &system_info.system.disk_usage {
        println!("  - {}: {} GB / {} GB ({:.1}%)", 
            disk.mount_point.display(),
            disk.used_gb,
            disk.total_gb,
            disk.percentage
        );
    }

    // Display software information
    println!("\n--- Software Information ---");
    println!("OS Name: {}", system_info.software.os_name);
    println!("OS Version: {}", system_info.software.os_version);
    println!("Kernel Version: {}", system_info.software.kernel_version);
    println!("Hostname: {}", system_info.software.hostname);

    // Display network information
    println!("\n--- Network Information ---");
    if let Some(gateway) = &system_info.network.default_gateway {
        println!("Default Gateway: {}", gateway);
    }
    
    println!("\nNetwork Interfaces:");
    for interface in &system_info.network.interfaces {
        println!("  - {} ({})", 
            interface.name,
            if interface.is_up { "UP" } else { "DOWN" }
        );
        if let Some(mac) = &interface.mac_address {
            println!("    MAC: {}", mac);
        }
        if !interface.ip_addresses.is_empty() {
            println!("    IPs: {}", interface.ip_addresses.join(", "));
        }
    }

    // Demonstrate caching
    println!("\n--- Caching Demo ---");
    println!("Fetching hardware info with 60-second cache...");
    let start = std::time::Instant::now();
    let _ = provider.get_hardware_info(Some(Duration::from_secs(60))).await?;
    let first_duration = start.elapsed();
    println!("First fetch took: {:?}", first_duration);

    println!("Fetching again (should use cache)...");
    let start = std::time::Instant::now();
    let _ = provider.get_hardware_info(Some(Duration::from_secs(60))).await?;
    let second_duration = start.elapsed();
    println!("Second fetch took: {:?}", second_duration);
    println!("Cache speedup: {:.2}x faster", first_duration.as_micros() as f64 / second_duration.as_micros() as f64);

    // Clear cache
    println!("\nClearing cache...");
    provider.clear_cache();
    println!("Cache cleared successfully");

    println!("\n=== Demo Complete ===");
    Ok(())
}
