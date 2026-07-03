import { render, screen } from '@testing-library/react';
import { SwarmMap } from './SwarmMap';
import type { SwarmMapViewData } from '../types';

const mockData: SwarmMapViewData = {
  nodes: [
    { id: 'agent-1', label: 'Agent Alpha', status: 'active', agentType: 'reasoner', x: 100, y: 100 },
    { id: 'agent-2', label: 'Agent Beta', status: 'idle', agentType: 'executor', x: 300, y: 200 },
    { id: 'agent-3', label: 'Agent Gamma', status: 'busy', agentType: 'validator', x: 500, y: 300 },
  ],
  edges: [
    { source: 'agent-1', target: 'agent-2', weight: 0.5 },
    { source: 'agent-2', target: 'agent-3', weight: 0.8 },
  ],
  pulseEvents: [
    { source: 'agent-1', target: 'agent-2', intentId: 'intent-1', timestamp: Date.now(), won: true },
  ],
};

describe('SwarmMap', () => {
  it('renders the panel with title', () => {
    render(<SwarmMap data={mockData} />);
    expect(screen.getByText('Swarm Map — Intent Negotiation')).toBeInTheDocument();
  });

  it('renders the canvas element', () => {
    render(<SwarmMap data={mockData} />);
    const canvas = document.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
    expect(canvas).toHaveAttribute('width', '600');
    expect(canvas).toHaveAttribute('height', '400');
  });

  it('renders the legend with all status types', () => {
    render(<SwarmMap data={mockData} />);
    expect(screen.getByText('active')).toBeInTheDocument();
    expect(screen.getByText('idle')).toBeInTheDocument();
    expect(screen.getByText('busy')).toBeInTheDocument();
  });

  it('renders empty state with no nodes', () => {
    const emptyData: SwarmMapViewData = { nodes: [], edges: [], pulseEvents: [] };
    render(<SwarmMap data={emptyData} />);
    expect(screen.getByText('Swarm Map — Intent Negotiation')).toBeInTheDocument();
    const canvas = document.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
  });

  it('renders with pulse events', () => {
    const dataWithPulses: SwarmMapViewData = {
      ...mockData,
      pulseEvents: [
        { source: 'agent-1', target: 'agent-2', intentId: 'intent-1', timestamp: Date.now(), won: true },
        { source: 'agent-2', target: 'agent-3', intentId: 'intent-2', timestamp: Date.now(), won: false },
      ],
    };
    render(<SwarmMap data={dataWithPulses} />);
    expect(screen.getByText('Swarm Map — Intent Negotiation')).toBeInTheDocument();
  });
});