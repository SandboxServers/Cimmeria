# Castle Rebuild — Session Resume Handoff

> Type: reference. Audience: the Claude Code coordinator resuming this campaign after a context rollover or `/clear`. Companions: [README.md](../README.md) (decisions, session record), [work-packets.md](../work-packets.md) (packet statuses), worknotes under [../worknotes/](../worknotes/), coordinator scratch notes at `C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\COORDINATOR-NOTES.md` and worker rules at `C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\WORKER-RULES.md`.
> First written 2026-09-18 ~13:00 UTC by the Castle coordinator session (`cimmeria-e5`); rewritten 2026-09-18 ~18:15 UTC by the resumed coordinator (`cimmeria-36`). State is as of that moment; verify with `gh pr list --search castle` and `git worktree list` before acting.

## Mandate

User authorization (2026-09-17): "do the work described in docs/analysis/castle-rebuild ... work autonomously until you complete all work packets, open PRs at regular intervals; if CodeRabbit is rate-limited ensure at least one round of Copilot review". Every PROPOSED decision was adopted at its recommended default (README session record; D-CA17/18/19 added). Merge policy in use: squash-merge after CI green, one bot review round (Copilot requested via `gh api -X POST repos/SandboxServers/Cimmeria/pulls/<n>/requested_reviewers -f "reviewers[]=copilot-pull-request-reviewer[bot]"`; the request sometimes does not register, so check for a review) and a `testing-validation-engineer` static review whose findings the packet worker applies. Commit trailers: `Co-Authored-By: <model> <noreply@anthropic.com>` and the current `Claude-Session:` URL.

## PR state

| PR | Packet | State | Next action |
|---|---|---|---|
| #647 | ledger | merged 0e98c496 | none |
| #651 | CA00 respawners | merged 784e0425 | UAT M1 |
| #659 | CA01+CA03 mission 701 | merged f930b4cb | UAT M1 |
| #663 | CA10 stargate events | merged a9d0fad7 | UAT M4 |
| #652 | CA04 minigame | merged 5c354b1e | UAT M1 |
| #661 | CA02 nullable bind | merged ddbc873e | UAT M1 |
| #667 | CA05 story actors | merged 364e736d | UAT M2; Harset `setval` collision (below) |
| #668 | CA08+CA09 missions 706/708 | merged bb47f526 | UAT M3/M4 |
| #660 | CA06+CA07 missions 702-704 | merged 2a37ba4c | UAT M2/M3 |
| #669 | closeout docs: ledger statuses, README session record, this handoff, agent-memory sweep | branch `castle/closeout-ledger` (worktree `castle-coordinator`) | merge when green |

Filed issue: #657 (`active_objective_ids = [step_id]` relog defect, cross-mission).

## What is left

1. Merge the closeout docs PR (#669) if it is still open. Every implementable packet (CA00-CA10) is merged.
2. User in-client UAT M1 to M5 (below). Only CA00 was on main before this session; everything else is now on main and needs a rebuild by the user (`setup.ps1`).
3. Design-gated, not started: CA13 (NPC over-time movement), CA14 (`castle.nav`), CA15 (optional Level-5 branches, D-CA14). CA11/CA12 come from Harset H03. CA16 (documentation sync) is Ready and carries the `mission-chains.md` / `zone-audit.md` sync deferred out of #667; CA17 (`display_dialog` `target_tag`) is a follow-up candidate.
4. Small open cleanups:
   - `origin/main` `Cargo.lock` is inconsistent (thiserror 2.0.19 edges, only 2.0.20 defined), so every cargo run re-dirties it. One standalone fix commit on main; workers discard that delta before each `git add`.
   - Harset merge rule: the `entity_templates.sql` `setval` line is 173 on #667 and 248 on Harset's branches; whoever merges second resolves to 248.
   - Cellblock chains 1053/1054 clear the mission-active glow on template 10 (`Preparation_ColMarsh`, shared with `Castle_ColMarsh`, baseline 0) and were not examined; the Cellblock session was not reachable at closeout.

## UAT (user in-client)

| Milestone | What to do |
|---|---|
| M1 | Arrive on the ring platform (688 complete, 1360 active); die once at each World 8 checkpoint and land on real ground; Human sees 2573, Jaffa 5861, `!` over Gerschon before accept, 701 accepted once; Copplemann advances without a wave; Livewire win shows 2575 once; 2576 completes 701 and accepts 702 (+703); relog at every step. |
| M2 | Zuritska (male, `Castle_Zuritska_Cell` at 268.0, 66.79, 1042.59) and Romney exist for every player; freeing Zuritska completes 702 once; killing Romney completes 703; hostile Castle mobs respawn after 120 s. Entering the Interrogation Block should fire 702 step 2402 (region boxes now have ceilings; watch for a box that does not fire). |
| M3 | Zuritska follows to the Communications room (Level 5) or the step advances on region entry; terminal Livewire grants 5029 once; delivery starts 706; ThroneRoom entry advances 2411; Access Panel completes 706. |
| M4 | Surrender or panel diagnosis reveals the crystal; Bravo or Muelbach (bunker above Bravo) grants 2790 once; Human reports to Marsh, Jaffa to Moh'katan, never both; DHD Livewire; gate opens after 4 s; crossing plays and lands on Harset once. |
| M5 | Two players at different steps do not disturb each other's indicators, actors or steps; a second player on step 2417 can still click Marsh after the first reports in. |

Provisional coordinates (RECONSTRUCTION, MEDIUM confidence): the four CA00 respawners, the Op-Core respawner, the Armory prefab, the comms-room placement. Report any bad spot.

## Worktrees

All under `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\`, each with `external/` junctioned to the primary checkout. Merged and removable once nothing is dirty: `castle-ca00`, `castle-ca02`, `castle-ca04`, `castle-ca05`, `castle-ca10`, `castle-m701`, `castle-m706` (the local branch delete after `gh pr merge` fails harmlessly while a worktree has the branch checked out). Keep `castle-coordinator` until #669 merges (`castle-m702` is now removable too). Each worktree's `target/` is 5-10 GB; `git worktree remove` reclaims it.

Workers were `rust-gameserver-dev` agents continued with `SendMessage`, plus two `testing-validation-engineer` reviewers; after `/clear` they are gone. Trap: a base sha recorded for a stacked branch can be rewritten by review rounds (m706 was stacked on CA10 `0d3c9147`, which no longer existed). Find the true fork point with `git log` / `git merge-base` and use `git rebase --onto`.

## Build lane v2

Scripts in `C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\` (not in git): `lane.sh` (counting semaphore, slot count in `%LOCALAPPDATA%\cimmeria-build\lane\SLOTS`, currently 2; `--exclusive` takes all), `reload-db.sh` (per-worktree database `sgw_<worktree>`), `live-db-test.sh <filter>`. Worktree-local `target/`, sccache at `%LOCALAPPDATA%\cimmeria-build\bin\sccache.exe`. D: must not be used. Do not raise SLOTS above 2 without re-measuring RAM (low-water was 11 GB free on 64 GB). Worker rules are in `WORKER-RULES.md` beside them.

Never run git mutations in the primary checkout (shared by several sessions). It holds uncommitted `.claude/agent-memory/rust-gameserver-dev/` edits (db-test-revert-verification.md and a MEMORY.md line, CA02's wire-format note) that were not swept for that reason: commit them from a worktree copy or ask the user.

## Cross-session agreements

- Harset H01 dropped its stargate-region routing; Castle CA10 owns `REGION_FLAG_STARGATE` routing and the dial state; H01 keeps `validate_gate_arrival` and the DHD display. Rule: exactly one `validate_gate_arrival` call at each destination-placement site (volume-entry travel, immediate-travel fallback). Whoever lands second verifies it.
- The Harset coordinator (`cimmeria-f0`) keeps an operator guide, `docs/analysis/zone-restoration-operator-guide.md` on branch `content/harset-wave2`, with a Castle block; send it one-line corrections when Castle status changes.
- Owner usage constraint (2026-09-18): at most 2-3 subagents at once, commit WIP early, no wide fan-outs.

## Findings

Recorded in the [README](../README.md) "Implementation Session Record" table: interaction pin, bind-slot fold, stargate, CA05 evidence, the shared-NPC glow rule, follow-ups.

## Exact next actions on resume

1. `gh pr list --search castle` and `git worktree list`; reconcile with the table above.
2. Merge #669 if still open; remove merged worktrees.
3. Check main CI is green (`gh run list --branch main --limit 3`).
4. Report to the user: UAT M1-M5, provisional coordinates, design-gated packets, the Cellblock 1053/1054 finding, the `Cargo.lock` cleanup.
