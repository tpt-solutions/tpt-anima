# tpt-anima-zkpa

All notable changes to the **tpt-anima-zkpa** crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to Semantic Versioning.

## [0.1.0] - 2026-08-11

### Added
- `SessionManager` ephemeral, task-scoped, revocable sessions.
- `ProofGenerator` with real Groth16 proofs of code integrity and policy compliance.
- `async_proofs` batch/async generation with `ProofCache`.
- `AttestationService` hardware attestation binding.
- `DelegationManager` scoped child-agent permissions.
- `LivenessMonitor` hash-chained heartbeat stream.
- `AgencyLedger` append-only, auditable agency event log.
- `mcp://` security context/resource adapters.
- Crate renamed from `anima-zkpa` to `tpt-anima-zkpa` (prefix + directory).
- Added `keywords` and `categories` crate metadata.

[0.1.0]: https://github.com/tpt-solutions/tpt-anima/releases/tag/v0.1.0
