# tpt-anima-inp

All notable changes to the **tpt-anima-inp** crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to Semantic Versioning.

## [0.1.0] - 2026-08-11

### Added
- `Intent` / `Capability` / `CapabilityBid` declarative goal + bid model.
- `CapabilityRegistry` and `NegotiationSession` / `NegotiationManager` state machine
  with fallback strategies.
- `IntentBroadcaster` intent fan-out.
- `ComputeLedger` with `ComputeCredits` and `Transaction` (plus `ExternalRails`).
- Consensus guards: `ReputationManager`, `RateLimiter`, `ProofOfWork`, `EconomicStake`.
- `SemanticContract` validation, adaptive `qos` negotiation, and `manifest` parser.
- `mcp://` broadcast, bid-query, negotiation, and result-retrieval tools.
- Crate renamed from `anima-inp` to `tpt-anima-inp` (prefix + directory).
- Added `keywords` and `categories` crate metadata.

[0.1.0]: https://github.com/tpt-solutions/tpt-anima/releases/tag/v0.1.0
