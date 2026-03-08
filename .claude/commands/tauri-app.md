# Tauri 2 Application Patterns

> Guidelines for Tauri 2 desktop application development. Apply when working on the Tauri shell, Rust commands, or frontend-backend bridge.

---

## Architecture Overview

Tauri 2 uses a Rust backend + web frontend architecture:

```
┌────────────────────────────┐
│  Frontend (WebView)        │
│  React + Vite + TypeScript │
│          ↕ IPC             │
│  Rust Backend (Core)       │
│  Commands, State, Plugins  │
└────────────────────────────┘
```

## Rust Commands

### Defining Commands

```rust
#[tauri::command]
async fn fetch_players(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Player>, String> {
    let db = state.db.lock().await;
    db.get_players().await.map_err(|e| e.to_string())
}
```

### Registering Commands

```rust
fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            fetch_players,
            save_draft,
            load_content,
        ])
        .run(tauri::generate_context!())
        .expect("error running app");
}
```

## Frontend IPC

### Invoking Commands

```typescript
import { invoke } from '@tauri-apps/api/core';

interface Player {
  id: number;
  name: string;
  level: number;
}

async function fetchPlayers(): Promise<Player[]> {
  return await invoke<Player[]>('fetch_players');
}
```

### Error Handling

```typescript
try {
  const result = await invoke<SaveResult>('save_draft', { draft: draftData });
} catch (error) {
  // Tauri returns string errors from Rust
  console.error('Save failed:', error);
  showToast({ type: 'error', message: String(error) });
}
```

## State Management

### Rust-Side State

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            db: Arc::new(Mutex::new(Database::connect().unwrap())),
            config: Arc::new(Config::load()),
        }
    }
}
```

### Frontend-Side State

- Use React state/context for UI state
- Call Tauri commands for persistence
- Graceful fallback when running in browser (no Tauri runtime)

## Browser Fallback Pattern

```typescript
// frontend/src/lib/tauri.ts
export async function invokeSafe<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  try {
    if (window.__TAURI__) {
      return await invoke<T>(cmd, args);
    }
    return null; // Browser mode: no Tauri available
  } catch {
    return null;
  }
}
```

This project supports both browser mode (Vite dev server) and desktop mode (Tauri window).

## Event System

### Emit from Rust

```rust
use tauri::Emitter;

app_handle.emit("entity-updated", &entity_data)?;
```

### Listen in Frontend

```typescript
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<EntityUpdate>('entity-updated', (event) => {
  updateEntityList(event.payload);
});

// Cleanup
onCleanup(() => unlisten());
```

## Window Management

```rust
use tauri::Manager;

// Create new window
let _window = tauri::WebviewWindowBuilder::new(
    &app,
    "space-viewer",
    tauri::WebviewUrl::App("space-viewer".into()),
)
.title("Space Viewer")
.inner_size(1024.0, 768.0)
.build()?;
```

## File System Access

```typescript
import { open, save } from '@tauri-apps/plugin-dialog';
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';

// Open file dialog
const path = await open({
  filters: [{ name: 'JSON', extensions: ['json'] }],
});

if (path) {
  const content = await readTextFile(path);
  // Process content
}
```

## Security Best Practices

1. **Allowlist commands** — only expose necessary Rust commands
2. **Validate inputs** — validate all IPC arguments in Rust
3. **Scope file access** — use Tauri's fs scope to restrict file access
4. **CSP headers** — configure Content Security Policy
5. **No eval** — never use `eval()` or `Function()` constructor
6. **Update dependencies** — keep Tauri and plugins current

## Build & Distribution

```bash
# Development
cargo tauri dev

# Production build
cargo tauri build

# Output: platform-specific installer
# Windows: .msi or .exe
# macOS: .dmg or .app
# Linux: .deb, .rpm, or .AppImage
```

## Project Context

This project uses Tauri 2 with:
- Frontend: React + Vite + TypeScript + Tailwind
- Backend: Rust with admin-api crate
- Key Tauri commands in `src-tauri/src/`:
  - `drafts.rs` — chain editor draft persistence
  - `content.rs` — content engine persistence
  - `main.rs` — command registration and state setup
  - `state.rs` — shared application state

The app supports both browser mode (for development) and Tauri desktop mode (for production).
