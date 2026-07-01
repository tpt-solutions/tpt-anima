import { useEffect, useRef } from 'react';

interface EconomicFlowProps {
  agents: { id: string; balance: number }[];
  totalCirculation: number;
}

const CANVAS_WIDTH = 300;
const CANVAS_HEIGHT = 200;
const BAR_GAP = 12;
const BAR_MAX_HEIGHT = 100;
const BAR_AREA_TOP = 10;
const FLOW_PARTICLE_COUNT = 20;

interface FlowParticle {
  progress: number;
  speed: number;
  size: number;
  color: string;
}

export function EconomicFlow({ agents, totalCirculation }: EconomicFlowProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animFrameRef = useRef<number>(0);
  const particlesRef = useRef<FlowParticle[]>([]);
  const startTimeRef = useRef(performance.now());

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Initialize flow particles
    if (particlesRef.current.length === 0) {
      for (let i = 0; i < FLOW_PARTICLE_COUNT; i++) {
        particlesRef.current.push({
          progress: Math.random(),
          speed: 0.002 + Math.random() * 0.004,
          size: 2 + Math.random() * 3,
          color: ['#22c55e', '#3b82f6', '#a78bfa', '#eab308'][Math.floor(Math.random() * 4)],
        });
      }
    }

    startTimeRef.current = performance.now();
    const maxBalance = Math.max(...agents.map((a) => a.balance), 1);
    const barWidth = Math.min(40, (CANVAS_WIDTH - 40 - (agents.length - 1) * BAR_GAP) / agents.length);

    function draw(now: number) {
      if (!ctx) return;
      const _elapsed = now - startTimeRef.current;
      ctx.clearRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

      // Background
      ctx.fillStyle = '#0a0a0f';
      ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

      // Flow particles along bottom
      const flowY = BAR_AREA_TOP + BAR_MAX_HEIGHT + 30;
      const flowWidth = CANVAS_WIDTH - 40;
      const flowX = 20;

      // Flow channel
      ctx.fillStyle = '#1a1a2e';
      ctx.fillRect(flowX, flowY - 4, flowWidth, 8);

      // Update and draw particles
      for (const p of particlesRef.current) {
        p.progress += p.speed;
        if (p.progress > 1) p.progress = 0;

        const px = flowX + p.progress * flowWidth;
        const py = flowY + Math.sin(p.progress * Math.PI * 4 + _elapsed * 0.002) * 3;

        ctx.beginPath();
        ctx.arc(px, py, p.size, 0, Math.PI * 2);
        ctx.fillStyle = p.color;
        ctx.globalAlpha = 0.6;
        ctx.fill();
        ctx.globalAlpha = 1;
      }

      // Agent balance bars
      const startX = (CANVAS_WIDTH - (agents.length * barWidth + (agents.length - 1) * BAR_GAP)) / 2;

      agents.forEach((agent, i) => {
        const x = startX + i * (barWidth + BAR_GAP);
        const height = (agent.balance / maxBalance) * BAR_MAX_HEIGHT;
        const y = BAR_AREA_TOP + BAR_MAX_HEIGHT - height;

        // Bar track
        ctx.fillStyle = '#1a1a2e';
        ctx.fillRect(x, BAR_AREA_TOP, barWidth, BAR_MAX_HEIGHT);

        // Bar fill
        const grad = ctx.createLinearGradient(0, y, 0, BAR_AREA_TOP + BAR_MAX_HEIGHT);
        grad.addColorStop(0, '#22c55e');
        grad.addColorStop(1, '#22c55e80');
        ctx.fillStyle = grad;
        ctx.fillRect(x, y, barWidth, height);

        // Bar border
        ctx.strokeStyle = '#22c55e';
        ctx.lineWidth = 1;
        ctx.strokeRect(x, BAR_AREA_TOP, barWidth, BAR_MAX_HEIGHT);

        // Balance value
        ctx.fillStyle = '#e2e8f0';
        ctx.font = 'bold 9px system-ui, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(`${agent.balance}`, x + barWidth / 2, y - 4);

        // Agent label
        ctx.fillStyle = '#94a3b8';
        ctx.font = '8px system-ui, sans-serif';
        const label = agent.id.length > 6 ? agent.id.slice(0, 6) + '…' : agent.id;
        ctx.fillText(label, x + barWidth / 2, BAR_AREA_TOP + BAR_MAX_HEIGHT + 12);
      });

      // Total circulation
      ctx.fillStyle = '#64748b';
      ctx.font = '10px system-ui, sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(
        `Total Circulation: ${totalCirculation.toLocaleString()} credits`,
        CANVAS_WIDTH / 2,
        BAR_AREA_TOP + BAR_MAX_HEIGHT + 50,
      );

      // Task cost breakdown area
      const breakdownY = BAR_AREA_TOP + BAR_MAX_HEIGHT + 62;
      ctx.fillStyle = '#1a1a2e';
      ctx.fillRect(10, breakdownY, CANVAS_WIDTH - 20, CANVAS_HEIGHT - breakdownY - 5);
      ctx.fillStyle = '#475569';
      ctx.font = '9px system-ui, sans-serif';
      ctx.textAlign = 'left';
      ctx.fillText('Task Cost Breakdown', 16, breakdownY + 12);

      // Show top 3 agents by balance as task cost breakdown
      const sorted = [...agents].sort((a, b) => b.balance - a.balance).slice(0, 3);
      sorted.forEach((agent, i) => {
        const rowY = breakdownY + 22 + i * 12;
        const pct = totalCirculation > 0 ? ((agent.balance / totalCirculation) * 100).toFixed(1) : '0.0';
        ctx.fillStyle = '#94a3b8';
        ctx.fillText(`${agent.id}: ${agent.balance} (${pct}%)`, 16, rowY);
      });

      animFrameRef.current = requestAnimationFrame(draw);
    }

    animFrameRef.current = requestAnimationFrame(draw);

    return () => {
      cancelAnimationFrame(animFrameRef.current);
    };
  }, [agents, totalCirculation]);

  return (
    <div className="panel">
      <h2 className="panel-title">Economic Flow — Compute Ledger</h2>
      <canvas
        ref={canvasRef}
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
        className="econ-canvas"
      />
    </div>
  );
}