import { useEffect, useRef } from 'react';
import type { MemoryTopoViewData } from '../types';

interface MemoryTopographyProps {
  data: MemoryTopoViewData;
}

const MODALITY_COLORS: Record<string, string> = {
  text: '#818cf8',
  visual: '#a78bfa',
  audio: '#c084fc',
};

const MODALITY_RADIUS: Record<string, number> = {
  text: 10,
  visual: 16,
  audio: 22,
};

const CANVAS_WIDTH = 600;
const CANVAS_HEIGHT = 400;
const DECAY_ANIM_SPEED = 0.0003;

export function MemoryTopography({ data }: MemoryTopographyProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animFrameRef = useRef<number>(0);
  const startTimeRef = useRef(performance.now());

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const nodeMap = new Map(data.nodes.map((n) => [n.id, n]));
    startTimeRef.current = performance.now();

    function draw(now: number) {
      if (!ctx) return;
      const elapsed = now - startTimeRef.current;
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

      // Chains with glow
      for (const chain of data.chains) {
        const s = nodeMap.get(chain.source);
        const t = nodeMap.get(chain.target);
        if (!s || !t) continue;

        const pulse = 0.5 + 0.5 * Math.sin(elapsed * 0.001 + chain.strength * 4);
        const alpha = 0.2 + chain.strength * 0.5 * pulse;

        // Glow layer
        ctx.beginPath();
        ctx.moveTo(s.x, s.y);
        ctx.lineTo(t.x, t.y);
        ctx.strokeStyle = `rgba(99, 102, 241, ${alpha * 0.4})`;
        ctx.lineWidth = 4 + chain.strength * 4;
        ctx.stroke();

        // Core line
        ctx.beginPath();
        ctx.moveTo(s.x, s.y);
        ctx.lineTo(t.x, t.y);
        ctx.strokeStyle = `rgba(99, 102, 241, ${alpha})`;
        ctx.lineWidth = 1 + chain.strength * 2;
        ctx.stroke();
      }

      // Nodes with temporal decay animation
      for (const node of data.nodes) {
        const baseColor = MODALITY_COLORS[node.modality] ?? '#818cf8';
        const r = MODALITY_RADIUS[node.modality] ?? 12;

        // Animated decay: strength oscillates to simulate decay/reinforcement
        const decayPhase = Math.sin(elapsed * DECAY_ANIM_SPEED + node.x * 0.01);
        const animatedStrength = Math.max(0.05, node.strength + decayPhase * 0.15);

        const glowR = r * 2.5;
        const glowOpacity = 0.1 + animatedStrength * 0.5;

        // Outer glow
        const glowGrad = ctx.createRadialGradient(
          node.x, node.y, r * 0.3,
          node.x, node.y, glowR,
        );
        glowGrad.addColorStop(0, baseColor + Math.round(glowOpacity * 90).toString(16).padStart(2, '0'));
        glowGrad.addColorStop(1, baseColor + '00');
        ctx.beginPath();
        ctx.arc(node.x, node.y, glowR, 0, Math.PI * 2);
        ctx.fillStyle = glowGrad;
        ctx.fill();

        // Core node
        ctx.beginPath();
        ctx.arc(node.x, node.y, r, 0, Math.PI * 2);
        ctx.fillStyle = baseColor;
        ctx.globalAlpha = 0.4 + animatedStrength * 0.6;
        ctx.fill();
        ctx.globalAlpha = 1;
        ctx.strokeStyle = '#a5b4fc';
        ctx.lineWidth = 1;
        ctx.stroke();

        // Strength ring
        ctx.beginPath();
        ctx.arc(node.x, node.y, r + 3, 0, Math.PI * 2 * animatedStrength);
        ctx.strokeStyle = baseColor;
        ctx.lineWidth = 1.5;
        ctx.globalAlpha = 0.6;
        ctx.stroke();
        ctx.globalAlpha = 1;

        // Label
        ctx.fillStyle = '#94a3b8';
        ctx.font = '10px system-ui, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(node.label, node.x, node.y + r + 16);
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
      <h2 className="panel-title">Memory Topography — Semantic State Engine</h2>
      <canvas
        ref={canvasRef}
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
        className="topo-canvas"
      />
    </div>
  );
}