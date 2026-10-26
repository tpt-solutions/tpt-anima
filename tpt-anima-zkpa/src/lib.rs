//! Zero-Knowledge Proof of Agency — the immune system of TPT Anima.
//!
//! Replaces OAuth/API keys with ephemeral, cryptographically bound identities
//! tied to specific tasks, backed by hardware attestation and ZK proofs.
//! Phase 3 implementation lives in sub-modules added during Months 7–9.
//!
//! ```
//! use std::time::Duration;
//! use uuid::Uuid;
//! use tpt_anima_zkpa::{ProofGenerator, ProofSystem, SessionManager, ZKCircuit};
//!
//! let agent_id = Uuid::new_v4();
//!
//! // No persistent API keys: sessions are ephemeral and task-scoped.
//! let mut sessions = SessionManager::new();
//! let session = sessions.create_session(
//!     agent_id,
//!     "quarterly-report-analysis".into(),
//!     10.0,
//!     Duration::from_secs(300),
//!     None,
//! );
//!
//! // Real ZK proof of code integrity, not a mock signature.
//! let proofs = ProofGenerator::new();
//! let circuit = ZKCircuit { circuit_type: "code_integrity".into(), proof_system: ProofSystem::Groth16 };
//! let proof = proofs
//!     .generate_code_integrity_proof(b"tpt-anima-ck::scheduler v0.1.0", &circuit)
//!     .expect("proof generation");
//! assert!(proofs.verify(&proof).is_ok());
//!
//! sessions.revoke_session(session.id).expect("instant revocation");
//! ```
//!
//! See `examples/session_and_attestation.rs` for the full walkthrough including
//! hash-chained liveness heartbeats and the append-only `AgencyLedger`.

#[cfg(feature = "async")]
pub mod async_proofs;
pub mod attestation;
pub mod delegation;
pub mod ledger;
pub mod liveness;
pub mod mcp;
pub mod proofs;
pub mod session;
pub mod types;

#[cfg(feature = "async")]
pub use async_proofs::{
    AsyncProofGenerator, BatchProofGenerator, ProofCache, ProofData, ProofJob, ProofResult,
    ProofSubmitError,
};
pub use attestation::AttestationService;
pub use delegation::DelegationManager;
pub use ledger::AgencyLedger;
pub use liveness::LivenessMonitor;
pub use mcp::{McpSecurityContextTool, McpSecurityResource};
pub use proofs::ProofGenerator;
pub use session::SessionManager;
pub use types::*;
