# tpt-anima-observatory

**Anima Observatory** — the real-time human telemetry window for
[TPT Anima](https://github.com/PhillipC05/tpt-anima).

This crate turns machine-speed AI cognition into observable, auditable
visualizations. It ships a **telemetry aggregation library** (`lib`) and a
**WebSocket + HTTP server binary** (`tpt-anima-observatory`) that bootstraps one real
operation through each of the four Anima layers (SSE, INP, CK, ZKPA) and broadcasts the
results.

## Why it exists

Autonomous agents operate faster than humans can read logs. The Observatory translates
that speed into five human-facing panels: Swarm Map, Memory Topography, Inference
Pulse, Agency Ledger, and Economic Flow.

## Key concepts

- **Layer events** — `SseEvent`, `InpEvent`, `CkEvent`, `ZkpaEvent` wrap real outcomes
  from each layer into a unified `TelemetryEvent`.
- **TelemetryServer** — HTTP + WebSocket server (`/ws/telemetry`, `/api/health`,
  `/api/telemetry/recent`, `/api/telemetry/count`) running by default on
  `127.0.0.1:9001`.
- **Aggregation** — `TelemetryCollector` records events and counts them per layer.
- **Visualization models** — `SwarmMapData` (nodes/edges/pulses) and
  `MemoryTopographyData` (nodes/chains) are serialization-ready shapes for the UI.

## Installation

```toml
[dependencies]
tpt-anima-observatory = { git = "https://github.com/PhillipC05/tpt-anima" }
```

## Quick start (run the server)

```bash
# Terminal 1 — telemetry backend
cargo run -p tpt-anima-observatory

# Terminal 2 — UI (separate workspace member: observatory-ui)
cd observatory-ui && npm install && npm run dev
```

Then open `http://localhost:5173`.

## Runnable example

```bash
cargo run -p tpt-anima-observatory --example full_stack_quickstart
```

Drives a golden path across all four layers in a single request: memory → negotiation
→ session → thermal budget check, emitting real telemetry.

## Modules

| Module | Purpose |
|--------|---------|
| `server` | `TelemetryServer`, `BroadcastMessage` |
| `routes` | `handle_request` HTTP route handler |
| (root) | `TelemetryEvent` + per-layer event enums |
| (root) | `TelemetryCollector`, `LayerCounts` |
| (root) | `SwarmMapData` / `MemoryTopographyData` visualization models |

## Related crates

- `tpt-anima-sse`, `tpt-anima-inp`, `tpt-anima-ck`, `tpt-anima-zkpa` — the four layers
  the Observatory observes.

## License

Apache License 2.0 — see [LICENSE](../../LICENSE).
