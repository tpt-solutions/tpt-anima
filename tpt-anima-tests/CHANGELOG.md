# tpt-anima-tests

All notable changes to the **tpt-anima-tests** crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to Semantic Versioning.

> This crate is not published (`publish = false`); its version tracks the workspace.

## [0.1.0] - 2026-08-11

### Added
- Cross-layer integration tests covering `tpt-anima-sse`, `tpt-anima-inp`,
  `tpt-anima-ck`, `tpt-anima-zkpa`, and `tpt-anima-observatory`.
- Memory lifecycle, negotiation + compute-ledger transfer, inference scheduling, and
  ZKPA session/agency-ledger assertions.
- Crate renamed from `anima-tests` to `tpt-anima-tests` (prefix + directory).
- Added `keywords` and `categories` crate metadata.

[0.1.0]: https://github.com/tpt-solutions/tpt-anima/releases/tag/v0.1.0
