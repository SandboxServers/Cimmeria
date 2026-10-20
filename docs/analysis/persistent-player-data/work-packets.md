# Persistent player data work packets

> Type: work packets. Ledger and decisions: [README.md](README.md). Owner answers: [OWNER-DECISIONS.md](OWNER-DECISIONS.md) (D1-D8; O1-O10 all decided).
>
> **Status of this file:** the architect's high-level requirements (goal, files, dependencies, acceptance, test intent). Before dispatch, a packet writer expands each section into prescriptive step-by-step text with code shapes, keeping the contract below verbatim. Don't hand these sections to a packet-coder as they stand.

## Dispatch rules

Every packet brief inlines these rules (packet-coder skips CLAUDE.md):

1. **One worktree per packet:** `pwsh tools/build-lane/mk-worktree.ps1 persistent-player-data/<packet>-<slug> pp-<nn>`, created through `ops`. Its test database is `sgw_pp_<nn>`. Never work in the root checkout, and never touch the play DB `sgw`.
2. **PowerShell only, absolute paths.** No bash, no WSL, no local `docker` (D-PP6). Shell scripts under `docker/` and `tools/*.sh` are proven in CI only. Say in the report that they weren't run locally.
3. **Never run cargo directly.** Every compile goes through `pwsh tools/build-lane/lane.ps1 cargo <fmt|clippy|nextest> -p <crate>`, in that order. Live-DB tests (`live_db_*`) go through `pwsh tools/build-lane/live-db-test.ps1` (lane-build skill). Run lanes in the foreground, with `Set-Location <worktree>;` at the start of each call. Check that the log lands under `logs/<worktree>/`. Plain pwsh test scripts (`bootstrap/tests/test-*.ps1`, `tools/build-lane/test-*.ps1`) are not cargo and run directly with `pwsh -NoProfile -File <absolute path>`.
4. **Tests per [TESTING.md](../../../TESTING.md).** Type 1 unit or lint, type 3 `live_db_*` guards with `0x7000_xxxx` sentinels, type 4 PL/pgSQL smoke (`tools/<feature>_smoke.sql`, `include_str!`'d into `crates/base/src/base/smoke_tests.rs`, wrapped in BEGIN/ROLLBACK, asserting with RAISE EXCEPTION). Each test must fail when its change is reverted. Say how in the report.
5. **File caps:** soft 500 lines, hard 700. Split along natural seams. `Initialize-CimmeriaDatabase.ps1` is already 557 lines: new logic goes in new files, and the edit there must not grow it.
6. **Docs under `docs/` are CRLF.** Keep them CRLF. Never hand-edit a `<!-- gen:NAME -->` block. Skills are edited only under `.claude/skills/`, then `python tools/agent-skills/sync.py`.
7. **Seeds are the source of truth.** Never add `db/scripts/*.sql` migrations. Never restructure `db/database.sql` (two parsers read its flat `\ir` list, F6).
8. **Git:** commit once locally with the given subject and the attribution lines from the brief. No `git stash`, no `git worktree prune`, no push, no PR.
9. **Report:** write the full report to `<worktree>\.claude\reports\persistent-player-data\<packet>-<role>.md`, then return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
10. No IPs, credentials or account names in commits, docs or agent memory.

Each wave is reviewed by one packet-review-fixer covering every packet in it. Wave 2 additionally gets a read-only `database-persistence` pass over PP-05 and PP-06; wave 3 one over PP-13.

## Contract

These names are shared across packets. Do not rename them.

### Repo files

| Path | Owner | Content |
|---|---|---|
| `db/persistent/meta.sql` | PP-01 | Creates schema `cimmeria_meta` and its two tables, idempotently (`IF NOT EXISTS`). Plain SQL: no psql meta-commands, no BEGIN/COMMIT. |
| `db/persistent/reconcile.sql` | PP-01 | Runs in `sgw_next` after carry-over and before post-data (no PKs, FKs or indexes yet). It also has to work in a full test DB with the FKs dropped. Plain SQL plus DO blocks, no transaction control. Logs every action to `cimmeria_meta.reconcile_log`. Used by the container (PP-06) and the local refresh (PP-13). |
| `db/persistent/seed-accounts-stage.sql` | PP-17 | Runs in `sgw_next` right after step 4: creates schema `cimmeria_seed`, moves the fresh seed rows of `public.account` into `cimmeria_seed.account`, and empties `public.account`. Plain SQL, no transaction control. |
| `db/persistent/seed-accounts.sql` | PP-17 | Runs in `sgw_next` after the carry-over and before `reconcile.sql`: inserts each `cimmeria_seed.account` row whose `account_id` and `account_name` are both absent from `public.account`, logs every insert and skip, then drops `cimmeria_seed`. Never updates or deletes a `public.account` row. |
| `db/persistent/reseed-public-tables.txt` | PP-02 | One `public` table name per line; `#` starts a comment. Content: `shards`. |
| `db/persistent/persistent-public-tables.txt` | PP-02 | Every other `public` table (README classification), same format. |
| `db/persistent/persistent-schema.snapshot` | PP-11 | The column snapshot of the persistent tables (format chosen by PP-11). |
| `docker/db-sync.sh` | PP-05 | The state machine, backup and retention. Calls `db-rebuild.sh`. |
| `docker/db-rebuild.sh` | PP-06 | The container rebuild pipeline. |
| `docker/s6/db-sync/` | PP-05 | s6-rc oneshot (`type`, `up`, `timeout-up` = 0, `dependencies.d/postgres`). `cimmeria-server/dependencies.d/db-sync` is added next to it. |
| `bootstrap/CimmeriaBootstrap/Private/Invoke-CimmeriaResourcesRefresh.ps1` | PP-13 | The local refresh pipeline (D-PP14). Private (not exported). |
| `bootstrap/tests/test-database-refresh.ps1` | PP-13 | pwsh test of the refresh against throwaway DBs. |
| `tools/persistent_reconcile_smoke.sql` | PP-01 | Type 4 smoke. |
| `tools/persistent_seed_accounts_smoke.sql` | PP-17 | Type 4 smoke. |
| `tools/container-persistence-smoke.sh` | PP-08 | CI scenario smoke. |

`db/database.sql` never references `cimmeria_meta`, `cimmeria_seed` or `db/persistent/`.

### Image paths

| Path | Content |
|---|---|
| `/opt/cimmeria/db/fresh.dump` | `pg_dump -Fc sgw` of the freshly loaded DB, taken **before** `meta.sql` runs. |
| `/opt/cimmeria/db/image-id` | One line, `<git-sha or unknown>-<db-source-sha256 first 12>-<build UTC yyyymmddThhmmssZ>`. Unique per build. |
| `/opt/cimmeria/db/db-source.sha256` | sha256 over the sorted `db/` tree (path and content). For diagnostics only. |
| `/opt/cimmeria/db/persistent/` | A copy of `db/persistent/`. |
| `/opt/cimmeria/bin/db-sync.sh`, `/opt/cimmeria/bin/db-rebuild.sh` | The scripts. |
| `/var/backups/cimmeria/` | Backups. A `VOLUME`; compose binds `${CIMMERIA_BACKUP_DIR:-./backups}` to it. |
| `/var/lib/postgresql/data` | PGDATA. Compose mounts the **external** named volume `cimmeria-server-pgdata` there (D-PP1). Never `cimmeria-pgdata`, which `setup.ps1 -UseDocker` owns. |

### Schema `cimmeria_meta`

```sql
cimmeria_meta.db_state (
  id                   smallint PRIMARY KEY CHECK (id = 1),
  state                text NOT NULL CHECK (state IN ('baked','live')),
  image_id             text NOT NULL,      -- image (or 'local-...' refresh) that last built this DB
  db_source_sha256     text NOT NULL,
  first_seeded_at      timestamptz,        -- set on the first boot, never changed after
  last_rebuild_at      timestamptz,
  rebuild_count        integer NOT NULL DEFAULT 0,
  last_failed_image_id text,               -- set in the OLD db when a rebuild fails
  last_failed_at       timestamptz,
  last_restore_sha256  text                -- D-PP12 idempotency
)
cimmeria_meta.reconcile_log (
  id          bigserial PRIMARY KEY,
  logged_at   timestamptz NOT NULL DEFAULT now(),
  image_id    text NOT NULL,
  action      text NOT NULL,   -- 'delete_dangling_item' | 'relocate_player' | 'count_dangling_ref' | 'sequence_fixup'
                               -- | 'insert_seed_account' | 'skip_seed_account' (PP-17)
  table_name  text NOT NULL,
  detail      jsonb NOT NULL   -- the full row for deletes and seed inserts; old and new values for relocations;
                               -- for a skip, the seed row and the id or name that blocked it; counts otherwise
)
```

`reconcile.sql` and `seed-accounts.sql` read the current image id from the session setting `cimmeria.image_id` (`current_setting('cimmeria.image_id')`), which `db-rebuild.sh` and the local refresh set with `-c` / `PGOPTIONS`. The smokes set it with `SET LOCAL`.

### db-sync states (in order of evaluation)

| State | Condition | Action | Log line prefix |
|---|---|---|---|
| `restore` | `CIMMERIA_DB_RESTORE` is set and its sha256 is not `last_restore_sha256` (disaster recovery only; never part of a normal deploy, owner D4) | Backup the current `sgw`, restore the file to `sgw_restore`, then rebuild with source `sgw_restore`. Record the sha. Drop `sgw_restore`. | `[db-sync] restore` |
| `first-boot` | `db_state.state = 'baked'` and `image_id` equals the image's | Set `state = 'live'` and `first_seeded_at = now()`. No rebuild. | `[db-sync] first-boot` |
| `adopt` | `sgw` exists without `cimmeria_meta` | Backup, then rebuild with source `sgw`. | `[db-sync] adopt` |
| `rebuild` | `image_id` differs, or `CIMMERIA_DB_FORCE_REBUILD=1` | Backup, then rebuild with source `sgw`. Resources only: player data is carried over (owner D3). | `[db-sync] rebuild` |
| `skip` | otherwise (same image: restart, crash, reboot) | Nothing. | `[db-sync] skip` |

Exit 0 means the server may start. Any non-zero exit means `sgw` is untouched and the container stops (`S6_BEHAVIOUR_IF_STAGE2_FAILS=2`). The failure is recorded in the old DB's `db_state` (`last_failed_*`). On a retry with the same target, db-sync reuses the newest backup instead of writing a new one. No state or env var ever wipes player data; the container's full wipe is the operator's volume removal (D-PP15).

### Rebuild steps (shared by db-rebuild.sh and the local refresh)

`db-rebuild.sh <source_db> <image_id>`, run as the postgres OS user over the local socket. The local refresh runs the same steps from PowerShell over TCP as `w-testing`, with the dump it made from `sgw_fresh` in place of `fresh.dump`. Steps, in order:

1. Drop any leftover `sgw_next`, then create `sgw_next`.
2. Apply `meta.sql`.
3. `pg_restore --section=pre-data` from the fresh dump.
4. `pg_restore --section=data`, filtered by a TOC list (`pg_restore -l` / `-L`) to `resources` TABLE DATA and SEQUENCE SET entries, plus the `public` tables in `reseed-public-tables.txt`, plus the TABLE DATA of `public.account` (the seed accounts, D-PP17).
   4a. `seed-accounts-stage.sql`: the seed accounts move to `cimmeria_seed.account`; `public.account` is empty again.
5. Carry-over: `pg_dump --data-only -Fc` of `<source_db>`, taking `public` minus the reseed list plus `cimmeria_meta` (when the source has it), piped to `pg_restore --data-only -d sgw_next`. The COPY is column-named, so additive columns take their defaults.
   5a. `seed-accounts.sql`: insert the seed accounts missing by id and by name, log, drop `cimmeria_seed`.
6. `reconcile.sql`, which also fixes up the sequences: every sequence owned by a `public` column is set to `GREATEST(carried value, max(column))`. It runs after 5a, so inserted seed ids are covered.
7. `pg_restore --section=post-data`. Any error aborts.
8. Verify: row counts of the persistent tables equal the source's, minus what reconcile logged as deleted, plus (for `account`) what 5a logged as inserted. `cimmeria_seed` no longer exists.
9. Upsert `db_state` (`state = 'live'`, `image_id`, `db_source_sha256`, `last_rebuild_at`, `rebuild_count + 1`, clear `last_failed_*`). `first_seeded_at` is kept (set to now() when the source had no row).
10. Swap: disallow connections on `sgw` and terminate its backends, drop `sgw_prev`, rename `sgw` to `sgw_prev` and `sgw_next` to `sgw`, and allow connections on the new `sgw`.

On any failure, drop `sgw_next` and exit non-zero (container) or throw (local).

### Local refresh interface (PP-13, PP-14)

```powershell
# Private, dot-sourced by the module. Throws on failure with sgw untouched.
Invoke-CimmeriaResourcesRefresh -PsqlExe <path> -Port <int> [-UseDocker] [-Database 'sgw'] [-BackupDir <path>] [-BackupKeep 3]
```

- `-Database` exists so the test can refresh a throwaway DB; `Initialize-CimmeriaDatabase` always passes `sgw`. The scratch, next and prev DBs are `<Database>_fresh`, `<Database>_next`, `<Database>_prev`.
- `pg_dump`/`pg_restore` are found next to `PsqlExe` (Windows bundle, F19), else on PATH.
- Image id: `local-<db-source-sha256 first 12>-<UTC yyyymmddThhmmssZ>`.
- Backup: `pg_dump -Fc <Database>` to `<BackupDir>/<Database>-<UTC yyyymmddThhmmssZ>.dump` before step 1; default `BackupDir` is `server/backups/` (gitignored); keep the newest `BackupKeep`.
- Prints one status line per step through `Write-Status`, and at the end the carried account and character counts, the seed accounts it inserted or skipped, and the reconcile-log actions of this run.

### Local flags (PP-14)

| Flag (`setup.ps1` / `Initialize-CimmeriaDatabase`) | `sgw` absent or without schema | `sgw` with schema (`public.account` exists) |
|---|---|---|
| none | full load | skip when fully seeded; an incomplete seed with accounts present routes to the refresh |
| `-ForceDatabase` / `-Force` | full load | `Invoke-CimmeriaResourcesRefresh` (player data kept) |
| `-WipeDatabase` (new) | full load | print counts, backup, drop `sgw`, full load |
| `-ResetDatabase` | delete pgdata, initdb, full load | same; prints counts first when reachable; help says it destroys player data |

`-ForceDatabase` together with `-WipeDatabase` throws before touching anything.

### Environment variables (container)

| Var | Default | Meaning |
|---|---|---|
| `CIMMERIA_DB_FORCE_REBUILD` | unset | `1` forces the `rebuild` state: resources only, player data kept. |
| `CIMMERIA_DB_RESTORE` | unset | Disaster recovery only: the path of a `-Fc` dump under `/var/backups/cimmeria` to restore player data from (D-PP12). Unset in every normal deploy, including the first. |
| `CIMMERIA_DB_BACKUP_KEEP` | `10` | How many backup files to keep. |
| `CIMMERIA_BACKUP_DIR` | `./backups` | Compose only: the host directory bound to `/var/backups/cimmeria`. |

Backup file name: `sgw-<UTC yyyymmddThhmmssZ>-<old image_id or 'pre-meta'>.dump`.

---

## Wave 1: SQL, lint, audit, test-DB guard

PP-01, PP-02, PP-03 and PP-15 run in parallel; PP-17 stacks on PP-01's branch. One packet-review-fixer covers all five; ops ships one PR.

### PP-01 Meta schema, reconcile SQL and the reconcile smoke

- **Agent:** database-persistence. Worktree `pp-01`. Commit subject: `feat(db): cimmeria_meta schema and persistent-data reconcile SQL`.
- **Goal:** the two SQL files in the contract, proven by a type 4 smoke against the seeded test DB. Both the container and the local refresh run them, so they must not assume a socket user, a superuser or a fresh-dump origin.
- **Files:** `db/persistent/meta.sql`, `db/persistent/reconcile.sql`, `tools/persistent_reconcile_smoke.sql`, plus one harness function in `crates/base/src/base/smoke_tests.rs` (124 lines now; it can include `db/persistent/*.sql` via `include_str!`).
- **Depends on:** none. The reconcile behaviour is owner D5 (delete orphans with a JSON log, relocate to Castle_CellBlock).
- **Acceptance:**
  - `meta.sql` matches the contract schema and runs twice without error, as the DB owner (`w-testing` locally).
  - `reconcile.sql` handles every case below, writing a `reconcile_log` row for each:
    - It deletes `sgw_inventory`, `sgw_gate_mail_item` and `sgw_organization_vault_items` rows whose `type_id` is missing from `resources.items`, logging each full row (`to_jsonb`).
    - It relocates `sgw_player` rows whose `world_id`/`world_location` is missing from `resources.worlds` to the `Castle_CellBlock` fallback (looked up by name; the coordinates are in D-PP9). It RAISEs when the fallback is missing.
    - It counts, without deleting, the non-FK ids from F9 that no longer resolve.
    - It fixes up the sequences.
  - It must not depend on PKs or indexes existing.
  - Check that the `sgw_inventory` inheritance (`sgw_inventory_base`) is handled: delete with or without `ONLY`, deliberately.
- **Test intent (type 4):** inside BEGIN/ROLLBACK: apply `meta.sql`, `SET LOCAL cimmeria.image_id`, drop the relevant FKs, insert sentinel dangling rows (one per FK case, one player in a fake world, one sequence lowered below its max), run `reconcile.sql`, and assert the deletions, the relocation, the log rows and the sequence value. A second smoke block asserts that a missing fallback world RAISEs. Reverting `reconcile.sql` (an empty file) fails every assert.
- **Lanes:** fmt, then clippy, then the new `live_db_*` smoke through `live-db-test.ps1`, all `-p base`.
- **Docs owed:** none (PP-10 writes the reference). Note `tools/persistent_reconcile_smoke.sql` for the tools catalog in the report.

### PP-17 Seed-account top-up SQL and smoke

- **Agent:** database-persistence. Worktree `pp-17`, stacked on `pp-01`'s branch (it needs `meta.sql` and adds a second function next to PP-01's in the smoke harness). Commit subject: `feat(db): insert missing seed accounts on every rebuild`.
- **Goal:** owner D6 / D-PP17. The two seed-account SQL files of the contract (steps 4a and 5a), proven by a type 4 smoke. Like `reconcile.sql`, they must not assume a superuser, a socket user, or that PKs and indexes exist.
- **Files:** `db/persistent/seed-accounts-stage.sql`, `db/persistent/seed-accounts.sql`, `tools/persistent_seed_accounts_smoke.sql`, plus one harness function in `crates/base/src/base/smoke_tests.rs`.
- **Depends on:** PP-01 (`meta.sql`, the `reconcile_log` table).
- **Acceptance:**
  - Stage: `cimmeria_seed.account` gets exactly the rows `public.account` held (all columns; `CREATE TABLE ... AS` or `LIKE` plus `INSERT`), and `public.account` ends empty. Runs on a table without its PK.
  - Top-up: a seed row is inserted only when neither its `account_id` nor its `account_name` exists in `public.account` (exact name match, as login does, F22). Each insert logs `insert_seed_account` with the row; each skip logs `skip_seed_account` with the seed row and what blocked it (id, name, or both). An identical carried row (same id and name) is a silent no-op, not a skip, so a normal rebuild logs nothing for the 14 seed accounts that are already there.
  - No `UPDATE` or `DELETE` on `public.account` anywhere in either file.
  - `cimmeria_seed` is dropped at the end, also when there was nothing to insert.
  - Only `account` rows: no characters or other rows are created.
- **Test intent (type 4):** inside BEGIN/ROLLBACK, on the seeded test DB: apply `meta.sql`, `SET LOCAL cimmeria.image_id`, run the stage file, then simulate a carried account set: three seed accounts carried unchanged, one carried with a changed `accesslevel`, one seed account missing, one seed id held by a sentinel `0x7000_xxxx`-named account, one seed name held by a sentinel id. Run the top-up and assert: the missing account is back with its seed values; the changed `accesslevel` is unchanged; the two conflicting sentinels are untouched and their seed rows are not inserted; the log has exactly one `insert_seed_account` and two `skip_seed_account` rows; `cimmeria_seed` is gone. An empty `seed-accounts.sql` fails the "missing account is back" assert; a version that upserts fails the `accesslevel` assert.
- **Lanes:** fmt, then clippy, then the new `live_db_*` smoke through `live-db-test.ps1`, all `-p base`.
- **Docs owed:** none (PP-09, PP-10, PP-16). Note the smoke for the tools catalog in the report.

### PP-02 Table classification lists and lint

- **Agent:** packet-coder. Worktree `pp-02`. Commit subject: `feat(db): classify public tables as persistent or reseed, with lint`.
- **Goal:** the two classification files and a type 1 lint that keeps them complete.
- **Files:** `db/persistent/reseed-public-tables.txt`, `db/persistent/persistent-public-tables.txt`, and one new lint test file. The packet writer picks the crate. `crates/content-engine/tests/it/` already parses `db/database.sql` (the `interact_tag_linter.rs` precedent); a crate with a db-lint home is preferred.
- **Depends on:** none.
- **Acceptance:** the lint fails when:
  - a `CREATE TABLE` under `db/sgw/**/Tables/` is in neither list, or in both;
  - a list names a table that doesn't exist;
  - any file under `db/sgw/`, `db/resources/` or `db/database.sql` mentions `cimmeria_meta` or `cimmeria_seed`.

  The lists match the README classification exactly (24 persistent tables including `account`, 1 reseed).
- **Test intent (type 1):** the lint runs on the real files. Add a unit test of the parser on an inline fixture that is missing one table, to prove the lint can fail. Removing a line from either list makes it fail.
- **Lanes:** fmt, then clippy, then nextest on the chosen crate.

### PP-03 Dangling non-FK id tolerance audit

- **Agent:** rust-gameserver-dev. Worktree `pp-03`. Commit subject: `test(db): guard server load of removed resource ids`.
- **Goal:** find out what the server does today when a persisted player row holds a resource id that the new resources no longer have (F9): known stargates, abilities, trained abilities, disciplines, blueprints, respawners, mission and step ids, tutorials, content cooldowns.
- **Files:** read-only research across `crates/` load paths (player load, mission load, ability grants). Add 1-2 `live_db_*` guards in the crate that owns the most important load path. The audit table goes in the report.
- **Depends on:** none.
- **Acceptance:**
  - The report has a table: id kind, load site (`file:line`), today's behaviour (skip/warn, error, panic, or a client-visible break), severity.
  - Every panic or login-blocking case is listed as a follow-up fix with a one-line recommendation.
  - No fixes in this packet beyond a one-line obvious guard, and only when the packet writer allows it.
- **Test intent (type 3):** a guard that loads a sentinel `0x7000_xxxx` character holding a removed id and asserts that the load succeeds (or documents the failure, as an ignored test with an issue link). Reverting the guard fix, if one was made, fails it.
- **Lanes:** fmt, then clippy, then `live-db-test.ps1` for the guard crate.
- **Docs owed:** none. The findings feed PP-10's reference doc.

### PP-15 Throwaway-DB scripts never target `sgw`

- **Agent:** packet-coder. Worktree `pp-15`. Commit subject: `fix(build-lane): keep reload-db and live-db-test off the play database`.
- **Goal:** D-PP16 per owner D8 (O10). From the main checkout, `reload-db.ps1` and `reload-db.sh` reload the main checkout's own test DB (default `sgwtest`) instead of `sgw`; both refuse `sgw` as a target, including through `CIMMERIA_TEST_DB`, with a one-line message naming `setup.ps1 -ForceDatabase` and `-WipeDatabase`. Worktree DBs (`sgw_<wt>`) are unchanged. Check every consumer of the old "main checkout keeps `sgw`" rule and keep it working: `live-db-test.ps1`/`.sh`, `tools/test-live-db.ps1`/`.sh` (slot-clone naming and its stale-DB drop), `rm-worktree` (must never drop `sgw` or `sgwtest`), and the lab server's DB target if it reads the same default. List each with its verdict in the report.
- **Files:** `tools/build-lane/reload-db.ps1`, `tools/build-lane/reload-db.sh`, whichever of the consumers above need a change, and one `tools/build-lane/test-*.ps1` case (new file or an existing one, whichever fits).
- **Depends on:** nothing. O10 is decided (owner D8: main checkout uses `sgwtest`, `reload-db` refuses `sgw`); wave 1.
- **Acceptance:** from the main checkout the printed `DATABASE_URL` names the new DB; `CIMMERIA_TEST_DB=sgw` exits non-zero before any DROP; a worktree still gets `sgw_<wt>`. The comment headers of the changed scripts say why.
- **Test intent:** the pwsh test drives the DB-name resolution without a live Postgres (extract it into a small function or a `-WhatIf`/dry-run mode if needed) and asserts the three cases. Restoring the old `sgw` default fails it. It runs in `build-lane-scripts.yml` automatically. The `.sh` twin is proven by review only; say so.
- **Docs owed:** `docs/agents/development-workflow.md:254` (the "main checkout keeps `sgw`" sentence), CRLF. The skills and TESTING.md go to PP-16.

## Wave 2: the container change (one PR)

All five packets build against the contract in parallel, on disjoint files. Ops creates the worktrees from the merged wave 1 main. The review-fixer combines them onto the PP-04 branch, and ops ships one PR, which triggers `pr-container.yml`.

### PP-04 Image build: bake the dump, meta and ids

- **Agent:** packet-coder. Worktree `pp-04`. Commit subject: `feat(docker): bake fresh.dump, cimmeria_meta and image id into the image`.
- **Files:** `docker/Dockerfile` only.
- **Goal:**
  - In the `db-init` stage, after `psql -f db/database.sql`:
    - write `fresh.dump`;
    - compute `db-source.sha256` and `image-id` (pass `CIMMERIA_GIT_SHA` in as a build ARG; it's builder-stage only today);
    - apply `meta.sql` and insert the `db_state` row with `state = 'baked'`.
  - In the runtime stage:
    - COPY `fresh.dump`, the id files and `db/persistent/` to `/opt/cimmeria/db/`, and the two scripts to `/opt/cimmeria/bin/` (mode 0755);
    - add the s6 `db-sync` service directory;
    - set `ENV S6_BEHAVIOUR_IF_STAGE2_FAILS=2` (check that `S6_CMD_WAIT_FOR_SERVICES_MAXTIME` doesn't kill a long rebuild; set it to 0 if needed);
    - add `/var/backups/cimmeria` to `VOLUME`;
    - raise the HEALTHCHECK `--start-period` to cover a rebuild (300s).
  - Update the header comment ("runtime image carries zero .sql files" is no longer true).
- **Depends on:** PP-01 (`meta.sql`), and the contract paths for PP-05 and PP-06.
- **Acceptance:** the image builds in CI. `docker run` of the image shows `[db-sync] first-boot` and then the server healthy (proved by PP-08).
- **Test intent:** CI only (PP-08). Say so in the report.

### PP-05 Entrypoint, s6 oneshot and the db-sync state machine

- **Agent:** packet-coder. Worktree `pp-05`. Commit subject: `feat(docker): keep pgdata across starts; db-sync oneshot state machine`.
- **Files:** `docker/entrypoint.sh`, `docker/db-sync.sh`, `docker/s6/db-sync/*`, and `docker/s6/cimmeria-server/dependencies.d/db-sync`.
- **Goal:**
  - The entrypoint copies the baked pgdata only when `PGDATA/PG_VERSION` is missing. The chown/chmod and the Discord block stay. Rewrite the long header comment to explain the new rule and why (owner D2: a restart keeps everything).
  - `db-sync.sh` implements the contract state table, the backup and its retention (`CIMMERIA_DB_BACKUP_KEEP`), the failed-attempt record and backup reuse, the `restore` idempotency (DR only; keep the branch small), and the log prefixes. It calls `db-rebuild.sh <source_db> <image_id>`. No path in it wipes player data.
  - The oneshot runs under `with-contenv`, waits for `pg_isready`, and has `timeout-up` set to 0.
- **Depends on:** wave 1 merged; the contract for `db-rebuild.sh`. Owner answers in force: D2 (restart keeps everything), D4 (no import at first deploy; `restore` is DR only), D7 (newest 10 backups, fail closed).
- **Acceptance:** POSIX sh only (no bashisms; the packet writer names a `shellcheck -s sh` CI step if one exists, else the review checks by hand). A restart of the same image logs `skip` and touches nothing. Every non-zero path leaves `sgw` untouched.
- **Test intent:** CI only (PP-08 scenarios first-boot, restart, adopt, restore, fail-closed, full wipe).

### PP-06 The container rebuild pipeline

- **Agent:** database-persistence. Worktree `pp-06`. Commit subject: `feat(docker): blue/green resources rebuild with player-data carry-over`.
- **Files:** `docker/db-rebuild.sh` only.
- **Goal:** the ten contract rebuild steps exactly, with sub-steps 4a and 5a.
  - TOC filtering by schema and table name for step 4, including the `public.account` TABLE DATA entry.
  - Steps 4a and 5a run `seed-accounts-stage.sql` and `seed-accounts.sql` from `/opt/cimmeria/db/persistent/`, with `cimmeria.image_id` set as for `reconcile.sql`.
  - The carry-over `--exclude-table` list built from `reseed-public-tables.txt`.
  - `cimmeria.image_id` passed to `reconcile.sql`.
  - Use `synchronous_commit=off` where it speeds the restore up safely: `sgw_next` is disposable until the swap.
  - A swap that tolerates the healthcheck's connections.
  - Write the TOC filter and exclude-list rules as plainly as possible: PP-13 ports them to PowerShell and the two must stay equivalent.
- **Depends on:** PP-01 (`reconcile.sql`), PP-17 (the seed-account files), PP-02 (the lists). Owner D7: fail closed, never wipe.
- **Acceptance:** any error, including a carry-over COPY rejected by a non-additive schema change (D-PP7), drops `sgw_next` and exits non-zero with a one-line cause. The step-8 count check matches source counts minus the logged deletes, plus the logged seed-account inserts. `sgw_prev` exists after a successful swap.
- **Test intent:** CI only (PP-08 scenarios forced rebuild, dangling, fail-closed). A design review by a second, read-only `database-persistence` pass.

### PP-07 Compose, env example and dependabot

- **Agent:** packet-coder. Worktree `pp-07`. Commit subject: `feat(docker): external pgdata volume and backup mount in compose`.
- **Files:** `docker/compose.yml`, `docker/.env.example` (or wherever the example env lives), `.github/dependabot.yml`.
- **Goal:**
  - Add the volume per D-PP1: a top-level `volumes:` entry `cimmeria-server-pgdata` with `external: true` and `name: cimmeria-server-pgdata`, mounted on `/var/lib/postgresql/data`, plus the backup bind.
  - A comment next to it: create it once with `docker volume create cimmeria-server-pgdata`; `compose down -v` can't remove it; removing it by hand is the deliberate full wipe (D-PP15).
  - Add `CIMMERIA_DB_RESTORE` (commented out, marked "disaster recovery only; leave unset"), `CIMMERIA_DB_BACKUP_KEEP` and `CIMMERIA_BACKUP_DIR` to the environment and the example with comments. `CIMMERIA_DB_FORCE_REBUILD`'s comment says "resources only; player data kept".
  - Rewrite the comments at 104-107 and 122-124 and the `WATCHTOWER_REMOVE_VOLUMES` comment (watchtower never removes an external volume; check this and say how in the report).
  - Make dependabot ignore postgres semver-major updates.
- **Depends on:** wave 1 merged. Owner D7: on-host backups, newest 10; dependabot ignores Postgres majors.
- **Acceptance:** `docker compose config` is valid (checked in CI by PP-08 or an existing job). No change to ports or existing env.
- **Test intent:** CI only.

### PP-08 CI persistence smoke

- **Agent:** testing-validation-engineer. Worktree `pp-08`. Commit subject: `test(ci): container persistence smoke`.
- **Files:** `tools/container-persistence-smoke.sh` and `.github/workflows/pr-container.yml`.
- **Goal:** a job after the image build that runs one container on the external named volume (created by the job) through these scenarios, asserting with `docker exec ... psql` and the `[db-sync]` log lines:
  1. First boot: `first-boot`, a seed account is present, `db_state.state = 'live'`.
  2. Restart: insert a sentinel character, `docker restart`, then `skip`, and the sentinel is still there.
  3. Forced rebuild with the seed-account top-up: mutate a resources row, delete one seed account that owns no characters (the packet writer picks it from the seed), change another seed account's `accesslevel`, then recreate with `CIMMERIA_DB_FORCE_REBUILD=1`, then `rebuild`. The resources row is restored, the sentinel is **kept** (owner D3), the deleted seed account is back with one `insert_seed_account` log row (owner D6), the changed `accesslevel` is unchanged, a backup file exists and `sgw_prev` exists.
  4. Dangling: add a temporary `resources.items` row in the live DB plus an inventory row using it, force a rebuild, and check that the row is gone and logged.
  5. Fail closed: in the live DB, drop a CHECK from a persistent table and insert a row that violates it. The forced rebuild exits, the container stops, and `sgw` is intact (start without the force flag, then `skip`).
  6. Adopt: drop `cimmeria_meta`, restart, then `adopt` with the data kept.
  7. Restore (disaster recovery): delete the sentinel character, point `CIMMERIA_DB_RESTORE` at the step 3 backup, then `restore`, the sentinel is back, and a second start with it still set logs `skip`.
  8. Full wipe: stop, `docker volume rm` and `create` the volume, start: `first-boot`, and the sentinel is gone. Also check that `docker compose down -v` (or the equivalent `docker rm -v`) leaves the external volume in place.

  Add `db/sgw/**`, `db/persistent/**` and the script to the PR `paths`.
- **Depends on:** the contract (log prefixes, paths, env). Runs against the combined wave 2 branch.
- **Acceptance:** the job is green on the combined branch, and fails when the entrypoint wipe is restored (state the check in the report; the review-fixer proves it once with a throwaway commit in CI, or argues it from the scenario 2 assert).
- **Test intent:** this is the integration test for the whole wave.

## Wave 3: the local refresh (one PR, parallel with wave 2)

Starts from the merged wave 1 main. PP-14 stacks on PP-13's branch. One review-fixer for both, then ops ships one PR.

### PP-13 Local resources-only refresh pipeline

- **Agent:** database-persistence. Worktree `pp-13`. Commit subject: `feat(bootstrap): resources-only refresh that keeps local player data`.
- **Goal:** `Invoke-CimmeriaResourcesRefresh` per the contract "Local refresh interface" and D-PP14: scratch load of `database.sql` into `<Database>_fresh` (reuse the existing `Resolve-SqlIncludes` and its BEGIN/COMMIT wrapper; `ON_ERROR_STOP`), `pg_dump -Fc` to a temp file, drop the scratch DB, backup and retention, then the ten shared rebuild steps (with 4a and 5a, the seed-account top-up of owner D6) with the same TOC filter and exclude rules as PP-06, `meta.sql`, the two seed-account files and `reconcile.sql` from `db/persistent/`, `cimmeria.image_id` set via `PGOPTIONS`. A source without `cimmeria_meta` is adopted (skip it in the carry-over). Cleanup of `_fresh`, `_next` and the temp dump on every exit path; `sgw` untouched on failure; the thrown message names `-WipeDatabase`.
- **Files:** the new private file and the new test (contract), plus the module's dot-source list in `CimmeriaBootstrap.psm1` if private files aren't globbed. No change to `Initialize-CimmeriaDatabase.ps1` (that's PP-14).
- **Depends on:** wave 1 merged (PP-01, PP-02, PP-17). Owner D7: keep the newest 3 local backups; fail closed, never wipe.
- **Acceptance:** works in local mode on the bundled Postgres :5433 and, by construction, over TCP for `-UseDocker` (same client tools, `-h localhost`). The step-8 verify and the step-10 swap match the contract. Runtime on a seeded DB stated in the report.
- **Test intent:** `bootstrap/tests/test-database-refresh.ps1`, run directly with pwsh against :5433 using only throwaway names (`sgw_pp13_refresh` and its `_fresh/_next/_prev`; never `sgw`). It loads `database.sql` into the throwaway DB, inserts a sentinel `0x7000_xxxx` account and character, mutates one `resources` row, deletes one seed account that owns no characters, runs the refresh, and asserts: the sentinel survives, the resource row is back to its seed value, the deleted seed account is back (one `insert_seed_account` log row), `cimmeria_meta.db_state` has a `local-` image id, a backup file exists and `_prev` exists. A second case makes the carry-over fail (a violating row under a dropped CHECK, as PP-08 scenario 5) and asserts the throw and an unchanged DB. The test skips with a clear message when :5433 isn't reachable, and drops everything it created. Replacing the refresh with today's drop-and-reload fails the sentinel assert; skipping step 5a fails the seed-account assert.
- **Docs owed:** none here (PP-16). Note the test for the tools catalog in the report.

### PP-14 setup.ps1 flags, wipe action and CI job

- **Agent:** packet-coder. Worktree `pp-14`, stacked on `pp-13`'s branch. Commit subject: `feat(setup): -ForceDatabase keeps player data; add -WipeDatabase`.
- **Goal:** the contract "Local flags" table.
  - `Invoke-SchemaLoad`'s `-Force` path calls `Invoke-CimmeriaResourcesRefresh` when `sgw` has the schema, and does today's drop-and-load otherwise. The incomplete-seed replay routes to the refresh when `public.account` has rows.
  - New `-WipeDatabase` on `Initialize-CimmeriaDatabase`, `Invoke-CimmeriaBootstrap` and `setup.ps1`: print the account and character counts, backup (reuse the refresh's backup helper), then today's drop-and-load. Mutually exclusive with `-ForceDatabase`.
  - `-ResetDatabase` prints the counts first when the DB is reachable; its help says it destroys player data.
  - Keep `Initialize-CimmeriaDatabase.ps1` at or below its current 557 lines: move the force/wipe branch into a new private file if needed.
  - `setup.ps1` help: the DATABASE FLAGS block, DATA SAFETY and COMMON RECIPES ("Refresh game data, keep characters" / "Wipe everything and reseed"), and the `.PARAMETER` entries.
  - CI: a job in `bootstrap-lint.yml` that runs `bootstrap/tests/test-database-refresh.ps1` against a PG 17 service with matching client tools (`pg_dump` must be at least the server major), so the test never skips in CI. Paths gain `db/persistent/**`.
- **Files:** `bootstrap/CimmeriaBootstrap/Public/Initialize-CimmeriaDatabase.ps1`, `Public/Invoke-CimmeriaBootstrap.ps1`, `setup.ps1`, at most one new private file, `bootstrap/tests/test-database-refresh.ps1` (add flag cases), `.github/workflows/bootstrap-lint.yml`.
- **Depends on:** PP-13.
- **Acceptance:** `-ForceDatabase` on a seeded throwaway DB keeps the sentinel; `-WipeDatabase` removes it and prints the counts it destroyed; both flags together throw before any SQL. The bootstrap-lint parse and export checks still pass (no new exported function).
- **Test intent:** add flag-routing cases to the PP-13 test, driving `Initialize-CimmeriaDatabase` against a throwaway DB name. If the public function can't take a database name today, add a hidden test-only parameter or test the private routing function directly; say which in the report. Reverting the `-Force` routing fails the keep-sentinel case.
- **Docs owed:** the `setup.ps1` help text (above). The rest goes to PP-16; list it in the report.

## Wave 4: docs and the schema guard

### PP-09 Ops docs and the first-deploy runbook

- **Agent:** documentation-writer. Worktree `pp-09`. Commit subject: `docs(ops): persistent player data runbook`.
- **Files:**
  - `docs/operations/colo-deploy.md`: replace § "The database resets on every start" (~133-142), fix :242 (`docker volume prune`), add the runbook, backups and the disaster-recovery restore (`CIMMERIA_DB_RESTORE`), forced rebuild (resources only), the deliberate full wipe, and the seed-account top-up (D-PP17: a new account in `account.sql` appears at the next release; existing accounts are never changed; retire one by removing it from the seed, lock one out with `enabled = false`).
  - `docs/operations/container.md` § Volume / persistence (112-131) and :53.
  - `docs/operations/telemetry.md:283,367`.
  - `docs/troubleshooting.md` (a container that stops at db-sync; a `compose up` that fails because the external volume is missing).
- **First-deploy runbook (content to write, per owner D4 = start fresh):**
  1. Stop watchtower.
  2. Optional safekeeping, not a gate: `docker exec cimmeria pg_dump -Fc -U postgres sgw` into the host backup dir. The characters made before the switch are not carried over; this dump is the only way back to them, through the DR restore.
  3. `docker volume create cimmeria-server-pgdata`.
  4. Pull the new compose.yml (and `.env.example` additions; leave `CIMMERIA_DB_RESTORE` unset).
  5. `docker compose pull && docker compose up -d`.
  6. Check for `[db-sync] first-boot`, then that the dev and lab accounts log in (owner D1).
  7. Start watchtower again.

  Operator advice, not a gated step: now that the seeded dev and lab GM accounts persist on colo (D1), consider rotating their shared seed password.
- **Restart and release behaviour (owner D2, D3):** restart, crash or reboot keeps everything; a new image or `CIMMERIA_DB_FORCE_REBUILD=1` rebuilds resources only and keeps player data; the only full wipe is the volume removal, written as its own clearly labelled procedure.
- **Depends on:** wave 2 merged. Owner answers in force: D1, D2, D3, D4, D6, D7.
- **Acceptance:** every runbook command matches the contract names. State plainly what was CI-proven and what was never run on colo (the D-PP12 restore and the seed-account top-up were proven only in CI). CRLF.
- **Doc-update-map rows:** Ops runbooks.

### PP-10 Rules, reference and agent memory

- **Agent:** documentation-writer. Worktree `pp-10`. Commit subject: `docs: persistent player data reference and rule updates`.
- **Files:**
  - New `docs/architecture/persistent-player-data.md`: the design (container and local), classification, D-PP7/8/9 policies for contributors, the seed-account top-up (D-PP17), the DR restore (D-PP12), the wipe actions (D-PP15), and the PP-03 audit summary.
  - `docs/agents/rules-and-gotchas.md:45,50`: the colo DB now persists player data; resources reseed per image and per forced rebuild; local `-ForceDatabase` keeps player data and `-WipeDatabase` is the wipe; throwaway DBs never use `sgw`; the additive-only rule and the never-renumber rule.
  - `docs/architecture/dev-console-channel.md:96`, `db/README.md:71` (the colo sentence; PP-16 owns the `-ForceDatabase` line), and `TESTING.md` § CI (container image).
  - `docs/guides/placing-npcs-and-objects.md`, `docs/commands.md`, `docs/gameplay/character-creation.md`, `docs/architecture/discord-notifications.md`.
  - The `.claude/agent-memory/main-session/reference_lab_parallel_clients_2026_10_10.md` and `.claude/agent-memory/movement-teleport-advisor/pr1-bounds-seam.md` memory files.
  - Start from a grep for `reseed|fresh DB|rebuilt from seeds|resets on every|wipe` across `docs/`, `.claude/`, `tools/` and `crates/*/README.md`, and fix every stale colo hit. Leave local-setup hits to PP-16 (coordinate through the report, not by editing its files).
  - The tools catalog gets one line for each new script and smoke (container smoke, reconcile smoke, refresh test).
- **Depends on:** waves 2 and 3 merged, PP-03's report.
- **Acceptance:** no stale "DB resets on every start" statement left (list the grep result in the report). CRLF. Generated blocks are regenerated with `tools/docs-gen/regen.py`, never hand-edited.
- **Doc-update-map rows:** test conventions (TESTING.md), CI triggers, live-DB infra if touched, tools scripts, how-to guides, architecture reference.
- **Note for the orchestrator:** the user's own auto-memory has a note that the colo DB refreshes on deploy. It's outside the repo, so the orchestrator tells the user it is now stale.

### PP-11 Persistent schema snapshot guard

- **Agent:** packet-coder. Worktree `pp-11`. Commit subject: `test(db): persistent schema snapshot guards additive-only changes`.
- **Files:** `db/persistent/persistent-schema.snapshot`, plus a type 1 test next to PP-02's lint.
- **Goal:** a committed snapshot of every persistent table's columns (name, type, nullability, has-default), generated by the test in an update mode or by hand.
- **Behaviour:**
  - The test fails on a removed or renamed column, a narrowed type, or a NOT NULL column without a default.
  - Additive changes produce a "snapshot out of date" failure with a fix-it message.
  - The failure text points to D-PP7 and says that a non-additive change needs an owner decision, and that it would also break every developer's `-ForceDatabase` refresh.
- **Depends on:** PP-02 (the list and the parser).
- **Test intent (type 1):** fixtures for each violating case plus one additive case.
- **Lanes:** fmt, then clippy, then nextest on the lint crate.

### PP-16 Local database docs and skills

- **Agent:** documentation-writer. Worktree `pp-16`. Commit subject: `docs: -ForceDatabase refreshes resources and keeps characters`.
- **Goal:** every local-setup statement matches the contract "Local flags" table and D-PP16.
- **Files (F21 and F20):**
  - `docs/building.md:48,52` (flag table: `-ForceDatabase`, new `-WipeDatabase`, `-ResetDatabase` warning), `docs/guides/getting-started.md:94`, `db/README.md:72`, `bootstrap/README.md:75`, `bootstrap/CimmeriaBootstrap/README.md`.
  - `docs/troubleshooting.md:229-239`: a partial load or "relation already exists" is now fixed with `-WipeDatabase` (or `-ResetDatabase`); a failed refresh entry (what the message means, the backup in `server/backups/`, how to restore it).
  - `docs/guides/write-a-database-migration.md:135,189`: resource seeds and new seed accounts reach a local DB with `-ForceDatabase` (D-PP17); other persistent-table seeds and DDL need `-WipeDatabase` or a throwaway DB; additive-only (D-PP7).
  - `.claude/skills/seed-change/SKILL.md` ("rerun setup.ps1 or reload-db.ps1": which flag does what; the main checkout's test DB name) and `.claude/skills/lane-build/SKILL.md` if it names the main checkout's DB; then `python tools/agent-skills/sync.py`.
  - `TESTING.md` live-DB section: the main checkout's test DB is no longer `sgw`.
  - Check `docs/analysis/castle-cellblock-rebuild/uat-guide.md:36`: if its steps need fresh characters, switch it to `-WipeDatabase`; else leave it. Say which.
- **Depends on:** waves 1 and 3 merged, PP-15 merged (O10 fixes the main checkout's test-DB name). Owner D6: a refresh adds missing seed accounts, so no local account recipe is needed.
- **Acceptance:** a grep for `ForceDatabase` and `keeps .sgw.` across `docs/`, `bootstrap/`, `db/`, `.claude/skills/`, `TESTING.md` and `setup.ps1` shows only accurate statements (list it in the report). CRLF for `docs/`. Generated blocks untouched.
- **Doc-update-map rows:** setup and building guides, troubleshooting, how-to guides, skills, test conventions.

## Close-out

### PP-12 Status docs and UAT steps

- **Agent:** documentation-writer. Worktree `pp-12`. Commit subject: `docs: persistent player data close-out`.
- **Files:** `docs/gap-analysis/` (the row that matches; first check `docs/gap-analysis/core-gameplay.md`), `docs/project-status.md`, `docs/guides/unified-uat.md` (the PP-U1 and PP-U2 steps), and this README's packet table.
- **Acceptance:** status docs change once, here. The UAT steps say plainly what is CI-proven and what is not yet tested on colo or on a developer's real `sgw`.

### PP-U1 Owner colo UAT

Run by the owner after the first persistence `/release`, using the PP-09 runbook:

1. The first deploy logs `first-boot` on the new volume, and the dev and lab accounts log in (owner D1, D4: a fresh start).
2. `docker restart cimmeria` keeps a newly made character (`skip`).
3. The next release swap logs `rebuild` and keeps the characters, with resources updated. If that release adds a seed account, it appears; existing accounts are unchanged.
4. The backup files appear in the host backup dir.

Status: UATPending.

### PP-U2 Local UAT

Run by the user on their own main checkout after wave 3 merges (it touches the real `sgw`, so it is never run by an agent):

1. Log in, make a character, note it.
2. `pwsh setup.ps1 -SkipBuild -ForceDatabase -NoLaunch`: the output shows the refresh steps and a backup path; the character is still there after login.
3. `pwsh setup.ps1 -SkipBuild -WipeDatabase -NoLaunch`: the output names the account and character counts it destroys; the character is gone.

Status: UATPending.
