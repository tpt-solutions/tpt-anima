# tpt-anima-edge-gateway

**Edge Translation Gateway** — the INP ↔ MCP / A2A bridge for
[TPT Anima](https://github.com/PhillipC05/tpt-anima).

Anima's internal substrate is philosophy-free of HTTP and JSON, but the outside world
speaks MCP and A2A. The Edge Gateway is a lightweight translation layer that converts a
native **INP intent** into either an **MCP** tool call or an **A2A** message, preserving
internal purity while interoperating with the broader agent ecosystem.

## What it does

- Accepts a native INP intent as JSON on **stdin**.
- Translates it to MCP (`InpToMcpTranslator`) or A2A (`InpToA2aTranslator`).
- Prints the translated JSON on **stdout**.

## Installation

```toml
[dependencies]
tpt-anima-edge-gateway = { git = "https://github.com/PhillipC05/tpt-anima" }
```

## Usage (binary)

```bash
# INP -> MCP
echo '{"id":"1","action":"do","payload":{}}' | cargo run -p tpt-anima-edge-gateway -- --mode mcp

# INP -> A2A
echo '{"id":"1","action":"do","payload":{}}' | cargo run -p tpt-anima-edge-gateway -- --mode a2a
```

## Library API

| Item | Purpose |
|------|---------|
| `InpIntent` | native INP intent model accepted from stdin |
| `InpToMcpTranslator` | translates an `InpIntent` into an MCP tool call |
| `InpToA2aTranslator` | translates an `InpIntent` into an A2A message |

## Related crates

- `tpt-anima-inp` — the native Intent & Negotiation Protocol being translated.
- `tpt-anima-edge-runtime` — the lightweight runtime the gateway fronts.

## License

Apache License 2.0 — see [LICENSE](../../LICENSE).
