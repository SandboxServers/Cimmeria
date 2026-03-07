export default function ContentEditor() {
  return (
    <div>
      <h1 style={{ margin: '0 0 24px 0', color: '#1a1a2e' }}>Content Editor</h1>
      <div style={{
        background: '#ffffff',
        "border-radius": '8px',
        padding: '24px',
        "box-shadow": '0 1px 3px rgba(0,0,0,0.12)',
      }}>
        <p style={{ color: '#666', margin: '0 0 16px 0' }}>
          Browse and edit game content: entities, abilities, items, missions, dialogs, effects, and loot tables.
        </p>
        <div style={{ display: 'flex', gap: '8px', "flex-wrap": 'wrap' }}>
          {['Entities', 'Abilities', 'Items', 'Missions', 'Dialogs', 'Effects', 'Loot'].map((tab) => (
            <button style={{
              padding: '8px 16px',
              border: '1px solid #ddd',
              "border-radius": '4px',
              background: '#f8f8f8',
              cursor: 'pointer',
              "font-size": '13px',
            }}>
              {tab}
            </button>
          ))}
        </div>
        <div style={{ "margin-top": '24px', padding: '40px', "text-align": 'center', color: '#aaa', border: '1px dashed #ddd', "border-radius": '4px' }}>
          Select a content type to begin editing
        </div>
      </div>
    </div>
  );
}
