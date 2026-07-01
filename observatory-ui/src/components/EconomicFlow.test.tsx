import { render, screen } from '@testing-library/react';
import { EconomicFlow } from './EconomicFlow';

const mockAgents = [
  { id: 'agent-1', balance: 1000 },
  { id: 'agent-2', balance: 500 },
  { id: 'agent-3', balance: 250 },
];

describe('EconomicFlow', () => {
  it('renders the panel with title', () => {
    render(<EconomicFlow agents={mockAgents} totalCirculation={1750} />);
    expect(screen.getByText('Economic Flow — Compute Ledger')).toBeInTheDocument();
  });

  it('renders the canvas element', () => {
    render(<EconomicFlow agents={mockAgents} totalCirculation={1750} />);
    const canvas = document.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
    expect(canvas).toHaveAttribute('width', '300');
    expect(canvas).toHaveAttribute('height', '200');
  });

  it('renders with agents and total circulation', () => {
    render(<EconomicFlow agents={mockAgents} totalCirculation={1750} />);
    const canvas = document.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
  });

  it('renders with empty agents array', () => {
    render(<EconomicFlow agents={[]} totalCirculation={0} />);
    const canvas = document.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
  });
});