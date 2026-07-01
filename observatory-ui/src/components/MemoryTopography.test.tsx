import { render, screen } from '@testing-library/react';
import { MemoryTopography } from './MemoryTopography';
import type { MemoryTopoData } from '../types';

const mockData: MemoryTopoData = {
  nodes: [
    { id: 'node-1', strength: 0.8, modality: 'text', label: 'Memory A', x: 100, y: 100 },
    { id: 'node-2', strength: 0.5, modality: 'visual', label: 'Memory B', x: 300, y: 200 },
    { id: 'node-3', strength: 0.3, modality: 'audio', label: 'Memory C', x: 500, y: 300 },
  ],
  chains: [
    { source: 'node-1', target: 'node-2', strength: 0.7 },
    { source: 'node-2', target: 'node-3', strength: 0.4 },
  ],
};

describe('MemoryTopography', () => {
  it('renders the panel with title', () => {
    render(<MemoryTopography data={mockData} />);
    expect(screen.getByText('Memory Topography — Semantic State Engine')).toBeInTheDocument();
  });

  it('renders the canvas element', () => {
    render(<MemoryTopography data={mockData} />);
    const canvas = document.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
    expect(canvas).toHaveAttribute('width', '600');
    expect(canvas).toHaveAttribute('height', '400');
  });

  it('renders empty data without errors', () => {
    const emptyData: MemoryTopoData = { nodes: [], chains: [] };
    render(<MemoryTopography data={emptyData} />);
    expect(screen.getByText('Memory Topography — Semantic State Engine')).toBeInTheDocument();
    const canvas = document.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
  });
});