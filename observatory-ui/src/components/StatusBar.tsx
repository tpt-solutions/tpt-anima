import { useEffect, useState } from 'react';

interface StatusBarProps {
  connected: boolean;
  eventCount: number;
}

export function StatusBar({ connected, eventCount }: StatusBarProps) {
  const [uptime, setUptime] = useState(0);

  useEffect(() => {
    const start = Date.now();
    const interval = setInterval(() => {
      setUptime(Math.floor((Date.now() - start) / 1000));
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const minutes = Math.floor(uptime / 60);
  const seconds = uptime % 60;
  const uptimeStr = `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;

  const statusClass = connected ? 'status-connected' : 'status-disconnected';
  const statusText = connected ? 'Connected' : 'Disconnected';

  return (
    <div className="status-bar">
      <span className={`status-indicator ${statusClass}`}>
        <span className="status-dot" />
        {statusText}
      </span>
      <span className="status-item">Events: {eventCount.toLocaleString()}</span>
      <span className="status-item">Uptime: {uptimeStr}</span>
    </div>
  );
}
