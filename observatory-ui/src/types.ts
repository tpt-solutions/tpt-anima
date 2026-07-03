// Wire types matching the real Rust backend (anima-observatory) serde output.
// See anima-observatory/src/lib.rs (TelemetryEvent, SseEvent, InpEvent,
// CkEvent, ZkpaEvent) and anima-observatory/src/server.rs (BroadcastMessage).

export type Layer = 'SSE' | 'INP' | 'CK' | 'ZKPA';

export interface SseEvent {
  MemoryAccessed?: { node_id: string; strength: number };
  MemoryPruned?: { count: number };
  DecayApplied?: { node_id: string; new_strength: number };
}

export interface InpEvent {
  IntentBroadcast?: { intent_id: string; action: string };
  BidReceived?: { intent_id: string; provider_id: string };
  NegotiationCompleted?: { intent_id: string; winner: string };
  CreditTransferred?: { from: string; to: string; amount: number };
}

export interface CkEvent {
  InferenceScheduled?: { inference_id: string; priority: string };
  CircuitBreakerTripped?: { reason: string };
  ThoughtBubbleCreated?: { bubble_id: string };
  HardwareUtilization?: { gpu: number; cpu: number; npu: number; ram: number };
}

export interface ZkpaEvent {
  SessionCreated?: { session_id: string; agent_id: string };
  AttestationVerified?: { platform: string };
  LivenessHeartbeat?: { agent_id: string; sequence: number };
}

/** A raw telemetry event as emitted by `TelemetryEvent` (externally tagged by layer). */
export interface TelemetryEvent {
  SSE?: SseEvent;
  INP?: InpEvent;
  CK?: CkEvent;
  ZKPA?: ZkpaEvent;
}

export interface SwarmNode {
  id: string;
  label: string;
  status: 'active' | 'idle' | 'busy';
  agent_type: string;
}

export interface SwarmEdge {
  source: string;
  target: string;
  weight: number;
}

export interface PulseEvent {
  source: string;
  target: string;
  intent_id: string;
  timestamp: number;
  won: boolean;
}

export interface SwarmMapData {
  nodes: SwarmNode[];
  edges: SwarmEdge[];
  pulse_events: PulseEvent[];
}

export interface TopoNode {
  id: string;
  strength: number;
  modality: string;
  label: string;
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

/** Messages broadcast over `/ws/telemetry`, tagged by `type` (see `BroadcastMessage`). */
export type BroadcastMessage =
  | ({ type: 'telemetry_event' } & TelemetryEvent)
  | ({ type: 'swarm_map' } & SwarmMapData)
  | ({ type: 'memory_topography' } & MemoryTopoData);

// ── View models ──────────────────────────────────────────────────────────
// The wire types above carry no layout information (the backend has no
// concept of pixel position). These view models add a deterministic,
// client-computed (x, y) so the canvas renderers have something stable to
// draw — computed from the real node/edge data, not randomly generated.

export interface SwarmViewNode {
  id: string;
  label: string;
  status: 'active' | 'idle' | 'busy';
  agentType: string;
  x: number;
  y: number;
}

export interface SwarmViewEdge {
  source: string;
  target: string;
  weight: number;
}

export interface PulseViewEvent {
  source: string;
  target: string;
  intentId: string;
  timestamp: number;
  won: boolean;
}

export interface SwarmMapViewData {
  nodes: SwarmViewNode[];
  edges: SwarmViewEdge[];
  pulseEvents: PulseViewEvent[];
}

export interface TopoViewNode {
  id: string;
  strength: number;
  modality: string;
  label: string;
  x: number;
  y: number;
}

export interface TopoViewChain {
  source: string;
  target: string;
  strength: number;
}

export interface MemoryTopoViewData {
  nodes: TopoViewNode[];
  chains: TopoViewChain[];
}

export interface LayerCounts {
  sse: number;
  inp: number;
  ck: number;
  zkpa: number;
}

export interface LedgerEntry {
  id: string;
  eventType: string;
  agentId: string;
  timestamp: number;
  details: string;
}

export interface AgentBalance {
  id: string;
  balance: number;
}

export interface EconomicFlowData {
  agents: AgentBalance[];
  totalCirculation: number;
}

export interface InferencePulseData {
  gpu: number;
  cpu: number;
  npu: number;
  ram: number;
  bubbles: number;
  queue: number;
}
