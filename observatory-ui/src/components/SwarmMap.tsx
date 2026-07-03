import { useEffect, useRef } from 'react';
import type { SwarmMapViewData } from '../types';

interface SwarmMapProps {
  data: SwarmMapViewData;
}

const STATUS_COLORS: Record<string, string> = {
  active: '#22c55e',
  idle: '#eab308',
  busy: '#ef4444',
};

const CANVAS_WIDTH = 600;
const CANVAS_HEIGHT = 400;
const PULSE_DURATION_MS = 2000;
const NODE_GLOW_RADIUS = 28;
const NODE_CORE_RADIUS = 12;

interface PulseAnim {
  sourceId: string;
  targetId: string;
  color: string;
  startTime: number;
  intentId: string;
}

export function SwarmMap({ data }: SwarmMapProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animFrameRef = useRef<number>(0);
  const pulsesRef = useRef<PulseAnim[]>([]);
  const lastPulseCountRef = useRef(0);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const nodeMap = new Map(data.nodes.map((n) => [n.id, n]));

    // Register new pulse events
    if (data.pulseEvents.length !== lastPulseCountRef.current) {
      const newPulses = data.pulseEvents.slice(lastPulseCountRef.current);
      for (const pe of newPulses) {
        pulsesRef.current.push({
          sourceId: pe.source,
          targetId: pe.target,
          color: pe.won ? '#22c55e' : '#ef4444',
          startTime: performance.now(),
          intentId: pe.intentId,
        });
      }
      lastPulseCountRef.current = data.pulseEvents.length;
    }

    function draw(now: number) {
      if (!ctx) return;
      ctx.clearRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

      // Background
      ctx.fillStyle = '#0a0a0f';
      ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

      // Grid
      ctx.strokeStyle = '#1a1a2e';
      ctx.lineWidth = 0.5;
      for (let x = 0; x < CANVAS_WIDTH; x += 40) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, CANVAS_HEIGHT);
        ctx.stroke();
      }
      for (let y = 0; y < CANVAS_HEIGHT; y += 40) {
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(CANVAS_WIDTH, y);
        ctx.stroke();
      }

      // Edges
      for (const edge of data.edges) {
        const s = nodeMap.get(edge.source);
        const t = nodeMap.get(edge.target);
        if (!s || !t) continue;
        ctx.beginPath();
        ctx.moveTo(s.x, s.y);
        ctx.lineTo(t.x, t.y);
        ctx.strokeStyle = `rgba(74, 85, 104, ${0.3 + edge.weight * 0.5})`;
        ctx.lineWidth = 1.5;
        ctx.stroke();
      }

      // Pulse animations
      const activePulses: PulseAnim[] = [];
      for (const pulse of pulsesRef.current) {
        const elapsed = now - pulse.startTime;
        if (elapsed > PULSE_DURATION_MS) continue;
        activePulses.push(pulse);

        const s = nodeMap.get(pulse.sourceId);
        const t = nodeMap.get(pulse.targetId);
        if (!s || !t) continue;

        const progress = elapsed / PULSE_DURATION_MS;
        const px = s.x + (t.x - s.x) * progress;
        const py = s.y + (t.y - s.y) * progress;

        // Trail
        const trailLen = 0.15;
        const trailStart = Math.max(0, progress - trailLen);
        const tsx = s.x + (t.x - s.x) * trailStart;
        const tsy = s.y + (t.y - s.y) * trailStart;

        const grad = ctx.createLinearGradient(tsx, tsy, px, py);
        grad.addColorStop(0, 'rgba(0,0,0,0)');
        grad.addColorStop(1, pulse.color);

        ctx.beginPath();
        ctx.moveTo(tsx, tsy);
        ctx.lineTo(px, py);
        ctx.strokeStyle = grad;
        ctx.lineWidth = 3;
        ctx.stroke();

        // Pulse head glow
        ctx.beginPath();
        ctx.arc(px, py, 5, 0, Math.PI * 2);
        ctx.fillStyle = pulse.color;
        ctx.shadowColor = pulse.color;
        ctx.shadowBlur = 12;
        ctx.fill();
        ctx.shadowBlur = 0;
      }
      pulsesRef.current = activePulses;

      // Nodes
      for (const node of data.nodes) {
        const color = STATUS_COLORS[node.status] ?? '#6b7280';

        // Outer glow
        const glowGrad = ctx.createRadialGradient(
          node.x, node.y, NODE_CORE_RADIUS * 0.5,
          node.x, node.y, NODE_GLOW_RADIUS,
        );
        glowGrad.addColorStop(0, color + '40');
        glowGrad.addColorStop(1, color + '00');
        ctx.beginPath();
        ctx.arc(node.x, node.y, NODE_GLOW_RADIUS, 0, Math.PI * 2);
        ctx.fillStyle = glowGrad;
        ctx.fill();

        // Core
        ctx.beginPath();
        ctx.arc(node.x, node.y, NODE_CORE_RADIUS, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.globalAlpha = 0.85;
        ctx.fill();
        ctx.globalAlpha = 1;
        ctx.strokeStyle = '#0a0a0f';
        ctx.lineWidth = 2;
        ctx.stroke();

        // Label
        ctx.fillStyle = '#a0aec0';
        ctx.font = '11px system-ui, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(node.label, node.x, node.y + NODE_CORE_RADIUS + 16);
      }

      animFrameRef.current = requestAnimationFrame(draw);
    }

    animFrameRef.current = requestAnimationFrame(draw);

    return () => {
      cancelAnimationFrame(animFrameRef.current);
    };
  }, [data]);

  return (
    <div className="panel">
      <h2 className="panel-title">Swarm Map — Intent Negotiation</h2>
      <canvas
        ref={canvasRef}
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
        className="swarm-canvas"
      />
      <div className="swarm-legend">
        {Object.entries(STATUS_COLORS).map(([status, color]) => (
          <span key={status} className="legend-item">
            <span className="legend-dot" style={{ background: color }} />
            {status}
          </span>
        ))}
      </div>
    </div>
  );
}