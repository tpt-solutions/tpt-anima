import type { TelemetryEvent, LayerCounts } from './types';

const DEFAULT_API_BASE = 'http://localhost:9001';
const DEFAULT_WS_URL = 'ws://localhost:9001';

export function connectWebSocket(
  url: string = DEFAULT_WS_URL,
  onEvent: (event: TelemetryEvent) => void,
): WebSocket {
  const ws = new WebSocket(`${url}/ws/telemetry`);

  ws.onmessage = (msg) => {
    try {
      const event = JSON.parse(msg.data) as TelemetryEvent;
      onEvent(event);
    } catch {
      // malformed frame — ignore
    }
  };

  return ws;
}

export async function fetchRecentEvents(
  url: string = DEFAULT_API_BASE,
): Promise<TelemetryEvent[]> {
  try {
    const res = await fetch(`${url}/api/events/recent`);
    if (!res.ok) return generateMockEvents();
    return (await res.json()) as TelemetryEvent[];
  } catch {
    return generateMockEvents();
  }
}

export async function fetchLayerCounts(
  url: string = DEFAULT_API_BASE,
): Promise<LayerCounts> {
  try {
    const res = await fetch(`${url}/api/counts`);
    if (!res.ok) return generateMockCounts();
    return (await res.json()) as LayerCounts;
  } catch {
    return generateMockCounts();
  }
}

export async function fetchHealth(
  url: string = DEFAULT_API_BASE,
): Promise<{ status: string }> {
  try {
    const res = await fetch(`${url}/health`);
    if (!res.ok) return { status: 'offline' };
    return (await res.json()) as { status: string };
  } catch {
    return { status: 'offline' };
  }
}

// --- Mock data generators for when backend is offline ---

let mockId = 0;
function nextId(): string {
  mockId += 1;
  return `mock-${mockId}`;
}

function generateMockEvents(): TelemetryEvent[] {
  const layers: Array<TelemetryEvent['layer']> = ['sse', 'inp', 'ck', 'zkpa'];
  const events: TelemetryEvent[] = [];
  for (let i = 0; i < 30; i++) {
    const layer = layers[i % layers.length];
    events.push({
      type: `${layer}_telemetry`,
      layer,
      data: { msg: `sample event ${i}`, value: Math.random() * 100 },
      timestamp: Date.now() - i * 1000,
    });
  }
  return events;
}

function generateMockCounts(): LayerCounts {
  return {
    sse: 1247,
    inp: 893,
    ck: 612,
    zkpa: 2084,
    total: 4836,
  };
}

export function generateMockSwarmData(): import('./types').SwarmMapData {
  const nodes: import('./types').SwarmNode[] = [
    { id: 'n1', label: 'Planner-α', status: 'active', agentType: 'planner', x: 200, y: 120 },
    { id: 'n2', label: 'Coder-β', status: 'busy', agentType: 'coder', x: 380, y: 80 },
    { id: 'n3', label: 'Reviewer-γ', status: 'idle', agentType: 'reviewer', x: 500, y: 200 },
    { id: 'n4', label: 'Researcher-δ', status: 'active', agentType: 'researcher', x: 320, y: 260 },
    { id: 'n5', label: 'Executor-ε', status: 'busy', agentType: 'executor', x: 140, y: 240 },
    { id: 'n6', label: 'Monitor-ζ', status: 'active', agentType: 'monitor', x: 460, y: 320 },
  ];
  const edges: import('./types').SwarmEdge[] = [
    { source: 'n1', target: 'n2', weight: 0.9 },
    { source: 'n1', target: 'n4', weight: 0.7 },
    { source: 'n2', target: 'n3', weight: 0.5 },
    { source: 'n3', target: 'n6', weight: 0.6 },
    { source: 'n4', target: 'n5', weight: 0.8 },
    { source: 'n5', target: 'n1', weight: 0.4 },
    { source: 'n6', target: 'n2', weight: 0.3 },
  ];
  const pulseEvents: import('./types').PulseEvent[] = [
    { source: 'n1', target: 'n2', intentId: 'INT-001', timestamp: Date.now() - 2000, won: true },
    { source: 'n4', target: 'n5', intentId: 'INT-002', timestamp: Date.now() - 5000, won: true },
    { source: 'n3', target: 'n6', intentId: 'INT-003', timestamp: Date.now() - 8000, won: false },
  ];
  return { nodes, edges, pulseEvents };
}

export function generateMockMemoryTopo(): import('./types').MemoryTopoData {
  const modalities = ['text', 'visual', 'audio', 'text', 'visual', 'text'];
  const labels = ['Core Memory', 'Skill Graph', 'Session Log', 'Embedding Index', 'Causal Chain', 'Decay Pool'];
  const nodes: import('./types').TopoNode[] = modalities.map((m, i) => ({
    id: `mem-${i}`,
    strength: 0.2 + Math.random() * 0.8,
    modality: m,
    label: labels[i],
    x: 60 + (i % 3) * 180 + Math.random() * 40,
    y: 60 + Math.floor(i / 3) * 140 + Math.random() * 40,
  }));
  const chains: import('./types').TopoChain[] = [
    { source: 'mem-0', target: 'mem-2', strength: 0.9 },
    { source: 'mem-1', target: 'mem-3', strength: 0.6 },
    { source: 'mem-2', target: 'mem-4', strength: 0.75 },
    { source: 'mem-3', target: 'mem-5', strength: 0.5 },
    { source: 'mem-0', target: 'mem-5', strength: 0.3 },
  ];
  return { nodes, chains };
}

let _nextLedgerId = 0;
export function generateMockLedger(): import('./types').LedgerEntry[] {
  const eventTypes = ['session_created', 'delegation', 'revocation', 'attestation', 'proof_generated', 'heartbeat'];
  const agentIds = ['agent-alpha', 'agent-beta', 'agent-gamma', 'agent-delta'];
  const details = [
    'Session credential issued for task T-42',
    'Delegated authority to child agent',
    'Revoked credential exp-2026-07',
    'Hardware attestation passed (TPM 2.0)',
    'ZK proof generated for integrity check',
    'Liveness heartbeat recorded',
  ];
  const entries: import('./types').LedgerEntry[] = [];
  for (let i = 0; i < 20; i++) {
    _nextLedgerId += 1;
    entries.push({
      id: `ledger-${_nextLedgerId}`,
      eventType: eventTypes[i % eventTypes.length],
      agentId: agentIds[i % agentIds.length],
      timestamp: Date.now() - i * 15000,
      details: details[i % details.length],
    });
  }
  return entries;
}

export function generateMockEconomicFlow(): { agents: { id: string; balance: number }[]; totalCirculation: number } {
  const agents = [
    { id: 'Planner-α', balance: 1240 },
    { id: 'Coder-β', balance: 890 },
    { id: 'Reviewer-γ', balance: 430 },
    { id: 'Researcher-δ', balance: 1100 },
    { id: 'Executor-ε', balance: 670 },
    { id: 'Monitor-ζ', balance: 320 },
  ];
  const totalCirculation = agents.reduce((s, a) => s + a.balance, 0);
  return { agents, totalCirculation };
}

export function generateMockInferencePulse(): { gpu: number; cpu: number; npu: number; ram: number; bubbles: number; queue: number } {
  return {
    gpu: 67 + Math.random() * 25,
    cpu: 42 + Math.random() * 30,
    npu: 20 + Math.random() * 40,
    ram: 55 + Math.random() * 20,
    bubbles: Math.floor(3 + Math.random() * 5),
    queue: Math.floor(2 + Math.random() * 8),
  };
}
