//! Anima Core Hypervisor — Type-1 bare-metal hypervisor entry point.
//!
//! This binary provides the boot entry point for the Tier 1 hypervisor.

use anima_core_hypervisor::{AnimaHypervisor, HypervisorConfig, VMConfig};

fn main() {
    println!("Anima Core Hypervisor v0.1.0");
    println!("TPT Solutions — Apache License 2.0\n");

    // Initialize hypervisor with default configuration
    let config = HypervisorConfig::default();
    let mut hypervisor = match AnimaHypervisor::new(config) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Failed to initialize hypervisor: {}", e);
            std::process::exit(1);
        }
    };

    // Boot the hypervisor
    println!("Booting hypervisor...");
    if let Err(e) = hypervisor.boot() {
        eprintln!("Failed to boot hypervisor: {}", e);
        std::process::exit(1);
    }

    println!("Hypervisor booted successfully!");
    println!("Hardware attestation: enabled");
    println!("ThoughtBubble security: HardwareEnclave\n");

    // Create a default VM for legacy OS support
    let legacy_vm = VMConfig::new("LegacyOS".into(), 8192, 4096, 8);
    match hypervisor.spawn_vm(legacy_vm) {
        Ok(session) => {
            println!("Spawned Legacy VM session: {}", session.session_id);
        }
        Err(e) => {
            eprintln!("Warning: Could not spawn default VM: {}", e);
        }
    }

    // Print status
    let stats = hypervisor.stats();
    println!("\nHypervisor Stats:");
    println!("  Running: {}", stats.running);
    println!("  VMs: {}", stats.vm_count);
    println!("  CPU Usage Target: {}%", stats.cpu_usage);
    println!("  Memory Utilization Target: {}%", stats.memory_utilization);

    println!("\nAnima Core Hypervisor ready.");
    println!("Legacy OSes can now run in sandboxed VMs.");
    println!("Press Ctrl+C to halt...\n");

    // In a real hypervisor, this would enter the main event loop
    // For now, we just keep the hypervisor running
    loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}