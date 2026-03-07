export default function Players() {
  return (
    <div>
      <h1 style={{ margin: '0 0 24px 0', color: '#1a1a2e' }}>Online Players</h1>
      <div style={{
        background: '#ffffff',
        "border-radius": '8px',
        "box-shadow": '0 1px 3px rgba(0,0,0,0.12)',
        overflow: 'hidden',
      }}>
        <table style={{ width: '100%', "border-collapse": 'collapse' }}>
          <thead>
            <tr style={{ background: '#f8f8f8', "text-align": 'left' }}>
              <th style={{ padding: '12px 16px', "font-size": '13px', color: '#888' }}>Name</th>
              <th style={{ padding: '12px 16px', "font-size": '13px', color: '#888' }}>Archetype</th>
              <th style={{ padding: '12px 16px', "font-size": '13px', color: '#888' }}>Level</th>
              <th style={{ padding: '12px 16px', "font-size": '13px', color: '#888' }}>World</th>
              <th style={{ padding: '12px 16px', "font-size": '13px', color: '#888' }}>Ping</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td colspan="5" style={{ padding: '24px', "text-align": 'center', color: '#aaa' }}>
                No players online
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}
