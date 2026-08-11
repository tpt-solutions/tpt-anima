# tpt-anima-co-processor

All notable changes to the **tpt-anima-co-processor** crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to Semantic Versioning.

## [0.1.0] - 2026-08-11

### Added
- `CoProcessor` coordinator recording and managing workload intercepts.
- `WorkloadIntercept` / `WorkloadType` with `to_intent()` translation.
- `CoProcessorConfig` enablement and pattern configuration.
- `bpf` pure eBPF bytecode generation/encoding (builds on every platform).
- `driver` Linux-gated kernel load path; depends on `tpt-anima-sse`, `tpt-anima-inp`,
  and `tpt-anima-edge-runtime`.
- Crate renamed from `anima-co-processor` to `tpt-anima-co-processor` (prefix + directory).
- Added `keywords` and `categories` crate metadata.

[0.1.0]: https://github.com/tpt-solutions/tpt-anima/releases/tag/v0.1.0
