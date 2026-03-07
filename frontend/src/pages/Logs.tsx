export default function Logs() {
  return (
    <div>
      <h1 style={{ margin: '0 0 24px 0', color: '#1a1a2e' }}>Server Logs</h1>
      <div style={{
        background: '#ffffff',
        "border-radius": '8px',
        padding: '24px',
        "box-shadow": '0 1px 3px rgba(0,0,0,0.12)',
      }}>
        <div style={{ display: 'flex', gap: '8px', "margin-bottom": '16px' }}>
          {['All', 'Info', 'Warn', 'Error', 'Debug'].map((level) => (
            <button style={{
              padding: '6px 12px',
              border: '1px solid #ddd',
              "border-radius": '4px',
              background: level === 'All' ? '#1a1a2e' : '#f8f8f8',
              color: level === 'All' ? '#fff' : '#333',
              cursor: 'pointer',
              "font-size": '12px',
            }}>
              {level}
            </button>
          ))}
        </div>
        <div style={{
          background: '#0d0d1a',
          color: '#a0ffa0',
          "font-family": 'monospace',
          "font-size": '12px',
          padding: '16px',
          "border-radius": '4px',
          height: '400px',
          overflow: 'auto',
          "white-space": 'pre',
        }}>
          Waiting for log stream connection...
        </div>
      </div>
    </div>
  );
}
