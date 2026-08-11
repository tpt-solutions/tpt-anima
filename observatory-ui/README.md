# Anima Observatory UI

This is the GUI for [TPT Anima](../README.md) — a React + TypeScript + Vite dashboard that visualizes what the four layers (SSE, INP, CK, ZKPA) are doing in real time. It connects over WebSocket to the `tpt-anima-observatory` Rust crate, which aggregates telemetry from the rest of the workspace.

Panels: **Swarm Map** (INP intent/negotiation flow), **Memory Topography** (SSE nodes and decay), **Inference Pulse** (CK hardware scheduling), **Agency Ledger** (ZKPA audit log), **Economic Flow** (compute-credit transfers). See the root [README's Observatory section](../README.md#the-tpt-anima-observatory) for what each panel means.

## Running locally

```bash
# Terminal 1: start the telemetry backend
cd ../tpt-anima-observatory
cargo run

# Terminal 2: start this UI
npm install
npm run dev
```

Then open `http://localhost:5173`.

## Other commands

```bash
npm test          # run the test suite once (Vitest)
npm run test:watch  # run tests in watch mode
npm run build      # type-check (tsc -b) and produce a production build
npm run lint        # oxlint
```
