---
name: db-fail-closed-startup
description: Database fail-closed contract (2026-10-10) — configured DB_URL connect failure is fatal; the account-1/access-99 developer login needs developer_mode AND an empty DB_URL; container checks DB login with psql before the server starts
metadata:
  type: project
---

# Database fail-closed startup (2026-10-10)

- `ServerConfig::database_configured()` = `db_connection_string` not empty/whitespace.
  Defaults and `loopback()` are configured, so a plain test/dev config fails closed.
- `crates/services/src/orchestrator_database.rs::connect_database`: configured +
  connect error or timeout (30 s, `Orchestrator::set_db_connect_timeout`) ->
  `OrchestratorError::DatabaseFailed`, logged `reason=database_connect_failed`,
  before any listener binds. Empty -> `Ok(None)` + WARN `no_database_configured`.
- `HandlerState.db_configured` gates the developer fallback in
  `auth/handlers.rs`: `developer_mode && !db_configured` only. Configured + no pool
  -> code 10, ERROR `db_pool_missing`. Startup WARN in `AuthService::start`
  (`dev_mode_no_db_login`).
- Tests that drive `AuthService` standalone in developer mode must set
  `db_connection_string: String::new()` or every login is refused.
- Container: `DEVELOPER_MODE=false` in the image; `docker/compose.yml` passes
  `${DEVELOPER_MODE:-false}`. s6 `run` runs `cimmeria-server --check-db` (reads DB_URL from env, never argv; sanitised `DbCheckFailure` reason; `crates/services/src/database_check.rs`)
  (DB_AUTH_CHECK_ATTEMPTS, default 10); `finish` writes non-zero exit to
  `/run/s6-linux-init-container-results/exitcode`. HEALTHCHECK uses the same `--check-db`. Watchtower strips env equal to the old image default, so the colo (which inherited true from the image) goes to false on its own at its first swap to this image.
- The bundled Postgres is initdb'd with default `trust` auth for loopback, so a
  wrong *password* is not refused in-container; wrong user/dbname is.

**Why:** a container where Postgres answered `pg_isready` but the app's own login
failed used to keep serving without a pool.

**How to apply:** any new no-DB code path must key on `database_configured()`, not
on `db_pool.is_none()` alone. Test URLs must pass the live-DB URL guard: use
`127.0.0.1:1` for a never-connecting pool or libpq key-value form for a fake server.

Related: [[security-audit-2026-05-31]] [[player-id-zero-sentinel-trap]]
