# tpt-anima-sse

All notable changes to the **tpt-anima-sse** crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to Semantic Versioning.

## [0.1.0] - 2026-08-11

### Added
- `MemoryNode` semantic tensor with vector embeddings, payload, and sensory modality.
- `InMemoryStorage` in-process memory store with decay, access reinforcement, and pruning.
- Temporal decay engine (`decay`) with `CollectionStats` and `MemoryType` classification.
- `CausalGraph` with typed `CausalLink` edges for causal topology.
- Multi-modal `SensoryManager` ingestion interface (`sensory`).
- Compute `Ledger` tracking storage/access cost (`ledger`).
- `mcp://` resource and tool adapters exposing memory for agent attendance.
- Crate renamed from `anima-sse` to `tpt-anima-sse` (prefix + directory).
- Added `keywords` and `categories` crate metadata.

[0.1.0]: https://github.com/tpt-solutions/tpt-anima/releases/tag/v0.1.0
