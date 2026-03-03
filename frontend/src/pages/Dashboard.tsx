import { createSignal, onMount } from 'solid-js';

const cardStyle = {
  background: '#ffffff',
  "border-radius": '8px',
  padding: '20px',
  "box-shadow": '0 1px 3px rgba(0,0,0,0.12)',
  "min-width": '200px',
};

const cardTitle = {
  "font-size": '13px',
  color: '#888',
  "text-transform": 'uppercase' as const,
  "letter-spacing": '0.5px',
  margin: '0 0 8px 0',
};

const cardValue = {
  "font-size": '32px',
  "font-weight": 'bold',
  color: '#1a1a2e',
  margin: '0',
};

export default function Dashboard() {
  const [uptime, setUptime] = createSignal(0);
  const [playerCount, setPlayerCount] = createSignal(0);
  const [entityCount, setEntityCount] = createSignal(0);

  onMount(() => {
    // TODO: Wire up to Tauri IPC or REST API
    setUptime(0);
    setPlayerCount(0);
    setEntityCount(0);
  });

  return (
    <div>
      <h1 style={{ margin: '0 0 24px 0', color: '#1a1a2e' }}>Dashboard</h1>
      <div style={{ display: 'flex', gap: '16px', "flex-wrap": 'wrap' }}>
        <div style={cardStyle}>
          <p style={cardTitle}>Uptime</p>
          <p style={cardValue}>{uptime()}s</p>
        </div>
        <div style={cardStyle}>
          <p style={cardTitle}>Players Online</p>
          <p style={cardValue}>{playerCount()}</p>
        </div>
        <div style={cardStyle}>
          <p style={cardTitle}>Active Entities</p>
          <p style={cardValue}>{entityCount()}</p>
        </div>
        <div style={cardStyle}>
          <p style={cardTitle}>Server Status</p>
          <p style={{ ...cardValue, color: '#2ecc71' }}>Running</p>
        </div>
      </div>
    </div>
  );
}
