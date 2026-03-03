export default function SpaceViewer() {
  return (
    <div>
      <h1 style={{ margin: '0 0 24px 0', color: '#1a1a2e' }}>Space Viewer</h1>
      <div style={{
        background: '#ffffff',
        "border-radius": '8px',
        padding: '24px',
        "box-shadow": '0 1px 3px rgba(0,0,0,0.12)',
      }}>
        <p style={{ color: '#666', margin: '0 0 16px 0' }}>
          3D visualization of world spaces, navigation meshes, spawn points, and entity positions.
        </p>
        <div style={{
          height: '500px',
          border: '1px dashed #ddd',
          "border-radius": '4px',
          display: 'flex',
          "align-items": 'center',
          "justify-content": 'center',
          color: '#aaa',
          background: '#1a1a2e',
        }}>
          <span style={{ color: '#555' }}>Three.js viewport will render here</span>
        </div>
      </div>
    </div>
  );
}
