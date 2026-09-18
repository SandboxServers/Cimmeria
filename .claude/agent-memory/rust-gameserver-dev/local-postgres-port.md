---
name: local-postgres-port
description: The dev Postgres port is NOT stable across sessions/hosts — seen both 5544 and 5433 (CLAUDE.md's documented default). Always verify before trusting a live-DB run.
type: project
---

**Update 2026-09-17 (C08b session):** the shared main-checkout Postgres (started via `db.bat`, `PGDATA` under `server/pgdata/`) was listening on **5433** — the CLAUDE.md/TESTING.md documented default — not 5544. An earlier session recorded 5544 as this host's port; that was true then, isn't now. Don't trust either number — verify per-session with `psql -h localhost -p <port> -U w-testing -d sgw -c "SELECT 1;"` before running anything against it.

Binary lives at `external/postgresql_server/bin/` (`psql.exe`, `postgres.exe`). Role `w-testing` (password same as the name) exists; role `sgw` does not.

**Don't reload the shared instance's schema from a worktree.** The shared `sgw` DB (main-checkout `server/pgdata/`) may have concurrent agent sessions' seed edits loaded into it that a `db/database.sql` re-run would stomp. For genuine (non-skipped) live-DB test execution during a worktree session, stand up an isolated scratch cluster instead — see [[live-db-scratch-cluster]] (recipe confirmed working 2026-09-17, scratch port 5599: `initdb` + `pg_ctl start` + `CREATE ROLE`/`CREATE DATABASE` + `psql -f db/database.sql` all succeeded in under 2 minutes combined). `pg_ctl -w stop` reported "failed" after ~90s wait once, but the server actually had stopped (`pg_ctl status` confirmed no server running immediately after) — don't assume a stop failure means the process is still up; verify with `status` before troubleshooting further.

**Why this matters:** `require_db_or_skip!` turns an unreachable DB into a **self-skip that still reports PASS**. On the wrong port every live-DB test "passes" without executing a line of its body — nextest shows green and the guard is worthless. The only tell is the line `skipping live-DB test (DATABASE_URL set but connect failed: pool timed out ...)`, which is invisible unless you pass `--no-capture`, and the `Summary` line's skip count.

**How to apply:** before trusting a live-DB run, confirm the tests actually executed — either check `Summary` shows `0 skipped` for the filtered set, or run once with `--no-capture` and confirm no "skipping live-DB test" line. To find the port on a host where it differs again:

```powershell
Get-NetTCPConnection -State Listen | Where-Object { $_.OwningProcess -in (Get-Process -Name postgres).Id } | Select LocalPort -Unique
```

See [[build-environment]] and [[db-test-revert-verification]].
