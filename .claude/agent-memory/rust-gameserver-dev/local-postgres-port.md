---
name: local-postgres-port
description: Verify the dev Postgres port and database name before every live-DB run — they move, and require_db_or_skip! turns a wrong one into a green self-skip.
type: project
---

**Probe, don't assume.** The port has been 5544 on this host in the past; as of 2026-09-17 the bundled cluster listens on the documented **5433**. Either way, check before trusting a run.

**The database name also moves.** During parallel campaign work the coordinator may stand up a per-campaign scratch DB (e.g. `sgw_harset`) because a sibling session drops and recreates `sgw` mid-run. Rows applied to the wrong database vanish without an error. List first:

```powershell
$env:PGPASSWORD='w-testing'
& external\postgresql_server\bin\psql.exe -h localhost -p 5433 -U w-testing -d postgres -tAc "select datname from pg_database order by 1"
```

A `FATAL: database "sgw" does not exist ... seems to have just been dropped or renamed` means a sibling session is mid-reload — retry, don't conclude the cluster is broken.

Binary lives at `external/postgresql_server/bin/` (`psql.exe`, `postgres.exe`). Role `w-testing` (password same as the name) exists; role `sgw` does not.

**Don't reload the shared instance's schema from a worktree.** The shared `sgw` DB (main-checkout `server/pgdata/`) may have concurrent agent sessions' seed edits loaded into it that a `db/database.sql` re-run would stomp. For genuine (non-skipped) live-DB test execution during a worktree session, stand up an isolated scratch cluster instead — see [[live-db-scratch-cluster]] (recipe confirmed working 2026-09-17, scratch port 5599: `initdb` + `pg_ctl start` + `CREATE ROLE`/`CREATE DATABASE` + `psql -f db/database.sql` all succeeded in under 2 minutes combined). `pg_ctl -w stop` reported "failed" after ~90s wait once, but the server actually had stopped (`pg_ctl status` confirmed no server running immediately after) — don't assume a stop failure means the process is still up; verify with `status` before troubleshooting further.

**Why this matters:** `require_db_or_skip!` turns an unreachable DB into a **self-skip that still reports PASS**. On the wrong port every live-DB test "passes" without executing a line of its body — nextest shows green and the guard is worthless. The only tell is the line `skipping live-DB test (DATABASE_URL set but connect failed: pool timed out ...)`, which is invisible unless you pass `--no-capture`, and the `Summary` line's skip count.

**How to apply:** before trusting a live-DB run, confirm the tests actually executed — either check `Summary` shows `0 skipped` for the filtered set, or run once with `--no-capture` and confirm no "skipping live-DB test" line. To find the port on a host where it differs again:

```powershell
Get-NetTCPConnection -State Listen | Where-Object { $_.OwningProcess -in (Get-Process -Name postgres).Id } | Select LocalPort -Unique
```

See [[build-environment]] and [[db-test-revert-verification]].
