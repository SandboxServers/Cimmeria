---
name: live-db-verification
description: How to actually verify live-DB behaviour locally, and why a green no-DB nextest run proves nothing for persistence changes
metadata:
  type: project
---

# Verifying live-DB behaviour locally

## The trap: `require_db_or_skip!` tests report as PASS, not skipped

**`require_db_or_skip!` early-`return`s from the test body when `DATABASE_URL` is
unset. nextest counts that as a PASS, not a skip.** A no-DB workspace run of
`cimmeria-services` reports something like `1690 tests run: 1690 passed, 1 skipped`
— the "1 skipped" is unrelated, and *hundreds* of DB tests silently did nothing.

**Why this matters:** for any persistence-layer change (dependency bump, type
mapping, WHERE clause, `rows_affected` check) a green no-DB run is not evidence.
A column decoding differently or a NULL-handling change compiles fine and
"passes" the no-DB suite.

**How to apply:** never report a persistence change as verified off a no-DB run.
Either get a live DB (recipe below) or state explicitly that live-DB coverage is
missing. Two signals distinguish a real run: the live-DB summary says
**`0 skipped`**, and wall time jumps roughly 10x (~6s no-DB → ~60s live for
`-p cimmeria-services --lib`).

## Proving a specific test really hit the DB

Timing alone is weak. Snapshot Postgres counters around the run:

```
select xact_commit, xact_rollback, tup_inserted, tup_deleted
  from pg_stat_database where datname='sgw';
```

The `base::smoke_tests::*` scripts each wrap in `BEGIN … ROLLBACK`, so running
those three moves `xact_rollback` by exactly +3. That is direct proof the SQL
executed rather than early-returning.

## Local live-DB recipe (mirrors the `test-live-db` CI job)

CI uses a `postgres:17.9` service container loaded from `db/database.sql`.
Reproduce with Docker (port 5433 to avoid clashing with a bundled local PG):

```
docker run -d --name cimmeria-sqlx-testdb \
  -e POSTGRES_USER=w-testing -e POSTGRES_PASSWORD=w-testing -e POSTGRES_DB=sgw \
  -p 5433:5432 postgres:17.9
docker cp db cimmeria-sqlx-testdb:/db
docker exec -e PGPASSWORD=w-testing -w /db cimmeria-sqlx-testdb \
  psql -h localhost -U w-testing -d sgw -v ON_ERROR_STOP=1 -f database.sql
```

`db/database.sql` uses `\ir` includes, so it **must** run with the `db/` tree as
the working directory — hence `docker cp` + `-w /db` rather than piping on stdin.

**Budget ~30-40 minutes for the seed load.** It is ~24 MB of single-row `INSERT`
statements with per-statement commits; it is not hung. Order runs roughly
`effects` → `items_event_sets` → `texts` (~29k rows, the long pole) →
`static_meshes`. Watch progress with `pg_stat_activity`, not the psql exit.

End state is 87 tables across `public` + `resources`, ~31 MB.

Then:

```
DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw \
  cargo nextest run --profile=ci-live-db -p cimmeria-services --lib
```

## Coverage gap worth knowing

CI's live-DB job runs **only** `-p cimmeria-services --lib`. `cimmeria-admin-api`,
`src-tauri`, `tools/ContentEditor`, and `tools/SceneEditor` all execute SQL but
have **no live-DB test coverage anywhere in CI** — `admin-api/src/routes/audit.rs`
and `editor.rs` have no tests at all. Changes to their query paths are compile-checked
only. Flag this when reviewing changes there.
