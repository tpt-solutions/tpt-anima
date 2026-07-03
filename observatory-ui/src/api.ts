import type {
  BroadcastMessage,
  LayerCounts,
  MemoryTopoViewData,
  PulseViewEvent,
  SwarmMapViewData,
  SwarmViewEdge,
  SwarmViewNode,
  TopoViewChain,
  TopoViewNode,
} from './types';

const DEFAULT_API_BASE = 'http://localhost:9001';
const DEFAULT_WS_URL = 'ws://localhost:9001';

export interface WebSocketHandlers {
  onMessage: (msg: BroadcastMessage) => void;
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (ev: Event) => void;
}

/** Open a WebSocket connection to the observatory's real `/ws/telemetry` endpoint. */
export function connectWebSocket(url: string = DEFAULT_WS_URL, handlers: WebSocketHandlers): WebSocket {
  const ws = new WebSocket(`${url}/ws/telemetry`);

  ws.onopen = () => handlers.onOpen?.();
  ws.onclose = () => handlers.onClose?.();
  ws.onerror = (ev) => handlers.onError?.(ev);
  ws.onmessage = (msg) => {
    try {
      const parsed = JSON.parse(msg.data) as BroadcastMessage;
      handlers.onMessage(parsed);
    } catch {
      // malformed frame — ignore
    }
  };

  return ws;
}

/**
 * Reconnect to the observatory WebSocket with capped exponential backoff.
 * Returns a `close()` function that stops any pending reconnect attempts
 * and closes the current socket.
 */
export function connectWithReconnect(
  url: string = DEFAULT_WS_URL,
  handlers: WebSocketHandlers,
): { close: () => void } {
  let stopped = false;
  let socket: WebSocket | null = null;
  let attempt = 0;
  let retryTimer: ReturnType<typeof setTimeout> | null = null;

  const MAX_DELAY_MS = 15000;
  const BASE_DELAY_MS = 500;

  function scheduleReconnect() {
    if (stopped) return;
    const delay = Math.min(BASE_DELAY_MS * 2 ** attempt, MAX_DELAY_MS);
    attempt += 1;
    retryTimer = setTimeout(connect, delay);
  }

  function connect() {
    if (stopped) return;
    socket = connectWebSocket(url, {
      onMessage: handlers.onMessage,
      onOpen: () => {
        attempt = 0;
        handlers.onOpen?.();
      },
      onClose: () => {
        handlers.onClose?.();
        scheduleReconnect();
      },
      onError: handlers.onError,
    });
  }

  connect();

  return {
    close: () => {
      stopped = true;
      if (retryTimer) clearTimeout(retryTimer);
      socket?.close();
    },
  };
}

export async function fetchLayerCounts(url: string = DEFAULT_API_BASE): Promise<LayerCounts | null> {
  try {
    const res = await fetch(`${url}/api/telemetry/count`);
    if (!res.ok) return null;
    return (await res.json()) as LayerCounts;
  } catch {
    return null;
  }
}

export async function fetchHealth(url: string = DEFAULT_API_BASE): Promise<{ status: string } | null> {
  try {
    const res = await fetch(`${url}/api/health`);
    if (!res.ok) return null;
    return (await res.json()) as { status: string };
  } catch {
    return null;
  }
}

// ── View-model helpers ──────────────────────────────────────────────────
// The backend has no notion of pixel layout, so positions are derived
// deterministically from real node ids (a stable hash-based ring layout),
// not randomly generated on every render.

const LAYOUT_WIDTH = 600;
const LAYOUT_HEIGHT = 400;

function hashToUnit(id: string): number {
  let h = 2166136261;
  for (let i = 0; i < id.length; i++) {
    h ^= id.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return (h >>> 0) / 0xffffffff;
}

function layoutPosition(id: string, index: number, total: number): { x: number; y: number } {
  const angle = ((index / Math.max(total, 1)) * 2 + hashToUnit(id)) * Math.PI;
  const radiusX = LAYOUT_WIDTH * 0.35;
  const radiusY = LAYOUT_HEIGHT * 0.35;
  return {
    x: LAYOUT_WIDTH / 2 + Math.cos(angle) * radiusX,
    y: LAYOUT_HEIGHT / 2 + Math.sin(angle) * radiusY,
  };
}

export function buildSwarmView(
  nodes: { id: string; label: string; status: string; agent_type: string }[],
  edges: SwarmViewEdge[],
  pulseEvents: PulseViewEvent[],
): SwarmMapViewData {
  const positioned: SwarmViewNode[] = nodes.map((n, i) => ({
    id: n.id,
    label: n.label,
    status: (n.status as SwarmViewNode['status']) ?? 'idle',
    agentType: n.agent_type,
    ...layoutPosition(n.id, i, nodes.length),
  }));
  return { nodes: positioned, edges, pulseEvents };
}

export function buildMemoryTopoView(
  nodes: { id: string; strength: number; modality: string; label: string }[],
  chains: TopoViewChain[],
): MemoryTopoViewData {
  const positioned: TopoViewNode[] = nodes.map((n, i) => ({
    ...n,
    ...layoutPosition(n.id, i, nodes.length),
  }));
  return { nodes: positioned, chains };
}
