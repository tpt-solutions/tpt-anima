import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  BroadcastMessage,
  InferencePulseData,
  LayerCounts,
  LedgerEntry,
  AgentBalance,
  PulseViewEvent,
  SwarmMapViewData,
  SwarmViewEdge,
  MemoryTopoViewData,
} from './types';
import { buildMemoryTopoView, buildSwarmView, connectWithReconnect } from './api';
import { StatusBar } from './components/StatusBar';
import { SwarmMap } from './components/SwarmMap';
import { MemoryTopography } from './components/MemoryTopography';
import { InferencePulse } from './components/InferencePulse';
import { AgencyLedger } from './components/AgencyLedger';
import { EconomicFlow } from './components/EconomicFlow';
import './App.css';

const MAX_LEDGER_ENTRIES = 50;
const MAX_PULSE_EVENTS = 20;

const EMPTY_SWARM: SwarmMapViewData = { nodes: [], edges: [], pulseEvents: [] };
const EMPTY_TOPO: MemoryTopoViewData = { nodes: [], chains: [] };
const EMPTY_INFERENCE: InferencePulseData = { gpu: 0, cpu: 0, npu: 0, ram: 0, bubbles: 0, queue: 0 };

function App() {
  const [connected, setConnected] = useState(false);
  const [eventCount, setEventCount] = useState(0);
  const [counts, setCounts] = useState<LayerCounts>({ sse: 0, inp: 0, ck: 0, zkpa: 0 });
  const [swarmData, setSwarmData] = useState<SwarmMapViewData>(EMPTY_SWARM);
  const [memTopo, setMemTopo] = useState<MemoryTopoViewData>(EMPTY_TOPO);
  const [inference, setInference] = useState<InferencePulseData>(EMPTY_INFERENCE);
  const [ledger, setLedger] = useState<LedgerEntry[]>([]);
  const [balances, setBalances] = useState<Record<string, number>>({});

  // Raw accumulators the view models are derived from.
  const swarmNodesRef = useRef<Map<string, { id: string; label: string; status: string; agent_type: string }>>(new Map());
  const swarmEdgesRef = useRef<SwarmViewEdge[]>([]);
  const pulseEventsRef = useRef<PulseViewEvent[]>([]);
  const topoNodesRef = useRef<Map<string, { id: string; strength: number; modality: string; label: string }>>(new Map());
  const topoChainsRef = useRef<{ source: string; target: string; strength: number }[]>([]);

  const handleMessage = useCallback((msg: BroadcastMessage) => {
    setEventCount((c) => c + 1);

    if (msg.type === 'swarm_map') {
      for (const node of msg.nodes) {
        swarmNodesRef.current.set(node.id, node);
      }
      swarmEdgesRef.current = msg.edges;
      setSwarmData(buildSwarmView([...swarmNodesRef.current.values()], swarmEdgesRef.current, pulseEventsRef.current));
      return;
    }

    if (msg.type === 'memory_topography') {
      for (const node of msg.nodes) {
        topoNodesRef.current.set(node.id, node);
      }
      topoChainsRef.current = msg.chains;
      setMemTopo(buildMemoryTopoView([...topoNodesRef.current.values()], topoChainsRef.current));
      return;
    }

    if (msg.type === 'telemetry_event') {
      if (msg.SSE) {
        setCounts((prev) => ({ ...prev, sse: prev.sse + 1 }));
        const ev = msg.SSE;
        if (ev.MemoryAccessed || ev.DecayApplied) {
          const detail = ev.MemoryAccessed ?? ev.DecayApplied!;
          const strength = 'strength' in detail ? detail.strength : detail.new_strength;
          topoNodesRef.current.set(detail.node_id, {
            id: detail.node_id,
            strength,
            modality: 'text',
            label: detail.node_id.slice(0, 8),
          });
          setMemTopo(buildMemoryTopoView([...topoNodesRef.current.values()], topoChainsRef.current));
        }
      } else if (msg.INP) {
        setCounts((prev) => ({ ...prev, inp: prev.inp + 1 }));
        const ev = msg.INP;
        if (ev.IntentBroadcast || ev.NegotiationCompleted) {
          const intentId = ev.IntentBroadcast?.intent_id ?? ev.NegotiationCompleted?.intent_id ?? 'unknown';
          const winner = ev.NegotiationCompleted?.winner;
          if (winner) {
            pulseEventsRef.current = [
              ...pulseEventsRef.current,
              { source: 'requester', target: winner, intentId, timestamp: Date.now(), won: true },
            ].slice(-MAX_PULSE_EVENTS);
            setSwarmData(buildSwarmView([...swarmNodesRef.current.values()], swarmEdgesRef.current, pulseEventsRef.current));
          }
        }
        if (ev.CreditTransferred) {
          const { from, to, amount } = ev.CreditTransferred;
          setBalances((prev) => ({
            ...prev,
            [from]: (prev[from] ?? 0) - amount,
            [to]: (prev[to] ?? 0) + amount,
          }));
        }
      } else if (msg.CK) {
        setCounts((prev) => ({ ...prev, ck: prev.ck + 1 }));
        const ev = msg.CK;
        if (ev.InferenceScheduled) {
          setInference((prev) => ({ ...prev, queue: prev.queue + 1 }));
        }
        if (ev.ThoughtBubbleCreated) {
          setInference((prev) => ({ ...prev, bubbles: prev.bubbles + 1 }));
        }
        if (ev.HardwareUtilization) {
          const { gpu, cpu, npu, ram } = ev.HardwareUtilization;
          setInference((prev) => ({ ...prev, gpu, cpu, npu, ram }));
        }
      } else if (msg.ZKPA) {
        setCounts((prev) => ({ ...prev, zkpa: prev.zkpa + 1 }));
        const ev = msg.ZKPA;
        const entry = ev.SessionCreated
          ? { eventType: 'session_created', agentId: ev.SessionCreated.agent_id, details: `Session ${ev.SessionCreated.session_id}` }
          : ev.AttestationVerified
            ? { eventType: 'attestation', agentId: '—', details: `Attested on ${ev.AttestationVerified.platform}` }
            : ev.LivenessHeartbeat
              ? { eventType: 'heartbeat', agentId: ev.LivenessHeartbeat.agent_id, details: `Sequence ${ev.LivenessHeartbeat.sequence}` }
              : null;
        if (entry) {
          setLedger((prev) =>
            [
              { id: `${Date.now()}-${prev.length}`, timestamp: Date.now(), ...entry },
              ...prev,
            ].slice(0, MAX_LEDGER_ENTRIES),
          );
        }
      }
    }
  }, []);

  useEffect(() => {
    const conn = connectWithReconnect(undefined, {
      onMessage: handleMessage,
      onOpen: () => setConnected(true),
      onClose: () => setConnected(false),
    });
    return () => conn.close();
  }, [handleMessage]);

  const agentBalances: AgentBalance[] = Object.entries(balances).map(([id, balance]) => ({ id, balance }));
  const totalCirculation = agentBalances.reduce((sum, a) => sum + Math.max(a.balance, 0), 0);
  const totalEvents = counts.sse + counts.inp + counts.ck + counts.zkpa;

  return (
    <div className="observatory">
      <header className="observatory-header">
        <h1 className="observatory-title">TPT Anima Observatory</h1>
        <StatusBar connected={connected} eventCount={eventCount || totalEvents} />
      </header>

      {!connected && (
        <div className="observatory-banner" role="status">
          Disconnected from observatory backend — reconnecting automatically…
        </div>
      )}

      <main className="observatory-grid">
        <div className="grid-span-2">
          <SwarmMap data={swarmData} />
        </div>
        <InferencePulse {...inference} />

        <MemoryTopography data={memTopo} />
        <AgencyLedger entries={ledger} />
        <EconomicFlow agents={agentBalances} totalCirculation={totalCirculation} />
      </main>
    </div>
  );
}

export default App;
