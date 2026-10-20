# Castle continuation: resume wave R1 packets and testing isolation

> **Type:** Reference. **Audience:** Castle orchestrator, ops, packet workers and reviewers.
> **Last updated:** 2026-10-17.
> **Companions:** [overlay ledger](README.md), [resume card](RESUME.md), [architect report](architect-report.md), [Castle packet menu](../four-zone-planning/castle-work-packets.md), [execution contract](../four-zone-planning/execution-contract.md), branch ledger `content/castle-campaign:docs/analysis/castle-continuation/{README.md,work-packets.md}`.

Main-checkout overlay, never committed to `main` (D-CA2). CS-INT0 copies this file to the branch as `docs/analysis/castle-continuation/resume-packets.md`. The branch `work-packets.md` (356 lines) stays as the gate contract and seal record; this file adds only the resume wave and the isolation recipe.

## Dispatch rules

- One packet, one worker, one worktree, one test DB. Worktrees live at `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\castle-<packet>`, and their DB is `sgw_castle_<packet>` (for example `sgw_castle_cs_00ra`). The integration worktree `zone-castle` uses `sgw_zone_castle`.
- Ops creates each worktree from the main checkout with `pwsh tools/build-lane/mk-worktree.ps1 <branch> <name>`, one ops dispatch per worktree. The packet's step 0 then moves it onto the feature SHA (see the [base check](#base-check)).
- Follow the D-CA6 review rule: a read-only `packet-reviewer` per wave, fixes by a fresh `packet-coder` or domain agent, then a fresh re-review, at most 3 rounds. Never send fixes back to the implementer.
- Integration uses cherry-picks onto `content/castle-campaign` in `zone-castle`, done by a `packet-coder` integration packet (CS-INT1). The ledger records each SHA. Retire each packet worktree with `pwsh tools/build-lane/rm-worktree.ps1 --force <name>`, except `castle-cs-01b`, which keeps the server and DB until CS-01c ends.
- Cellblock is live. Never touch `zone-cellblock`, `cellblock-*` worktrees, any `sgw_zone_cellblock`/`sgw_cellblock_*` DB, Cellblock's server process, `docs/analysis/cellblock-continuation/` or `.claude/state/active-campaign`.
- Size: a packet-coder packet takes about 40k working tokens. A specialist packet takes about 50k; if it runs past that, it commits what is sealed and returns `blocked`, naming the remainder.

### Repo rules every packet carries

1. Use PowerShell only, never bash. Use absolute paths. Start every shell call with `Set-Location <worktree>;`.
2. Never run `cargo` directly. Run `pwsh tools/build-lane/lane.ps1 cargo <cmd> -p <crate>` in the foreground. Read the lane's failures file instead of rerunning. `cargo fmt --all -- --check` does not compile and may run directly.
3. Use `pwsh tools/build-lane/live-db-test.ps1 <filter>` for live-DB tests. It reloads this worktree's own DB.
4. No `git stash`, no `git worktree prune`, no push, no PR. Stage files by explicit path, never `git add -A`. Commit once with the packet's subject, plus the attribution trailer `Co-Authored-By: Claude <noreply@anthropic.com>`.
5. Edit only the packet's allowlist. If you need a file outside it, stop and return `blocked`.
6. Files under `docs/` are CRLF. After writing, run `$t=[IO.File]::ReadAllText($p) -replace "(?<!`r)`n","`r`n"; [IO.File]::WriteAllText($p,$t)` with an absolute `$p`. Then run `git diff --check`.
7. Files have a soft cap of 500 lines and a hard cap of 700. Split a new doc before it reaches 500.
8. Seeds are the source of truth: edit `db/resources/`, never `db/scripts/`. No packet in R1 edits seeds.
9. Never write an IP address other than loopback, a password, a token or an account password into any file.
10. Report and return: write the full report at the path in your brief, then return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.

## Contract excerpt

- Feature branch: `content/castle-campaign` in `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle`. `FEATURE_SHA` is its tip after CS-INT0, and `RESUME.md` records it. Packet branch: `castle/<packet-lower>-<slug>`.
- Statuses: Ready / BlockedDependency / BlockedDecision / BlockedEvidence / Writing / Review / Integrated / UATPending / Done.
- Provenance labels, one per claim: `SHIPPED_DATA`, `RECOVERED_SCRIPT`, `CLIENT_MAP`, `RECONSTRUCTION` (bounded, O14-03), `ORAL_HISTORY`, `CIMMERIA_AUTHORING`. A claim with no proof is `UNRESOLVED (<gate>)`. A blank field or a guessed mechanic is never a resolution.
- ID blocks: Castle restoration chains come from 1366-1400. 1366-1370 are claimed and 1371-1400 are free. Authored chains come from 10000-10999. Any other new ID (spawn, template, item, dialog, objective) needs a no-collision check across `db/resources/` on `FEATURE_SHA` and on `content/cellblock-campaign`, `content/harset-campaign` and `content/tollana-campaign` (`git -C <wt> grep -n <id> <branch> -- db/resources`).
- `MissionOverride` patches only missions already in `CookedDataMissions.pak` (`crates/resources/src/base/resources/apply_overrides.rs:236-245`). 1520 and 1521 ship in the pak, so they need no catalog insertion.
- The client is evidence, and so are the docs. A ticket or draft is a claim. Check `docs/` and `docs/protocol/*-dispatch-table.md` before Ghidra.

### Base check

This is step 0 of every packet except CS-INT0. ops runs `mk-worktree` at `origin/main`, so the new branch must be moved onto the feature tip before any work:

```powershell
Set-Location <wt>; git status --porcelain            # must be empty
git -C <wt> rev-list --count origin/main..HEAD       # must be 0 (no own commits yet)
git -C <wt> rebase --onto <FEATURE_SHA> origin/main <branch>
git -C <wt> rev-parse HEAD                           # must equal <FEATURE_SHA>
git -C <wt> merge-base HEAD <FEATURE_SHA>            # must equal <FEATURE_SHA>
```

If any value differs, stop and return `blocked`.

## Testing isolation

Castle and Cellblock share one workstation, one Postgres (`localhost:5433`) and one lab daemon (`cimmeria-lab.exe`, `127.0.0.1:8779`). Castle never shares a worktree, DB, server process or lab instance with Cellblock.

### Databases

- `tools/build-lane/reload-db.ps1` names the DB `sgw_<worktree dir>`, lowercased, with non-`[A-Za-z0-9_]` characters turned into `_`. `CIMMERIA_TEST_DB` overrides the name, `PGPORT` defaults to `5433` and `PGPASSWORD` defaults to `w-testing`. psql lives at `C:\Users\Steve\source\projects\Cimmeria\external\postgresql_server\bin\psql.exe`.
- Castle DBs:
  - each packet: `sgw_castle_cs_<id>`;
  - integration: `sgw_zone_castle`;
  - the lab server: `sgw_castle_cs_01b`.

  None collides with Cellblock's `sgw_zone_cellblock` or `sgw_cellblock_*`.
- **Never run `reload-db.ps1` or `live-db-test.ps1` in `castle-cs-01b` while the Castle server runs.** Both drop and recreate the DB under it.

### The Castle server instance

The server is configured only by environment variables (`crates/server/src/main.rs:9-53`, read at `:419-498`). It writes logs to `<cwd>\logs\`. With no `config\discord.toml` in the worktree, Discord stays off. Of the listeners:

- `AUTH_PORT` (13001) and `CELL_PORT` (50000) bind nothing (`docs/operations/container.md:62,73,78`).
- Minigame SmartFox binds `0.0.0.0:30000` and has **no port env var** until CS-INF1 lands.
- Do not use 8444, because `cimmeria-supervisor` takes it by default.

| Listener | Env var | Default (Cellblock/default server) | Castle |
|---|---|---|---|
| Login HTTP (SOAP, dev-session mint) | `LOGON_PORT` | 8081 | 18081 |
| BaseApp UDP | `BASE_PORT` | 32832 | 32932 |
| Admin REST (loopback) | `ADMIN_PORT` | 8443 | 18443 |
| Lab MCP | `CIMMERIA_LAB_MCP_BIND` (+ `CIMMERIA_LAB_MCP_TOKEN`, ≥32 bytes) | 127.0.0.1:8451 (local-setup user env) | 127.0.0.1:8452 |
| Minigame TCP | `MINIGAME_PORT` (after CS-INF1) | 30000 | 30100 |
| Telemetry upload base | `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` | `http://localhost:8443/api/telemetry` | `http://127.0.0.1:18081/api/telemetry` |
| DB | `DB_URL` | `dbname=sgw` | `dbname=sgw_castle_cs_01b` |
| SigNoz split | `CIMMERIA_DEPLOY_ENV` | `dev` | `dev-castle` |

Build, check and start the server from the `castle-cs-01b` worktree in one PowerShell call. The token and the HMAC secret come from the User env that `lab local-setup` wrote. Never print them.

```powershell
Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\castle-cs-01b
pwsh tools/build-lane/lane.ps1 cargo build -p cimmeria-server --release   # note the target dir it prints
Copy-Item "$env:CIMMERIA_TARGET_ROOT\castle-cs-01b\release\cimmeria-server.exe" .\cimmeria-server.exe -Force
$env:DB_URL = 'host=localhost port=5433 user=w-testing password=w-testing dbname=sgw_castle_cs_01b'
$env:LOGON_PORT = '18081'; $env:BASE_PORT = '32932'; $env:ADMIN_PORT = '18443'; $env:MINIGAME_PORT = '30100'
$env:CIMMERIA_LAB_MCP_BIND = '127.0.0.1:8452'
$env:CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT = 'http://127.0.0.1:18081/api/telemetry'
$env:CIMMERIA_DEPLOY_ENV = 'dev-castle'
.\cimmeria-server.exe --check-db                                           # exit 0 required
$p = Start-Process -FilePath .\cimmeria-server.exe -WorkingDirectory (Get-Location) -WindowStyle Hidden -PassThru
$p.Id                                                                      # record it; stop with Stop-Process -Id <pid>
```

If `CIMMERIA_TARGET_ROOT` is unset, the exe is at `<wt>\target\release\`. Use the target dir the lane printed. The bootstrap `Start-CimmeriaServer.ps1` is not usable for this, because it hardcodes 8081/32832 and `target\<profile>`.

### Lab routing

- **Lab client to the Castle server.** The login row comes from `<install>\SGWGame\Content\UI\Startup\Login\LoginInternal.lua`. The `server` field in `<install>\Binaries\sessions\lab-account.<instance>.json` picks the row ([lab contributor setup](../../guides/lab-contributor-setup.md), [live research lab, parallel clients](../../guides/live-research-lab.md#parallel-clients-up-to-five)). There is no `lab_server_switch`, and `CIMMERIA_LAB_SERVER_URL` in `labd.env` would move every instance. Castle therefore needs a `CastleLocal` row (`LoginMod.servers["CastleLocal"]="http://127.0.0.1:18081"`), plus instance account files for the player and observer naming `"server": "CastleLocal"`. The proposal uses instances `p4` (account `lab4`) and `p5` (`lab5`), off `p2`, which the two-player UAT rows use. This edits shared lab install state: **D-CA9, BlockedDecision**.
- **The lab-server MCP.** The server's in-process MCP is what `server_sessions` and similar tools query. Its URL comes from `CIMMERIA_LAB_MCP_URL` when a Claude session starts. `.mcp.json` falls back to the colo host's `:8444/mcp` URL (not recorded here), which is why it was ECONNREFUSED. The daemon's own `CIMMERIA_LAB_MCP_URL` in `labd.env` drives `lab_timeline` and the `server`/`packet` clauses of `lab_uat_run`. It is single-valued, so Castle leaves it alone and expects those clauses to be UNVERIFIED. The verifier queries the Castle lab-server MCP directly instead.
- **What must be started before CS-01c:**
  1. the Castle server (CS-01b), left running;
  2. the Castle orchestrator session, restarted with `CIMMERIA_LAB_MCP_URL=http://127.0.0.1:8452/mcp` and the same `CIMMERIA_LAB_MCP_TOKEN` in its environment, then `/mcp` reconnected;
  3. `lab status` and `lab instances status`, showing `p4` and `p5` hosted (`CIMMERIA_LAB_INSTANCES` lists them) and not leased by Cellblock;
  4. the D-CA9 login row and account files in place.

  If any of these is missing, CS-01c stays UATPending (O14-09).
- Lab use is authorized for this campaign (brief; O14-08). Take the lease per instance and release it after each run. Memory gotchas: stand 4-6 m off targets, clear dialogs first, and check every driver claim against `logs\server.log`.

## Wave R1

The order is:

1. CS-INT0.
2. CS-00Ra, CS-00Rb, CS-01a and CS-INF1 in parallel (R1a).
3. One read-only reviewer for R1a, then fixes and re-review (D-CA6).
4. CS-INT1 cherry-picks R1a and records the new `FEATURE_SHA`.
5. CS-01b on that SHA (R1b), with its own small review.
6. CS-01c.

### CS-INT0 - resume the integration branch

**Worker:** `packet-coder`. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle` (existing). **Branch:** `content/castle-campaign`. **Gate:** D-CA1 answered "adopt". **Commit:** `docs(castle): resume campaign after main merge`.

**Allowlist:**
- the merge commit;
- `docs/analysis/castle-continuation/README.md`, `RESUME.md` and `resume-packets.md` (new, copied from this overlay file);
- `docs/analysis/castle-continuation/worknotes/CS-INT0.md`.

Never stage `castle-pg.log`.

1. `Set-Location <wt>; git status --porcelain`: the only untracked file may be `castle-pg.log`. Then run `git fetch origin` and `git merge --no-ff origin/main -m "Merge origin/main into content/castle-campaign"`. On any conflict, run `git merge --abort` and return `blocked`, listing the conflicted paths.
2. `pwsh tools/build-lane/reload-db.ps1` must print `reload-db: OK ... into sgw_zone_castle`.
3. `pwsh tools/build-lane/lane.ps1 cargo check --workspace --all-targets --exclude cimmeria-app --exclude cimmeria-content-editor --exclude cimmeria-scene-editor --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab`. It must report `status=ok`.
4. Port the overlay into the branch ledger:
   - README: append overlay findings F1-F9 as a dated "2026-10-17 resume" section, and decisions D-CA1..D-CA10 to its Decisions table. Replace the packet status rows with the overlay's.
   - RESUME: rewrite it from the overlay `RESUME.md`, with the new `FEATURE_SHA` set to the merge commit. It stays at most 40 lines.
   - Copy this file to `resume-packets.md`.
   - Convert each to CRLF (rule 6) and keep it under 500 lines.
5. In `worknotes/CS-INT0.md`, record the merge SHA, the `reload-db` line, the lane summary line and `git diff --check`. Stage only the allowlist, then commit.

Report path: `<wt>\.claude\reports\castle\CS-INT0-coder.md`. The returned commit SHA is the new `FEATURE_SHA`.

### CS-00Ra - seal G-MED (mission 1520 dispenser, minigame, radio)

**Worker:** `game-archaeology-specialist`. **Worktree:** `castle-cs-00ra`. **Branch:** `castle/cs-00ra-g-med`. **DB:** `sgw_castle_cs_00ra` (only if a seed query is needed). **Commit:** `docs(castle): seal G-MED evidence for mission 1520`.

**Allowlist:** `docs/analysis/castle-continuation/evidence/g-med.md` (new, under 400 lines) and `docs/analysis/castle-continuation/worknotes/CS-00Ra.md`. Docs only: no runtime, seed, test or shared planning edits.

**Read first:**
- branch `work-packets.md` §§ "CS-02/03 entry-only seal", "2026-10-14 stock prop supplement; no binding seal", "CS-02/03 conditional graph candidates" and "CS-01 offline route boundary";
- branch `evidence/cooked-interaction-maps.md`, `evidence/seed-excerpts.md`, `evidence/dialog-excerpts.md` and `worknotes/CS-02b2.md`;
- `docs/gameplay/minigame-system.md`;
- the `re-lookup` skill, before Ghidra;
- the client tree `C:\Users\Steve\source\projects\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC`.

**Answer each question** as a row: value, provenance label, proof (file:line, map actor path or Ghidra address), and confidence.
1. Which Castle map actor or event-set object is the Med Dispenser (moniker 8896) that mission 1520 targets? Give its map package, actor name, location, and how the client targets it: a server-spawned entity or a map-bound interactable.
2. Which minigame type, if any, does the dispenser start, and with which parameters? Cite the minigame-system tables.
3. What starts the 1520 radio beat (objective 5174, dialog 4985, map 5826)? Which step or objective transition fires it, and on which actor?
4. Which step/objective transitions for steps 4452-4454 and objectives 5171-5174 are now proven, and which remain inferred?
5. Where the evidence ends, propose a bounded `RECONSTRUCTION` (O14-03). Give it exact candidate IDs and a no-collision check per the contract excerpt, including the command and its output.

**Acceptance:**
- Every row is proven or `UNRESOLVED (<gate>)` with the exact missing evidence named.
- One negative claim is checked per provenance class.
- There are no guessed IDs.
- `git diff --check` is clean, and the files are CRLF.
- The worknote lists the sources read and the open questions for CS-02b2's split.

Report: `<wt>\.claude\reports\castle\CS-00Ra-archaeology.md`.

### CS-00Rb - seal G-SYM (mission 1521 tank, minigame, implant)

**Worker:** `game-archaeology-specialist`. **Worktree:** `castle-cs-00rb`. **Branch:** `castle/cs-00rb-g-sym`. **Commit:** `docs(castle): seal G-SYM evidence for mission 1521`.

**Allowlist:** `docs/analysis/castle-continuation/evidence/g-sym.md` (new, under 400 lines) and `worknotes/CS-00Rb.md`. Docs only.

**Read first:** the CS-00Ra list.

**Answer each question** in the same row format as CS-00Ra:
1. Which map actor is the Symbiote Tank (moniker 8897), and how does the client target it?
2. Which minigame type and parameters does it use, if any?
3. How does the symbiote implant behave: which item, its pouch or slot, and its grant and consume points? This is the CS-00 exit's "symbiote item/pouch" owner question; report the evidence and leave the decision to the owner.
4. What triggers Moh'katan's report for 1521, and on which objective?
5. Which transitions for steps 4455-4458 and objectives 5175-5178 are proven, and which are inferred?
6. Propose a bounded `RECONSTRUCTION` with a collision audit where the evidence ends.

The owner choice in question 3 goes into the report as a proposed `BlockedDecision` row (D-CA-SYM), never as a decided value. Acceptance matches CS-00Ra.

Report: `<wt>\.claude\reports\castle\CS-00Rb-archaeology.md`.

### CS-01a - CS-01 walk step file

**Worker:** `mission-systems-advisor`. **Worktree:** `castle-cs-01a`. **Branch:** `castle/cs-01a-walk-steps`. **Commit:** `docs(castle): CS-01 walk step file`.

**Allowlist:** `docs/analysis/castle-continuation/uat/CS-01-walk.md` (new, under 450 lines) and `worknotes/CS-01a.md`.

**Read first:**
- `db/resources/Content/Seed/castle_701_chains.sql`, `castle_702_704_chains.sql` and `castle_706_708_chains.sql`, plus chain 1109 in `castle_cellblock_chains.sql:2596-2628`;
- the Castle rows of `db/resources/Worlds/Seed/spawnlist.sql`;
- the Castle section of `docs/content/mission-chains.md`;
- `docs/analysis/playtests/2026-09-18-colo-castle/`;
- branch `uat/CS-01.md` and branch `work-packets.md` § "CS-01 offline route boundary";
- the lab-driver tool list in `.claude/agents/lab-driver.md`;
- `docs/guides/live-research-lab.md`;
- the lab-server MCP tool names in `crates/lab-mcp/` (Glob for the tool registry).

**Write a step file a Haiku `lab-driver` can follow without judgment.** It needs:
- **Front-loaded gotchas.** Use a lease per instance (`p4` player, `p5` observer). Stand 4-6 m off targets. Clear dialogs before moving. Never trust a driver claim without a server-log line.
- **Preconditions.** Two characters, ordinary and Jaffa, with archetypes named from the seeds. The GM setup commands (`.gotolocation Castle`, mission grant or reset) come from `docs/` or the GM command inventory; never invent one.
- **Numbered steps** from Cellblock arrival (chain 1109 `cross_world_teleport` fallback) through missions 701, 702, 703, 704, 706 and 708, then the Harset gate. Each step gives:
  - the exact tool call and arguments;
  - the coordinates or entity tag;
  - the expected client observation;
  - the server-side check: the lab-server MCP tool and field, or a `logs\server.log` grep string;
  - a PASS or FAIL rule.
- **Relog points** at the Zuritska rescue and after the DHD.
- **An observer check** on `p5` for shared actor presentation, including the duplicate Zuritska.
- **The G-ROUTE natural walks:** Infirmary to the chamber cluster (398.83,55.04,929-941), the Armory to the DHD (806.27,55.10,517.24), and the Harset-to-Castle landing.
- **Per-step record fields:** position, timestamp, step state and evidence.

Mark every step whose expectation comes from inference rather than a seed row. List missing GM commands or tools as gaps in the worknote, not as steps.

Report: `<wt>\.claude\reports\castle\CS-01a-mission.md`.

### CS-INF1 - `MINIGAME_PORT` env override

**Worker:** `packet-coder`. **Worktree:** `castle-cs-inf1`. **Branch:** `castle/cs-inf1-minigame-port`. **DB:** none. **Commit:** `feat(server): MINIGAME_PORT env override for a second local server`.

**Why:** A second local server cannot bind the minigame SmartFox listener, because `0.0.0.0:30000` has no env override. `config.minigame_port` already reaches the listener (`crates/services/src/orchestrator.rs:312`) and the client (`crates/base/src/base/service.rs:109` `minigame_external_port: config.minigame_port`), so this is a server-only change. It needs no client patch.

**Allowlist:** `crates/server/src/main.rs`, `docs/operations/container.md` and `docs/analysis/castle-continuation/worknotes/CS-INF1.md`.

1. In `config_from_env` (`main.rs:419`), directly after the `MINIGAME_HOST` block (`:456-458`), add:

   ```rust
   if let Ok(v) = std::env::var("MINIGAME_PORT") {
       if let Ok(p) = v.parse() {
           cfg.minigame_port = p;
       } else {
           tracing::warn!(value = %v, "MINIGAME_PORT is not a port number; keeping default");
       }
   }
   ```

2. Add a header-table row after `MINIGAME_HOST` (`main.rs:21`): ``//! | `MINIGAME_PORT` | `30000` | Minigame SmartFoxServer TCP port; also the port the client is told to dial. Change it only to run a second server on one host. |``
3. Add a unit test in the existing `mod tests` (`main.rs:501`). Edition 2021, so `set_var` is safe. nextest runs each test in its own process.

   ```rust
   #[test]
   fn minigame_port_env_overrides_default() {
       std::env::set_var("MINIGAME_PORT", "30100");
       assert_eq!(super::config_from_env().minigame_port, 30100);
       std::env::set_var("MINIGAME_PORT", "not-a-port");
       assert_eq!(super::config_from_env().minigame_port, 30000);
       std::env::remove_var("MINIGAME_PORT");
   }
   ```

   Type: unit (TESTING.md). It fails when the env read is reverted: the first assertion gets 30000.
4. In `docs/operations/container.md`, add a row to the "Not baked into the image" table: ``| `MINIGAME_PORT` | `30000` | Minigame SmartFox port, also advertised to the client. Publish the same port (`-p <port>:<port>`). |``. CRLF (rule 6).
5. Lane checks, in order:
   1. `cargo fmt --all -- --check`
   2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-server --all-targets -- -D warnings`
   3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server -E 'test(minigame_port)'`, which must run exactly 1 test and pass.

   `main.rs` is 516 lines, already over the soft cap. Stay under 560 and do not split here.
6. The worknote records the lane summary lines and `git diff --check`. Commit.

Report: `<wt>\.claude\reports\castle\CS-INF1-coder.md`.

### CS-01b - isolated Castle stack preflight

**Worker:** `packet-coder`. **Worktree:** `castle-cs-01b`. **Branch:** `castle/cs-01b-stack`. **DB:** `sgw_castle_cs_01b`. **Gate:** CS-INF1 is integrated, so `FEATURE_SHA` includes it. **Commit:** `test(castle): isolated Castle server preflight`.

**Allowlist:** `docs/analysis/castle-continuation/uat/CS-01-preflight.md` (new). No code. **Never edit lab install files or `labd.env`** (D-CA9).

1. Do the base check. Then run `pwsh tools/build-lane/reload-db.ps1`, which must print `into sgw_castle_cs_01b`.
2. Port check. Each command must print nothing; if one prints, return `blocked` with the owner PID:
   - `Get-NetTCPConnection -State Listen -LocalPort 18081,18443,8452,30100 -ErrorAction SilentlyContinue`
   - `Get-NetUDPEndpoint -LocalPort 32932 -ErrorAction SilentlyContinue`
3. Secrets check. Do not print the values. `([Environment]::GetEnvironmentVariable('CIMMERIA_LAB_MCP_TOKEN','User')).Length -ge 32` must be True, and the same for `CIMMERIA_TELEMETRY_HMAC_SECRET`.
4. Build, `--check-db` and start, exactly as in [The Castle server instance](#the-castle-server-instance). Wait up to 60 s for the listeners.
5. Verify:
   - steps 2's commands now show the Castle PID on 18081, 18443, 8452, 30100 and UDP 32932;
   - `logs\server.log` contains the bind lines and no `database_connect_failed`;
   - `Test-NetConnection 127.0.0.1 -Port 8452` succeeds.
6. Run `lab instances status`, which is read-only. Record whether `p4` and `p5` are hosted and seeded, and whether a client runs. Using psql, record `select username, accesslevel from accounts where username in ('lab4','lab5')`; take the column names from `db/sgw/` and never record passwords.
7. Write `uat/CS-01-preflight.md` with:
   - the `FEATURE_SHA` and `HEAD`;
   - the exe SHA-256 (`Get-FileHash`);
   - the ports table, the PID, the DB name and the start time;
   - the stop command `Stop-Process -Id <pid>`;
   - the session env line for the orchestrator restart (`CIMMERIA_LAB_MCP_URL=http://127.0.0.1:8452/mcp`);
   - the lab-instance facts.

   CRLF. Commit. **Leave the server running.**

Report: `<wt>\.claude\reports\castle\CS-01b-coder.md`.

### CS-01c - lab walk (orchestrator-run, not a packet worker)

**Gate:** D-CA9 is answered, and every item in "What must be started" is true. The orchestrator loads the `lab-uat` skill. The run goes like this:

1. Haiku `lab-driver` agents run `uat/CS-01-walk.md` on `p4` and `p5`, one lease each.
2. A verifier, `mission-systems-advisor` in the restarted session, checks each claim against the Castle lab-server MCP and `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\castle-cs-01b\logs\server.log`.
3. A `packet-coder` writes the result into branch `uat/CS-01.md`, and appends to `docs/guides/unified-uat.md` only what stays owed.
4. Each reproduced defect becomes its own sealed packet under the CS-01 allowlist rules in the menu.
5. Afterwards: `Stop-Process` the Castle server, then retire `castle-cs-01b`.

## Dispatch briefs (R1)

Every brief starts with: "Read `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\castle-continuation\work-packets.md` § <section> (on the branch: `resume-packets.md`). Follow its repo rules and step 0 base check. `FEATURE_SHA=<sha from RESUME.md>`."

| Packet | ops call (from the main checkout) | Agent | Worktree | Report path | Return |
|---|---|---|---|---|---|
| CS-INT0 | none | packet-coder | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle` | `...\zone-castle\.claude\reports\castle\CS-INT0-coder.md` | `CS-INT0 <done\|blocked\|failed> <sha\|-> <report>` |
| CS-00Ra | `pwsh tools/build-lane/mk-worktree.ps1 castle/cs-00ra-g-med castle-cs-00ra` | game-archaeology-specialist | `...\.claude\worktrees\castle-cs-00ra` | `...\castle-cs-00ra\.claude\reports\castle\CS-00Ra-archaeology.md` | `CS-00Ra <done\|blocked\|failed> <sha\|-> <report>` |
| CS-00Rb | `pwsh tools/build-lane/mk-worktree.ps1 castle/cs-00rb-g-sym castle-cs-00rb` | game-archaeology-specialist | `...\.claude\worktrees\castle-cs-00rb` | `...\castle-cs-00rb\.claude\reports\castle\CS-00Rb-archaeology.md` | `CS-00Rb ...` |
| CS-01a | `pwsh tools/build-lane/mk-worktree.ps1 castle/cs-01a-walk-steps castle-cs-01a` | mission-systems-advisor | `...\.claude\worktrees\castle-cs-01a` | `...\castle-cs-01a\.claude\reports\castle\CS-01a-mission.md` | `CS-01a ...` |
| CS-INF1 | `pwsh tools/build-lane/mk-worktree.ps1 castle/cs-inf1-minigame-port castle-cs-inf1` | packet-coder | `...\.claude\worktrees\castle-cs-inf1` | `...\castle-cs-inf1\.claude\reports\castle\CS-INF1-coder.md` | `CS-INF1 ...` |
| CS-01b | `pwsh tools/build-lane/mk-worktree.ps1 castle/cs-01b-stack castle-cs-01b` | packet-coder | `...\.claude\worktrees\castle-cs-01b` | `...\castle-cs-01b\.claude\reports\castle\CS-01b-coder.md` | `CS-01b ...` |

Here `...` is `C:\Users\Steve\source\projects\Cimmeria`.

**CS-INT1, after the R1 review gate.** A `packet-coder` in `zone-castle` cherry-picks the reviewed CS-00Ra, CS-00Rb, CS-01a and CS-INF1 commits, in that order, onto `content/castle-campaign`. It updates the branch README rows with each SHA, runs `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server -E 'test(minigame_port)'`, and commits `docs(castle): integrate resume wave R1`. Report `...\zone-castle\.claude\reports\castle\CS-INT1-coder.md`. The CS-01b commit is cherry-picked after CS-01c.
