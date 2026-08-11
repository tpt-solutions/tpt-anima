# tpt-anima-zkpa

**Zero-Knowledge Proof of Agency (ZKPA)** — the immune system of
[TPT Anima](https://github.com/tpt-solutions/tpt-anima).

> *Identity is ephemeral and provable.*

Replaces OAuth / API keys / IAM with **ephemeral, cryptographically bound identities**
tied to specific tasks. ZK proofs demonstrate code integrity, policy compliance, and
hardware attestation without exposing model weights or source. Parent agents spawn
child agents with mathematically scoped permissions, and continuous **liveness
heartbeats** form a tamper-evident hash chain.

## Key concepts

- **Ephemeral sessions** — `SessionManager` issues task-scoped, time-limited
  credentials (`SessionCredential`) with instant revocation.
- **Real ZK proofs** — `ProofGenerator` produces Groth16 proofs of code integrity and
  policy compliance over `ZKCircuit`s (built on `ark-*`).
- **Attestation** — `AttestationService` binds proofs to hardware.
- **Delegation** — `DelegationManager` scopes child-agent permissions.
- **Liveness** — `LivenessMonitor` emits hash-chained heartbeats.
- **Agency ledger** — `AgencyLedger` is the append-only, auditable proof/event log.
- **Async proofs** — `AsyncProofGenerator` / `BatchProofGenerator` with `ProofCache`.

## Installation

```toml
[dependencies]
tpt-anima-zkpa = { git = "https://github.com/tpt-solutions/tpt-anima" }
```

## Quick start

```rust
use std::time::Duration;
use uuid::Uuid;
use tpt_anima_zkpa::{ProofGenerator, ProofSystem, SessionManager, ZKCircuit};

let agent_id = Uuid::new_v4();

// No persistent API keys: sessions are ephemeral and task-scoped.
let mut sessions = SessionManager::new();
let session = sessions.create_session(
    agent_id,
    "quarterly-report-analysis".into(),
    10.0,
    Duration::from_secs(300),
    None,
);

// Real ZK proof of code integrity, not a mock signature.
let proofs = ProofGenerator::new();
let circuit = ZKCircuit { circuit_type: "code_integrity".into(), proof_system: ProofSystem::Groth16 };
let proof = proofs
    .generate_code_integrity_proof(b"tpt-anima-ck::scheduler v0.1.0", &circuit)
    .expect("proof generation");
assert!(proofs.verify(&proof).is_ok());

sessions.revoke_session(session.id).expect("instant revocation");
```

## Modules

| Module | Purpose |
|--------|---------|
| `session` | `SessionManager` + `SessionCredential` lifecycle |
| `proofs` | `ProofGenerator` Groth16 code-integrity/policy proofs |
| `async_proofs` | async/batch proof generation with caching |
| `attestation` | `AttestationService` hardware binding |
| `delegation` | `DelegationManager` scoped child agents |
| `liveness` | `LivenessMonitor` hash-chained heartbeats |
| `ledger` | `AgencyLedger` append-only audit log |
| `types` | sessions, circuits, errors, ledger event types |
| `mcp` | `mcp://` security context/resource adapters |

## Runnable example

```bash
cargo run -p tpt-anima-zkpa --example session_and_attestation
```

Creates an ephemeral session, generates/verifies a real ZK proof, and drives the
hash-chained liveness heartbeat into the `AgencyLedger`.

## Related crates

- `tpt-anima-sse`, `tpt-anima-inp`, `tpt-anima-ck` — the other three Anima layers.

## License

Dual-licensed under the [MIT License](../../LICENSE-MIT) or the [Apache License 2.0](../../LICENSE-APACHE).
