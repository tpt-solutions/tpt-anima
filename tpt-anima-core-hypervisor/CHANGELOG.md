# tpt-anima-core-hypervisor

All notable changes to the **tpt-anima-core-hypervisor** crate are documented in this
file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to Semantic Versioning.

## [0.1.0] - 2026-08-11

### Added
- `AnimaHypervisor` real software-level orchestration of the four Anima layers with
  boot-time health checks.
- `HypervisorConfig` / `HypervisorStats` configuration and runtime statistics.
- `VMConfig` / `VMSession` legacy-OS VM bookkeeping.
- `HypervisorError` error taxonomy.
- `tpt-anima-hypervisor` binary booting the hypervisor and reporting subsystem health.
- Depends on `tpt-anima-sse`, `tpt-anima-inp`, `tpt-anima-ck`, `tpt-anima-zkpa`, and
  `tpt-anima-edge-runtime`.
- Crate renamed from `anima-core-hypervisor` to `tpt-anima-core-hypervisor`
  (prefix + directory).
- Added `keywords` and `categories` crate metadata.

[0.1.0]: https://github.com/tpt-solutions/tpt-anima/releases/tag/v0.1.0
