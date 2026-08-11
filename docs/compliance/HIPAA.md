# HIPAA Compliance Review

> Copyright TPT Solutions · Apache License 2.0

TPT Anima is infrastructure — a memory substrate, agent negotiation
protocol, inference scheduler, and identity/audit layer. It does not itself
define what counts as PHI (Protected Health Information); that
classification belongs to whatever application stores PHI in `MemoryNode`
payloads. This review covers what Anima provides toward the HIPAA Security
Rule's technical safeguards, and what a deploying application must still
supply.

## Technical Safeguards (45 CFR §164.312) mapping

| Safeguard | Requirement | Anima support |
|---|---|---|
| §164.312(a)(1) Access control | Unique user identification, emergency access, automatic logoff, encryption | `SessionCredential` provides unique, task-scoped, time-limited identity (`tpt-anima-zkpa/src/session.rs`). No persistent credentials to leak. Automatic "logoff" is `expires_at` enforcement in `validate_session`. |
| §164.312(b) Audit controls | Record and examine activity in systems containing ePHI | `AgencyLedger` (`tpt-anima-zkpa/src/ledger.rs`) is an append-only log of every authentication/authorization event across all four layers. |
| §164.312(c)(1) Integrity | Protect ePHI from improper alteration/destruction | `CausalGraph::validate_causal_integrity()` (`tpt-anima-sse`) detects cycles/contradictions; `CircuitBreaker` (`tpt-anima-ck`) halts and rolls back inference that violates causal integrity. |
| §164.312(d) Person/entity authentication | Verify identity before granting access | ZK proof generation (`tpt-anima-zkpa/src/proofs.rs`: authorization, hardware attestation, delegation-authority proofs) plus continuous liveness heartbeats (`tpt-anima-zkpa/src/liveness.rs`) for ongoing verification, not just at login. |
| §164.312(e)(1) Transmission security | Guard against unauthorized access to ePHI in transit | Edge gateway translates INP↔MCP/A2A at the boundary (`tpt-anima-edge-gateway`); internal substrate is never exposed as raw HTTP/JSON. Transport-layer encryption (TLS) for the gateway's external HTTP surface is an infrastructure/deployment concern, not something the crate enforces itself — **verify TLS termination is configured before handling real PHI**. |

## What Anima does NOT provide (deployer responsibility)

- **PHI classification/tagging** — nothing in `MemoryNode` or `SensoryModality`
  marks a payload as containing PHI. A HIPAA-covered deployment must add
  this at the application layer before ingesting data into SSE.
- **Minimum necessary access enforcement** — `DelegationManager` enforces
  scoped permission subsets (see [SOC2 prep](SOC2.md)), but mapping HIPAA's
  "minimum necessary" standard onto specific `scoped_permissions` strings is
  an application-level policy decision.
- **Business Associate Agreements (BAAs)** — organizational/legal, not code.
- **Breach notification procedures** — see [Incident Response](../INCIDENT_RESPONSE.md);
  HIPAA's 60-day notification clock and content requirements are a legal
  process built on top of, not inside, the ledger.
- **Encryption at rest for `MemoryNode` payloads** — `tpt-anima-sse/src/storage.rs`
  persists raw payloads; whether they are encrypted at rest depends on the
  underlying storage backend configuration, not the crate.

## Recommended pre-certification actions

1. Decide and document which `MemoryNode` fields/modalities may carry PHI in
   your deployment, and enforce encryption at rest for that storage tier.
2. Confirm the edge gateway's HTTP listener is deployed behind TLS in every
   environment that will see real PHI.
3. Extend `AgencyLedger::generate_compliance_report("HIPAA")` output (already
   present) into your organization's formal audit-log retention process —
   HIPAA requires 6-year retention of audit records, which the in-memory/
   exportable ledger does not itself guarantee without external archival.
4. Run a HIPAA-focused risk assessment (org-level exercise, not a code task)
   before onboarding real patient data.

This review does not constitute a certification and should be paired with
qualified legal/compliance counsel before storing real PHI.
