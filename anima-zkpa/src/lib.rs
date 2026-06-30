//! Zero-Knowledge Proof of Agency — the immune system of TPT Anima.
//!
//! Replaces OAuth/API keys with ephemeral, cryptographically bound identities
//! tied to specific tasks, backed by hardware attestation and ZK proofs.
//! Phase 3 implementation lives in sub-modules added during Months 7–9.

pub mod attestation;
pub mod delegation;
pub mod ledger;
pub mod liveness;
pub mod mcp;
pub mod proofs;
pub mod session;
pub mod types;

pub use attestation::AttestationService;
pub use delegation::DelegationManager;
pub use ledger::AgencyLedger;
pub use liveness::LivenessMonitor;
pub use mcp::{McpSecurityContextTool, McpSecurityResource};
pub use proofs::ProofGenerator;
pub use session::SessionManager;
pub use types::*;
