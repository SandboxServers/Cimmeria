export default function Config() {
  return (
    <div>
      <h1 style={{ margin: '0 0 24px 0', color: '#1a1a2e' }}>Server Configuration</h1>
      <div style={{
        background: '#ffffff',
        "border-radius": '8px',
        padding: '24px',
        "box-shadow": '0 1px 3px rgba(0,0,0,0.12)',
      }}>
        <p style={{ color: '#666', margin: '0 0 24px 0' }}>
          View and modify server configuration. Changes require a restart to take effect.
        </p>
        <div style={{ display: 'flex', "flex-direction": 'column', gap: '16px' }}>
          {[
            { label: 'Auth Port', value: '13001' },
            { label: 'Base Port (Shard)', value: '32832' },
            { label: 'Cell Port', value: '50000' },
            { label: 'Admin API Port', value: '8443' },
            { label: 'Developer Mode', value: 'false' },
            { label: 'Database', value: 'localhost:5433/sgw' },
          ].map((item) => (
            <div style={{ display: 'flex', "align-items": 'center', gap: '16px' }}>
              <label style={{ width: '180px', "font-size": '13px', color: '#666' }}>{item.label}</label>
              <input
                type="text"
                value={item.value}
                readonly
                style={{
                  flex: '1',
                  padding: '8px 12px',
                  border: '1px solid #ddd',
                  "border-radius": '4px',
                  "font-size": '13px',
                  background: '#f8f8f8',
                  color: '#333',
                }}
              />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
