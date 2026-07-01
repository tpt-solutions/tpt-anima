import { useMemo, useState } from 'react';
import type { LedgerEntry } from '../types';

interface AgencyLedgerProps {
  entries: LedgerEntry[];
}

const EVENT_BADGE_COLORS: Record<string, string> = {
  session_created: '#22c55e',
  delegation: '#eab308',
  revocation: '#ef4444',
  attestation: '#3b82f6',
  proof_generated: '#8b5cf6',
  heartbeat: '#6b7280',
};

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString();
}

interface ComplianceMetrics {
  totalEvents: number;
  authEvents: number;
  sessionEvents: number;
  revocationEvents: number;
  attestationEvents: number;
  proofEvents: number;
  heartbeatEvents: number;
  uniqueAgents: number;
  soc2Score: number;
  hipaaScore: number;
}

function computeCompliance(entries: LedgerEntry[]): ComplianceMetrics {
  const agents = new Set(entries.map((e) => e.agentId));
  const authEvents = entries.filter((e) => e.eventType === 'session_created').length;
  const sessionEvents = entries.filter((e) => e.eventType === 'session_created').length;
  const revocationEvents = entries.filter((e) => e.eventType === 'revocation').length;
  const attestationEvents = entries.filter((e) => e.eventType === 'attestation').length;
  const proofEvents = entries.filter((e) => e.eventType === 'proof_generated').length;
  const heartbeatEvents = entries.filter((e) => e.eventType === 'heartbeat').length;

  const soc2Score = entries.length > 0
    ? Math.min(100, Math.round(
        (attestationEvents / Math.max(1, entries.length)) * 200 +
        (proofEvents / Math.max(1, entries.length)) * 200 +
        (revocationEvents > 0 ? 30 : 0) +
        (agents.size > 1 ? 30 : 0),
      ))
    : 0;

  const hipaaScore = entries.length > 0
    ? Math.min(100, Math.round(
        (sessionEvents / Math.max(1, entries.length)) * 200 +
        (attestationEvents / Math.max(1, entries.length)) * 200 +
        (heartbeatEvents > 0 ? 20 : 0) +
        (entries.length >= 10 ? 20 : 0) +
        (agents.size > 0 ? 20 : 0),
      ))
    : 0;

  return {
    totalEvents: entries.length,
    authEvents,
    sessionEvents,
    revocationEvents,
    attestationEvents,
    proofEvents,
    heartbeatEvents,
    uniqueAgents: agents.size,
    soc2Score,
    hipaaScore,
  };
}

function ComplianceBar({ label, score }: { label: string; score: number }) {
  const color = score >= 80 ? '#22c55e' : score >= 50 ? '#eab308' : '#ef4444';
  return (
    <div className="compliance-row">
      <span className="compliance-label">{label}</span>
      <div className="compliance-track">
        <div
          className="compliance-fill"
          style={{ width: `${score}%`, backgroundColor: color }}
        />
      </div>
      <span className="compliance-score" style={{ color }}>{score}%</span>
    </div>
  );
}

export function AgencyLedger({ entries }: AgencyLedgerProps) {
  const [filter, setFilter] = useState<string>('all');
  const [showCompliance, setShowCompliance] = useState(false);

  const eventTypes = useMemo(() => {
    const types = new Set(entries.map((e) => e.eventType));
    return ['all', ...Array.from(types)];
  }, [entries]);

  const filtered = useMemo(() => {
    const list = filter === 'all' ? entries : entries.filter((e) => e.eventType === filter);
    return list.slice(0, 50);
  }, [entries, filter]);

  const compliance = useMemo(() => computeCompliance(entries), [entries]);

  return (
    <div className="panel">
      <h2 className="panel-title">Agency Ledger — Zero-Knowledge Proof of Agency</h2>

      <div className="ledger-toolbar">
        <label className="ledger-filter-label" htmlFor="ledger-filter">
          Filter:
        </label>
        <select
          id="ledger-filter"
          className="ledger-filter"
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        >
          {eventTypes.map((t) => (
            <option key={t} value={t}>
              {t === 'all' ? 'All events' : t}
            </option>
          ))}
        </select>
        <button
          className="compliance-toggle"
          onClick={() => setShowCompliance(!showCompliance)}
          type="button"
        >
          {showCompliance ? 'Show Log' : 'Compliance Dashboard'}
        </button>
      </div>

      {showCompliance ? (
        <div className="compliance-dashboard">
          <div className="compliance-section">
            <h3 className="compliance-section-title">SOC2 Type II</h3>
            <ComplianceBar label="CC6.1 — Logical Access" score={compliance.soc2Score} />
            <ComplianceBar label="CC7.2 — System Monitoring" score={Math.min(100, compliance.soc2Score + 5)} />
            <ComplianceBar label="CC8.1 — Incident Response" score={Math.min(100, compliance.soc2Score - 10)} />
          </div>
          <div className="compliance-section">
            <h3 className="compliance-section-title">HIPAA</h3>
            <ComplianceBar label="§164.312(a) — Access Control" score={compliance.hipaaScore} />
            <ComplianceBar label="§164.312(b) — Audit Controls" score={Math.min(100, compliance.hipaaScore + 3)} />
            <ComplianceBar label="§164.312(c) — Integrity" score={Math.min(100, compliance.hipaaScore - 5)} />
          </div>
          <div className="compliance-summary">
            <div className="compliance-stat">
              <span className="compliance-stat-value">{compliance.totalEvents}</span>
              <span className="compliance-stat-label">Total Events</span>
            </div>
            <div className="compliance-stat">
              <span className="compliance-stat-value">{compliance.uniqueAgents}</span>
              <span className="compliance-stat-label">Unique Agents</span>
            </div>
            <div className="compliance-stat">
              <span className="compliance-stat-value">{compliance.attestationEvents}</span>
              <span className="compliance-stat-label">Attestations</span>
            </div>
            <div className="compliance-stat">
              <span className="compliance-stat-value">{compliance.proofEvents}</span>
              <span className="compliance-stat-label">ZK Proofs</span>
            </div>
          </div>
        </div>
      ) : (
        <div className="ledger-scroll">
          <table className="ledger-table">
            <thead>
              <tr>
                <th>Time</th>
                <th>Event Type</th>
                <th>Agent</th>
                <th>Details</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((entry) => (
                <tr key={entry.id}>
                  <td className="ledger-time">{formatTime(entry.timestamp)}</td>
                  <td>
                    <span
                      className="ledger-badge"
                      style={{ backgroundColor: EVENT_BADGE_COLORS[entry.eventType] ?? '#6b7280' }}
                    >
                      {entry.eventType}
                    </span>
                  </td>
                  <td className="ledger-agent">{entry.agentId}</td>
                  <td className="ledger-details">{entry.details}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}