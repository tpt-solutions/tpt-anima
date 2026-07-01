import { useCallback, useEffect, useRef, useState } from 'react';
import type { LayerCounts, LedgerEntry, MemoryTopoData, SwarmMapData } from './types';
import {
  connectWebSocket,
  generateMockEconomicFlow,
  generateMockInferencePulse,
  generateMockLedger,
  generateMockMemoryTopo,
  generateMockSwarmData,
} from './api';
import { StatusBar } from './components/StatusBar';
import { SwarmMap } from './components/SwarmMap';
import { MemoryTopography } from './components/MemoryTopography';
import { InferencePulse } from './components/InferencePulse';
import { AgencyLedger } from './components/AgencyLedger';
import { EconomicFlow } from './components/EconomicFlow';
import './App.css';

function App() {
  const [connected, setConnected] = useState(false);
  const [eventCount, setEventCount] = useState(0);
  const [counts, setCounts] = useState<LayerCounts>({ sse: 0, inp: 0, ck: 0, zkpa: 0, total: 0 });
  const [swarmData, setSwarmData] = useState<SwarmMapData>(generateMockSwarmData);
  const [memTopo, setMemTopo] = useState<MemoryTopoData>(generateMockMemoryTopo);
  const [inference, setInference] = useState(generateMockInferencePulse);
  const [ledger, setLedger] = useState<LedgerEntry[]>(generateMockLedger);
  const [econ, setEcon] = useState(generateMockEconomicFlow);
  const wsRef = useRef<WebSocket | null>(null);

  const handleEvent = useCallback(() => {
    setEventCount((c) => c + 1);
  }, []);

  useEffect(() => {
    const ws = connectWebSocket('ws://localhost:9001', handleEvent);
    wsRef.current = ws;

    ws.onopen = () => setConnected(true);
    ws.onclose = () => {
      setConnected(false);
      // Reconnect after 3s
      setTimeout(() => {
        // Trigger re-render by toggling — in production this would re-init WS
      }, 3000);
    };

    // Periodic mock data refresh
    const interval = setInterval(() => {
      setCounts((prev) => ({
        sse: prev.sse + Math.floor(Math.random() * 5),
        inp: prev.inp + Math.floor(Math.random() * 3),
        ck: prev.ck + Math.floor(Math.random() * 2),
        zkpa: prev.zkpa + Math.floor(Math.random() * 4),
        total: prev.total + Math.floor(Math.random() * 14),
      }));
      setInference(generateMockInferencePulse());
      setSwarmData(generateMockSwarmData());
      setMemTopo(generateMockMemoryTopo());
    }, 5000);

    return () => {
      clearInterval(interval);
      ws.close();
    };
  }, [handleEvent]);

  return (
    <div className="observatory">
      <header className="observatory-header">
        <h1 className="observatory-title">TPT Anima Observatory</h1>
        <StatusBar connected={connected} eventCount={eventCount || counts.total} />
      </header>

      <main className="observatory-grid">
        <div className="grid-span-2">
          <SwarmMap data={swarmData} />
        </div>
        <InferencePulse {...inference} />

        <MemoryTopography data={memTopo} />
        <AgencyLedger entries={ledger} />
        <EconomicFlow agents={econ.agents} totalCirculation={econ.totalCirculation} />
      </main>
    </div>
  );
}

export default App;
