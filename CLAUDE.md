# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build --workspace

# Lint (must pass clean — -D warnings is enforced)
cargo clippy --workspace -- -D warnings

# Test all crates
cargo test --workspace

# Test a single crate
cargo test -p tpt-anima-sse

# Run a single test by name
cargo test -p tpt-anima-sse memory_node::tests::apply_decay_for_follows_exponential_formula

# Format
cargo fmt --all

# Format check only
cargo fmt --all -- --check
```

Rustfmt config (`rustfmt.toml`): max_width 100, 4-space tabs, `imports_granularity = "Module"`, `group_imports = "StdExternalCrate"`.

## Architecture

TPT Anima is a four-layer AI runtime rebuilt from first principles. Each layer is a separate workspace crate. Two supporting crates handle external interop and observability.

```
tpt-anima-sse        — Semantic State Engine   (memory substrate)
tpt-anima-inp        — Intent & Negotiation Protocol (agent nervous system)
tpt-anima-ck         — Cognitive Kernel         (inference scheduler / brainstem)
tpt-anima-zkpa       — Zero-Knowledge Proof of Agency (identity / immune system)

tpt-anima-edge-gateway  — INP ↔ MCP/A2A JSON translation for external agents
tpt-anima-observatory   — Real-time telemetry backend
```

### Layer 1 — SSE (`tpt-anima-sse`)

The memory substrate. Core type is `MemoryNode`: a vector embedding + raw payload + causal links + temporal decay state + per-node compute ledger. Key design invariants:

- **Memory is attended, not queried.** `InMemoryStorage::attention_retrieve()` ranks by `current_strength`, not cosine similarity. Cosine search is available separately via `InMemoryStorage::semantic_search()`, backed by the `AnnIndex` in `ann.rs`.
- **Decay is exponential and real-time.** `apply_decay()` computes `strength *= exp(-rate * Δhours)`. `apply_decay_for(Duration)` is the testable variant (no wall-clock dependency). `record_access()` applies a multiplicative reinforcement boost.
- **Causal graph is a first-class citizen.** `CausalGraph` is an adjacency list of typed `CausalLink`s (`Caused`, `Preceded`, `PartOf`, `SimilarTo`, `Contradicts`, `Abstracts`). `get_context_chain()` walks backward to ancestors, not just cosine neighbors. `validate_causal_integrity()` detects cycles and contradictions and is called by the CK circuit breaker.
- **Three memory-type presets** in `decay.rs`: `Episodic` (fast decay, λ=0.05), `Semantic` (slow, λ=0.005), `Procedural` (very slow, λ=0.001).
- MCP exposure in `mcp.rs` defines `mcp://resources/sse/node/{id}`, `mcp://resources/sse/substrate`, and `mcp://resources/sse/causal/{id}` — agents attend via these, not SQL/REST.

### Layer 2 — INP (`tpt-anima-inp`)

The agent nervous system. Agents broadcast `Intent` structs; capability providers respond with `CapabilityBid`s; `NegotiationSession` routes execution through a state machine: `Created → Broadcasting → CollectingBids → Executing → Completed/Failed`. Automatic fallback to next-best provider on failure. `QoSConstraints` bound latency, accuracy, cost, region, and deadline. A `ComputeLedger` tracks credit transfers between agents per negotiation.

`manifest.rs` contains `ManifestParser` — parses human-readable YAML/TOML capability declarations into `CapabilityManifest` structs for registration in `CapabilityRegistry`.

### Layer 3 — ZKPA (`tpt-anima-zkpa`)

The immune system. `SessionCredential` is task-scoped and time-limited — no persistent API keys. `ProofGenerator` produces ZK proofs (code integrity, authorization, policy compliance, hardware attestation, liveness, computation correctness, delegation authority). `LivenessMonitor` emits hash-chained heartbeats; gap detection identifies tampered or halted execution. `DelegationManager` enforces parent→child scoped permission constraints with instant cryptographic revocation. `AgencyLedger` is append-only and used for SOC2/HIPAA/FedRAMP compliance export.

### Layer 4 — CK (`tpt-anima-ck`)

The brainstem. Fundamental unit of compute is an `Inference`, not a thread. `InferenceScheduler` uses attention-weight preemption (highest-weight inference wins, not round-robin). `MemoryFabric` abstracts VRAM/RAM/NVMe into a single address space with page-in/page-out and predictive prefetch. `CircuitBreaker` monitors output entropy and repetition loops, halting and rolling back inferences that violate causal integrity (integrates with SSE's `validate_causal_integrity()`). `ThoughtBubble` provides cryptographically isolated sandboxes; compromised bubbles are quarantined without halting the scheduler.

`tpt-anima-ck/src/jit/` implements the JIT hardware-fluid engine: hardware detection (`detection.rs`) and backend dispatch (`dispatch.rs`, `backend.rs`) for CUDA/ANE/CPU. The CoreML/ANE backend emits a real CoreML model container (`coreml.rs`, `build_identity_model`), but its execution path (`AnEBackend::execute`) still returns a stub string — real ANE execution is the remaining gap.

### Edge Gateway (`tpt-anima-edge-gateway`)

Translates native INP to/from MCP and A2A JSON for external agent interop. Keeps the internal substrate free of HTTP/JSON. Implemented, with a CLI entry point and test coverage.

## Key Cross-Layer Relationships

- **SSE → CK**: `CircuitBreaker` in CK calls SSE's `validate_causal_integrity()` to detect causal violations during inference.
- **INP → SSE**: `ComputeLedgerEntry` structs in SSE mirror the micro-economy pattern from INP's `ComputeLedger` — same philosophy, per-node tracking.
- **ZKPA → All**: Every agent operation should carry a `SessionCredential` from ZKPA; the `AgencyLedger` records all authentication/authorization events across all layers.
- **All → Observatory**: Each layer emits telemetry to `tpt-anima-observatory` (Phase 4).

## `SystemTime` Serialization

Each crate defines its own `serde_system_time` helper module (not shared via workspace dep). SSE uses nanoseconds as `u64`; CK uses milliseconds as `u128`; ZKPA uses `(secs: u64, nanos: u32)`. This is intentional isolation — don't unify them without considering cross-crate serialization compatibility.
