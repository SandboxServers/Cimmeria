# Castle Rebuild — Session Resume Handoff

> Type: reference. Audience: the Claude Code coordinator resuming this campaign after a context rollover or `/clear`. Companions: [README.md](../README.md) (decisions, session record), [work-packets.md](../work-packets.md) (packet statuses), worknotes under [../worknotes/](../worknotes/), coordinator scratch notes at `C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\COORDINATOR-NOTES.md` and worker rules at `C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\WORKER-RULES.md`.
> Written 2026-09-18 ~13:00 UTC by the Castle coordinator session (`cimmeria-e5`). Everything below is the state at that moment; verify with `gh pr list` / `git worktree list` before acting.

## Mandate

User authorization (2026-09-17): "do the work described in docs/analysis/castle-rebuild ... work autonomously until you complete all work packets, open PRs at regular intervals; if CodeRabbit is rate-limited ensure at least one round of Copilot review". Every PROPOSED decision was adopted at its recommended default (README session record; D-CA17/18/19 added). Merge policy in use: squash-merge after CI green, one bot review round (Copilot requested via `gh api -X POST repos/SandboxServers/Cimmeria/pulls/<n>/requested_reviewers -f "reviewers[]=copilot-pull-request-reviewer[bot]"`) and a `testing-validation-engineer` static review whose findings the packet worker applies. Coordinator commit footer: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` / `Claude-Session: https://claude.ai/code/session_01JzRDcyoErCQGee6pM4AaZM` (workers may use their own model line).

## PR state

| PR | Packet | Branch | State | Next action |
|---|---|---|---|---|
| #647 | ledger | `castle/ledger-session-record` | merged 0e98c496 | none |
| #651 | CA00 respawners | `castle/ca00-respawners` | merged 784e0425 | ledger: flip CA00 to Integrated/UATPending |
| #658 | fmt fix for main | `fix/fmt-main-after-650` | merged e93de435 | none |
| #652 | CA04 minigame | `castle/ca04-minigame-hardening` | open, CI green, TVE + 5 fixes applied | worker `ca04-minigame` owes per-comment dispositions for the 11 bot comments (fixture 0 vs 2 label, split `livewire_pairs.rs`, doc paths, worknote); then squash-merge |
| #659 | CA01+CA03 mission 701 | `castle/m701-gerschon-copplemann` | open, CI green, TVE 9 items applied, Copilot round-2 nits applied locally | worker `m701-chains` owes the push of the 3-nit commit; then squash-merge |
| #661 | CA02 nullable bind | `castle/ca02-nullable-dialog-bind` | open, CI green on first push; TVE 6 items applied locally | worker `ca02-nullable-bind` owes push + dispositions; rebase after #659 (same interaction dir); then merge |
| #663 | CA10 stargate events | `castle/ca10-stargate-events` | open; TVE 8 items applied locally (2 blockers incl. sentinel move to `0x7000_62x0`) + Copilot date nit | worker `ca10-stargate-events` owes validation + push; then merge; then rebase m706 |
| #660 | CA06+CA07 missions 702-704 | `castle/m702-704-zuritska-romney` | open, 186 live-DB tests green, TVE 4 items applied, Copilot threads answered | merge only after CA05 (region linter needs the two point sets); still needs the `Castle_Zuritska_Cell` coordinate from CA05 for chain 1291's walk-home `move_waypoint` |
| none | CA05 recon + actors | `castle/ca05-story-actors` (not pushed) | worker `ca05-recon` finishing: respawn timers on all hostile World 8 rows, Zuritska `move_speed`/no patrol, two point sets | open PR, Copilot + TVE, ledger; then merge, then #660 |
| none | CA08+CA09 missions 706/708 | `castle/m706-708-throne-stargate` (worktree `castle-m706`, stacked on CA10 @ 0d3c9147) | 28 chains + tests committed; regression proof queued | worker `m706-708-chains` owes: `git rebase --onto origin/castle/ca10-stargate-events 0d3c9147`, push, report; open PR after #663 merges |

Merge order: #652, #659, #661 (rebase), #663, CA05 PR, #660, 706/708 PR. Filed issue: #657 (`active_objective_ids = [step_id]` relog defect, cross-mission, not worked around).

## Worktree to branch map

All under `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\`, each with `external/` junctioned to the primary checkout.

| Worktree | Branch | Owner |
|---|---|---|
| castle-coordinator | `castle/coordinator-session-resume` (this file) | coordinator |
| castle-ca02 | `castle/ca02-nullable-dialog-bind` | ca02-nullable-bind |
| castle-ca04 | `castle/ca04-minigame-hardening` | ca04-minigame |
| castle-ca05 | `castle/ca05-story-actors` | ca05-recon |
| castle-ca10 | `castle/ca10-stargate-events` | ca10-stargate-events |
| castle-m701 | `castle/m701-gerschon-copplemann` | m701-chains |
| castle-m702 | `castle/m702-704-zuritska-romney` | m702-704-chains |
| castle-m706 | `castle/m706-708-throne-stargate` | m706-708-chains |
| castle-ca00 | merged; worktree can be removed | none |

Worker agents were spawned by this session with `Agent` (names above) and continued with `SendMessage`. After `/clear` they are gone: relaunch a fresh `rust-gameserver-dev` per pending item with the worktree path, the branch, the "Next action" column above and `WORKER-RULES.md`.

## Build lane v2 (2026-09-18, replaced the mutex at the user's request)

Scripts in `C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\`: `lane.sh` (counting semaphore; slot count in `%LOCALAPPDATA%\cimmeria-build\lane\SLOTS`, currently 2; `--exclusive` takes all), `reload-db.sh` (per-worktree database `sgw_<worktree>`; primary checkout uses `sgw`), `live-db-test.sh <filter>` (reload own DB then `cargo nextest run --profile=ci-live-db -p cimmeria-services --lib <filter>` in one slot). Worktree-local `target/`; `RUSTC_WRAPPER` is sccache at `%LOCALAPPDATA%\cimmeria-build\bin\sccache.exe` with cache `%LOCALAPPDATA%\cimmeria-build\sccache-cache` (40G cap). D: must not be used (spinning disk). MemFree low-water with 2 slots was 11 GB on this 64 GB box, so do not raise SLOTS above 2 without re-measuring. v1 copies kept as `*-v1.sh`. Harset uses database `sgw_harset`. Cellblock and Harset sessions both use these scripts; their addresses: Cellblock `uds:\\.\pipe\LOCAL\cc-msg-87d1522a18b5a49b2c66a768d449bb92`, Harset `uds:\\.\pipe\LOCAL\cc-msg-2eb43acf7bc5065913a9f97764871d88`.

Disk: 20 dead worktrees plus the stale primary `target/x86_64-pc-windows-gnu` were removed; C: went from 49 to 225 GB free. Never run git in the primary checkout (shared by three sessions).

## Cross-session agreements

- Harset H01 dropped its stargate-region routing; Castle CA10 owns the `REGION_FLAG_STARGATE` routing and the dial state; H01 keeps `validate_gate_arrival` and the DHD display. Rebase rule: exactly one `validate_gate_arrival` call at each destination-placement site (volume-entry travel, immediate-travel fallback).
- CA11/CA12 come from Harset H03 (D-CA19). CA13/CA14/CA15 stay design-gated; not started.

## Findings to fold into the ledger at closeout

- Interaction-pin fix (#659): chain-handled interacts now pin `last_interaction_target`, monologue dialogs bind the player first, and the content-chain interact path now has a distance/existence gate (server-authority fix).
- CA02 wire evidence: the dialog-set bind push is `SGWSpawnableEntity.InteractionType` (method 3, one `UINT64 TypeId`); a flag-only bind is legal; the bind-time push now folds all sibling binds.
- CA05: comms room, Armory prefab and Op-Core respawner were not locatable in map assets, so those rows are provisional; CA00 row 3 is provisional; nothing in the shipped seed respawned before CA05.
- CA10: sequences resolve from the origin gate; `generic_regions` is dead data; 18 of about 30 stargate worlds have no gate volume and keep immediate travel with a warn; rejected dials now cancel an armed dial (2009 parity).
- Follow-up candidates: CA17 `display_dialog` `target_tag` param; the shared-bit gap on escort step 2405; issue #657.
- Uncommitted agent-memory edits to sweep into one docs commit: primary checkout `.claude/agent-memory/rust-gameserver-dev/`; castle-coordinator (mission-systems-advisor, npc-ai-spawn-advisor); castle-m702 (mission-systems-advisor, server-authority-enforcer).

## Exact next actions on resume

1. Run `gh pr list --search castle` and `git worktree list`; reconcile with the tables above.
2. For each open PR: check `gh pr checks <n>` and whether the worker's owed push landed (`git log origin/<branch>`); if the worker is gone, relaunch it from its worktree with the "Next action" item; merge when green (squash, `--delete-branch`; the local `main`-is-checked-out error after merge is harmless).
3. CA05: if there is no PR yet, inspect `castle-ca05` (`git status`, worknote), finish per its brief plus the three column requirements and respawn timers on all hostile rows, open the PR, run the TVE review, merge; then send the `Castle_Zuritska_Cell` coordinate to #660's owner for the walk-home action and merge #660.
4. Merge #663, rebase `castle-m706` onto it, open its PR, run the TVE review, merge.
5. Closeout: ledger statuses to Integrated/UATPending, README session record (lane v2 and the findings above), the agent-memory sweep commit, and the final report to the user with the UAT list (M1 to M5) and the design-gated packets (CA13/CA14/CA15) left open.
