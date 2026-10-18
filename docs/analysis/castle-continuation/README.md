# Castle continuation: orchestration overlay

> **Type:** Reference. **Audience:** Castle campaign orchestrator, ops, packet workers and reviewers.
> **Last updated:** 2026-10-17.
> **Companions:** [resume card](RESUME.md), [overlay packets](work-packets.md), [R2 packets](r2-packets.md), [R3 packets](r3-packets.md) and [part 2](r3-packets-2.md), [architect report](architect-report.md), [R2 architect report](architect-report-r2.md), [R3 architect report](architect-report-r3.md), canonical branch ledger `content/castle-campaign:docs/analysis/castle-continuation/README.md` (worktree `.claude\worktrees\zone-castle`).

**This folder is a main-checkout overlay and is never committed to `main`** (D-CA2, the same rule as Cellblock's D-CC2). The canonical Castle ledger, evidence and worknotes live on `content/castle-campaign` in the `zone-castle` worktree. Before anyone pulls the merged Castle campaign PR into the main checkout, delete this overlay folder or git refuses to overwrite the untracked files.

## Purpose

Resume the Castle continuation (four-zone campaign, world 8): restore missions 1520/1521, the 1664 exit grant and the Castle legs of Harset carry-ins 1360/567, verify the 701-708 walk, then add 5-8 authored side stories, return-hub services and repeatables, and close out with one campaign PR. The governing contracts are [the Castle packet menu](../four-zone-planning/castle-work-packets.md), [the execution contract](../four-zone-planning/execution-contract.md), [the ID table](../four-zone-planning/id-allocation.md) and the branch-only `four-zone-planning/owner-decisions-2026-10-14.md` (O14-01..O14-22).

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | **The campaign was already started.** `content/castle-campaign` exists at `de750bf07` (2026-10-14 19:30 -0500) in worktree `zone-castle`: 64 commits ahead of `origin/main`, 59 behind. It holds the full continuation ledger, evidence, the CS-01 probe record and worknotes for CS-00, CS-02a, CS-02b-prep, CS-02b0, CS-02b1, CS-02b2 and CS-08a. The brief's "nothing is started" premise is wrong. | `git log origin/main..content/castle-campaign`; branch `RESUME.md` | D-CA1 |
| F2 | The Castle zone session (O14-02) has been idle about three days. Its worktree is clean apart from an untracked `castle-pg.log`, and no Castle server, client or Postgres process is running. Per campaign-packet section 0, nobody messages it; the owner says whether this session adopts it. | `git -C zone-castle status`; process list 2026-10-17 | D-CA1 |
| F3 | `#1734` (CB-N05a) is already merged into the Castle branch (`94b4a5754` is an ancestor, via merge `a2d6debb6`). Since the merge base `817d3792c`, `main` has not changed `docs/analysis/four-zone-planning/`, `db/database.sql`, the Content/Missions seeds, `mission-chains.md` or `unified-uat.md`, so merging `origin/main` again should not conflict on Castle's files. | `git merge-base --is-ancestor`; `git diff --stat 817d3792c origin/main -- ...` (empty) | CS-INT0 |
| F4 | Leftover branches: `castle/cs-00-contract` (1 commit, cherry-picked into the campaign as `d5b679061`), `castle/cs-08a1-letter-bridge` (merged at `03100628b`) and `castle/ledger-session-record` (September, historical). Their worktrees are already retired. They are not in-flight work. | `git branch --list castle*` | D-CA7 |
| F5 | Harset's HA-1360a/b work (needed for CS-08a2) sits on `content/harset-campaign` (`7965f653d`, 2026-10-14) with seven `harset-ha-1360*` worktrees. Their tips are not ancestors of the Harset branch, so they were cherry-picked. The receiving interface has not yet been recorded in the Castle ledger. | `git log origin/main..content/harset-campaign --grep=1360` | CS-08a2 |
| F6 | Cellblock is live in another session: `zone-cellblock` (`content/cellblock-campaign` `452c15340`, 2026-10-17), eight `cellblock-cb-n0*` worktrees and `.claude/state/active-campaign = cellblock-continuation`. Castle must not touch those worktrees, DBs, server, the main-checkout `docs/analysis/cellblock-continuation/` overlay or the pointer file. The only shared files are `db/database.sql`, `id-allocation.md`, `docs/content/mission-chains.md` and `docs/guides/unified-uat.md`, and they collide only at each zone's final PR. CB-N05b (Castle recipient) is Cellblock's and is held on HA-1360. | Cellblock overlay `RESUME.md`, README F8 | D-CA3 |
| F7 | The 2026-10-14 CS-01 probe ran the default lab client against an unidentified server: `server_sessions` showed zero sessions while the client was in world 8, and the build hashes were not captured. A walk only counts when the client's server, the queried lab-server endpoint and the Castle build are proven to be the same process. | branch `uat/CS-01.md` | CS-01b |
| F8 | At session start the `lab-server` MCP (`CIMMERIA_LAB_MCP_URL`, default `http://10.0.30.71:8444/mcp`) was ECONNREFUSED. The local lab daemon `cimmeria-lab.exe` is up on `127.0.0.1:8779`. The shared test Postgres on `:5433` is up (Docker). | `.mcp.json`; `Get-NetTCPConnection` | CS-01b |
| F9 | Open evidence gates on the branch: G-MED (1520 dispenser actor, minigame, radio), G-SYM (1521 tank, minigame, implant), G-ROUTE (Infirmary to chamber, Armory to DHD, Harset to Castle landing), G-1664 (exit grant and HA-00b receiver), G-567 (Romney's Files grant), G-706 (why the ZIP classifier omits it) and the full G-REVIEW. The only claimed chains were 1366-1370 (R1a later claimed 1371-1377). | branch `README.md` Decisions, `work-packets.md` seal record | CS-00Ra/b, CS-01a |
| F10 | **The lab is paused (2026-10-17).** The user's direction is to abandon lab work, return to content restoration, and leave all UAT to the user at the end. The CS-01 walk got through setup, the GM 688 grant and the Castle landing; walking was blocked by lab input. Those rows stay UATPending. | orchestrator brief 2026-10-17 | D-CA11 |
| F11 | The Castle lab server (PID 36540, `castle-cs-01b\cimmeria-server.exe`, started 2026-10-17 18:52) is still running on 18081/32932/18443/8452/30100 against `sgw_castle_cs_01b`. Cellblock's CB-U08 lab attempts logged into it by mistake. Cellblock's test character `Cellutb` lives on that DB. CS-01b's commit `009ab0536` is not cherry-picked. | process list; Cellblock overlay RESUME | D-CA13, CS-01d |
| F12 | The R1a worktrees `castle-cs-00ra` (`ed0c44657`), `-00rb` (`b9492dfee`), `-01a` (`966a4fd7d`) and `-inf1` (`4c13b9d13`) are integrated but still on disk, because ops refuses `rm-worktree --force`. | ops return | D-CA14 |
| F13 | The runtime never reads `mission_steps.step_enabled` or `mission_objectives.is_enabled`; only `console_authoring` tests read them, and mission 708's step 2417 is `false` yet works. Enabling 1520/1521 needs no flag flips. | grep `crates/`; `mission_steps.sql:2052-2064` | CS-02c1, CS-03c1 |
| F14 | Of the seeded minigame victory callbacks (1017, 1025, 1042, 1061, 1234, 1293, 13005, 1357, 3024, 7005), none has `content_conditions`. Routing conditioned callbacks through `fire_guarded_minigame_chain_by_id` (which exists, `event_dispatch/direct_invoke.rs`) keeps legacy parity. Today the Cell fires a victory for a missing entity with player 0 (`base_messages/minigame.rs:38-54`). | seed scan; code read | D-CA12, CS-MGF1/2 |
| F15 | Only chain 1366 is seeded. 1367-1377 are claimed but not seeded, per the "entry chains ship only with a completable branch" rule. `data/spaces/castle.nav` is tracked, so `nav_inspect` and `castle_navmesh.rs` can prove placement and routes without the lab. | `castle_carry_in_chains.sql`; `git ls-files` | CS-02S, CS-03S |
| F16 | Three test files are already past the 500-line soft cap: `crates/services/src/mission_round_trip_tests.rs` (638), `chain_replay_tests/mission_1360.rs` (555) and `resources/tests/mission_additions.rs` (610). HA-1360b's draft extends the first in place, so it would grow toward the 700 hard cap. | line counts at `da2b82c70`; Harset `work-packets.md:172+` | CS-08a2M, D-CA15 |
| F17 | `mission_completed` is a working trigger (`class_start_sgc/ordinance_rewards.rs:99`), and `Action::CompleteMission` awaits `fire_mission_completed` inside the executor arm (`mission_1324.rs:803-806`). The 1664 candidate is mechanically plausible. What the world reads at fire time, and what a failed travel leaves behind, are still unproven. | code read | CS-06S |
| F18 | Production `MISSION_ADDITIONS` is one `const` array (`mission_additions.rs:14`), and `resources/tests/mission_additions.rs:125` asserts that it is empty. Cellblock's CB-N05 fills the same array, so both final PRs conflict there. | code read; Cellblock `cb-n05-seal.md` | D-CA17 |
| F19 | The contract holds every authored quest after the first until a live-client catalog proof. Under D-CA11, that proof only comes at the end UAT, which forces a second UAT round. Cellblock's own first proof (CB-N05) is BlockedEvidence on the user's CB-U09 run. | execution contract; Cellblock overlay | D-CA16 |
## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-CA1 | Adopt the idle Castle zone session's branch and ledger (recommended: adopt `content/castle-campaign` as-is and do not restart CS-00). The owner confirms that the old Castle main session is not running. | Decided (adopted 2026-10-17) | Two coordinators on one branch would collide (O14-02 is one session per zone). Restarting would discard 64 reviewed commits. |
| D-CA2 | This main-checkout folder is an uncommitted overlay. The branch ledger stays canonical, and CS-INT0 ports the new rows (F1-F9, D-CA*, the isolation recipe) into it. | Decided | One campaign branch, one ledger (execution contract). Committing to `main` would collide with the branch files. |
| D-CA3 | Castle is a **second concurrent campaign**. It never writes `.claude/state/active-campaign`; the Castle orchestrator reads `docs/analysis/castle-continuation/RESUME.md` and the branch ledger itself after compaction. | Decided | The Cellblock session owns the pointer (brief). |
| D-CA4 | Test isolation: one worktree and DB per packet (`sgw_castle_cs_<id>`), integration DB `sgw_zone_castle`, and a separate Castle server on non-default ports against its own DB for lab work ([recipe](work-packets.md#testing-isolation)). | Decided | Repo rule (one worktree and DB per agent), plus F6 and F7. |
| D-CA5 | Lab use: the owner authorized it for this campaign (brief; O14-08 standing approval). The lease is taken per instance and released after each run. Castle uses a lab instance that Cellblock is not leasing, and it checks `lab status` first. | Decided | Authorization given. Shared daemon, one lease per instance. |
| D-CA6 | Review rule: read-only `packet-reviewer`, fixes by a fresh `packet-coder` or domain agent, then a fresh re-review, at most 3 rounds (owner contract; matches Cellblock D-CC6). | Decided | Execution contract section "Review and local CI gate": separate fixer, fresh reviewer. |
| D-CA7 | Delete leftover branches `castle/cs-00-contract` and `castle/cs-08a1-letter-bridge` through ops after D-CA1. Keep `castle/ledger-session-record` (historical). | Decided (after D-CA1) | Both are already integrated (F4). |
| D-CA8 | The CS-00B gate is split. Its evidence half (the merged #1734 API, the Castle authored-ID reservation in 10000-10999 and the stock-catalog collision audit) is Ready after CS-INT0. Its client-proof half stays UATPending until the first authored quest in any zone passes journal, dialog, advance and relog in the client. | Decided | Execution contract "Client mission catalog prerequisite"; the production `MISSION_ADDITIONS` is empty, so no client proof exists yet. |
| D-CA9 | Route a Castle lab client to the Castle server by adding a non-destructive `CastleLocal` row (`http://127.0.0.1:18081`) to the shared lab install's `LoginInternal.lua`, and pointing `lab-account.p4.json` and `lab-account.p5.json` at it (`"server": "CastleLocal"`). The alternative is an exclusive lab window with `labd.env` switched. Recommended: the row and p4/p5. | Moot while the lab is paused (D-CA11); re-ask before the end UAT if an agent drives it | It edits shared lab install state that Cellblock's UAT also reads. `labd.env` holds one server URL and one lab-server MCP URL, so a switch would hijack Cellblock ([isolation](work-packets.md#lab-routing)). |
| D-CA10 | Add a `MINIGAME_PORT` env override (CS-INF1) on the Castle branch, so a second local server does not collide on `0.0.0.0:30000`. | Decided | It is server-only: `config.minigame_port` already feeds the listener and the client's `minigame_external_port`, so no client patch is needed and the default stays 30000. |
| D-CA11 | **No lab until the end.** No packet drives the client or the lab. Each client-only claim becomes `UATPending (end UAT)` in branch `uat/end-uat.md`, which the user runs at the end. CS-01c and the CS-00B client proof stay UATPending. | Decided (user, 2026-10-17) | The user's direction. |
| D-CA12 | **b2-lite replaces the full CS-02b2 for Castle callbacks.** The internal `BaseToCellMsg::MinigameResult` carries the receipt's `player_id`; the Cell refuses a victory when the entity is missing or the player mismatches; and conditioned callback chains go through `fire_guarded_minigame_chain_by_id`. | Conditional on CS-02L `accept`; a reject means an architect pass splits the full b2 | Base already proves E/P/I before and after the close (b0/b1). The guard re-evaluates world and step at fire time, and mutation is mission-state only. It reverses the branch's reviewed "b2 required" gate (`work-packets.md:197-271`), so trust and minigame advisors seal it rather than the architect. |
| D-CA13 | Stop the Castle lab server (PID 36540) now, through CS-01d. Keep `sgw_castle_cs_01b` and the `castle-cs-01b` worktree; restart with the CS-01b recipe for the end UAT. | Decided (architect recommendation; the orchestrator confirms) | The lab is paused, and the server captured Cellblock's lab login (F11). The DB holds `Cellutb` and the end-UAT state. |
| D-CA14 | ops refuses `--force`, so the owner retires integrated packet worktrees: `pwsh tools/build-lane/rm-worktree.ps1 <name> --force` from the main checkout. `castle-cs-01b` is retired last, after the end UAT. | Decided | F12. Never `git worktree prune`. |
| D-CA-SYM | The symbiote implant: Option A (`add_item 5169 {"container":2}`, optionally `apply 1926` on accept) or Option B (no items). R2 builds the shared mission-state skeleton, and CS-03d adds A's deltas. | **BlockedDecision** | Owner content call (the item's purpose is client-visible). Evidence: branch `evidence/g-sym.md`. |
| D-CA15 | CS-08a2M moves the Frost's Letter tests into `frosts_letter_round_trip_tests.rs`. Whichever campaign's PR reaches `main` second owns the recipient round trip (CS-08a2 or Harset's). The orchestrator tells the Harset coordinator to point HA-1360b at the new file. | Decided (architect) | F16. It avoids two campaigns editing one near-cap file, and only a branch holding both sides can test the round trip. |
| D-CA16 | Authored quests after `medical_q1`: (A) hold them until a live catalog proof passes (the contract default), or (B) build them now and accept rework if the proof fails. Sub-question: does Cellblock's CB-N05 live proof count for Castle too? | **BlockedDecision (owner)**. Recommended: A, and yes to the sub-question. | F19. Under A, R3d needs a second UAT round. B saves that round but goes against the execution contract. |
| D-CA17 | Castle's authored XML goes in `mission_additions_castle.rs`; `MISSION_ADDITIONS` lists its entries in ID order; the tests go in `tests/mission_additions_castle.rs`; the chains go in `castle_side_stories_chains.sql` and `castle_repeatable_chains.sql`. | Decided (architect) | F18. The Cellblock conflict shrinks to array entries that CS-07R unions. |
| D-CA18 | CS-06a seeds chain 1398 only once Harset's HA-01b receiver is on `content/harset-campaign`. The campaign PR merges only once HA-01b is on `main` or in the same merge window. Otherwise CS-06a/b are reverted out of the PR. | Decided (architect) | Never ship a stranded mission. |

## Packet status

Status vocabulary per the execution contract. The canonical row text is on the branch ledger; this table reconciles it as of 2026-10-17.

| Packet | Wave | Status | Gate / next action |
|---|---|---|---|
| CS-INT0 | R1 | Integrated | `c481008904`. |
| CS-00 | W0 | Integrated (entry amendment signed) | Delivered through `fae1614af`. Follow-up gates moved to CS-00Ra/b (G-MED/G-SYM), then CS-00R2 (G-567/G-706/1376). |
| CS-00Ra / CS-00Rb / CS-01a / CS-INF1 | R1a | Integrated (CS-INT1, `da2b82c70`) | Chains: Ra 1371-1373, Rb 1374-1377. Worktrees await owner retire (D-CA14). |
| CS-INT1 | R1 | Integrated | `da2b82c70` is the current `FEATURE_SHA`. |
| CS-01b | R1b | Done (local commit `009ab0536`) | CS-INT2a cherry-picks it. The server runs as PID 36540 until CS-01d. |
| CS-01c | R1c | UATPending (D-CA11) | Got through setup, the GM 688 grant and the Castle landing; walking was blocked by lab input. Rows go to the end UAT. |
| CS-01d | R2-0 | Ready | packet-coder stops PID 36540 (D-CA13). |
| CS-00R2 | R2a | Ready | game-archaeology-specialist: G-567, G-706, 1376 lineage. |
| CS-00B / CS-00Be | R2a | Ready (evidence half, CS-00Be) / UATPending (client proof) | D-CA8. Gates CS-04a/b/c and CS-05. |
| CS-02S | R2a | Ready | mission-systems-advisor: 1520 frozen rows (placement, Ogilvie, Livewire, closes, relog). |
| CS-03S | R2a | Ready | mission-systems-advisor: 1521 Option B skeleton rows; G-ROUTE placement choice. |
| CS-02L | R2a | Ready | server-authority-enforcer, then minigame-systems-advisor: b2-lite verdict (D-CA12). |
| CS-INT2a | R2 | BlockedDependency (R2a review) | Cherry-pick R2a and `009ab0536`; copy `r2-packets.md` to the branch. |
| CS-MGF1 | R2b | BlockedDependency (CS-02L accept) | `player_id` on `MinigameResult`; `chain_has_conditions`. |
| CS-02c1 | R2b | BlockedDependency (CS-02S sealed) | 1520 seeds and entry replay tests. |
| CS-03c1 | R2b | BlockedDependency (CS-03S sealed) | 1521 skeleton seeds and entry tests. |
| CS-MGF2 | R2c | BlockedDependency (CS-MGF1 review) | The player-bound victory guard. |
| CS-02c2 / CS-03c2 | R2c | BlockedDependency (c1 review) | 1520/1521 graph replay and navmesh route tests. |
| CS-DOC2 | R2c | BlockedDependency (CS-INT2a) | `mission-chains.md` rows; `uat/end-uat.md` rows. |
| CS-INT2 | R2 | BlockedDependency (R2c review) | Integrate R2b/R2c; the seed packets never land without CS-MGF2. |
| CS-01 | W1 | UATPending | Now CS-01a/b/c; see above. |
| CS-02 | W1 | Superseded by CS-02S/02c1/02c2 | Do not offer a stranded mission. |
| CS-02a / b0 / b-prep / b1 | W1 | Integrated | Branch worknotes. |
| CS-02b2 | W1 | Superseded by b2-lite if CS-02L accepts (D-CA12) | Otherwise an architect pass splits it into b2-prep/b2a/b2b. |
| CS-03 | W1 | Superseded by CS-03S/03c1/03c2 | G-ROUTE is decided in CS-03S. |
| CS-08a1 | W1 | Integrated / UATPending | Chain 1366 at `03100628b`. |
| CS-06S | R3a | Ready (after CS-INT2a) | The 1664 seal: firing order, chain 1398 (and 1399 if needed), travel failure, HA-01b receiver. |
| CS-08bM | R3a | BlockedEvidence (G-567) | Dispatch when CS-00R2 returns `SEALED`. It signs off 1394-1397, the 2698 grant once, and authority. |
| CS-03dS | R3a | BlockedDecision (D-CA-SYM) | Option A only; it freezes the 5169/1926 deltas. |
| CS-05S | R3a | Ready (after CS-INT2a) | The services, repeatables and barks seal, each with its own verdict. |
| CS-04aD / CS-04bD / CS-04cS | R3a | Ready (after CS-INT2a, given CS-00Be's sub-blocks) | Design seals for 5-8 quests in total. `medical_q1` is the catalog proof. CS-04cS also covers the return route and the Harset contract. |
| CS-INT3a | gate | BlockedDependency (R3a review) | Integrates the seals and copies both R3 files to the branch. |
| CS-08a2M | R3b | BlockedDependency (CS-INT3a) | Behavior-neutral split of the letter tests (D-CA15). |
| CS-08b1 / CS-08b2 | R3b / R3c | BlockedEvidence (G-567) | CS-08bM `SEALED`. |
| CS-06a / CS-06b | R3b / R3c | BlockedDependency | CS-06S `SEALED`, plus HA-01b on the Harset branch (D-CA18). |
| CS-03d | R3b | BlockedDecision (D-CA-SYM) | CS-03dS `SEALED` and CS-INT2. |
| CS-05a / CS-05c | R3b / R3c | BlockedDependency | CS-05S services / barks `SEALED`. |
| CS-04a1s / CS-04a1c | R3b / R3c | BlockedDependency | CS-04aD `medical_q1` `SEALED`. |
| CS-08a2S | R3c | BlockedDependency (HA-1360a/b on Harset) | Records the interface and the D-CA15 call. |
| CS-DOC3 / CS-INT3 | R3c / gate | BlockedDependency | CS-INT3a / the R3c review. |
| CS-04a2..CS-04c2 (s/c), CS-05b | R3d | BlockedDecision (D-CA16) | D-CA16 answered, then the catalog proof. |
| CS-08a2 | final | BlockedDependency (HA-1360a/b on `main`) | Castle is second to merge (D-CA15); otherwise Dropped (Harset owns). |
| CS-07R / 07a / 07b / 07c, PR | final | BlockedDependency | Every active wave integrated. CS-07c is the user's end UAT. |

## Wave plan

| Wave | Packets | Entry and exit gate |
|---|---|---|
| R1 (resume) | CS-INT0; R1a = CS-00Ra, CS-00Rb, CS-01a, CS-INF1 in parallel; CS-INT1; R1b = CS-01b; R1c = CS-01c | D-CA1 answered. Exit: R1a reviewed per D-CA6 and integrated; the isolated stack proven to be the same server the lab-server MCP queries; CS-01c run or UATPending. |
| R2 (no lab, [r2-packets.md](r2-packets.md)) | R2-0 = CS-01d; R2a = CS-00R2, CS-00Be, CS-02S, CS-03S, CS-02L; CS-INT2a; R2b = CS-MGF1, CS-02c1, CS-03c1; R2c = CS-MGF2, CS-02c2, CS-03c2, CS-DOC2; CS-INT2 | D-CA11. Each wave gets a D-CA6 review. Exit: 1520/1521 server-side completable and replay-tested; b2-lite integrated; end-UAT rows written. |
| R3 (no lab, [r3-packets.md](r3-packets.md), [part 2](r3-packets-2.md)) | R3a = CS-06S, CS-08bM, CS-03dS, CS-05S, CS-04aD, CS-04bD, CS-04cS; CS-INT3a; R3b = CS-08a2M, CS-08b1, CS-06a, CS-03d, CS-05a, CS-04a1s; R3c = CS-08b2, CS-06b, CS-04a1c, CS-05c, CS-08a2S, CS-DOC3; CS-INT3; R3d = remaining authored quests and CS-05b, then CS-INT4; final = CS-07R, CS-08a2, CS-07a, CS-07b, CS-07c, PR | Each gate is checked at dispatch. One D-CA6 review per wave. Exit: restoration, the hub and 5-8 authored quests are server-side tested; the end UAT has run; one campaign PR (O14-06). FZ-99 owns the status docs. |
| W0-W4 (old menu waves) | Superseded by R2 and R3 | The menu sections remain the content rules. |
