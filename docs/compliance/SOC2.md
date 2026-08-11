# SOC 2 Type II — Certification Preparation

> Copyright TPT Solutions · Dual-licensed under MIT or Apache-2.0

This document maps TPT Anima's existing controls to the SOC 2 Trust Services
Criteria (TSC) and lists the evidence-collection work required before
engaging an auditor. It is a preparation aid, not a certification — SOC 2
Type II requires an independent third-party audit over an observation
period (typically 6–12 months) and cannot be self-certified.

## Control mapping

| TSC Category | Control | Implementation |
|---|---|---|
| CC6.1 — Logical access | Task-scoped, time-limited credentials (no persistent API keys) | `tpt-anima-zkpa/src/session.rs::SessionCredential`, `SessionManager::validate_session` enforces `expires_at` |
| CC6.1 — Least privilege | Delegated authority cannot exceed the delegator's own granted scope | `tpt-anima-zkpa/src/delegation.rs::DelegationManager::delegate` (permission-subset check) |
| CC6.1 — Revocation | Instant, cryptographically witnessed revocation | `DelegationManager::revoke`, `AuthorityDelegation::revocation_witness` |
| CC6.6 — Boundary protection | Hardware-attested execution contexts | `tpt-anima-zkpa/src/attestation.rs` (TPM, SGX, TrustZone, NVIDIA CC, Apple SE) |
| CC6.7 — Data transmission | INP/A2A/MCP payload envelopes are versioned and validated before execution | `tpt-anima-inp/src/contracts.rs` |
| CC7.1 — Anomaly detection | Continuous liveness heartbeats with tamper/gap detection | `tpt-anima-zkpa/src/liveness.rs::LivenessMonitor::detect_gaps` |
| CC7.2 — Incident monitoring | Output entropy and repetition-loop monitoring halts and rolls back compromised inference | `tpt-anima-ck/src/circuit_breaker.rs` |
| CC7.3 — Change/causal integrity | Cycle and contradiction detection across the causal memory graph | `tpt-anima-sse` `validate_causal_integrity()` |
| CC8.1 — Audit logging | Append-only, immutable event ledger | `tpt-anima-zkpa/src/ledger.rs::AgencyLedger` |
| A1.2 — Capacity/availability | Thermal- and power-constrained compute allocation prevents hardware-driven outages | `tpt-anima-ck/src/budgeting.rs::BudgetManager` (per-tier throttling) |

## What already exists

- `AgencyLedger::generate_compliance_report("SOC2")` produces a machine-readable
  summary (total/session/delegation/attestation event counts) directly from
  the append-only ledger — this is the seed of the evidence package, not a
  substitute for it.
- `AgencyLedger::export_format()` supports exporting the full audit trail for
  external review.

## Remaining prep work (non-code, requires this org's action)

1. **Evidence collection window** — SOC 2 Type II evidences controls
   *operating* over time, not just existing in code. Requires running the
   system in a production-like environment for the observation period with
   ledger export snapshots taken at regular intervals.
2. **Policy documentation** — written information security policy, access
   control policy, incident response policy (see [Incident Response](../INCIDENT_RESPONSE.md)),
   vendor management policy. These are organizational documents, not code.
3. **Auditor engagement** — SOC 2 Type II requires an independent licensed
   CPA firm. This cannot be performed internally or by an AI assistant.
4. **Penetration test report** — a third-party pentest report is typically
   requested as supporting evidence (see `todo.md` §5.3).
5. **Employee/access review artifacts** — org-chart-driven access reviews are
   outside this codebase's scope.

## Gaps to close before engaging an auditor

- `generate_compliance_report` currently returns free-text strings, not a
  structured, versioned schema. If auditors require machine-parseable
  evidence, extend it to emit structured JSON (tracked as follow-up work,
  not blocking preparation).
- No automated evidence-snapshot scheduler exists yet — evidence is only
  produced when `generate_compliance_report`/`export_format` are invoked
  manually. For an observation-period audit, this should be run on a
  schedule and the output archived immutably (e.g., WORM storage).
