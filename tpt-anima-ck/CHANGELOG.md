# tpt-anima-ck

All notable changes to the **tpt-anima-ck** crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to Semantic Versioning.

## [0.1.0] - 2026-08-11

### Added
- `InferenceScheduler` with attention-weighted preemption (`scheduler`).
- Inference resource model: `Inference`, `ResourceRequest`, `KVCacheConfig` (`types`).
- `circuit_breaker` safety halts for causal violations and repetition loops.
- `memory_fabric` unified VRAM/RAM/NVMe address space.
- `kv_swap_optimization` KV-cache swap planning.
- `budgeting` compute/thermal budget enforcement.
- `thought_bubble` sandboxed inference execution.
- `jit` hardware-fluid detection and scheduling tuning.
- `mcp://` scheduler/tools adapters.
- Crate renamed from `anima-ck` to `tpt-anima-ck` (prefix + directory).
- Added `keywords` and `categories` crate metadata.

[0.1.0]: https://github.com/tpt-solutions/tpt-anima/releases/tag/v0.1.0
