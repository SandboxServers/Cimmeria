---
name: local-postgres-port
description: This host's dev Postgres listens on 5544, not the 5433 that CLAUDE.md and TESTING.md document — live-DB tests silently self-skip on the wrong port.
type: project
---

The bundled Postgres on this host listens on **port 5544**, not the 5433 documented in CLAUDE.md / TESTING.md. Working `DATABASE_URL`:

```
postgres://w-testing:w-testing@localhost:5544/sgw
```

Binary lives at `external/postgresql_server/bin/` (`psql.exe`, `postgres.exe`). Role `w-testing` (password same as the name) exists; role `sgw` does not.

**Why this matters:** `require_db_or_skip!` turns an unreachable DB into a **self-skip that still reports PASS**. On the wrong port every live-DB test "passes" without executing a line of its body — nextest shows green and the guard is worthless. The only tell is the line `skipping live-DB test (DATABASE_URL set but connect failed: pool timed out ...)`, which is invisible unless you pass `--no-capture`, and the `Summary` line's skip count.

**How to apply:** before trusting a live-DB run, confirm the tests actually executed — either check `Summary` shows `0 skipped` for the filtered set, or run once with `--no-capture` and confirm no "skipping live-DB test" line. To find the port on a host where it differs again:

```powershell
Get-NetTCPConnection -State Listen | Where-Object { $_.OwningProcess -in (Get-Process -Name postgres).Id } | Select LocalPort -Unique
```

See [[build-environment]] and [[db-test-revert-verification]].
