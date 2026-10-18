# Castle continuation: architect report (castle-setup)

> **Type:** Report. **Audience:** Castle orchestrator.
> **Date:** 2026-10-17. **Result:** `blocked`. The deliverables are written, but nothing can be dispatched until the owner answers D-CA1.
> **Files:** [README.md](README.md) (findings, decisions, packet status, waves), [work-packets.md](work-packets.md) (rules, contract, testing isolation, R1 packets and briefs), [RESUME.md](RESUME.md) (resume card, 37 lines). All four files are CRLF, uncommitted, and a main-checkout overlay only (D-CA2).

## 1. The brief's premise is wrong

The brief says Castle "was only planned; nothing is started". That is not true:

- `content/castle-campaign` exists at `de750bf07` (2026-10-14), checked out in worktree `.claude\worktrees\zone-castle`. It is 64 commits ahead of `origin/main` and 59 behind.
- It already holds the full continuation ledger, `evidence/`, the CS-01 probe in `uat/CS-01.md`, and worknotes for CS-00, CS-02a, CS-02b-prep, CS-02b0, CS-02b1, CS-02b2 and CS-08a.
- CS-00 (the archaeology contract the brief asks to start) is **Integrated**, and its follow-up gates are what remains.

So I did not restart CS-00. I defined a resume wave instead. Restarting would discard 64 reviewed commits and put two coordinators on one zone branch, which O14-02 forbids. Whether this session adopts the idle branch is **D-CA1, BlockedDecision**.

## 2. Audit and conflicts

| # | Conflict | Effect | Handling |
|---|---|---|---|
| C1 | The Castle zone session has been idle for about 3 days. The `zone-castle` worktree is clean apart from an untracked `castle-pg.log`, whose Postgres log shows a crash on 2026-10-15 01:30. No Castle process is running. | Two sessions could collide on one branch. | D-CA1. Do not message the idle session (campaign-packet § 0). |
| C2 | Leftover branches: `castle/cs-00-contract` (cherry-picked as `d5b679061`), `castle/cs-08a1-letter-bridge` (merged at `03100628b`) and `castle/ledger-session-record` (September). Their worktrees are already gone. | None. They are not in-flight work. | D-CA7: ops deletes the first two after D-CA1. |
| C3 | Cellblock is live: `zone-cellblock` (`452c15340`, 2026-10-17), worktrees `cellblock-cb-n02a/b/c/d/s/s2`, `cb-n03s`, `cb-n04s`/`n04a`, a main-checkout overlay at `docs/analysis/cellblock-continuation/`, and `.claude/state/active-campaign = cellblock-continuation`. | Shared files: `db/database.sql`, `id-allocation.md`, `docs/content/mission-chains.md` and `docs/guides/unified-uat.md`. They collide only at each zone's final PR. | I touched none of Cellblock's files. Castle does not write the pointer file (D-CA3). The second PR to land rebases. |
| C4 | The Castle ledger lives on its branch. Writing tracked-path files to `main` would block the branch merge. | The new folder must not be committed to `main`. | D-CA2: overlay only. CS-INT0 ports it into the branch (`resume-packets.md`). |
| C5 | Harset's HA-1360a/b sits on `content/harset-campaign` (`7965f653d`), with seven `harset-ha-1360*` worktrees. | CS-08a2 needs its interface. CB-N05b (Cellblock) is held on the same work. | Kept as BlockedDependency. |
| C6 | Minigame SmartFox binds `0.0.0.0:30000` and has no env var. | A second local server cannot run minigames beside the default one. | D-CA10 / CS-INF1: a server-only `MINIGAME_PORT` read. No client patch is needed, because `minigame_external_port` comes from config. |
| C7 | The lab daemon holds a single `labd.env` server URL and a single lab-server MCP URL, and there is no `lab_server_switch`. Lab install files (`LoginInternal.lua`, `lab-account.pN.json`) are shared. | Castle cannot reach its own server without editing shared lab state or hijacking Cellblock. | D-CA9, BlockedDecision. The recommended path is the `CastleLocal` row plus instances p4/p5. |
| C8 | The 2026-10-14 CS-01 probe ran the default lab client against an unidentified server: `server_sessions` showed 0 sessions and no build hash was captured. | The probe proves nothing about the Castle build. | CS-01b proves that the client's server, the lab-server endpoint and the build are the same process. |
| C9 | `origin/main` has moved since the merge base `817d3792c`, but it has not touched four-zone-planning, `db/database.sql`, Content/Missions seeds, `mission-chains.md` or `unified-uat.md`. #1734 is already merged into the branch. | Merging `origin/main` again should not conflict. | CS-INT0 merges and stops on any conflict. |

## 3. CS-00B status given #1734

CB-N05a (#1734) is on `main` and already in the Castle branch. Production `MISSION_ADDITIONS` is empty, and the CB-N05a seal uses test-only IDs 61000-61003. So:

- The **evidence half** is Ready after CS-INT0: the merged API, the Castle authored reservation in 10000-10999, and the stock-catalog collision audit.
- The **client-proof half** is UATPending until a new authored ID passes the client checks for journal, dialog, advance and relog in some zone (D-CA8).

1520 and 1521 already ship in the pak and do not depend on it. CS-04a/b/c stay blocked on it.

## 4. Decisions

| ID | Status | Summary |
|---|---|---|
| D-CA1 | **BlockedDecision** | Adopt `content/castle-campaign` and its ledger as-is (recommended), and confirm the old Castle session is not running. |
| D-CA2 | Decided | The main-checkout folder is an uncommitted overlay. The branch ledger is canonical. |
| D-CA3 | Decided | Castle is a second concurrent campaign and never writes `.claude/state/active-campaign`. |
| D-CA4 | Decided | One worktree and DB per packet (`sgw_castle_cs_<id>`), integration DB `sgw_zone_castle`, and a separate Castle server on non-default ports. |
| D-CA5 | Decided | Lab use is authorized (brief; O14-08). The lease is per instance, and Castle never takes an instance Cellblock holds. |
| D-CA6 | Decided | Reviews are read-only `packet-reviewer`. A fresh fixer handles findings, followed by a fresh re-review, at most 3 rounds. |
| D-CA7 | Decided (after D-CA1) | Delete the two integrated leftover branches. |
| D-CA8 | Decided | CS-00B is split into an evidence half (Ready) and a client-proof half (UATPending). |
| D-CA9 | **BlockedDecision** | Add a `CastleLocal` login row to the shared lab install and point `lab-account.p4/p5.json` at it, or wait for an exclusive lab window with `labd.env` switched. |
| D-CA10 | Decided | Add a `MINIGAME_PORT` env override (CS-INF1). |

**BlockedDecision list for the orchestrator to ask the owner:**

- **D-CA1:** adopt the idle branch. This gates everything.
- **D-CA9:** lab routing. This gates CS-01c only.

**Expected later:** D-CA-SYM, the symbiote implant item and pouch behavior, which CS-00Rb will raise as an evidence-backed question.

## 5. Testing isolation (summary; the full recipe is in work-packets.md § Testing isolation)

**Databases.** `reload-db.ps1` names each DB `sgw_<worktree dir>`:

- packets use `sgw_castle_cs_<id>`;
- integration uses `sgw_zone_castle`;
- the lab server uses `sgw_castle_cs_01b`.

All are on the shared `:5433` (it is up). Never reload `sgw_castle_cs_01b` while the Castle server runs.

**Server.** It runs as one process from the `castle-cs-01b` worktree root, with logs in `<wt>\logs\`.

| Setting | Castle value |
|---|---|
| `DB_URL` | `...dbname=sgw_castle_cs_01b` |
| `LOGON_PORT` | 18081 |
| `BASE_PORT` | 32932 |
| `ADMIN_PORT` | 18443 |
| `MINIGAME_PORT` | 30100 (after CS-INF1) |
| `CIMMERIA_LAB_MCP_BIND` | 127.0.0.1:8452 |
| `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` | `http://127.0.0.1:18081/api/telemetry` |
| `CIMMERIA_DEPLOY_ENV` | `dev-castle` |

- `AUTH_PORT` and `CELL_PORT` bind nothing.
- Avoid 8444, the supervisor's port.
- Build with `pwsh tools/build-lane/lane.ps1 cargo build -p cimmeria-server --release`. Then copy the exe from the printed target dir, run `--check-db`, and `Start-Process`.
- `Start-CimmeriaServer.ps1` cannot be used, because it hardcodes the default ports.

**What must be started before any lab walk (lab-server MCP was ECONNREFUSED):**

1. The Castle server, with `CIMMERIA_LAB_MCP_BIND=127.0.0.1:8452` and the User-env `CIMMERIA_LAB_MCP_TOKEN` (CS-01b). Today nothing listens on 8451 or 8452. `.mcp.json` falls back to the colo URL `:8444`, which is why the lab-server MCP was ECONNREFUSED.
2. The Castle orchestrator session, restarted with `CIMMERIA_LAB_MCP_URL=http://127.0.0.1:8452/mcp` and the same token, then `/mcp` reconnected.
3. Lab instances p4 and p5, hosted (`CIMMERIA_LAB_INSTANCES`) and not leased. The daemon `cimmeria-lab.exe` on `127.0.0.1:8779` is up.
4. The D-CA9 login row and account files.

`lab_timeline` and the `server`/`packet` clauses of `lab_uat_run` stay pointed at labd's single URL. For Castle they report UNVERIFIED, and the verifier queries the Castle lab-server MCP and server log instead.

## 6. First dispatchable wave (R1)

| Step | Packet | Agent | Worktree (`C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\`) | Branch | Report path (in that worktree) |
|---|---|---|---|---|---|
| 1 | CS-INT0 | packet-coder | `zone-castle` | `content/castle-campaign` | `.claude\reports\castle\CS-INT0-coder.md` |
| 2 | CS-00Ra (G-MED) | game-archaeology-specialist | `castle-cs-00ra` | `castle/cs-00ra-g-med` | `.claude\reports\castle\CS-00Ra-archaeology.md` |
| 2 | CS-00Rb (G-SYM) | game-archaeology-specialist | `castle-cs-00rb` | `castle/cs-00rb-g-sym` | `.claude\reports\castle\CS-00Rb-archaeology.md` |
| 2 | CS-01a (walk steps) | mission-systems-advisor | `castle-cs-01a` | `castle/cs-01a-walk-steps` | `.claude\reports\castle\CS-01a-mission.md` |
| 2 | CS-INF1 (`MINIGAME_PORT`) | packet-coder | `castle-cs-inf1` | `castle/cs-inf1-minigame-port` | `.claude\reports\castle\CS-INF1-coder.md` |
| 3 | CS-INT1 | packet-coder | `zone-castle` | `content/castle-campaign` | `.claude\reports\castle\CS-INT1-coder.md` |
| 4 | CS-01b (stack preflight) | packet-coder | `castle-cs-01b` | `castle/cs-01b-stack` | `.claude\reports\castle\CS-01b-coder.md` |
| 5 | CS-01c (lab walk) | orchestrator through lab-uat: Haiku lab-driver on p4/p5 and a mission-systems-advisor verifier | uses `castle-cs-01b` | - | - |

**Brief template**, about 6 lines per packet:

> Worktree `<path>`. Read `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\castle-continuation\work-packets.md` § `<packet>` (on the branch after CS-INT0: `docs/analysis/castle-continuation/resume-packets.md`). Follow its repo rules and step 0 base check with `FEATURE_SHA=<sha>`. Commit trailer `Co-Authored-By: Claude <noreply@anthropic.com>`. Write the report to `<worktree>\.claude\reports\castle\<packet>-<role>.md`. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.

The ops worktree commands are in work-packets.md § Dispatch briefs. There is one per worktree, run from the main checkout as `pwsh tools/build-lane/mk-worktree.ps1 <branch> <name>`.

## 7. Risks

- **CS-INT0 merge conflicts.** Unlikely (C9), but if `origin/main` has moved further, the packet stops and a fresh architect pass resolves it.
- **Specialist size.** CS-00Ra and CS-00Rb may exceed 50k. Each commits what is sealed and returns `blocked`, naming the remainder.
- **Server data paths.** The Castle server runs from a worktree with no `external/`. CS-01b reads `logs\cimmeria-server.log` for resource-load errors and returns `blocked` if a resource lives only in the main checkout.
- **Seeded lab accounts.** `lab` and `lab2`-`lab5` are in the seed (`lab-contributor-setup.md:108`), but their GM level and characters in `sgw_castle_cs_01b` are unverified. CS-01b records them, and CS-01a's preconditions create characters through the walk.
- **The overlay folder.** It must be deleted before the merged Castle PR is pulled into the main checkout.
