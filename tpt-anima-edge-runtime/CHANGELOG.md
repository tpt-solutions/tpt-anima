# tpt-anima-edge-runtime

All notable changes to the **tpt-anima-edge-runtime** crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to Semantic Versioning.

## [0.1.0] - 2026-08-11

### Added
- `EdgeMemory` minimal embedded-friendly memory store.
- `EdgeIntentHandler` + `EdgeIntent` on-device intent handling.
- `EdgeRuntime` coordinator with `RuntimeStats`.
- `cross_tier` protocol: envelopes, handshakes, heartbeats, and `TierRoute` routing
  between Anima tiers.
- Crate renamed from `anima-edge-runtime` to `tpt-anima-edge-runtime` (prefix + directory).
- Added `keywords` and `categories` crate metadata.

[0.1.0]: https://github.com/tpt-solutions/tpt-anima/releases/tag/v0.1.0
