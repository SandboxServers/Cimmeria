export default function ChainEditor() {
  return (
    <div>
      <h1 style={{ margin: '0 0 24px 0', color: '#1a1a2e' }}>Chain Editor</h1>
      <div style={{
        background: '#ffffff',
        "border-radius": '8px',
        padding: '24px',
        "box-shadow": '0 1px 3px rgba(0,0,0,0.12)',
      }}>
        <p style={{ color: '#666', margin: '0 0 16px 0' }}>
          Visual node-graph editor for event sequences, dialog chains, and mission flows.
        </p>
        <div style={{
          height: '400px',
          border: '1px dashed #ddd',
          "border-radius": '4px',
          display: 'flex',
          "align-items": 'center',
          "justify-content": 'center',
          color: '#aaa',
          background: '#fafafa',
        }}>
          Chain editor canvas (node graph will render here)
        </div>
      </div>
    </div>
  );
}
