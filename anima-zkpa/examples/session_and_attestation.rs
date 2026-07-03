//! ZKPA — Ephemeral Session, ZK Proof, and Liveness
//!
//! Issues a task-scoped `SessionCredential` (no persistent API key), generates
//! a real Groth16 code-integrity proof and verifies it, emits a hash-chained
//! liveness heartbeat, and appends both to the append-only `AgencyLedger`.
//!
//! Run with: `cargo run -p anima-zkpa --example session_and_attestation`

use std::time::Duration;

use sha2::{Digest, Sha256};
use uuid::Uuid;

use anima_zkpa::{
    AgencyLedger, LedgerEventType, LivenessMonitor, ProofGenerator, ProofSystem, SessionManager,
    ZKCircuit,
};

fn main() {
    let agent_id = Uuid::new_v4();
    let mut ledger = AgencyLedger::new();

    // ── Ephemeral, task-scoped session (no persistent API key) ──────────
    let mut sessions = SessionManager::new();
    let session = sessions.create_session(
        agent_id,
        "quarterly-report-analysis".into(),
        10.0,
        Duration::from_secs(300),
        None,
    );
    println!("1. Issued session {} for agent {} (expires in 300s)", session.id, agent_id);
    ledger
        .append(LedgerEventType::SessionCreated, agent_id, format!("session {}", session.id))
        .unwrap();

    // ── Real ZK proof of code integrity ──────────────────────────────────
    let proofs = ProofGenerator::new();
    let circuit = ZKCircuit { circuit_type: "code_integrity".into(), proof_system: ProofSystem::Groth16 };
    let code_hash = Sha256::digest(b"anima-ck::scheduler v0.1.0").to_vec();
    let proof = proofs
        .generate_code_integrity_proof(&code_hash, &circuit)
        .expect("proof generation");
    let verified = proofs.verify(&proof).is_ok();
    println!("2. Generated {} proof, verify() = {}", proof.proof_type, verified);
    ledger
        .append(LedgerEventType::ProofGenerated, agent_id, format!("{} proof verified={verified}", proof.proof_type))
        .unwrap();

    // ── Hash-chained liveness heartbeats + gap detection ─────────────────
    let mut liveness = LivenessMonitor::new();
    for _ in 0..3 {
        let hb = liveness.emit_heartbeat(agent_id, None).expect("emit heartbeat");
        println!("3. Heartbeat sequence {} recorded", hb.sequence_num);
    }
    println!(
        "4. Chain length: {}, validate_chain() = {:?}",
        liveness.chain_length(agent_id),
        liveness.validate_chain(agent_id)
    );
    ledger
        .append(LedgerEventType::LivenessProven, agent_id, "3-heartbeat chain validated".into())
        .unwrap();

    // ── Revoke the session and show the ledger is append-only ───────────
    sessions.revoke_session(session.id).expect("revoke");
    ledger
        .append(LedgerEventType::SessionRevoked, agent_id, format!("session {} revoked", session.id))
        .unwrap();
    println!("5. Session revoked. Agency ledger now has {} entries for this agent.", ledger.query_by_agent(agent_id).len());
}
