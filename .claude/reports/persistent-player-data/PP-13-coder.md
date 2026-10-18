# PP-13 coder report

Status: done. Commit subject `feat(bootstrap): resources-only refresh that keeps local player data`.

## Files
- NEW `bootstrap/CimmeriaBootstrap/Private/Invoke-CimmeriaResourcesRefresh.ps1` (146 lines)
- NEW `bootstrap/CimmeriaBootstrap/Private/Invoke-CimmeriaPsql.ps1` (psql/pg_dump wrapper, throws on non-zero exit)
- NEW `bootstrap/tests/test-database-refresh.ps1` (110 lines)
- `Initialize-CimmeriaDatabase.ps1` not touched.

## Tested (all ran locally against :5433, DB `sgw_pp13_refresh`, never `sgw`)
- Parse check of the function: 0 errors.
- `test-database-refresh.ps1`: case 1 (10 checks) and case 2 (6 checks) pass; it did not skip.
- `test-install-launcher.ps1`: still passes.
- Revert proof: with the carry-over failure swallowed (the nearest edit to "swap despite a failed carry"), all 6 case 2 checks FAIL; restored afterwards. I did not literally move the rename ahead of the carry-over.

## Deviations and mismatches (for the reviewer / PP-14)
1. `db/persistent/seed-accounts-stage.sql` ends with `TRUNCATE account;`, which fails in a DB with foreign keys ("Truncate table sgw_player at the same time"). I run the file with that one line removed (the copy into `cimmeria_seed.account` is unchanged) and truncate all carried tables in one statement before the carry-over. `seed-accounts.sql` and `reconcile.sql` are used verbatim. `docker/db-rebuild.sh` runs the stage file after the carry, where the same TRUNCATE would fail and, without FKs, wipe the carried accounts. I did not touch it or verify it in a container; someone should check it.
2. Wave 2 SQL names matched what is on main (`seed-accounts-stage.sql`, `seed-accounts.sql`, `reconcile.sql`, `meta.sql`). `meta.sql` plus the `db_state` insert (as in `docker/Dockerfile`) is applied to `_fresh`; `db_source_sha256` is the hash of the resolved `database.sql`.
3. Carry-over is `pg_dump -a --disable-triggers` to a file per run, loaded as `postgres`. Carry list = persistent list minus reseed list, plus `account`.
4. Not implemented: the row-count check (db-rebuild.sh step 8); the packet did not list it.
5. `Resolve-SqlIncludes` writes a fixed temp file name, so two refreshes at once would collide.

## NOT tested
- `-UseDocker`, non-Windows, a game DB lacking `cimmeria_meta` (code path tolerates it, no test), backup pruning to `-BackupKeep`, a rename failure after the swap's first rename.
- The read-only `database-persistence` pass over the diff required before PP-14 has not been run.
