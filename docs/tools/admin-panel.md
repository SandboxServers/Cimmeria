# Cimmeria Admin Panel

A standalone web dashboard for administering the Cimmeria server emulator during development. Provides account management, player inspection, live server interaction via the Python console bridge, and database operations through a polished browser-based interface.

## Motivation

The emulator currently offers three admin interfaces, none of which are convenient for day-to-day development:

| Interface | Limitation |
|-----------|-----------|
| Python console (TCP ports 8989/8990) | Binary protocol, requires custom client, no GUI |
| ServerEd (Qt desktop app) | Heavyweight, must be built separately, limited to DB browsing |
| In-game GM commands | Requires a running game client and an authenticated character |

A web panel solves all of these: it's accessible from any browser, connects to both the database and live server processes, and provides a unified interface for every admin task.

## Architecture

Standalone web service that ships in `tools/admin/` alongside the emulator.

```
┌─────────────────────────────────────────┐
│           React SPA (Vite + Tailwind)   │
│         served as static files          │
└──────────────────┬──────────────────────┘
                   │ JSON API (HTTP)
┌──────────────────▼──────────────────────┐
│           Flask API Backend             │
│  ┌─────────────┐  ┌──────────────────┐  │
│  │  psycopg2   │  │  console_client  │  │
│  │  (direct DB)│  │  (TCP bridge)    │  │
│  └──────┬──────┘  └────────┬─────────┘  │
└─────────┼──────────────────┼────────────┘
          │                  │
    ┌─────▼─────┐    ┌───────▼───────┐
    │ PostgreSQL │    │ BaseApp:8989  │
    │  (sgw DB)  │    │ CellApp:8990  │
    └───────────┘    └───────────────┘
```

**Why standalone, not embedded in C++:** Boost 1.55 lacks Beast (HTTP library). The embedded Python is 3.4 (no modern async web frameworks). A standalone service is practical, ships in the repo, and connects to running services via the existing py_client TCP protocol and direct PostgreSQL queries.

## Tech Stack

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Frontend | React 19 + Vite + TypeScript | Largest ecosystem, fast dev server |
| Styling | Tailwind CSS 4 | Utility-first, polished look with minimal CSS |
| API | Flask 3.x + flask-cors | Minimal Python backend, JSON-only API |
| Database | psycopg2-binary | Direct PostgreSQL access (same DB as game servers) |
| Server bridge | Custom TCP client | Implements the py_client binary protocol |

## File Structure

```
tools/admin/
├── README.md
├── backend/
│   ├── requirements.txt          # flask, flask-cors, psycopg2-binary
│   ├── app.py                    # Flask entry, CORS config, static file serving
│   ├── config.py                 # DB/port/password config (env var overrides)
│   ├── db.py                     # psycopg2 connection pool helper
│   ├── console_client.py         # py_client TCP protocol implementation
│   └── routes/
│       ├── auth.py               # POST /api/login, /api/logout, GET /api/me
│       ├── dashboard.py          # GET /api/dashboard (counts, server status)
│       ├── accounts.py           # CRUD /api/accounts
│       ├── characters.py         # GET /api/characters, /api/characters/:id
│       ├── console.py            # POST /api/console/eval, /api/console/exec
│       ├── chat.py               # GET/POST /api/chat
│       └── query.py              # POST /api/query (raw SQL)
└── frontend/
    ├── package.json
    ├── vite.config.ts            # Proxy /api to Flask dev server
    ├── index.html
    ├── src/
    │   ├── main.tsx
    │   ├── App.tsx               # Router + layout shell
    │   ├── api/
    │   │   └── client.ts         # Fetch wrapper with auth handling
    │   ├── components/
    │   │   ├── Layout.tsx        # Sidebar nav, header, content area
    │   │   ├── DataTable.tsx     # Sortable/filterable table
    │   │   ├── StatusBadge.tsx   # Online/offline indicators
    │   │   ├── Modal.tsx         # Dialog component
    │   │   └── Terminal.tsx      # Console REPL component
    │   ├── pages/
    │   │   ├── Login.tsx
    │   │   ├── Dashboard.tsx     # Server status + aggregate counts
    │   │   ├── Accounts.tsx      # Account list + create/edit
    │   │   ├── Characters.tsx    # Character browser with search
    │   │   ├── CharacterDetail.tsx
    │   │   ├── Console.tsx       # Python REPL interface
    │   │   ├── Chat.tsx          # Chat channel viewer/sender
    │   │   └── Query.tsx         # SQL query tool
    │   └── hooks/
    │       ├── useAuth.ts        # Auth context/provider
    │       └── usePolling.ts     # Interval-based data refresh
    └── tailwind.config.js
```

## Authentication

The admin panel reuses the game's `account` table. Only accounts with `accesslevel >= 1` can log in.

### Account Table Schema

```sql
-- db/sgw/Accounts/Tables/account.sql
CREATE TABLE account (
    account_id   integer NOT NULL DEFAULT nextval('accounts_account_id_seq'),
    account_name character varying(32) NOT NULL,
    password     character varying(64) NOT NULL,   -- upper(sha1(plaintext))
    accesslevel  integer DEFAULT 0 NOT NULL,
    enabled      boolean DEFAULT true NOT NULL
);
```

### Password Verification

Matches the authentication server's logic in `src/authentication/logon_queue.cpp:27-50`:

```python
import hashlib

password_hash = hashlib.sha1(password.encode()).hexdigest().upper()

# Query: SELECT account_id, upper(password) as password, accesslevel, enabled
#         FROM account WHERE account_name = %s
# Compare: password_hash == row['password']
```

The auth server validates the password as a 40-character uppercase hex SHA-1 hash. The web panel computes this client-side (or server-side in the Flask backend) and compares against the stored value.

### Username Validation

From `logon_queue.cpp:48-50`:
- Length: 3-20 characters
- Allowed characters: `[a-zA-Z0-9_-]`

### Session Management

Flask session cookie. No separate admin user table. The session stores `account_id`, `account_name`, and `accesslevel` after successful login.

### Access Levels

| Level | Role | Panel Access |
|-------|------|-------------|
| 0 | Player | No panel access |
| 1+ | GM/Admin | Full panel access |

## Console TCP Bridge

The `console_client.py` module implements the py_client binary protocol, enabling the web panel to execute Python commands on running BaseApp and CellApp instances.

### Wire Format

From `src/mercury/unified_connection.hpp:675-683` and `unified_connection.cpp:102-136`:

```
Message frame:
┌──────────────────┬────────────┬──────────────┐
│ uint32_le length │ uint8 id   │ payload ...  │
│ (total frame sz) │ (msg type) │              │
└──────────────────┴────────────┴──────────────┘

length = sizeof(uint32) + sizeof(uint8) + sizeof(payload) = 5 + len(payload)

Strings within payload:
┌──────────────────────┬─────────────────┐
│ uint32_le byte_count │ utf-8 bytes ... │
└──────────────────────┴─────────────────┘
```

The `MessageHeader` struct is packed (`#pragma pack(push, 1)` on MSVC, `__attribute__((packed))` on GCC):
```c
struct MessageHeader {
    uint32_t length;     // total frame size including this header
    MessageId messageId; // uint8_t command code
};
```

### Protocol Messages

From `src/entity/py_client.hpp:6-14`:

| ID | Name | Direction | Payload |
|----|------|-----------|---------|
| `0x01` | `REQ_AUTHENTICATE` | Client -> Server | `string(password)` |
| `0x02` | `ACK_AUTHENTICATE` | Server -> Client | `uint32(result)` — 0 = success, 1 = failure |
| `0x03` | `REQ_EVALUATE` | Client -> Server | `string(expression)` |
| `0x04` | `ACK_EVALUATE` | Server -> Client | `uint8(type) + value` |
| `0x05` | `REQ_EXECUTE` | Client -> Server | `string(statement)` |
| `0x06` | `ACK_EXECUTE` | Server -> Client | `uint8(type) + value` |

### Response Value Types

From `src/entity/py_client.hpp:22-28`:

| Type ID | Name | Payload |
|---------|------|---------|
| 0 | `PY_EXEC_NONE` | (no additional data) |
| 1 | `PY_EXEC_EXCEPTION` | `string(error_message)` |
| 2 | `PY_EXEC_INTEGER` | `int32_le(value)` |
| 3 | `PY_EXEC_FLOAT` | `float32_le(value)` |
| 4 | `PY_EXEC_STRING` | `string(value)` |

### Evaluate vs Execute

From `src/entity/py_client.cpp`:

- **Evaluate** (`REQ_EVALUATE`, 0x03): Calls `bp::eval()` — evaluates a Python *expression* and returns the result. Supports bool, int, float, str, and NoneType return values. Unknown types return `PY_EXEC_NONE`.
- **Execute** (`REQ_EXECUTE`, 0x05): Calls `bp::exec()` — executes a Python *statement*. Always returns `PY_EXEC_NONE` on success (statements don't produce values).

Both require prior authentication. Unauthenticated requests cause the server to close the connection (`py_client.cpp:55-60`).

### Connection Strategy

Open-close per request for simplicity. Each API call to `/api/console/eval` or `/api/console/exec`:
1. Opens a TCP socket to the target service
2. Sends `REQ_AUTHENTICATE` with the configured password
3. Reads `ACK_AUTHENTICATE`, verifies `result == 0`
4. Sends the eval/exec request
5. Reads the response
6. Closes the socket

### Console Port Configuration

| Service | Config File | Config Key | Default Port |
|---------|------------|------------|-------------|
| BaseApp | `config/BaseService.config` | `<console_port>` | 8989 |
| CellApp | `config/CellService.config` | `<console_port>` | 8990 |

The console server only starts if `<py_console_password>` is set to a non-empty value.

## Database Queries

### Dashboard

```sql
SELECT count(*) FROM account;
SELECT count(*) FROM account WHERE enabled = true;
SELECT count(*) FROM sgw_player;
```

Server status is determined by TCP port probes to each service's known port.

### Account CRUD

**List accounts:**
```sql
SELECT account_id, account_name, accesslevel, enabled FROM account
ORDER BY account_id;
```

**Create account** (validation matches `logon_queue.cpp:43-50`):
```python
# Username: 3-20 chars, [a-zA-Z0-9_-]
# Password: stored as upper(sha1(plaintext))
password_hash = hashlib.sha1(password.encode()).hexdigest().upper()
```
```sql
INSERT INTO account (account_name, password, accesslevel, enabled)
VALUES (%s, %s, %s, true);
```

**Update account:**
```sql
UPDATE account SET accesslevel = %s, enabled = %s WHERE account_id = %s;
```

**Reset password:**
```sql
UPDATE account SET password = %s WHERE account_id = %s;
```

### Character Browser

**List characters:**
```sql
SELECT p.player_id, p.player_name, p.level, p.archetype, p.alignment,
       p.world_location, a.account_name
FROM sgw_player p
JOIN account a ON p.account_id = a.account_id
ORDER BY p.player_id;
```

**Character detail:**
```sql
-- Character info
SELECT p.*, a.account_name
FROM sgw_player p
JOIN account a ON p.account_id = a.account_id
WHERE p.player_id = %s;
```

### Character Detail Sub-Queries

**Inventory:**
```sql
SELECT i.item_id, i.type_id, i.stack_size, i.charges, i.container_id,
       i.slot_id, i.durability, i.flags, i.bound, i.ammo,
       ri.name AS item_name
FROM sgw_inventory i
LEFT JOIN resources.items ri ON i.type_id = ri.item_id
WHERE i.character_id = %s
ORDER BY i.container_id, i.slot_id;
```

**Missions:**
```sql
SELECT m.mission_id, m.current_step_id, m.status, m.repeats,
       rm.name AS mission_name
FROM sgw_mission m
LEFT JOIN resources.missions rm ON m.mission_id = rm.mission_id
WHERE m.player_id = %s;
```

### Character Table Schema Reference

```sql
-- db/sgw/Players/Tables/sgw_player.sql
CREATE TABLE sgw_player (
    account_id            integer NOT NULL,
    player_id             integer NOT NULL DEFAULT nextval('sgw_characters_character_id_seq'),
    level                 integer DEFAULT 1 NOT NULL,     -- CHECK 0-20
    alignment             integer NOT NULL,               -- CHECK 0-5
    archetype             integer NOT NULL,               -- CHECK 0-8
    gender                integer NOT NULL,               -- CHECK 1-3
    player_name           varchar(64) NOT NULL,
    extra_name            varchar(64) NOT NULL,
    world_location        varchar(64) NOT NULL,
    bodyset               varchar(64) NOT NULL,
    title                 integer DEFAULT 0 NOT NULL,
    pos_x                 real NOT NULL,
    pos_y                 real NOT NULL,
    pos_z                 real NOT NULL,
    heading               smallint DEFAULT 0 NOT NULL,
    naquadah              integer DEFAULT 0 NOT NULL,
    exp                   integer DEFAULT 0 NOT NULL,
    first_login           integer DEFAULT 1 NOT NULL,
    world_id              integer,
    known_stargates       integer[] DEFAULT '{}',
    components            varchar(200)[] DEFAULT '{}',
    abilities             integer[] DEFAULT '{}',
    access_level          integer DEFAULT 0 NOT NULL,
    skin_color_id         integer NOT NULL,               -- CHECK 0-15
    bandolier_slot        integer DEFAULT 0 NOT NULL,     -- CHECK 0-3
    interaction_maps      player_interaction_map[],
    training_points       integer DEFAULT 0 NOT NULL,
    discipline_ids        integer[] DEFAULT '{}',
    racial_paradigm_levels integer[] DEFAULT '{}',
    applied_science_points integer DEFAULT 0 NOT NULL,
    blueprint_ids         integer[] DEFAULT '{}',
    known_respawners      integer[] DEFAULT '{}'
);
```

### Inventory Table Schema Reference

```sql
-- db/sgw/Inventory/Tables/sgw_inventory_base.sql (parent)
CREATE TABLE sgw_inventory_base (
    item_id      integer NOT NULL DEFAULT nextval('sgw_inventory_base_item_id_seq'),
    stack_size   integer DEFAULT 1 NOT NULL,
    charges      integer DEFAULT 0 NOT NULL,
    container_id integer NOT NULL,
    slot_id      integer NOT NULL,
    durability   integer DEFAULT -1 NOT NULL,
    type_id      integer NOT NULL,
    flags        integer DEFAULT 0 NOT NULL,
    ammo_type    resources."EAmmoType" DEFAULT 'AMMO_NONE',
    ammo_types   resources."EAmmoType"[] DEFAULT '{}'
);

-- db/sgw/Inventory/Tables/sgw_inventory.sql (inherits sgw_inventory_base)
CREATE TABLE sgw_inventory (
    character_id integer NOT NULL,
    bound        boolean DEFAULT false NOT NULL,
    ammo         integer DEFAULT 0 NOT NULL,
    CONSTRAINT local_id_check CHECK (item_id >= 10000)
) INHERITS (sgw_inventory_base);
```

### Mission Table Schema Reference

```sql
-- db/sgw/Missions/Tables/sgw_mission.sql
CREATE TABLE sgw_mission (
    player_id              integer NOT NULL,
    mission_id             integer NOT NULL,
    current_step_id        integer,
    status                 integer NOT NULL,
    completed_step_ids     integer[] NOT NULL,
    completed_objective_ids integer[] NOT NULL,
    active_objective_ids   integer[] NOT NULL,
    failed_objective_ids   integer[] NOT NULL,
    repeats                integer DEFAULT 0 NOT NULL
);
```

## Configuration

All config values have defaults matching the emulator's existing config files and are overridable via environment variables.

| Setting | Env Var | Default | Source |
|---------|---------|---------|--------|
| DB host | `CIMMERIA_DB_HOST` | `localhost` | `BaseService.config` |
| DB port | `CIMMERIA_DB_PORT` | `5433` | `BaseService.config` |
| DB name | `CIMMERIA_DB_NAME` | `sgw` | `BaseService.config` |
| DB user | `CIMMERIA_DB_USER` | `w-testing` | `BaseService.config` |
| DB password | `CIMMERIA_DB_PASS` | `w-testing` | `BaseService.config` |
| BaseApp console host | `CIMMERIA_BASE_HOST` | `127.0.0.1` | `BaseService.config` |
| BaseApp console port | `CIMMERIA_BASE_CONSOLE_PORT` | `8989` | `BaseService.config:88` |
| CellApp console host | `CIMMERIA_CELL_HOST` | `127.0.0.1` | `CellService.config` |
| CellApp console port | `CIMMERIA_CELL_CONSOLE_PORT` | `8990` | `CellService.config:33` |
| Console password | `CIMMERIA_CONSOLE_PASSWORD` | (empty) | `py_console_password` |
| Auth server port | `CIMMERIA_AUTH_PORT` | `8081` | `AuthenticationService.config:9` |
| BaseApp game port | `CIMMERIA_BASE_PORT` | `32832` | `BaseService.config:10` |
| Flask secret key | `FLASK_SECRET_KEY` | (random) | — |
| Flask port | `ADMIN_PORT` | `5000` | — |

## API Routes

### Authentication

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/login` | Authenticate with account credentials |
| `POST` | `/api/logout` | Clear session |
| `GET` | `/api/me` | Return current session info |

### Dashboard

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/dashboard` | Aggregate counts + service status probes |

Response:
```json
{
  "accounts": { "total": 42, "enabled": 38 },
  "characters": { "total": 156 },
  "services": {
    "auth":    { "host": "127.0.0.1", "port": 8081,  "online": true },
    "base":    { "host": "127.0.0.1", "port": 32832, "online": true },
    "console": { "host": "127.0.0.1", "port": 8989,  "online": false }
  }
}
```

### Accounts

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/accounts` | List all accounts |
| `POST` | `/api/accounts` | Create new account |
| `PUT` | `/api/accounts/:id` | Update access level / enabled |
| `PUT` | `/api/accounts/:id/password` | Reset password |

### Characters

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/characters` | List characters (with search) |
| `GET` | `/api/characters/:id` | Character detail (stats, inventory, missions) |

### Console Bridge

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/console/eval` | Evaluate Python expression on BaseApp/CellApp |
| `POST` | `/api/console/exec` | Execute Python statement on BaseApp/CellApp |

Request body:
```json
{
  "target": "base",
  "code": "Atrea.getGameTime()"
}
```

Response:
```json
{
  "type": "string",
  "value": "14:30:00"
}
```

### Chat

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/chat` | Poll recent messages via console bridge |
| `POST` | `/api/chat` | Send message via console bridge |

### Raw SQL

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/query` | Execute arbitrary SQL (read-only by default) |

Request body:
```json
{
  "sql": "SELECT * FROM account LIMIT 10",
  "write": false
}
```

## Implementation Phases

### Phase 1: Foundation + Core Pages

**Backend:**
1. Flask scaffolding: `app.py`, `config.py`, `db.py`, `requirements.txt`
2. Auth routes: login/logout/me using account table + SHA-1 verification
3. Dashboard route: aggregate counts + TCP port probes for service status
4. Account CRUD routes: list, create, edit, reset password
5. Character routes: list with search, detail view with inventory + missions

**Frontend:**
6. Vite + React + Tailwind + React Router scaffolding
7. Login page with session management
8. Dashboard page with status badges
9. Accounts page with create/edit modal
10. Characters page with search, detail page with tabbed inventory/missions
11. Reusable components: Layout, DataTable, StatusBadge, Modal

### Phase 2: Live Server Interaction

1. `console_client.py`: Full TCP protocol implementation (auth, eval, exec)
2. Console API routes: `/api/console/eval`, `/api/console/exec`
3. Console page: Terminal component with REPL (command history, output formatting)
4. Chat routes: poll messages + send via console bridge
5. Chat page: channel viewer, message input

### Phase 3: Power Tools

1. Raw SQL query tool with read-only default and explicit write toggle
2. GM command palette: curated UI buttons for common commands (parsed from `python/cell/ConsoleCommands.py` — 66 commands with access levels)
3. Character editing: modify level, position, naquadah, experience, abilities
4. Enhanced dashboard: live player list via console bridge

## Prerequisites

| Requirement | Notes |
|-------------|-------|
| PostgreSQL | Running on port 5433 with `sgw` database populated |
| Node.js + npm | Frontend build toolchain |
| Python 3.10+ | Backend runtime (system Python, not the embedded 3.4) |
| `py_console_password` set | Required for Phase 2+ console bridge features |

## Verification Checklist

1. **Backend API**: `curl http://localhost:5000/api/dashboard` returns JSON with counts
2. **Account creation**: Create account via UI, verify login works with game client
3. **Console bridge**: Open Console page, run `Atrea.getGameTime()`, verify response
4. **Character browsing**: View character list, click through to detail with inventory and missions
5. **Server status**: Dashboard shows green/red indicators matching actual service state
6. **Full E2E**: Register account via web panel -> log in with game client -> use GM commands via web console

## Source File References

| File | Relevance |
|------|-----------|
| `src/entity/py_client.hpp` | Protocol message IDs and response type enums |
| `src/entity/py_client.cpp` | Protocol handler: auth, eval, exec logic |
| `src/mercury/unified_connection.hpp:503-683` | Wire format: MessageId, Writer, Reader, MessageHeader |
| `src/mercury/unified_connection.cpp:102-136` | Frame serialization (beginMessage/endMessage) |
| `src/authentication/logon_queue.cpp:27-50` | Password hash validation + username rules |
| `db/sgw/Accounts/Tables/account.sql` | Account table schema |
| `db/sgw/Players/Tables/sgw_player.sql` | Character table schema |
| `db/sgw/Inventory/Tables/sgw_inventory.sql` | Inventory schema (inherits sgw_inventory_base) |
| `db/sgw/Missions/Tables/sgw_mission.sql` | Mission progress schema |
| `python/cell/ConsoleCommands.py` | 66 GM commands with access levels |
| `python/base/Chat.py` | Chat channel manager |
| `python/base/Account.py` | Character creation/login flow |
| `config/BaseService.config` | Default ports, DB connection, console password |
| `config/CellService.config` | CellApp console port, DB connection |
| `config/AuthenticationService.config` | Auth server ports |
