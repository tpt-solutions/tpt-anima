//! Example: boot the Tier 1 Core Hypervisor and spawn a legacy-OS VM.
//!
//! Run with: `cargo run -p tpt-anima-core-hypervisor --example boot`

use tpt_anima_core_hypervisor::{AnimaHypervisor, HypervisorConfig, VMConfig};

fn main() {
    let config = HypervisorConfig::default();
    let mut hypervisor = AnimaHypervisor::new(config).expect("hypervisor constructs");

    // Boot: health-checks the SSE, INP, CK, and ZKPA subsystems.
    hypervisor.boot().expect("boot succeeds");
    println!("Hypervisor booted. Running: {}", hypervisor.stats().running);

    // Spawn a sandboxed VM for a legacy operating system.
    let legacy_os = VMConfig::new("LegacyOS".into(), 8192, 4096, 8);
    let session = hypervisor.spawn_vm(legacy_os).expect("vm spawns");
    println!(
        "Spawned VM '{}' (session {})",
        session.config.name, session.session_id
    );

    let stats = hypervisor.stats();
    println!(
        "Stats: running={}, vms={}, cpu_target={}%, mem_target={}%",
        stats.running, stats.vm_count, stats.cpu_usage, stats.memory_utilization
    );

    hypervisor.halt();
    println!("Hypervisor halted. Running: {}", hypervisor.stats().running);
}
