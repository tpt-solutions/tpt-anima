import { render, screen } from '@testing-library/react';
import { InferencePulse } from './InferencePulse';

describe('InferencePulse', () => {
  it('renders the panel with title', () => {
    render(<InferencePulse gpu={50} cpu={30} npu={20} ram={40} bubbles={0} queue={0} />);
    expect(screen.getByText('Inference Pulse — Cognitive Kernel')).toBeInTheDocument();
  });

  it('renders the canvas element', () => {
    render(<InferencePulse gpu={50} cpu={30} npu={20} ram={40} bubbles={0} queue={0} />);
    const canvas = document.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
    expect(canvas).toHaveAttribute('width', '300');
    expect(canvas).toHaveAttribute('height', '200');
  });

  it('renders with zero utilization', () => {
    render(<InferencePulse gpu={0} cpu={0} npu={0} ram={0} bubbles={0} queue={0} />);
    const canvas = document.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
  });
});