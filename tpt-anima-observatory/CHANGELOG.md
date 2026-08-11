# tpt-anima-observatory

All notable changes to the **tpt-anima-observatory** crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to Semantic Versioning.

## [0.1.0] - 2026-08-11

### Added
- `TelemetryServer` HTTP + WebSocket server (default `127.0.0.1:9001`).
- Unified `TelemetryEvent` plus per-layer `SseEvent` / `InpEvent` / `CkEvent` /
  `ZkpaEvent` models.
- `TelemetryCollector` with per-layer counting and recent-event queries.
- `SwarmMapData` and `MemoryTopographyData` serialization-ready visualization models.
- `handle_request` HTTP route handler.
- Bootstrap that drives one real operation through each of the four Anima layers.
- `full_stack_quickstart` example.
- Crate renamed from `anima-observatory` to `tpt-anima-observatory` (prefix + directory).
- Added `keywords` and `categories` crate metadata.

[0.1.0]: https://github.com/PhillipC05/tpt-anima/releases/tag/v0.1.0
