// Layer event types matching the Rust backend
export type Layer = 'sse' | 'inp' | 'ck' | 'zkpa';

export interface TelemetryEvent {
  type: string;
  layer: Layer;
  data: Record<string, unknown>;
  timestamp: number;
}

export interface SwarmNode {
  id: string;
  label: string;
  status: 'active' | 'idle' | 'busy';
  agentType: string;
  x: number;
  y: number;
}

export interface SwarmEdge {
  source: string;
  target: string;
  weight: number;
}

export interface PulseEvent {
  source: string;
  target: string;
  intentId: string;
  timestamp: number;
  won: boolean;
}

export interface SwarmMapData {
  nodes: SwarmNode[];
  edges: SwarmEdge[];
  pulseEvents: PulseEvent[];
}

export interface TopoNode {
  id: string;
  strength: number;
  modality: string;
  label: string;
  x: number;
  y: number;
}

export interface TopoChain {
  source: string;
  target: string;
  strength: number;
}

export interface MemoryTopoData {
  nodes: TopoNode[];
  chains: TopoChain[];
}

export interface LayerCounts {
  sse: number;
  inp: number;
  ck: number;
  zkpa: number;
  total: number;
}

export interface LedgerEntry {
  id: string;
  eventType: string;
  agentId: string;
  timestamp: number;
  details: string;
}

export interface CreditTransfer {
  id: string;
  fromAgent: string;
  toAgent: string;
  amount: number;
  intentId: string;
  timestamp: number;
}
