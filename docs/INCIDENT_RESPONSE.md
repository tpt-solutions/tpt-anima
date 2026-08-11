# Incident Response Procedures

> Copyright TPT Solutions · Apache License 2.0

This document defines how to detect, triage, contain, and recover from
security and reliability incidents in a TPT Anima deployment, and which
in-crate signals to use at each stage.

## Detection sources

| Signal | Layer | What it indicates |
|---|---|---|
| `CircuitBreaker` trip (entropy spike, repetition loop, causal violation) | CK | A specific inference has gone off-policy or is corrupted |
| `LivenessMonitor::detect_gaps()` | ZKPA | An agent's heartbeat chain has a gap — tampering or halted execution |
| `AgencyLedger` anomalous authorization/delegation events | ZKPA | Unexpected privilege grants, repeated `DelegationDenied`, session churn |
| `validate_causal_integrity()` failures | SSE | Memory graph corruption or a poisoning attempt (cycles/contradictions) |
| `BudgetManager` `ThermalCritical` / `PowerBudgetExceeded` | CK | Hardware-level runaway (thermal, power) — may indicate resource-exhaustion attack or hardware fault |
| `NegotiationSession` repeated fallback exhaustion | INP | Capability spoofing or coordinated bid manipulation |

## Severity classification

- **SEV1 — Critical**: Confirmed unauthorized privilege escalation (e.g., a
  delegation bypass), ledger tampering, or cryptographic credential
  compromise. Immediate containment required.
- **SEV2 — High**: Circuit breaker trips indicating causal-integrity
  violations at scale, sustained liveness gaps across multiple agents,
  thermal/power runaway threatening hardware.
- **SEV3 — Medium**: Isolated `ThoughtBubble` quarantine events, single-agent
  liveness gaps, negotiation fallback exhaustion for one intent.
- **SEV4 — Low**: Individual capability bid rejections, single expired-session
  errors, routine pruning/decay behavior.

## Response procedure

### 1. Detect & triage
- Pull the relevant window from `AgencyLedger::export_format()` for full
  event context.
- Cross-reference with `LivenessMonitor` heartbeat chains for the affected
  agent(s) to establish a timeline.
- Classify severity per the table above.

### 2. Contain
- **Compromised inference**: rely on `CircuitBreaker`'s automatic halt +
  rollback (`tpt-anima-ck/src/circuit_breaker.rs`); confirm the rollback
  completed and the `ThoughtBubble` was quarantined without halting the
  scheduler (`tpt-anima-ck/src/thought_bubble.rs`).
- **Compromised agent identity**: revoke the agent's `SessionCredential`
  immediately via `SessionManager::revoke_session`; if the agent has
  delegated authority downstream, revoke via
  `DelegationManager::revoke` — revocation is designed to propagate
  instantly via the cryptographic witness, not by polling.
- **Suspected delegation privilege escalation**: audit
  `DelegationManager::delegate` calls in the ledger; the permission-subset
  check (delegate cannot exceed grantor's own granted scope) should have
  rejected this — if it didn't, treat as a SEV1 code-level bug, not just an
  operational incident.
- **Thermal/power runaway**: `BudgetManager::allocate_flops_constrained`
  refuses new allocations once `ThermalCritical` or `PowerBudgetExceeded` is
  hit; confirm existing allocations are being released
  (`release_flops`/`release_power_watts`) and the affected hardware tier is
  shedding load.

### 3. Eradicate
- For causal-graph poisoning: run `prune_pass()` after quarantining bad
  nodes; re-validate with `validate_causal_integrity()`.
- For capability spoofing in INP: deregister the offending capability from
  `CapabilityRegistry` and audit its bid history for the negotiation
  sessions it participated in.

### 4. Recover
- Re-issue fresh `SessionCredential`s for affected agents (never reuse a
  revoked session id).
- Re-establish delegation chains only after confirming the parent's own
  granted-permission set is correct.
- Resume scheduling for quarantined `ThoughtBubble`s only after a clean
  circuit-breaker re-evaluation pass.

### 5. Post-incident
- Export the full incident window from `AgencyLedger` and retain per your
  compliance retention policy (see [SOC2](compliance/SOC2.md),
  [HIPAA](compliance/HIPAA.md) — HIPAA requires 6-year audit log retention if
  PHI was in scope).
- Root-cause: did a control fail to trigger (e.g., circuit breaker didn't
  trip when it should have), or did it trigger correctly and this is a
  process gap? File the former as a code defect, not just an incident
  report.

## Escalation contacts

Deployment-specific — populate with your organization's on-call rotation,
security team contact, and (if applicable) breach-notification legal
contact before going to production. This template intentionally does not
hardcode contact details.
