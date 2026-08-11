# tpt-anima-edge-gateway

All notable changes to the **tpt-anima-edge-gateway** crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to Semantic Versioning.

## [0.1.0] - 2026-08-11

### Added
- `InpIntent` native INP intent model accepted from stdin.
- `InpToMcpTranslator` translating INP intents into MCP tool calls.
- `InpToA2aTranslator` translating INP intents into A2A messages.
- `tpt-anima-edge-gateway` binary reading INP JSON from stdin and printing the
  translated MCP/A2A JSON to stdout.
- Crate renamed from `anima-edge-gateway` to `tpt-anima-edge-gateway` (prefix + directory).
- Added `keywords` and `categories` crate metadata.

[0.1.0]: https://github.com/tpt-solutions/tpt-anima/releases/tag/v0.1.0
