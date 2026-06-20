# config/ — Legacy C++ Service Configuration

**Deprecated / reference-only.** These are the **original C++ server's** XML
configuration files, retained for reference. **The Rust server does not read
them.** The Rust server configures itself entirely from environment variables —
see `config_from_env()` and the env-var table at the top of
[crates/server/src/main.rs](../../../crates/server/src/main.rs). (The
`load_config` function in `crates/common/src/config.rs` is a stub that logs
`load_config is a stub; returning default configuration` and ignores the path
you hand it.)

3 files.

## Files

| File | Service | Key Settings (as authored for the C++ server) |
|---|---|---|
| `AuthenticationService.config` | AuthenticationServer | `base_service_port` (13001, BaseApp comms), `logon_service_port` (8081, client-facing login), `db_connection_string`, `protocol_digest`, `log_level` |
| `BaseService.config` | BaseApp | `db_connection_string`, shard name/id/address/port (32832), grid + vision/AoI params, tick params, `console_port` (8989), `developer_mode` |
| `CellService.config` | CellApp | `cell_id`, BaseApp `baseapp_address`/`baseapp_port`, tick + idle-update params, `console_port` (8990), `developer_mode` |

## Rust server configuration (current)

The Rust server reads **bare** environment variable names — there is no
`CIMMERIA_` prefix on the core service config (that prefix is reserved for
telemetry/build variables). The full list and defaults live in the env-var
table at the top of [crates/server/src/main.rs](../../../crates/server/src/main.rs);
the variables it consumes are:

| Variable | Purpose |
|---|---|
| `AUTH_HOST` | Auth service bind address |
| `AUTH_PORT` | Auth service port (BaseApp connections) |
| `LOGON_PORT` | Auth HTTP port (SOAP client login) |
| `BASE_HOST` | BaseApp UDP bind address |
| `BASE_EXTERNAL` | BaseApp address advertised to game clients |
| `BASE_PORT` | BaseApp UDP port |
| `CELL_PORT` | CellApp port |
| `ADMIN_PORT` | Admin REST API port |
| `DB_URL` | PostgreSQL connection string |
| `PROTOCOL_DIGEST` | 32-char hex digest sent in the auth response |
| `DEVELOPER_MODE` | Enable relaxed auth / multi-login |

There is no `CIMMERIA_DB_CONNECTION_STRING`; the Rust equivalent is `DB_URL`.

## Historical notes (how the old C++ server used these files)

The following describe the *original* C++ server's behavior. They are recorded
here for archaeology only — the Rust server ignores these files, so none of
this is a live configuration step.

- **Default/template values** — the base configs shipped with values suitable
  for local development. For production or multi-machine setups the C++ server
  loaded `*.local` override files (gitignored) alongside the base configs.
- **Database connection** — the `db_connection_string` field in each config
  used test credentials by default and was overridden via a `.local` file.
- **Python console** — `BaseService.config` exposed `console_port` 8989 (and
  `CellService.config` 8990) for live Python debugging. The console server only
  started when `py_console_password` was set.
- **Developer mode** — the `developer_mode` flag enabled relaxed auth (multiple
  logins per account, max access level for all players, logging to the player,
  and TRACE-level logging).
