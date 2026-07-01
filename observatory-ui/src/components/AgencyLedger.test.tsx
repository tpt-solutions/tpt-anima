import { render, screen, fireEvent } from '@testing-library/react';
import { AgencyLedger } from './AgencyLedger';
import type { LedgerEntry } from '../types';

const mockEntries: LedgerEntry[] = [
  { id: '1', eventType: 'session_created', agentId: 'agent-1', timestamp: Date.now(), details: 'New session started' },
  { id: '2', eventType: 'delegation', agentId: 'agent-1', timestamp: Date.now(), details: 'Delegated to agent-2' },
  { id: '3', eventType: 'attestation', agentId: 'agent-1', timestamp: Date.now(), details: 'TPM attestation verified' },
  { id: '4', eventType: 'proof_generated', agentId: 'agent-1', timestamp: Date.now(), details: 'ZK proof generated' },
  { id: '5', eventType: 'heartbeat', agentId: 'agent-1', timestamp: Date.now(), details: 'Liveness heartbeat' },
];

describe('AgencyLedger', () => {
  it('renders the panel with title', () => {
    render(<AgencyLedger entries={mockEntries} />);
    expect(screen.getByText('Agency Ledger — Zero-Knowledge Proof of Agency')).toBeInTheDocument();
  });

  it('renders ledger entries in the table', () => {
    render(<AgencyLedger entries={mockEntries} />);
    // Use getAllByText to handle multiple elements with same text (dropdown + table)
    const sessionElements = screen.getAllByText('session_created');
    expect(sessionElements.length).toBeGreaterThan(0);
    const delegationElements = screen.getAllByText('delegation');
    expect(delegationElements.length).toBeGreaterThan(0);
    const attestationElements = screen.getAllByText('attestation');
    expect(attestationElements.length).toBeGreaterThan(0);
  });

  it('renders empty state with no entries', () => {
    render(<AgencyLedger entries={[]} />);
    expect(screen.getByText('Agency Ledger — Zero-Knowledge Proof of Agency')).toBeInTheDocument();
    expect(screen.getByText('All events')).toBeInTheDocument();
  });

  it('renders compliance dashboard when toggled', () => {
    render(<AgencyLedger entries={mockEntries} />);
    const toggleButton = screen.getByText('Compliance Dashboard');
    fireEvent.click(toggleButton);
    expect(screen.getByText('SOC2 Type II')).toBeInTheDocument();
    expect(screen.getByText('HIPAA')).toBeInTheDocument();
  });
});
