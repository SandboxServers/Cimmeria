---
title: "Cimmeria Admin API"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Cimmeria Admin API

REST and WebSocket API for administering the Cimmeria server emulator. Built with Axum in the `crates/admin-api/` crate, consumed by the Tauri-based ServerEd desktop app (`frontend/` + `src-tauri/`).

## Architecture

```
┌──────────────────────────────────────────────────────┐
│       Tauri Desktop App (React + Vite)               │
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
    "cell_port": 50000,
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

**Not implemented — the roster is always empty.** The handler
(`crates/admin-api/src/routes/players.rs:62-89`) unconditionally returns
`available: false` with `reason: "Live player roster is not implemented yet."`
and an empty `players` array. The `summary` and `services` blocks *are*
real — they reflect actual service-running state and a live DB health check.

**Response (what it actually returns):**
```json
{
  "status": "ok",
  "available": false,
  "reason": "Live player roster is not implemented yet.",
  "players": [],
  "summary": {
    "online_count": 0,
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

The plumbing to make this live already exists on the services side and is
simply not wired up: `OnlinePlayer` (`crates/services/src/base/mod.rs:73`),
`archetype_name()` (`crates/services/src/base/mod.rs:84`), and
`online_players()` (`crates/services/src/base/service.rs:90`). Nothing in
`admin-api` calls `online_players()` today. When it is wired, the per-player
fields are expected to be:

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

#### `GET /api/content/pickers`

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

**Frontend:** none — no fetcher wraps this route today. Intended to populate dropdown menus for trigger/condition/action node forms in a chain editor UI that has not been built. See [Chain Flow Editor](#chain-flow-editor--no-frontend-consumer-exists).

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

All WebSocket endpoints accept upgrade at the listed path. Two of the three are fully implemented.

| Path | Purpose | Status |
|------|---------|--------|
| `/ws/logs` | Server log output stream | **Live** — replays the ring buffer on connect, then forwards the live `broadcast` channel (`crates/admin-api/src/ws/log_stream.rs:22-70`) |
| `/ws/events` | Game event notifications | **Live** — replays the login-audit buffer, then streams live events (`crates/admin-api/src/ws/event_stream.rs:19-60`) |
| `/ws/entities` | Real-time entity property updates | Stub — accepts the upgrade and does nothing (`crates/admin-api/src/ws/entity_stream.rs:26-31`) |

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

### Chain Flow Editor — **no frontend consumer exists**

The backend half of the chain editor is real: the four Tauri IPC commands
below are registered in `src-tauri/src/main.rs:25-32`, the draft handlers
live in `src-tauri/src/drafts.rs`, and `GET /api/content/pickers` is a live
HTTP route. But **nothing in `frontend/src` or `tools/ContentEditor/ui/src`
calls any of them.** There is no `ChainFlowWorkbench` file anywhere in the
repo, and no `fetchContentEditorPickers` symbol — the exported fetchers in
`frontend/src/lib/admin-api.ts` are `fetchAdminConfig`, `fetchAdminStatus`,
`fetchPlayers`, `fetchSpaces`, `fetchContentSummary`, and the audit fetcher.

The intended wiring, once a UI is built:

```
GET /api/content/pickers                  ──▶ dropdown options for node forms
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

> **There is no authentication on any `/api` route.** `middleware.rs`
> contains only the CORS layer; the JWT auth middleware is a TODO comment
> block (`crates/admin-api/src/middleware.rs:19-28`) with no implementation.
> Combined with `allow_origin(Any)`, this means anything that can reach port
> 8443 can call `POST /api/config/start`, `POST /api/config/stop`, and the
> `/api/telemetry/upload-*` endpoints. Do not expose the admin port beyond
> localhost or a trusted network. The `/api/auth/*` login endpoints are
> themselves stubs and grant nothing.

---

## Implementation Status

| Endpoint | Status | Notes |
|----------|--------|-------|
| `GET /api/config` | Live | Reads from `ServerConfig` |
| `POST /api/config` | Stub | |
| `GET /api/config/status` | Live | Uptime + service health check |
| `GET /api/players` | Stub | Always returns `available: false` and an empty roster; `summary`/`services` blocks are real |
| `GET /api/players/{id}` | Stub | |
| `POST /api/players/{id}/kick` | Stub | |
| `GET /api/spaces` | Live | From `resources.worlds` table |
| `GET /api/spaces/{id}` | Live | From `resources.worlds` table |
| `POST /api/spaces` | Stub | |
| `GET /api/content` | Stub | Hardcoded category list |
| `GET /api/content/summary` | Live | Aggregate DB queries |
| `GET /api/content/pickers` | Live | Multi-table picker data |
| `GET /api/content/items` | Stub | |
| `GET /api/content/items/{id}` | Stub | |
| `GET /api/entities` | Stub | |
| `GET /api/entities/{id}` | Stub | |
| `POST /api/entities/{id}/property` | Stub | |
| `POST /api/auth/login` | Stub | |
| `POST /api/auth/logout` | Stub | |
| `GET /api/auth/me` | Stub | |
| `GET /ws/entities` | Stub | |
| `GET /ws/logs` | Live | Ring-buffer replay + live broadcast |
| `GET /ws/events` | Live | Login-audit replay + live stream |

### Routes not covered above

The following are registered and reachable but are not documented in the
endpoint reference above. Listed here so the table is not mistaken for the
complete surface:

| Route | Evidence |
|---|---|
| `POST /api/config/start` | `routes/config.rs:62` — live, calls `orchestrator.start_all()` |
| `POST /api/config/stop` | `routes/config.rs:63` — live |
| `POST /api/content/reload` | `routes/content.rs:56` |
| `GET`/`DELETE /api/editor/content/{id}` | `routes/editor.rs:136` |
| `GET`/`DELETE /api/editor/content/{scope_id}/{mission_id}` | `routes/editor.rs:137-140` |
| `POST /api/editor/content` | `routes/editor.rs:141` |
| `GET /api/editor/draft/{scope_id}` | `routes/editor.rs:142` |
| `GET /api/editor/draft/{scope_id}/{mission_id}` | `routes/editor.rs:143-146` |
| `POST /api/editor/draft` | `routes/editor.rs:147` |
| `GET /api/audit/logins` | `routes/audit.rs:52` — live |
| `POST /api/auth/dev-session` | `routes/dev_session.rs:155` |
| `POST /api/auth/dev-session/refresh` | `routes/dev_session.rs:156` |
| `POST /api/telemetry/upload-chunk` | `routes/telemetry/mod.rs:91-94` |
| `POST /api/telemetry/upload-bundle` | `routes/telemetry/mod.rs:95-98` |
| `GET /swagger-ui`, `GET /api-docs/openapi.json` | `crates/admin-api/src/lib.rs:117` |

Note that `/api/editor/*` is an **HTTP** chain-editor persistence surface —
the Tauri IPC commands below are a second, parallel path to the same job.

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
| `crates/services/src/base/mod.rs` | `OnlinePlayer` struct (line 73) + `archetype_name()` (line 84) |
| `crates/services/src/base/service.rs` | `online_players()` (line 90) — not yet called by admin-api |
| `crates/admin-api/src/routes/editor.rs` | HTTP chain-editor content + draft persistence |
| `crates/admin-api/src/routes/audit.rs` | `GET /api/audit/logins` |
| `crates/admin-api/src/routes/dev_session.rs` | Launcher dev-session token mint + refresh |
| `crates/admin-api/src/routes/telemetry/` | Launcher telemetry chunk + bundle ingest |
| `frontend/src/lib/admin-api.ts` | TypeScript API client + dashboard builders |
| `frontend/src/lib/view-models.ts` | UI utility functions |
| `frontend/src/lib/ws.ts` | WebSocket connection helper |
| `frontend/src/lib/tauri.ts` | Tauri IPC interop |
| `src-tauri/src/main.rs` | Tauri app entry + IPC command registration |
| `src-tauri/src/content.rs` | Chain editor persistence |
| `src-tauri/src/state.rs` | App state with orchestrator + DB pool |
