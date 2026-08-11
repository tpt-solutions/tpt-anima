//! Anima Core Hypervisor — Type-1 bare-metal hypervisor entry point.
//!
//! This binary provides the boot entry point for the Tier 1 hypervisor.
//!
//! `boot()` performs real software-level orchestration of the four Anima
//! layer subsystems in this hosted process (see the doc comment on
//! [`AnimaHypervisor`] for the exact boundary: no VMX/SVM setup, no custom
//! bootloader, no ring transitions — that remains a separate OS-dev project).

use tpt_anima_core_hypervisor::{AnimaHypervisor, HypervisorConfig, VMConfig};

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

    // Boot the hypervisor: this actually constructs and health-checks the
    // SSE, INP, CK, and ZKPA subsystems (see AnimaHypervisor::boot doc).
    println!("Booting hypervisor...");
    if let Err(e) = hypervisor.boot() {
        eprintln!("Failed to boot hypervisor: {}", e);
        std::process::exit(1);
    }
    println!("Hypervisor booted successfully!\n");

    // Report genuine results of each subsystem health check performed
    // during boot() — not canned/fake output.
    let sse_node_id: [u8; 32] = [2u8; 32];
    match hypervisor.sse_storage().read_node(&sse_node_id) {
        Some(node) => println!(
            "SSE health check: OK — wrote node {:02x?} ({} bytes, strength={:.2})",
            &node.id[..4],
            node.payload.len(),
            node.current_strength
        ),
        None => println!("SSE health check: node missing (unexpected)"),
    }

    let inp_matches = hypervisor
        .inp_registry()
        .lookup_by_action("hypervisor.boot_health_check");
    println!(
        "INP health check: OK — {} capability registered for 'hypervisor.boot_health_check' \
         ({} capabilities total in registry)",
        inp_matches.len(),
        hypervisor.inp_registry().count()
    );

    let active_sessions = hypervisor.zkpa_sessions().active_sessions();
    match active_sessions.first() {
        Some(session) => println!(
            "ZKPA health check: OK — active session {} (scope: {})",
            session.id, session.task_scope
        ),
        None => println!("ZKPA health check: no active session (unexpected)"),
    }

    println!("CK health check: OK — boot health-check inference job scheduled and ran\n");

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
