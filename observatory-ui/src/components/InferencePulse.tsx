import { useEffect, useRef } from 'react';

interface InferencePulseProps {
  gpu: number;
  cpu: number;
  npu: number;
  ram: number;
  bubbles: number;
  queue: number;
}

const CANVAS_WIDTH = 300;
const CANVAS_HEIGHT = 200;
const BAR_WIDTH = 40;
const BAR_GAP = 20;
const BAR_MAX_HEIGHT = 100;
const METER_Y = 10;

function barColor(pct: number): string {
  if (pct >= 80) return '#ef4444';
  if (pct >= 60) return '#eab308';
  return '#22c55e';
}

interface ThoughtBubbleAnim {
  x: number;
  y: number;
  r: number;
  color: string;
  vx: number;
  vy: number;
  alpha: number;
  hardware: 'gpu' | 'cpu' | 'npu';
}

export function InferencePulse({ gpu, cpu, npu, ram, bubbles, queue }: InferencePulseProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animFrameRef = useRef<number>(0);
  const bubbleAnimsRef = useRef<ThoughtBubbleAnim[]>([]);
  const prevBubblesRef = useRef(bubbles);
  const startTimeRef = useRef(performance.now());

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Spawn new bubbles when count increases
    if (bubbles > prevBubblesRef.current) {
      const hw: Array<'gpu' | 'cpu' | 'npu'> = ['gpu', 'cpu', 'npu'];
      const hwColors: Record<string, string> = { gpu: '#22c55e', cpu: '#3b82f6', npu: '#a78bfa' };
      for (let i = prevBubblesRef.current; i < bubbles; i++) {
        const hardware = hw[i % 3];
        const baseX = 30 + (i % 4) * 60;
        bubbleAnimsRef.current.push({
          x: baseX + Math.random() * 20,
          y: 180 + Math.random() * 10,
          r: 6 + Math.random() * 6,
          color: hwColors[hardware],
          vx: (Math.random() - 0.5) * 0.8,
          vy: -0.3 - Math.random() * 0.5,
          alpha: 0.7 + Math.random() * 0.3,
          hardware,
        });
      }
    }
    prevBubblesRef.current = bubbles;

    // Trim bubbles if count decreased
    if (bubbleAnimsRef.current.length > bubbles) {
      bubbleAnimsRef.current = bubbleAnimsRef.current.slice(0, bubbles);
    }

    startTimeRef.current = performance.now();

    function draw(_now: number) {
      if (!ctx) return;
      ctx.clearRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

      // Background
      ctx.fillStyle = '#0a0a0f';
      ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

      // Utilization bars
      const metrics = [
        { label: 'GPU', value: gpu },
        { label: 'CPU', value: cpu },
        { label: 'NPU', value: npu },
        { label: 'RAM', value: ram },
      ];

      const startX = (CANVAS_WIDTH - (metrics.length * BAR_WIDTH + (metrics.length - 1) * BAR_GAP)) / 2;

      metrics.forEach((m, i) => {
        const x = startX + i * (BAR_WIDTH + BAR_GAP);
        const clamped = Math.min(100, Math.max(0, m.value));
        const h = (clamped / 100) * BAR_MAX_HEIGHT;
        const color = barColor(clamped);

        // Bar track
        ctx.fillStyle = '#1a1a2e';
        ctx.fillRect(x, METER_Y, BAR_WIDTH, BAR_MAX_HEIGHT);

        // Bar fill with gradient
        const grad = ctx.createLinearGradient(0, METER_Y + BAR_MAX_HEIGHT - h, 0, METER_Y + BAR_MAX_HEIGHT);
        grad.addColorStop(0, color);
        grad.addColorStop(1, color + '80');
        ctx.fillStyle = grad;
        ctx.fillRect(x, METER_Y + BAR_MAX_HEIGHT - h, BAR_WIDTH, h);

        // Bar border
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;
        ctx.strokeRect(x, METER_Y, BAR_WIDTH, BAR_MAX_HEIGHT);

        // Percentage text
        ctx.fillStyle = '#e2e8f0';
        ctx.font = 'bold 11px system-ui, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(`${Math.round(clamped)}%`, x + BAR_WIDTH / 2, METER_Y + BAR_MAX_HEIGHT + 14);

        // Label
        ctx.fillStyle = '#94a3b8';
        ctx.font = '10px system-ui, sans-serif';
        ctx.fillText(m.label, x + BAR_WIDTH / 2, METER_Y + BAR_MAX_HEIGHT + 28);
      });

      // Thermal / VRAM / FLOP meters (horizontal bars at bottom area)
      const meterY = METER_Y + BAR_MAX_HEIGHT + 40;
      const meterW = CANVAS_WIDTH - 40;
      const meterX = 20;
      const meterH = 10;

      // Thermal meter
      const thermalPct = Math.min(100, (gpu * 0.6 + cpu * 0.3 + npu * 0.1));
      ctx.fillStyle = '#94a3b8';
      ctx.font = '9px system-ui, sans-serif';
      ctx.textAlign = 'left';
      ctx.fillText('THERMAL', meterX, meterY - 2);
      ctx.fillStyle = '#1a1a2e';
      ctx.fillRect(meterX + 60, meterY - 10, meterW - 60, meterH);
      ctx.fillStyle = barColor(thermalPct);
      ctx.fillRect(meterX + 60, meterY - 10, (meterW - 60) * (thermalPct / 100), meterH);

      // VRAM meter
      const vramPct = ram;
      ctx.fillStyle = '#94a3b8';
      ctx.fillText('VRAM', meterX, meterY + 12);
      ctx.fillStyle = '#1a1a2e';
      ctx.fillRect(meterX + 60, meterY + 2, meterW - 60, meterH);
      ctx.fillStyle = barColor(vramPct);
      ctx.fillRect(meterX + 60, meterY + 2, (meterW - 60) * (vramPct / 100), meterH);

      // FLOP meter
      const flopPct = Math.min(100, (gpu * 0.7 + npu * 0.3));
      ctx.fillStyle = '#94a3b8';
      ctx.fillText('FLOP', meterX, meterY + 24);
      ctx.fillStyle = '#1a1a2e';
      ctx.fillRect(meterX + 60, meterY + 14, meterW - 60, meterH);
      ctx.fillStyle = barColor(flopPct);
      ctx.fillRect(meterX + 60, meterY + 14, (meterW - 60) * (flopPct / 100), meterH);

      // ThoughtBubbles area
      const bubbleAreaY = meterY + 30;
      ctx.fillStyle = '#1a1a2e';
      ctx.fillRect(10, bubbleAreaY, CANVAS_WIDTH - 20, CANVAS_HEIGHT - bubbleAreaY - 5);
      ctx.fillStyle = '#475569';
      ctx.font = '9px system-ui, sans-serif';
      ctx.textAlign = 'left';
      ctx.fillText(`ThoughtBubbles: ${bubbles}  Queue: ${queue}`, 16, bubbleAreaY + 12);

      // Update and draw thought bubbles
      const activeBubbles: ThoughtBubbleAnim[] = [];
      for (const b of bubbleAnimsRef.current) {
        b.x += b.vx;
        b.y += b.vy;
        b.alpha -= 0.001;

        // Bounce off walls
        if (b.x - b.r < 12 || b.x + b.r > CANVAS_WIDTH - 12) b.vx *= -1;
        if (b.y - b.r < bubbleAreaY + 16) b.vy *= -0.8;

        if (b.alpha <= 0 || b.y > CANVAS_HEIGHT) continue;
        activeBubbles.push(b);

        // Bubble glow
        const glowGrad = ctx.createRadialGradient(b.x, b.y, 0, b.x, b.y, b.r * 2);
        glowGrad.addColorStop(0, b.color + Math.round(b.alpha * 60).toString(16).padStart(2, '0'));
        glowGrad.addColorStop(1, b.color + '00');
        ctx.beginPath();
        ctx.arc(b.x, b.y, b.r * 2, 0, Math.PI * 2);
        ctx.fillStyle = glowGrad;
        ctx.fill();

        // Bubble core
        ctx.beginPath();
        ctx.arc(b.x, b.y, b.r, 0, Math.PI * 2);
        ctx.fillStyle = b.color;
        ctx.globalAlpha = b.alpha * 0.6;
        ctx.fill();
        ctx.globalAlpha = 1;
        ctx.strokeStyle = b.color;
        ctx.lineWidth = 1;
        ctx.stroke();
      }
      bubbleAnimsRef.current = activeBubbles;

      animFrameRef.current = requestAnimationFrame(draw);
    }

    animFrameRef.current = requestAnimationFrame(draw);

    return () => {
      cancelAnimationFrame(animFrameRef.current);
    };
  }, [gpu, cpu, npu, ram, bubbles, queue]);

  return (
    <div className="panel">
      <h2 className="panel-title">Inference Pulse — Cognitive Kernel</h2>
      <canvas
        ref={canvasRef}
        width={CANVAS_WIDTH}
        height={CANVAS_HEIGHT}
        className="pulse-canvas"
      />
    </div>
  );
}