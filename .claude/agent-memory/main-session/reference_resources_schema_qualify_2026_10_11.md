---
name: reference-resources-schema-qualify
description: resources tables (items, ...) must be schema-qualified in server_db_query / spec SQL; the search path is "$user", public
metadata:
  type: reference
---

2026-10-11 (lab-record): the sgw database keeps game data in the `resources` schema (`db/database.sql:10-13`, `\ir resources/_schema.sql`) and player state in `public` (`sgw_*`). The search path is the default `"$user", public`, so `items` alone fails with `relation "items" does not exist`; write `resources.items` (as `crates/lab-mcp/src/tools/fixture/seed.rs:107` does).

Why it mattered: LR-06's snapshot SQL joined bare `items`; the sampler drops a whole snapshot when one statement fails, so a live recording would have had no snapshots. No unit test can see it (pinned SQL text only; `cimmeria-lab` has no live-DB tier).

How to apply: any SQL a lab spec, clause or sampler sends through `server_db_query` that names a resources table needs the `resources.` prefix. Check new statements read-only against the local test DB (bundled psql under `external/`, port 5433, `BEGIN READ ONLY; ... ROLLBACK;`) before shipping.
