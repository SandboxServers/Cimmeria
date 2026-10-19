# Persistent player data: wave 3 and close-out packets

Packets PP-13, PP-14 (wave 3), PP-09, PP-10, PP-11, PP-16 (wave 4), PP-12 and the UAT scripts PP-U1, PP-U2 (close-out).
Ledger: [README.md](../README.md). Contract: [work-packets.md](../work-packets.md) (sections "Contract" and "Dispatch rules"). Decisions: [OWNER-DECISIONS.md](../OWNER-DECISIONS.md) (D1-D8).

## Rules every packet inherits (inline, from work-packets.md Dispatch rules)

1. PowerShell only, absolute paths. No bash, no local docker, no direct `cargo`, no `git stash`, never `git worktree prune`.
2. Every compiling cargo call goes through `pwsh C:\Users\Steve\source\projects\Cimmeria\tools\build-lane\lane.ps1 cargo <cmd> -p <crate>`. Live-DB tests go through `live-db-test.ps1`. Plain pwsh test scripts run directly: `pwsh -NoProfile -File <abs path>`.
3. Docs are CRLF. After any Edit/Write on a `.md`, check: `(Get-Content -Raw <file>) -match "(?<!\r)\n"` must be False. Fix with `-replace "(?<!\r)\n","`r`n"`.
4. File caps: soft 500, hard 700 lines. `bootstrap/CimmeriaBootstrap/Public/Initialize-CimmeriaDatabase.ps1` is 556 lines and MUST NOT grow (new logic goes in new `Private/*.ps1` files; the psm1 globs `Private/*.ps1`, no registration, and `CimmeriaBootstrap.psd1` `FunctionsToExport` must not gain private names).
5. Never hand-edit a `<!-- gen:NAME -->` block. For tools scripts run `python tools/docs-gen/regen.py --skip-crate-graph` and do not commit the regenerated block.
6. No `db/scripts/*.sql` migrations. Seeds in `db/resources/` are the source of truth.
7. Never touch the play DB `sgw`. Packet test DBs are `sgw_pp_<nn>` (worktree pp-<nn>) or the throwaway names given below.
8. Skills are edited only under `.claude/skills/`, then `python tools/agent-skills/sync.py`.
9. No design decisions: if a step does not fit reality, stop and report `blocked` with the mismatch.
10. Commit once at the end, local only (no push, no PR). Commit message ends with:
    `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_01HmEMKDEBUMeCrR5YzkAMZJ`.
11. Work in the packet worktree: `pwsh tools/build-lane/mk-worktree.ps1 persistent-player-data/<packet>-<slug> pp-<nn>`. Run every lane call with `Set-Location <worktree>;` first.
12. Report: `<worktree>\.claude\reports\persistent-player-data\<packet>-<role>.md`. Return exactly one line: `<packet> <done|blocked|failed> <sha|-> <report-path>`. The report states what was tested and what was NOT.

## Contract excerpt (what wave 3 relies on)

- Blue/green rebuild: build `sgw_next` from seeds, carry the old DB's persistent tables over, swap by rename (`sgw` -> `sgw_prev`, `sgw_next` -> `sgw`). Resources-only reseed; player tables are carried, never wiped (D3). Fail closed: any failure leaves `sgw` untouched (D7).
- Table lists: `db/persistent/reseed-public-tables.txt` (reseeded `public` tables, e.g. `shards`) and `db/persistent/persistent-public-tables.txt` (24 tables including `account`).
- `cimmeria_meta` schema holds `db_state` and `reconcile_log`. `cimmeria_seed.account` is transient. Seed accounts missing from the DB are inserted, existing accounts never touched (D6; log action `insert_seed_account`). Dangling ids: orphans deleted with JSON log, players in removed worlds relocated to Castle_CellBlock (D5). `cimmeria.image_id` is a session setting; locally passed by `PGOPTIONS`.
- Schema policy D-PP7: additive only.
- Server (wave 2): volume `cimmeria-server-pgdata`; env `CIMMERIA_BACKUP_DIR`, `CIMMERIA_DB_RESTORE`, `CIMMERIA_DB_FORCE_REBUILD`, `CIMMERIA_DB_BACKUP_KEEP`; entrypoint log prefixes `[db-sync] first-boot|skip|rebuild|adopt|restore`; 10 backups kept (D7); first deploy fresh, restore is disaster recovery only (D4); restart keeps everything (D2).
- Local interface (produced by PP-13):
  `Invoke-CimmeriaResourcesRefresh -PsqlExe <path> -Port <int> [-UseDocker] [-Database 'sgw'] [-BackupDir <path>] [-BackupKeep 3]`.
  DBs `<Database>_fresh`, `_next`, `_prev`; image id `local-<sha12>-<UTC yyyymmddThhmmssZ>`; backup `<BackupDir>/<Database>-<UTC>.dump` (default `server/backups/`, newest 3 kept); one `Write-Status` line per step.

Names from earlier waves are referenced, not redefined. If a name below (for example the reseed SQL file names under `db/persistent/`) differs in the merged wave 1-2 code, use the merged name and say so in the report.

---

## PP-13 Local resources-only refresh (agent: database-persistence)

Worktree `pp-13`, test DB `sgw_pp13_refresh` on :5433. Commit subject: `feat(bootstrap): resources-only refresh that keeps local player data`.
Depends on: wave 1 (`db/persistent/*` lists, reconcile SQL) and wave 2 (reconcile/stage SQL, `seed-accounts-stage.sql`, `seed-accounts.sql`). Read them first (Glob `db/persistent/**`) and mirror the same 10 contract steps locally.

### Files
- NEW `bootstrap/CimmeriaBootstrap/Private/Invoke-CimmeriaResourcesRefresh.ps1` (the function above; target under 400 lines).
- NEW `bootstrap/CimmeriaBootstrap/Private/Invoke-CimmeriaPsql.ps1` only if a small psql wrapper is needed (admin `-U postgres`, work `-U w-testing`, `-h localhost` when `-UseDocker`, `ON_ERROR_STOP=1`). Do not call `$PSCmdlet` in private helpers.
- NEW `bootstrap/tests/test-database-refresh.ps1`.
- DO NOT edit `Initialize-CimmeriaDatabase.ps1` (PP-14 does).

### Steps
1. Read `Initialize-CimmeriaDatabase.ps1` `Invoke-SchemaLoad` (lines 387-556) for the connection pattern: admin `& $PsqlExe @hostArgs -p $Port -U postgres`, work `-U "w-testing" -d <db>`; role `w-testing`. Reuse `Resolve-SqlIncludes` (line 316) to build `database.sql` into `<Database>_fresh`.
2. Implement `Invoke-CimmeriaResourcesRefresh` with these ordered steps, each preceded by one `Write-Status`:
   a. Refuse unless `<Database>` exists with `public.account`. Drop stale `_fresh`, `_next`.
   b. `pg_dump -Fc` of `<Database>` to `<BackupDir>/<Database>-<UTC yyyyMMddTHHmmssZ>.dump` (create dir; `pg_dump.exe` sits beside `psql.exe`); prune to newest `-BackupKeep`.
   c. Create `<Database>_fresh` owner `w-testing`, load seeds via Resolve-SqlIncludes with `ON_ERROR_STOP=1`.
   d. Create `<Database>_next` from `_fresh` (`CREATE DATABASE ... TEMPLATE`, terminating backends on `_fresh` first), then carry the old DB's persistent tables over using the lists in `db/persistent/` (pipe `pg_dump --data-only` per table, or the wave 2 carry-over SQL if one exists; use it verbatim).
   e. Run the wave 2 reconcile SQL (dangling ids, relocate) and the seed-account top-up (`seed-accounts-stage.sql` before carry, `seed-accounts.sql` after) with `PGOPTIONS="-c cimmeria.image_id=<id>"`.
   f. Terminate backends on `<Database>`, `ALTER DATABASE <Database> RENAME TO <Database>_prev`, `ALTER DATABASE <Database>_next RENAME TO <Database>`; drop `_fresh`. Any failure before this step: drop `_fresh`/`_next`, rethrow, leave `<Database>` untouched.
   g. Final `Write-Status` with image id and backup path.
3. Test script `test-database-refresh.ps1`, style of `bootstrap/tests/test-install-launcher.ps1` (`Check` function printing ok/FAIL, `$ErrorActionPreference='Stop'`, `Import-Module` the module so `$script:ScriptStartTime` exists; call private functions via `& (Get-Module CimmeriaBootstrap) { ... }`). Skip with exit 0 and a message if :5433 is unreachable or `psql` is missing.
   Case 1 (throwaway DB `sgw_pp13_refresh`, load via `Initialize-CimmeriaDatabase`-equivalent full load, never `sgw`): insert sentinel account and character with ids `0x7000_xxxx`; mutate one `resources` row; delete one seed account; run refresh. Assert: sentinel account+character survive; mutated row restored; deleted seed account back with exactly one `insert_seed_account` row in `cimmeria_meta.reconcile_log`; image id in `cimmeria_meta.db_state` starts with `local-`; backup file exists; `sgw_pp13_refresh_prev` exists.
   Case 2: drop a CHECK and insert a violating row so carry-over fails; assert the call throws and `sgw_pp13_refresh` is byte-for-byte unchanged (sentinel still there, no `_next`/`_fresh` left).
   Cleanup in `finally`: drop all `sgw_pp13_refresh*` DBs and the test backup dir.
4. Revert proof: temporarily make step 2f swap before the carry-over and show case 2 fails; restore.

### Checks (in order)
1. `Set-Location <wt>; pwsh -NoProfile -Command "[System.Management.Automation.Language.Parser]::ParseFile('<abs>\Invoke-CimmeriaResourcesRefresh.ps1',[ref]$null,[ref]$e)"` (no parse errors).
2. `pwsh -NoProfile -File <wt>\bootstrap\tests\test-database-refresh.ps1` (must not skip locally; state it ran).
3. `pwsh -NoProfile -File <wt>\bootstrap\tests\test-install-launcher.ps1` still passes.
4. Line counts: new files under 500.

### Docs owed
`.SYNOPSIS` comment block on the function only. Guides are PP-16.

### Acceptance
Both cases pass; sentinel rows survive; failure leaves DB unchanged; no edit to the 556-line file; read-only `database-persistence` pass over the diff reported before PP-14 starts.

---

## PP-14 `-ForceDatabase` keeps player data; add `-WipeDatabase` (agent: packet-coder)

Worktree `pp-14`, stacked on pp-13. Commit subject: `feat(setup): -ForceDatabase keeps player data; add -WipeDatabase`.

### Files
- NEW `bootstrap/CimmeriaBootstrap/Private/Invoke-CimmeriaDatabaseWipe.ps1` (counts, backup, drop, returns).
- NEW `bootstrap/CimmeriaBootstrap/Private/Resolve-CimmeriaDatabaseAction.ps1` (pure routing function, see Test).
- EDIT `bootstrap/CimmeriaBootstrap/Public/Initialize-CimmeriaDatabase.ps1` (net lines must be 0 or negative: move code out, not in).
- EDIT `bootstrap/CimmeriaBootstrap/Public/Invoke-CimmeriaBootstrap.ps1`.
- EDIT `setup.ps1`.
- EDIT `bootstrap/tests/test-database-refresh.ps1` (add routing cases).
- EDIT `.github/workflows/bootstrap-lint.yml`.

### Routing table (implement exactly)
| Flags | `sgw` state | Action |
|---|---|---|
| none | absent or no schema | full load |
| none | fully seeded | skip |
| none | schema present, seed incomplete, `public.account` has rows | resources refresh (PP-13) |
| none | schema present, seed incomplete, no accounts | existing replay branch |
| `-ForceDatabase` | `public.account` exists | resources refresh |
| `-ForceDatabase` | no schema | full load |
| `-WipeDatabase` | any | print counts, backup, drop `sgw`, full load |
| `-ResetDatabase` | any | unchanged behaviour, but print counts first when reachable; help text says it destroys player data |
| `-ForceDatabase` + `-WipeDatabase` | n/a | throw before any SQL |

### Steps
1. `Resolve-CimmeriaDatabaseAction -Force -Wipe -Reset -SchemaExists -SeedComplete -HasAccounts` returns one of `Throw|FullLoad|Skip|Refresh|Replay|Wipe`. Pure, no SQL. Write it from the table.
2. `Invoke-CimmeriaDatabaseWipe`: print `SELECT count(*)` for `account` and the character table (find its name in `db/persistent/persistent-public-tables.txt`), backup via `pg_dump -Fc` to `server/backups/`, then terminate backends and `DROP DATABASE`.
3. In `Initialize-CimmeriaDatabase.ps1`: add `[switch]$WipeDatabase` to params (lines 41-46). At the top (before any docker/psql work) call the resolver with only the flag arguments and throw on `-Force` + `-WipeDatabase`. In `Invoke-SchemaLoad`, replace the Force drop/recreate block (lines 442-456) with the resolver-driven calls; replace the incomplete-seed replay branch (lines 490-503) with the refresh call when accounts exist. For `-ResetDatabase` with `-UseDocker`, print counts BEFORE `Initialize-DockerPostgreSQL` removes the volume (lines 55-60). Delete code to keep the file at or under 556 lines; check `(Get-Content <f>).Count`.
4. `Invoke-CimmeriaBootstrap.ps1`: add `[switch]$WipeDatabase` to the param block (lines 71-84), a `.PARAMETER WipeDatabase` doc, update `.PARAMETER ForceDatabase` (lines 39-40) to say it refreshes resources and keeps accounts and characters, `.PARAMETER ResetDatabase` (42-43) to say it destroys player data, and pass `-WipeDatabase:$WipeDatabase` at the call on line 218.
5. `setup.ps1`: add `[switch]$WipeDatabase` to the param block (lines 209-221, beside 216-217). Update help: DATABASE FLAGS lines 51-65, `-ForceDatabase` at 54-55 (resources only, keeps characters), add `-WipeDatabase` line, DATA SAFETY lines 67-78, recipe at 90-91 ("Wipe and reload the database" becomes two recipes: refresh resources, and wipe), `.PARAMETER` blocks at 150-159, `.EXAMPLE` at 195-198, STANDALONE COMMANDS line 119. `@PSBoundParameters` already flows the new switch.
6. Tests: extend `test-database-refresh.ps1` with a block that calls `Resolve-CimmeriaDatabaseAction` for every table row (type-1 logic test, no DB), plus one end-to-end case: `Initialize-CimmeriaDatabase -Port 5433 -ForceDatabase` equivalent against `sgw_pp14_force` showing a sentinel account survives (needs a test-only hidden `[string]$DatabaseName='sgw'` parameter on `Initialize-CimmeriaDatabase`; mark it `DontShow`; the report must state whether this or the private-resolver route was used for the end-to-end case). The throw case asserts the message and that no psql was invoked.
7. `bootstrap-lint.yml`: add `db/persistent/**` to `on.pull_request.paths`; add job `database-refresh` (ubuntu-latest) with a `postgres:17` service (user postgres/password, port 5433 mapped), install `postgresql-client-17` from PGDG so `pg_dump`/`pg_restore` major is at least the server's, install `pwsh` if absent, create role `w-testing` superuser, run `pwsh -NoProfile -File bootstrap/tests/test-database-refresh.ps1`. The test must not skip in CI: add an env `CI_REQUIRE_PG=1` branch that fails instead of skipping. Leave the `validate` and `install-launcher` jobs unchanged. Windows PG availability is unverified, so use ubuntu and say so.

### Checks
1. Parse all changed ps1 files.
2. `pwsh -NoProfile -File <wt>\bootstrap\tests\test-database-refresh.ps1`.
3. `pwsh -NoProfile -File <wt>\bootstrap\tests\test-install-launcher.ps1`.
4. `pwsh <wt>\setup.ps1 -?` shows `-WipeDatabase`; run `Get-Help` smoke only, no builds.
5. `(Get-Content <wt>\bootstrap\CimmeriaBootstrap\Public\Initialize-CimmeriaDatabase.ps1).Count` is 556 or fewer.
6. Revert proof: remove the throw and show the routing test fails.

### Docs owed
Help text only; guides are PP-16.

### Acceptance
Routing table matches; mutual exclusion throws before SQL; `Initialize-CimmeriaDatabase.ps1` not larger; CI job added.

---

## PP-09 Persistent player data runbook (agent: documentation-writer)

Worktree `pp-09`. Subject `docs(ops): persistent player data runbook`. Prerequisite: wave 2 merged (read `docker/compose.yml` and `docker/` scripts for the real flag names; if they differ from the contract names, use the real ones and report).

### Files and exact edits
1. NEW `docs/operations/persistent-player-data.md` (CRLF), Diátaxis how-to, sections: What persists (D1, D2); Normal update and restart; Backups (location from `CIMMERIA_BACKUP_DIR`, newest 10, how to list); Restore (disaster recovery only, `CIMMERIA_DB_RESTORE`, D4); Forced resources rebuild (`CIMMERIA_DB_FORCE_REBUILD`, D3); Reading `[db-sync]` log lines (`first-boot|skip|rebuild|adopt|restore`); Fail-closed behavior and what to do (D7); Docker volume `cimmeria-server-pgdata` and never `docker volume prune` blindly. Every command in PowerShell with absolute paths; mark commands not run on the colo as untested.
2. EDIT `docs/operations/colo-deploy.md`: replace section `## The database resets on every start` (lines 134-142) with `## The database persists across restarts` covering paragraphs L136, L138, L140, L142 (entrypoint copy-over, watchtower anonymous volume, "mounting a volume does not make the database persistent", 04:30 reboot). New text: data lives in the named volume `cimmeria-server-pgdata`; restart and reboot keep accounts and characters; resources reseed on image change; link the new runbook. Fix line 242 (anonymous volume sentence: add that the named `cimmeria-server-pgdata` is never pruned). Check lines 102-120 (setup steps 4-5) get a step mentioning the named volume and first-boot fresh seed, and line 128 (stop grace) notes the DB shutdown time.
3. EDIT `docs/operations/container.md`: L53 (no-volume bullet) to describe the named volume; `## Volume / persistence` (lines 112-131) rewritten (L114 blockquote, L116, L118, L120, L122, and `### Reset to a fresh server` lines 124-131 with the new wipe path); env table (~line 104) gains the four `CIMMERIA_*` vars; L138, L144, L150-151, L254 corrected; `## Healthcheck` (line 155) mentions the longer start-period; `## CI login smoke` (line 164) unchanged except add one line for the persistence smoke (`tools/container-persistence-smoke.sh`).
4. EDIT `docs/operations/telemetry.md` lines 282-287 and 367-369: replace "reseeds its database on every start" with "keeps accounts and characters across the restart; resources reseed".
5. EDIT `docs/readme.md`: add an index row for the new runbook beside the other operations docs (Grep `colo-deploy` in `docs/readme.md` to find the table).

### Checks
`python tools/docs-gen/regen.py --skip-crate-graph` (do not commit its block); CRLF check on every edited file; Grep that the phrases "resets on every start", "reseeds pgdata", "reseeds its database on every start" return no hits in `docs/operations/`; links resolve (Grep each new anchor).

### Acceptance
No stale reset claim in `docs/operations/`; new doc indexed; doc-update-map rows listed in the report (Ops runbooks, Top-Level Documents, CI triggers).

---

## PP-10 Reference doc and rule updates (agent: documentation-writer)

Worktree `pp-10`. Subject `docs: persistent player data reference and rule updates`.

### Files and exact edits
1. NEW `docs/architecture/persistent-player-data.md` (reference + decisions D-PP1..D-PP12 summarized with owner decisions D1-D8; link README and runbook). Index row in `docs/readme.md`.
2. `docs/agents/rules-and-gotchas.md`: line 46 (authoring-commands bullet, "Deploys rebuild the database from seeds, so a live-only write is lost.") -> "Deploys reseed resources only and keep player data, so a live-only authoring write is still lost on the next resources reseed until committed as seed SQL." Line 50 (seed bullet, "There is no long-lived production database to migrate: every environment, including the colo, is rebuilt from seeds.") -> the colo now keeps a long-lived DB; resources reseed from seeds; the persistent schema is additive-only (D-PP7), see the new architecture doc. Do not touch `development-workflow.md:254` (PP-15).
3. `docs/architecture/dev-console-channel.md` lines 92-96 ("The write is **transient** ... the next deploy rebuilds from seeds and wipes it"): the write to resources tables is wiped by the next resources reseed; player-table writes persist.
4. `docs/commands.md` lines 570-573 ("the next deploy will wipe it"): same fix, keep the bold sentence structure.
5. `docs/guides/placing-npcs-and-objects.md` lines 25-26 ("or the next server update wipes it"): "or the next server update's resources reseed discards it".
6. `docs/gameplay/character-creation.md` L159 and L163: replace "rebuilds its database from this seed on every deploy/start" with: seed accounts are inserted if missing and characters persist; seeded characters are no longer reset each deploy. Drop the K24 sentence in L163 only if PP-12 has not yet; otherwise leave K24 for PP-12.
7. `docs/architecture/discord-notifications.md` L311 ("repeats on every deploy") and L316 (spawner row, "restarts on every deploy"): L316 is a restart, not a DB reset; leave unless it claims a reset, and list the decision in the report.
8. `.claude/skills/seed-change/SKILL.md`: L10 ("every deploy) is built from it via db/database.sql") -> "resources are reseeded from it"; L38-41 (reload-db rebuilds `sgw_<worktree>`, main checkout uses `sgw`) leave for PP-15 and note it; L82 "Say 'takes effect on the colo at the next deploy'" keep (still true for resources); L85 check. Then `python tools/agent-skills/sync.py`. `.claude/skills/lane-build/SKILL.md:53`: no edit unless it names the main DB.
9. The two agent-memory files `.claude/agent-memory/main-session/reference_lab_parallel_clients_2026_10_10.md` and `.claude/agent-memory/movement-teleport-advisor/pr1-bounds-seam.md` hold no DB-reset claim: Grep to confirm and make NO edit.
10. `TESTING.md`: L374 (`### Test-DB hygiene`) add the main checkout test DB is `sgwtest` after PP-15 (confirm PP-15 merged, else leave); L420-431 keep; `### CI (container image)` at L451-453 add `db/sgw/**`, `db/persistent/**`, `tools/container-persistence-smoke.sh` to the trigger list and one sentence: the persistence smoke restarts the container and checks a sentinel account and character survive; the smoke does not enter the world.

### Checks
CRLF on all edited docs (LF line endings in the two `.claude` files are fine, keep their existing style); Grep for "wipes it", "rebuilds its database", "rebuilt from seeds" across `docs/` and list every remaining hit in the report with keep/changed.

### Acceptance
No contradictory claim remains in the listed files; reference doc indexed; skills synced.

---

## PP-11 Persistent schema snapshot guard (agent: packet-coder)

Worktree `pp-11`, test DB `sgw_pp_11`. Subject `test(db): persistent schema snapshot guards additive-only changes`. Depends on PP-02 (the persistent table lists and where PP-02 put its lint: use that crate; precedent `crates/content-engine/tests/it/interact_tag_linter.rs`).

### Files
- NEW `db/persistent/persistent-schema.snapshot.json` (generated, committed).
- NEW test in PP-02's crate `tests/it/persistent_schema_snapshot.rs` and its `mod` line in that crate's `tests/it/main.rs`.
- NEW generator `tools/persistent-schema-snapshot.ps1` (writes the snapshot from a live DB; one-line `.SYNOPSIS`; add a row to `tools/README.md#tools-catalog`, then `python tools/docs-gen/regen.py --skip-crate-graph`).

### Steps
1. Snapshot shape: for each table in `persistent-public-tables.txt`, sorted columns `{name, data_type, is_nullable, column_default}` from `information_schema.columns`, sorted constraint names with type, sorted index names.
2. Test (type 1, live-DB test through `live-db-test.ps1`, DB `sgw_pp_11`): load current schema, build the same structure, compare to the snapshot. Rule: every snapshot column/constraint/index must still exist with identical type and nullability (no drop, no type change, no nullable -> not-null without default); additions are allowed only if the snapshot is regenerated, so the test also fails when the live schema has an addition not in the snapshot, with the message "run tools/persistent-schema-snapshot.ps1 and commit; additive only (D-PP7)".
3. Revert proof: drop a column in a scratch copy of the snapshot logic input (test helper takes the live schema as a value) and assert the diff function reports it; a unit-level test of the diff function (no DB) covers drop, type change, addition.

### Checks
`pwsh C:\...\tools\build-lane\lane.ps1 cargo fmt -p <crate> -- --check`; `... cargo clippy -p <crate> --all-targets`; `... cargo test -p <crate>` (diff unit tests); live-DB run through `live-db-test.ps1` per `docs/architecture/integration-test-infra.md` for the snapshot test.

### Docs owed
`TESTING.md` test-inventory line for the new test; `tools/README.md` row.

### Acceptance
Snapshot committed; unit test fails if the diff function stops flagging drops; live test passes.

---

## PP-16 `-ForceDatabase` documentation sweep (agent: documentation-writer)

Worktree `pp-16`, after PP-14. Subject `docs: -ForceDatabase refreshes resources and keeps characters`.

### Exact edits (new wording: "-ForceDatabase refreshes the resource tables and keeps accounts and characters; -WipeDatabase backs up and deletes everything; -ResetDatabase also removes the Docker volume")
- `db/README.md` lines 71-72 (`# Database only (wipe and reload):` / `pwsh setup.ps1 -SkipBuild -ForceDatabase`): retitle "refresh resources, keep characters", add a `-WipeDatabase` snippet. (The ledger calls this "the colo sentence"; there is no colo sentence in this file.)
- `docs/troubleshooting.md` lines 229-239: entry command at L236 stays; L239 prose "drops and recreates the `sgw` database before loading... escalate to `-ResetDatabase`" -> refreshes resources; for a corrupt DB use `-WipeDatabase`, then `-ResetDatabase`.
- `docs/building.md` L48 (flag table `-ForceDatabase`) and L52 (`-SkipBuild` "useful with `-ForceDatabase` to re-seed"); add a `-WipeDatabase` row.
- `docs/guides/getting-started.md` L94.
- `bootstrap/README.md` L41 (`Initialize-CimmeriaDatabase -Force   # Wipe and reload database`) and L74-75; add `-WipeDatabase`.
- `docs/guides/write-a-database-migration.md` L17 ("rebuilt from seeds" note), L53, L135, L189 (command stays; fix the "wipes" context).
- `bootstrap/CimmeriaBootstrap/README.md` and `docs/analysis/castle-cellblock-rebuild/uat-guide.md:36`: Grep `ForceDatabase` in each; edit any "wipe/drop and recreate" wording the same way; report if none.

### Checks
CRLF; Grep `ForceDatabase` across `docs/`, `bootstrap/`, `db/`, `README.md` and list each hit as changed or kept-with-reason; no remaining "wipe and reload" next to `-ForceDatabase`.

### Acceptance
Every `-ForceDatabase` mention agrees with PP-14 behavior.

---

## PP-12 Close-out (agent: documentation-writer)

Worktree `pp-12`. Subject `docs: persistent player data close-out`. Runs once, after PP-U1 results are recorded.

### Exact edits
1. `docs/guides/unified-uat.md`: row K4 (line 84, "**The colo database is wiped on every deploy.** ...") -> mark fixed/removed per the file's convention for retired known issues (read lines 75-119 first and follow it); line 124 ("rebuilds its database from the repository seed on every deploy (K4)") -> accounts and characters persist; line 1164-1165 rows DA-U43/DA-U44 reference K24 (seeded characters are level-1 again after deploy): update expectation to "a character you progressed keeps its state; seed accounts are only inserted if missing"; keep step ids. Add rows or a section for PP-U1 steps copied from the UAT script below, keeping the checklist format.
2. `docs/project-status.md`: once only. Edit line 70 (Database persistence row: "No migration framework yet") and the World state row at line 140 (note deploy no longer resets characters) to mention persistent player data. Do NOT touch `<!-- gen: -->` blocks; if the rows are inside one, stop and report.
3. `docs/gap-analysis/infrastructure.md` or `server-infrastructure.md`: Grep for "persist" rows; update the one that claims the DB is rebuilt on deploy (report which).
4. `docs/analysis/persistent-player-data/README.md`: mark packets merged; link results.
5. Retire worktrees per the usual flow is the orchestrator's job, not this packet's.

### Checks
CRLF; Grep "K4" and "K24" across `docs/` and list hits; regen script run, generated blocks not committed.

### Acceptance
Known-issue table, status and gap docs agree with the shipped behavior; UAT rows present.

---

## PP-U1 Colo UAT script (owner/lab-gated; ask the user before any lab use)

Not run by a coder. Steps, each with expected result and what to record:
1. Before the first persistent deploy, note the character names and levels of one dev account (D1). Record in the result.
2. Deploy the PP image. Expect `[db-sync] first-boot` in the container log (D4: fresh from seeds), seed accounts and dev/lab accounts present.
3. Log in, create or use a character, gain a visible change (move item, change position, spend ability points). Record the name.
4. `docker restart cimmeria` on the colo. Expect `[db-sync] skip`. Log in again: character and change are intact (D2).
5. Deploy again with a changed resources row (a trivial seed edit, agreed in advance). Expect `[db-sync] rebuild`; the character persists, the resources change is live.
6. Delete a seed account row via an agreed psql command on the test setup (not on production data); redeploy; expect it reinserted and `insert_seed_account` in `cimmeria_meta.reconcile_log` (D6).
7. Check backups: newest 10 retained under `CIMMERIA_BACKUP_DIR`.
8. Fail-closed check (only if safe on a copy): not part of colo UAT; covered by PP-U2.
Results go in `docs/analysis/persistent-player-data/` and feed PP-12. Mark any step not executed as untested.

## PP-U2 Fail-closed and restore drill (optional, scratch container only)

Follow the PP-09 runbook on a scratch container, never the colo: corrupt the carry-over (drop a CHECK), expect the entrypoint to keep serving the old DB or fail closed per D7 with a clear log line; run the `CIMMERIA_DB_RESTORE` path from a backup and confirm characters return. Record raw output.

---

## Design issues

1. Ledger cites `rules-and-gotchas.md:45,50`; the stale lines are 46 and 50 (45 is the `.`-console bullet).
2. Ledger says `db/README.md:71` is "the colo sentence"; lines 71-72 are the `-ForceDatabase` snippet (PP-16), and the file has no colo statement.
3. The two agent-memory files named in PP-10 contain no DB-reset claim ("seed" and "reseed-only" hits are unrelated); the stale memory is only in the user's out-of-repo auto-memory. PP-10 confirms and edits nothing there.
4. `character-creation.md` L159/L163 and `unified-uat.md` K4/K24/DA-U43/DA-U44 are stale but not in the ledger file lists; assigned to PP-10 (character-creation) and PP-12 (UAT guide).
5. The ledger's "K24" is the seeded-characters known issue; the colo wipe known issue is K4 (line 84, anchor `#the-database-resets-on-every-start`). PP-09 renames that heading, so PP-12 (or PP-09) must update the K4 link anchor, or it breaks.
6. `Invoke-SchemaLoad` is not a CmdletBinding function and uses `$PSCmdlet.ShouldProcess` via dynamic scoping; extracted private helpers must not call `$PSCmdlet`.
7. `Initialize-CimmeriaDatabase.ps1` is 556 lines against a 557 cap quoted in the ledger; any growth breaks the soft-cap rule, so PP-14 must be net-negative.
8. With `-UseDocker`, psql needs `-h localhost`; `-ResetDatabase` removes the Docker volume inside `Initialize-DockerPostgreSQL` before `Invoke-SchemaLoad`, so PP-14 must print counts before that call, not inside the schema load.
9. The `-ForceDatabase` + `-WipeDatabase` throw must run before `Install-CimmeriaDependencies` side effects; the ledger only says "before any SQL".
10. The CI refresh job needs `pg_dump` of at least the server major; Windows runner Postgres availability is unverified, so the packet uses ubuntu. The ledger names no runner.
11. PP-14's end-to-end test needs a hidden `-DatabaseName` parameter or a private-function route; the ledger leaves it open and the packet lets the coder pick but requires the report to say which.
12. Wave 1-2 names (reseed SQL file names under `db/persistent/`, compose flag names, PP-02's lint crate) were not available when this was written; packets say to use the merged names and report differences.
13. discord-notifications.md L316 ("restarts on every deploy") is a restart statement and may be accurate; PP-10 judges it.
