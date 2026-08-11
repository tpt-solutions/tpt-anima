# tpt-anima-tests

**Integration test suite** for the [TPT Anima](https://github.com/PhillipC05/tpt-anima)
workspace.

This crate is **not published** (`publish = false`). It exercises the real public APIs
of the core Anima layers end-to-end: it writes memory through `tpt-anima-sse`, runs a
negotiation + credit transfer through `tpt-anima-inp`, schedules an inference through
`tpt-anima-ck`, and creates a session + records an agency event through
`tpt-anima-zkpa`, then asserts the cross-layer behavior holds together.

## What it covers

- Semantic State Engine: node lifecycle, decay, causal links.
- Intent & Negotiation Protocol: registry, negotiation, compute-ledger transfers.
- Cognitive Kernel: inference scheduling and circuit-breaker behavior.
- Zero-Knowledge Proof of Agency: session lifecycle and agency ledger events.
- Cross-layer integration through `tpt-anima-observatory` telemetry types.

## Running the tests

```bash
cargo test -p tpt-anima-tests
```

Or run the whole workspace suite:

```bash
cargo test --workspace
```

## Dependencies

Pulls in `tpt-anima-sse`, `tpt-anima-inp`, `tpt-anima-ck`, `tpt-anima-zkpa`, and
`tpt-anima-observatory` to drive real cross-layer integration tests.

## License

Apache License 2.0 — see [LICENSE](../../LICENSE).
