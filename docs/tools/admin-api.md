# Cimmeria Admin API

REST and WebSocket API for administering the Cimmeria server emulator. Built with Axum in the `crates/admin-api/` crate, consumed by the Tauri-based ServerEd desktop app (`frontend/` + `src-tauri/`).

## Architecture

```
┌──────────────────────────────────────────────────────┐
│       Tauri Desktop App (SolidJS + Vite)             │
│                                                      │
│   frontend/src/lib/admin-api.ts   (REST fetch)       │
│   frontend/src/lib/ws.ts          (WebSocket)        │
│   src-tauri/src/content.rs        (Tauri IPC)        │
│   src-tauri/src/ipc.rs            (Tauri IPC)        │
└────────────────┬────────────────────┬────────────────┘
                 │ HTTP/WS :8443      │ Tauri IPC
┌────────────────▼─────────┐  ┌───────▼────────────────┐
│   cimmeria-admin-api     │  │    src-tauri backend    │
│   (Axum, port 8443)      │  │    (direct DB access)   │
│                          │  │                         │
│   Arc<Orchestrator>      │  │  Arc<Orchestrator>      │
└────────────┬─────────────┘  └───────┬────────────────┘
             │                        │
     ┌───────▼────────────────────────▼──────┐
     │          Orchestrator                  │
     │    ┌──────────┬──────────┬──────────┐  │
     │    │AuthService│BaseService│CellService│ │
     │    └──────────┴──────────┴──────────┘  │
     │    DatabasePool (sqlx/PostgreSQL)       │
     └────────────────────────────────────────┘
```

The admin API runs as a background Axum server on port 8443 within the same process as the game server. It accesses game state through a shared `Arc<Orchestrator>` reference — no inter-process communication needed.

The Tauri desktop app has **two** paths to the backend:
1. **HTTP fetch** via `admin-api.ts` — calls the Axum REST endpoints (same as any browser client)
2. **Tauri IPC** via `src-tauri/` — direct Rust function calls for chain editor persistence and server status

## Base URL

| Context | Origin |
|---------|--------|
| Default | `http://127.0.0.1:8443` |
| Browser | `{window.location.protocol}//{window.location.hostname}:8443` |
| Override | `VITE_ADMIN_API_ORIGIN` environment variable |

All REST endpoints are prefixed with `/api/`. WebSocket endpoints are prefixed with `/ws/`.

## REST Endpoints

### Configuration

#### `GET /api/config`

Returns current server configuration.

**Response:**
```json
{
  "status": "ok",
  "config": {
    "auth_host": "0.0.0.0",
    "auth_port": 13001,
    "base_host": "0.0.0.0",
    "base_port": 32832,
    "cell_host": "0.0.0.0",
    "cell_port": 32833,
    "admin_port": 8443,
    "developer_mode": true
  }
}
```

**Frontend:** `fetchAdminConfig()` → `AdminConfigResponse`
**Page:** Config.tsx — displays settings grouped into Network and Runtime sections via `buildConfigSections()`

#### `POST /api/config`

Update server configuration. **Not implemented.**

#### `GET /api/config/status`

Returns server uptime and per-service health.

**Response:**
```json
{
  "status": "ok",
  "uptime_seconds": 3742,
  "services": {
    "auth": true,
    "base": true,
    "cell": true,
    "database": true
  }
}
```

**Frontend:** `fetchAdminStatus()` → `AdminStatusResponse`
**Page:** Dashboard.tsx — uptime stat, service health cards via `buildServiceHealth()`

---

### Players

#### `GET /api/players`

Lists all currently online players. Returns live data from BaseService's connected client map, filtered to players who have completed world entry.

**Response:**
```json
{
  "status": "ok",
  "available": true,
  "reason": null,
  "players": [
    {
      "id": 2,
      "name": "TestChar",
      "archetype": "Soldier",
      "level": 5,
      "zone": "Agnos",
      "ping": null,
      "status": "in_world",
      "session": "127.0.0.1:54321"
    }
  ],
  "summary": {
    "online_count": 1,
    "ready": true
  },
  "services": {
    "auth": true,
    "base": true,
    "cell": true,
    "database": true
  }
}
```

| Field | Source |
|-------|--------|
| `id` | `player_entity_id` from world entry |
| `name` | `player_name` from character DB query |
| `archetype` | `player_archetype` mapped via `archetype_name()` (Soldier, Commando, Scientist, etc.) |
| `level` | `player_level` from character DB query |
| `zone` | `world_name` set during world entry |
| `ping` | Always `null` (not yet implemented) |
| `status` | `"loading"` if pending world entry phase B, else `"in_world"` |
| `session` | Socket address string |
| `summary.ready` | `true` when both base and cell services are running |

**Frontend:** `fetchPlayers()` → `PlayersResponse`
**Page:** Players.tsx — searchable table with Name, Archetype, Level, Zone, Status, Ping, Session columns

#### `GET /api/players/{id}`

Get player details. **Not implemented** — returns stub with player_id.

#### `POST /api/players/{id}/kick`

Kick a player. **Not implemented** — returns stub with player_id.

---

### Spaces

#### `GET /api/spaces`

Lists all game worlds from the `resources.worlds` database table with mission counts.

**Response:**
```json
{
  "status": "ok",
  "available": true,
  "reason": null,
  "spaces": [
    {
      "world_id": 1,
      "world": "Agnos",
      "client_map": "Agnos",
      "has_script": true,
      "flags": 0,
      "mission_count": 12
    }
  ],
  "summary": {
    "total_spaces": 24,
    "scripted_spaces": 8,
    "mission_links": 156
  }
}
```

**Frontend:** `fetchSpaces()` → `SpacesResponse`
**Page:** SpaceViewer.tsx — space browser, Dashboard.tsx — space count stat via `buildDashboardStats()`

#### `GET /api/spaces/{id}`

Returns a single space record by `world_id`.

**Response:**
```json
{
  "status": "ok",
  "available": true,
  "space_id": 1,
  "space": { "world_id": 1, "world": "Agnos", "..." : "..." }
}
```

#### `POST /api/spaces`

Create a new space instance. **Not implemented.**

---

### Content

#### `GET /api/content`

List content categories. Returns a hardcoded list of category names. Placeholder for future browsing.

#### `GET /api/content/summary`

High-level content counts for the dashboard.

**Response:**
```json
{
  "status": "ok",
  "available": true,
  "reason": null,
  "summary": {
    "world_count": 24,
    "scripted_world_count": 8,
    "mission_count": 156,
    "story_mission_count": 42,
    "hidden_mission_count": 11,
    "scripted_mission_count": 63
  },
  "top_space_mission_counts": [
    { "scope": "Agnos", "mission_count": 18 },
    { "scope": "Castle_CellBlock", "mission_count": 5 }
  ]
}
```

**Frontend:** `fetchContentSummary()` → `ContentSummaryResponse`
**Page:** Dashboard.tsx — mission count stat and activity feed via `buildDashboardStats()` and `buildDashboardActivity()`

#### `GET /api/content/editor-pickers`

Loads all dropdown options for the chain flow editor. Queries multiple `resources.*` tables.

**Response:**
```json
{
  "status": "ok",
  "available": true,
  "reason": null,
  "spaces": [{ "value": "Agnos", "label": "Agnos" }],
  "missions": [{ "value": "622", "label": "622 - Arm Yourself", "space_id": "Castle_CellBlock" }],
  "dialogs": [{ "value": "2982", "label": "2982 - Intro Monologue" }],
  "items": [{ "value": "55", "label": "55 - Mk1 Intar" }],
  "regions": [{ "value": "Agnos.town_center", "label": "Agnos.town_center", "space_id": "Agnos" }],
  "steps": [{ "value": "2113", "label": "2113 - Obtain a weapon", "mission_id": "622" }]
}
```

**Frontend:** `fetchContentEditorPickers()` → `ContentEditorPickersResponse`
**Page:** ChainFlowWorkbench.react.tsx — populates dropdown menus for trigger/condition/action node forms

#### `GET /api/content/items`

List all items. **Not implemented.**

#### `GET /api/content/items/{id}`

Get item by ID. **Not implemented.**

---

### Entities

#### `GET /api/entities`

List active entities. **Not implemented.**

#### `GET /api/entities/{id}`

Get entity details. **Not implemented.**

#### `POST /api/entities/{id}/property`

Set entity property. **Not implemented.**

---

### Auth

#### `POST /api/auth/login`

Admin panel authentication. **Not implemented.**

#### `POST /api/auth/logout`

Invalidate session. **Not implemented.**

#### `GET /api/auth/me`

Current session info. **Not implemented.**

---

## WebSocket Endpoints

All WebSocket endpoints accept upgrade at the listed path. None are implemented yet — handlers are stubs.

| Path | Purpose |
|------|---------|
| `/ws/entities` | Real-time entity property updates |
| `/ws/logs` | Server log output stream |
| `/ws/events` | Game event notifications |

**Frontend:** `connectWs(path, onMessage)` in `ws.ts` — creates WebSocket connection, parses JSON messages, returns cleanup function.

---

## Tauri IPC Commands

These are invoked directly by the frontend via `tauriInvoke()` when running inside the Tauri desktop app. They bypass the HTTP API.

| Command | Description |
|---------|-------------|
| `get_server_status` | Returns `"running"` |
| `get_player_count` | Returns `0` (stub) |
| `get_uptime` | Returns orchestrator uptime in seconds |
| `load_chain_editor_content` | Load persisted chain content for a space/mission scope |
| `save_chain_editor_content` | Persist chain editor nodes to database |
| `validate_chain_editor_content` | Validate chain structure without saving |
| `load_chain_editor_draft` | Load unsaved editor draft |
| `save_chain_editor_draft` | Save editor draft for later resumption |

The chain editor commands use a separate `content_*` table schema (auto-created on first use) in the same PostgreSQL database.

---

## Frontend Pages and Data Flow

### Dashboard (`Dashboard.tsx`)

Fetches four endpoints in parallel on mount:

```
fetchAdminStatus()  ──┐
fetchPlayers()      ──┼──▶ buildDashboardStats()  ──▶ stat cards
fetchSpaces()       ──┤    buildServiceHealth()   ──▶ health badges
fetchContentSummary()─┘    buildDashboardActivity()──▶ activity feed
```

### Players (`Players.tsx`)

```
fetchPlayers()  ──▶ filteredPlayers (search memo)  ──▶ sortable table
                    getPlayerStatusVariant()       ──▶ status badge color
```

### Spaces (`SpaceViewer.tsx`)

```
fetchSpaces()  ──▶ space list with mission counts
```

### Config (`Config.tsx`)

```
fetchAdminConfig()  ──▶ buildConfigSections()  ──▶ grouped setting display
fetchAdminStatus()  ──▶ uptime + service health
```

### Chain Flow Editor (`ChainFlowWorkbench.react.tsx`)

```
fetchContentEditorPickers()  ──▶ dropdown options for node forms
tauriInvoke('load_chain_editor_content')  ──▶ restore saved chains
tauriInvoke('save_chain_editor_content')  ──▶ persist to DB
tauriInvoke('load_chain_editor_draft')    ──▶ restore WIP state
tauriInvoke('save_chain_editor_draft')    ──▶ auto-save WIP state
```

---

## Availability Pattern

All database-backed endpoints follow a consistent availability pattern:

```json
{
  "status": "ok",
  "available": false,
  "reason": "Database unavailable.",
  "data_field": []
}
```

When the database pool is `None` or a query fails, `available` is `false` and `reason` explains why. The frontend checks `available` before rendering data and falls back to the `reason` string. This means the dashboard and pages work in degraded mode even when the database is down.

---

## CORS

The middleware (`crates/admin-api/src/middleware.rs`) is configured permissively for development: any origin, any method, any headers. This allows the Tauri webview (`tauri://localhost`) and browser dev server (`http://localhost:5173`) to reach the API.

---

## Implementation Status

| Endpoint | Status | Notes |
|----------|--------|-------|
| `GET /api/config` | Live | Reads from `ServerConfig` |
| `POST /api/config` | Stub | |
| `GET /api/config/status` | Live | Uptime + service health check |
| `GET /api/players` | Live | Real-time from connected clients |
| `GET /api/players/{id}` | Stub | |
| `POST /api/players/{id}/kick` | Stub | |
| `GET /api/spaces` | Live | From `resources.worlds` table |
| `GET /api/spaces/{id}` | Live | From `resources.worlds` table |
| `POST /api/spaces` | Stub | |
| `GET /api/content` | Stub | Hardcoded category list |
| `GET /api/content/summary` | Live | Aggregate DB queries |
| `GET /api/content/editor-pickers` | Live | Multi-table picker data |
| `GET /api/content/items` | Stub | |
| `GET /api/content/items/{id}` | Stub | |
| `GET /api/entities` | Stub | |
| `GET /api/entities/{id}` | Stub | |
| `POST /api/entities/{id}/property` | Stub | |
| `POST /api/auth/login` | Stub | |
| `POST /api/auth/logout` | Stub | |
| `GET /api/auth/me` | Stub | |
| `GET /ws/entities` | Stub | |
| `GET /ws/logs` | Stub | |
| `GET /ws/events` | Stub | |

**7 live endpoints**, 16 stubs, 3 WebSocket stubs.

---

## Source Files

| File | Role |
|------|------|
| `crates/admin-api/src/lib.rs` | Router builder, mounts `/api` and `/ws` |
| `crates/admin-api/src/routes/mod.rs` | Route aggregator |
| `crates/admin-api/src/routes/config.rs` | Config + status endpoints |
| `crates/admin-api/src/routes/players.rs` | Player roster endpoint |
| `crates/admin-api/src/routes/spaces.rs` | Space listing endpoints |
| `crates/admin-api/src/routes/content.rs` | Content summary + editor pickers |
| `crates/admin-api/src/routes/entities.rs` | Entity inspection stubs |
| `crates/admin-api/src/routes/auth.rs` | Auth stubs |
| `crates/admin-api/src/ws/mod.rs` | WebSocket route aggregator |
| `crates/admin-api/src/ws/*.rs` | WebSocket stream stubs |
| `crates/admin-api/src/middleware.rs` | CORS configuration |
| `crates/services/src/orchestrator.rs` | Shared state provider |
| `crates/services/src/base/mod.rs` | `OnlinePlayer` struct + `online_players()` |
| `frontend/src/lib/admin-api.ts` | TypeScript API client + dashboard builders |
| `frontend/src/lib/view-models.ts` | UI utility functions |
| `frontend/src/lib/ws.ts` | WebSocket connection helper |
| `frontend/src/lib/tauri.ts` | Tauri IPC interop |
| `src-tauri/src/main.rs` | Tauri app entry + IPC command registration |
| `src-tauri/src/content.rs` | Chain editor persistence |
| `src-tauri/src/state.rs` | App state with orchestrator + DB pool |
