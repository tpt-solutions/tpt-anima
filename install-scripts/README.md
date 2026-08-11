# Anima Installation Scripts

> Copyright TPT Solutions · Apache License 2.0

This directory contains installation scripts for all three deployment tiers of TPT Anima.

## Scripts

| Script | Tier | Platform | Description |
|--------|------|----------|-------------|
| `install-core.sh` | Tier 1 | Linux | Bare-metal hypervisor boot image installation |
| `install-core.ps1` | Tier 1 | Windows | Hyper-V based hypervisor installation |
| `install-coproc.sh` | Tier 2 | Linux | eBPF kernel module for workload interception |
| `install-coproc.ps1` | Tier 2 | Windows | Kernel driver installation for workload interception |
| `install-edge.sh` | Tier 3 | Linux/macOS | WebAssembly/native binary for phones and IoT |
| `install-edge.ps1` | Tier 3 | Windows | Edge runtime installation |

## Usage

### Tier 1: Anima Core Hypervisor

**Linux (bare-metal):**
```bash
sudo ./install-core.sh
```

**Windows:**
```powershell
.\install-core.ps1
```

### Tier 2: Anima Co-Processor

**Linux (eBPF):**
```bash
sudo ./install-coproc.sh
sudo systemctl start tpt-anima-co-processor
```

**Windows:**
```powershell
.\install-coproc.ps1
```

### Tier 3: Anima Edge Runtime

**Linux/macOS:**
```bash
./install-edge.sh
```

**Windows:**
```powershell
.\install-edge.ps1
```

## Uninstallation

All scripts support the `UNINSTALL=true` environment variable:

```bash
# Linux
UNINSTALL=true ./install-core.sh
UNINSTALL=true ./install-coproc.sh
UNINSTALL=true ./install-edge.sh
```

```powershell
# Windows
$env:UNINSTALL="true"; .\install-core.ps1
```

## Custom Installation Path

Set the `INSTALL_PATH` environment variable to customize where binaries are installed:

```bash
INSTALL_PATH=/custom/path ./install-edge.sh
```

## Prerequisites

- **Tier 1**: Requires root/boot access, hardware virtualization support
- **Tier 2**: Requires kernel module loading privileges (CAP_SYS_ADMIN on Linux)
- **Tier 3**: No special privileges required for user-space installation

## Support

For installation issues, please open a GitHub issue or contact support@tptsolutions.io.