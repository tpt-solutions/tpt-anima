# tpt-anima-core-hypervisor

**Tier 1 Anima Core Hypervisor** — a Type-1 bare-metal hypervisor for the full
[TPT Anima](https://github.com/tpt-solutions/tpt-anima) stack.

> Software-level orchestration of the four Anima layers — not literal bare-metal
> virtualization.

This crate provides the **core hypervisor** that runs directly on hardware and manages
all four Anima layers (SSE, INP, CK, ZKPA) with full hardware control. During
`AnimaHypervisor::boot()` it constructs one real instance of each subsystem and drives
one genuine operation through each subsystem's real API as a health check, failing boot
if any subsystem does not come up cleanly. Legacy operating systems run as isolated VMs
on top.

> Note: this performs real *software-level* orchestration within a hosted process. It
> does **not** perform bare-metal hardware virtualization (no VMX/SVM, bootloader, or
> ring transitions). `spawn_vm` / `VMSession` model VM *bookkeeping* only.

## Key concepts

- **AnimaHypervisor** — owns the four layer subsystems and orchestrates boot/health.
- **HypervisorConfig** — CPU/memory targets, hardware attestation, bubble security.
- **VMConfig / VMSession** — bookkeeping for legacy-OS VMs.
- **HypervisorError** — boot, spawn, and hardware-init failures.

## Installation

```toml
[dependencies]
tpt-anima-core-hypervisor = { git = "https://github.com/tpt-solutions/tpt-anima" }
```

## Quick start

```rust
use tpt_anima_core_hypervisor::{AnimaHypervisor, HypervisorConfig};

let mut hypervisor = AnimaHypervisor::new(HypervisorConfig::default())?;
hypervisor.boot()?;
assert!(hypervisor.stats().running);
```

## Binary

The crate also builds the `tpt-anima-hypervisor` binary (`src/main.rs`) which boots the
hypervisor and reports subsystem health.

## Modules

| Item | Purpose |
|------|---------|
| `AnimaHypervisor` | subsystem orchestration + boot health checks |
| `HypervisorConfig` / `HypervisorStats` | configuration + runtime stats |
| `VMConfig` / `VMSession` | legacy-OS VM bookkeeping |
| `HypervisorError` | error taxonomy |

## Related crates

- `tpt-anima-sse`, `tpt-anima-inp`, `tpt-anima-ck`, `tpt-anima-zkpa` — the four layers
  the hypervisor orchestrates.
- `tpt-anima-co-processor` / `tpt-anima-edge-runtime` — other deployment tiers.

## License

Dual-licensed under the [MIT License](../../LICENSE-MIT) or the [Apache License 2.0](../../LICENSE-APACHE).
