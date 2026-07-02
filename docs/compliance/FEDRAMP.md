# FedRAMP Authorization — Preparation Notes

> Copyright TPT Solutions · Apache License 2.0

FedRAMP authorization applies to a cloud *service offering*, not to a
library/workspace in isolation. These notes prepare TPT Anima's control
baseline story for whichever team eventually stands up a FedRAMP-authorized
deployment; they are not themselves an authorization package and cannot
substitute for 3PAO (Third-Party Assessment Organization) assessment.

## Baseline selection

FedRAMP baselines are Low, Moderate, and High, layered on NIST SP 800-53.
Given Anima's role as an AI agent runtime handling task-scoped credentials
and audit logging, a **Moderate** baseline is the most plausible starting
point for most deployments; High applies if the deployment handles
national-security-adjacent or law-enforcement data.

## Control family mapping (representative, not exhaustive)

| NIST 800-53 Family | Relevant Anima capability |
|---|---|
| AC (Access Control) | `SessionCredential` scoping + `DelegationManager` least-privilege delegation (`anima-zkpa`) |
| AU (Audit & Accountability) | `AgencyLedger` append-only event log (`anima-zkpa/src/ledger.rs`) |
| IA (Identification & Authentication) | ZK authorization/hardware-attestation proofs, continuous liveness heartbeats |
| SC (System & Communications Protection) | Edge gateway boundary translation (`anima-edge-gateway`); cryptographically isolated `ThoughtBubble` sandboxes (`anima-ck/src/thought_bubble.rs`) |
| SI (System & Information Integrity) | `CircuitBreaker` entropy/repetition/causal-integrity monitoring (`anima-ck/src/circuit_breaker.rs`) |
| CP (Contingency Planning) | See [Incident Response](../INCIDENT_RESPONSE.md) — DR/contingency procedures are organizational, not implemented in-crate |
| CM (Configuration Management) | Cargo workspace pinning + `cargo clippy -D warnings` gate provide a reproducible, linted build; formal CM plan/change-board process is organizational |

## What's out of reach for a code-level pass

FedRAMP authorization requires, at minimum:

1. A System Security Plan (SSP) describing the *deployed environment*
   (cloud provider, network diagram, boundary), which does not exist until
   a hosting decision is made.
2. An accredited 3PAO to perform the security assessment — this cannot be
   self-performed or performed by an AI assistant.
3. A sponsoring federal agency (or the FedRAMP PMO for the marketplace
   path).
4. Continuous monitoring (ConMon) infrastructure feeding monthly
   vulnerability scans and POA&M (Plan of Action & Milestones) tracking —
   this is operational tooling around a live deployment, not something
   addressable in the anima-* crates themselves.

## Recommended next step

Treat this document as the control-mapping input to an SSP once a hosting
environment (e.g., AWS GovCloud, Azure Government) is selected. Do not
represent TPT Anima as "FedRAMP compliant" or "FedRAMP ready" externally
until an actual authorization or 3PAO-reviewed readiness assessment exists.
