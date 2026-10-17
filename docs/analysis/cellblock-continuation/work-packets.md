# Cellblock continuation: work packets

> **Type:** Work packets (canonical; ported from the main-checkout overlay on 2026-10-16, D-CC2)
> **Audience:** The Cellblock orchestrator and the agents it dispatches
> **Last updated:** 2026-10-17
> **Scope source:** [cellblock-work-packets.md](../four-zone-planning/cellblock-work-packets.md). Packets that already have a section there are referenced by its heading and are not retyped here. This file holds the dispatch rules, the contract, the waves, the W1b advisor and integration packets. The W1b coder packets (CB-N02b, CB-N02c, CB-N02d) are in [w1b-packets.md](w1b-packets.md). Wave 2 (CB-N04s, CB-N04m, CB-N04a, W2-INT, and the CB-N07 hold) is in [w2-packets.md](w2-packets.md), which carries its own contract with the W2 feature SHA.

## Dispatch rules

1. One worktree and one test database per worker. Worktrees come from the recipe in the contract below. The orchestrator dispatches no packet until the worktree's HEAD equals the recorded feature SHA.
2. Workers commit locally and stop. They never push, open a PR, merge, or edit another worktree. Reviewed packet commits are cherry-picked into `content/cellblock-campaign` by the integration packet of the wave, in the order it lists. One remote PR per campaign: `content/cellblock-campaign` to `main` at CB-N08.
3. **Review is split from fixing (D-CC6, owner ruling 2026-10-16).** A read-only `packet-reviewer` reviews; a `packet-coder` implements the verified findings in the packet's worktree; a fresh `packet-reviewer` re-reviews the fixed packets. At most 3 rounds, after which the packet goes back to the architect. Do not use `packet-review-fixer` in this campaign.
4. Haiku agents (`packet-coder`) must stay under 100k total tokens; the 90k hook is a deliberate stop-gap. Packets are sized at about 40k working tokens.
5. No packet in this file takes a lab lease. Lab rows (CB-U03, CB-U05, CB-U09) run only after the orchestrator asks the user and the lab is free (D-CC3 covers this orchestrator under O14-08).
6. Seed header bookkeeping (the chain-range comments at the top of `castle_cellblock_chains.sql`) and `id-allocation.md` are edited only by the integration packet, never by a coder packet (D-CC8). This keeps parallel packets free of conflicts.

## Contract

| Item | Value |
|---|---|
| Feature branch | `content/cellblock-campaign` (worktree `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock`) |
| Recorded feature SHA | **`e10fd1f0a86383129d3bcb7b44a0b6cebabecc72`** (W1a integrated, 2026-10-16). History: `14658b3b1` (D-CC1 = B checkpoint), then CB-N02s/CB-N02a cherry-picks. |
| Packet branch / worktree | `cellblock/<id>-<slug>` in `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-<id>` |
| Worktree recipe | 1. Ops: `pwsh tools/build-lane/mk-worktree.ps1 cellblock/<id>-<slug> cellblock-<id>` (branches from `main`). 2. Ops, or a `packet-coder` if ops refuses (§ W1b-R0): `git -C <wt> rebase --onto <feature-sha> HEAD cellblock/<id>-<slug>` (the new branch has no commits, so this moves it onto the feature SHA). 3. Verify `git -C <wt> rev-parse HEAD` = `<feature-sha>` and `git -C <wt> status --porcelain` is empty. Stop on any mismatch; never reset. |
| Status vocabulary | Ready, BlockedDependency, BlockedDecision, BlockedEvidence, Writing, Review, Integrated, UATPending, Done |
| Report path | `<wt>\.claude\reports\cellblock-continuation\<packet>-<role>.md` (inside the worker's own worktree, never the root checkout). Before retiring a worktree, the orchestrator copies anything worth keeping to `worknotes/` on the branch. |
| Return line | Exactly one line: `<packet> <done\|blocked\|failed> <commit-sha\|-> <report-path>` |
| Measurement | Every report ends with `## Measurement`: working tokens (the number if the harness shows it, else `not exposed`), wall-clock start and end (UTC, from `Get-Date -AsUTC`), lane runs, review rounds (filled in by the reviewer). |
| Attribution | Commit trailer lines are given in the dispatch brief; copy them exactly. |
| Relog restore shape | Trigger `player_loaded`, event key `Castle_CellBlock` (exact spelling, capital B), scope `player`, `once` false. Conditions gate on the exact step that needs the state. Context keys the engine fills on login: `mission_<m>_status`, `mission_<m>_step_<s>_status`, `mission_<m>_obj_<o>_status` (values `active`, `completed`; a missing key reads as `not_active`). Precedents: 1045 (`set_interaction_type` restore), 1009 (`set_aggression` re-arm). |
| Test shape (D-CC4) | Seed-only restores are tested at resolve level: load the chain with `load_single_chain_for_test`, register it in a `ChainEngine`, fire a `TriggerType::PlayerLoaded` event with `world_name = "Castle_CellBlock"` and the step params, and assert on `engine.resolve_event(..).actions` filtered by chain id. Test fn names must contain `live_db` (the live-DB group guard fails otherwise). |

### Repo rules every packet carries

1. **PowerShell only**, no bash. Begin every command with `Set-Location <wt>;` and use absolute paths. `[IO.File]` calls take absolute paths only.
2. **Never run `cargo` directly.** Every fmt, compile, test or clippy goes through `pwsh tools/build-lane/lane.ps1 cargo ...`. Live-DB tests go through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads this worktree's own `sgw_<wt>` database from seeds first. Run lanes in the foreground and read their logs under `logs/<worktree-name>/`.
3. **No `git stash`, no `git worktree prune`, no `git reset`, no push.** Commit locally once the checks pass.
4. **Seeds are the source of truth.** Edit `db/resources/` only. Never add `db/scripts/*.sql`.
5. **File caps:** soft 500 lines, hard 700. Check `(Get-Content <file>).Count` before and after. If a file you must extend would end above 500, stop and report `blocked`.
6. **Line endings.** Docs under `docs/` are CRLF: after writing one, run `$p='<abs path>'; $t=[IO.File]::ReadAllText($p) -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($p,$t)`, then after `git add` check `git -C <wt> ls-files --eol <path>` shows `w/crlf`. `castle_cellblock_chains.sql` is stored CRLF (`i/crlf`): keep it that way, and check it the same way. Rust files are `i/lf`; the Edit tool keeps them as they are.
7. **Edit only the files your packet lists.** Any other file needed means `blocked`. Never edit the seed's top-of-file chain-range header or `id-allocation.md` (D-CC8: the integration packet owns them).
8. **Chain IDs** come only from the packet (claimed in [id-allocation.md](../four-zone-planning/id-allocation.md)). Never pick a free ID yourself.
9. Run `git -C <wt> diff --check` before committing.
10. If a lane check fails twice for the same reason, stop and report `failed` with the log path. Do not widen the change to make it pass.

## Waves

| Wave | Packets | Gate |
|---|---|---|
| W1a (pilot) | CB-N02s, then CB-N02a | **Integrated** at `e10fd1f0a`. CB-U03 client check of 1026/1027 UATPending. |
| W1b | W1b-R0 (only if ops refuses the rebase), then in parallel CB-N02b, CB-N02c, CB-N02d, CB-N02s2, CB-N03s; review; W1b-INT | **Integrated** at `0fca3114f`. CB-U03 UATPending for 1028/1036/1037. |
| W1c | Children sealed by CB-N02s2 or CB-N03s (for example CB-N03a), each from a new architect pass | **Not triggered**: neither advisor sealed a child (2026-10-17 architect pass). CB-N03a reopens only if CB-U05 shows a seed fault. |
| W2 | [w2-packets.md](w2-packets.md): CB-N04s, then CB-N04m, then CB-N04a; review; W2-INT; CB-U08 (ask first). CB-N07 held (BlockedEvidence on CB-U05 and CB-U09, BlockedDecision D-CC11). | W1 integrated (met). Feature SHA `0fca3114f`. |
| W3 | CB-N05 (Remaining Pods), then CB-N05b (held on HA-1360), then CB-N06 (conditional) | CB-U09 measured |
| W4 | CB-N08 | All above Integrated or UATPending with documented lab unavailability (O14-09) |

---

## W1a (Integrated)

- **CB-N02s** (`mission-systems-advisor`): relog-restore audit, [worknotes/cb-n02s.md](worknotes/cb-n02s.md). Sealed gaps A, B and C, which became CB-N02b, CB-N02c and CB-N02d in [w1b-packets.md](w1b-packets.md). Two corrections to that worknote: `route-baseline.md` is present on this branch (its core route matches the worknote's assumption), and the three `unchecked` objects are restored on login: `Cellblock_WoodenCrate` by 1104 (step 2354), `Cellblock_TerminalX` by 1110 (step 2356), `Cellblock_ArmoryRingSwitch` by 1111 (step 80688).
- **CB-N02a** (`packet-coder`): chains 1026/1027 restore Prisoner 329's archetype dialog on login while 638 is active, [worknotes/cb-n02a.md](worknotes/cb-n02a.md).
- Measurements: CB-N02s about 78k tokens and 5 minutes; CB-N02a about 66k tokens and 22 minutes.

## W1b dispatch plan

| Packet | Agent | Worktree | Runs |
|---|---|---|---|
| W1b-R0 | `packet-coder` | the five below | Only if ops refuses `rebase --onto`. Before everything else. |
| [CB-N02b](w1b-packets.md#cb-n02b--keep-the-329-cell-door-button-live-across-relog) | `packet-coder` | `cellblock-cb-n02b` | parallel |
| [CB-N02c](w1b-packets.md#cb-n02c--keep-the-ambernol-vial-usable-across-relog) | `packet-coder` | `cellblock-cb-n02c` | parallel |
| [CB-N02d](w1b-packets.md#cb-n02d--keep-the-retrieval-drone-hostile-across-relog) | `packet-coder` | `cellblock-cb-n02d` | parallel |
| CB-N02s2 | `mission-systems-advisor` | `cellblock-cb-n02s2` | parallel |
| CB-N03s | `npc-ai-spawn-advisor` | `cellblock-cb-n03s` | parallel |
| Review | `packet-reviewer` x2 | read-only | after all five return: reviewer A takes CB-N02b/c/d, reviewer B takes CB-N02s2/CB-N03s |
| Fixes | `packet-coder` per worktree | that packet's worktree | only for packets with verified findings; brief = the finding list and the packet section |
| Re-review | fresh `packet-reviewer` | read-only | fixed packets only; at most 3 rounds |
| W1b-INT | `packet-coder` | `zone-cellblock` | after every W1b packet has a `ship` verdict |

The five packets share no file except the seed, where CB-N02b, CB-N02c and CB-N02d each insert one block at a different anchor (after chain 1027, after chain 1031, after chain 1032). Each test lives in a different file. So they run in parallel and cherry-pick cleanly in the W1b-INT order.

Each reviewer report gives a per-packet verdict (`ship` or `fix`) and verified findings only. Reviewers check: the diff touches only the packet's files; the chain matches the packet's SQL exactly; the test fails on revert (rerun the revert proof if in doubt); line endings; file caps; the worknote's claims against the seed.

---

## W1b-R0 — Rebase the W1b worktrees onto the feature SHA (fallback)

| Field | Value |
|---|---|
| Agent | `packet-coder`. Dispatch only if ops refuses step 2 of the worktree recipe. |
| Size | Small: five git commands and five checks |
| Writes | No files. No commit. |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\W1b-R0-coder.md` |

Worktrees (`<wt>` is under `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\`):

| `<wt>` | `<branch>` |
|---|---|
| `cellblock-cb-n02b` | `cellblock/cb-n02b-638-door-button-relog` |
| `cellblock-cb-n02c` | `cellblock/cb-n02c-639-vial-relog` |
| `cellblock-cb-n02d` | `cellblock/cb-n02d-639-drone-relog` |
| `cellblock-cb-n02s2` | `cellblock/cb-n02s2-restore-bits` |
| `cellblock-cb-n03s` | `cellblock/cb-n03s-flank-seal` |

For each row:

1. `git -C <wt> status --porcelain` must be empty and `git -C <wt> branch --show-current` must print `<branch>`. Otherwise stop and report `blocked`.
2. `git -C <wt> rebase --onto e10fd1f0a86383129d3bcb7b44a0b6cebabecc72 HEAD <branch>`
3. `git -C <wt> rev-parse HEAD` must print `e10fd1f0a86383129d3bcb7b44a0b6cebabecc72`.

Report one row per worktree (branch, HEAD after, OK or the error). Repo rules 1 and 3. Return `W1b-R0 done - <report-path>`.

---

## CB-N02s2 — Bit-level check of the existing relog restores

| Field | Value |
|---|---|
| Agent | `mission-systems-advisor` |
| Size | About 40k working tokens. Static: no lab, no code, no build. |
| Branch / worktree | `cellblock/cb-n02s2-restore-bits` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n02s2` |
| Writes | `docs/analysis/cellblock-continuation/worknotes/cb-n02s2.md` (new) only |
| Commit | `docs(cellblock): verify relog restores bit by bit` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n02s2\.claude\reports\cellblock-continuation\CB-N02s2-mission-systems-advisor.md` |

**Why.** CB-N02s matched the 622, 640, 641, 680, 687 and 688 restores by chain existence only ([cb-n02s.md](worknotes/cb-n02s.md), note under the binding table). A restore that covers the wrong step or misses a bit strands the player as surely as no restore. Two presentation states were also never checked: the cell door opened by `play_sequence 1749` (chain 1017, Livewire victory at 2115 -> 2116) and sequence 10001 (chain 1032).

**Read.** `db/resources/Content/Seed/castle_cellblock_chains.sql` with Grep only (`-B 20`, `output_mode: content`) for `'set_interaction_type'`, `'set_aggression'`, `'add_dialog_set'`, `'play_sequence'` and `'player_loaded'`. Context keys: `crates/cell-content/src/cell/content/mission_context.rs` lines 110-195.

**Do.**

1. For every setter of `set_interaction_type`, `set_aggression` or `add_dialog_set` in missions 622, 640, 641, 680, 687 and 688, one row: tag or dialog set, mask/level, setter chain and step, clearer chain and step, restore chain, the restore's exact condition, and a verdict: `exact` (same tag, same mask, gate covers exactly the window), `over` (restores outside the window: say what that does), `under` (a step in the window has no restore), or `missing`.
2. For `play_sequence 1749` and `10001`: after a relog in the step that follows them, is the state they produce (door open, indicator shown) needed to progress? Answer from the seed and the route baseline. If the door's state lives in the client map, say so and name the owner from [presentation-handoff.md](presentation-handoff.md) (asset audit CA-03C/D). Do not propose a client patch.
3. For each `under`, `missing` or harmful `over`, write a sealed child section in the shape of CB-N02b in [w1b-packets.md](w1b-packets.md) (fault, paper repro, exact chain SQL, test file and its line count, `IDs: coordinator allocates`).
4. End with `## Verdict` (`no new gaps` or the child list) and `## Measurement`.

**Checks.** `git -C <wt> diff --check`; rule 6 CRLF; `pwsh tools/lint-md.ps1 docs/analysis/cellblock-continuation/worknotes/cb-n02s2.md` (warnings OK, errors not).

**Not in scope:** fixing anything, lab runs, the Marsh escort follow state (known gap, out of scope per cb-n02s.md), chain 1109.

**Repo rules:** 1, 3, 6, 7, 8 and 9.

---

## CB-N03s — Seal check for flank and cover reachability

| Field | Value |
|---|---|
| Agent | `npc-ai-spawn-advisor` |
| Size | About 40k working tokens. Static: no lab, no Ghidra, no code, no build. |
| Branch / worktree | `cellblock/cb-n03s-flank-seal` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n03s` |
| Writes | `docs/analysis/cellblock-continuation/worknotes/cb-n03-seal.md` (new) only |
| Commit | `docs(cellblock): seal check for flank and cover reachability` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n03s\.claude\reports\cellblock-continuation\CB-N03s-npc-ai-spawn-advisor.md` |

**Why.** [CB-N03](../four-zone-planning/cellblock-work-packets.md#cb-n03--restore-flank-and-cover-reachability) needs a present, sealed miss in 681/686 before any coder runs. [route-baseline.md](route-baseline.md) row CB-B06 establishes no current defect, and no lab run has measured CB-U05. This packet decides from static evidence whether there is a fault to fix.

**Read.**

1. `route-baseline.md`: rows CB-B06, CB-U05, and the world-12 spawn table rows for `MessHall_Guard1/2` (spawns 29/28) and `Hallway05_Guard1/2` (spawns 32/33), all template 24.
2. `castle_cellblock_chains.sql`: Grep `player_flanked_npc` with `-B 45 -A 20` (chains 1141/1142 and the C06 note).
3. `crates/cell-content/src/cell/content/chain_replay_tests/mission_681_686_flank.rs` and `mission_681_686_backstop.rs` (what is already guarded).
4. `crates/cell-content/src/cell/content/event_dispatch/cover.rs` (where `player_flanked_npc` is raised and what it requires).
5. The NPC cover-seek code: find it from `crates/cell/src/cell/service/tests/npc_ai_cover.rs` (search radius, node eligibility, navmesh checks).
6. `db/resources/AI/Seed/cover_sets.sql` and `cover_nodes.sql`: world 12 rows only (set id = 12 * 100000 + n). Grep, never read whole files (10k lines).

**Do.**

1. For each of the four guards: the nearest cover nodes (set id, node position, distance), whether each is inside the cover-seek radius and eligible by the code's rules, and whether `player_flanked_npc` can fire for this guard as the code stands (its name match `'NID Guard'`, its required AI state).
2. Verdict, exactly one:
   - **Seed fault**: write a sealed **CB-N03a** body for `packet-coder` in the shape of CB-N02b in [w1b-packets.md](w1b-packets.md): exact rows to change in `spawnlist.sql` or the cover seeds, the test to extend in `mission_681_686_flank.rs` or `mission_681_686_backstop.rs` (with line counts) and why it fails on revert, `IDs: coordinator allocates` for any new row.
   - **AI defect**: name the exact `crates/cell-combat/src/cell/` (or `crates/cell/`) file and function, the failing condition, and the test to add. This becomes CB-N03b for `rust-gameserver-dev`, never a Haiku packet.
   - **No static fault**: say what only a client run can show, and write the CB-U05 lab steps (positions, what to watch in SigNoz). CB-N03 then becomes BlockedEvidence on CB-U05.
3. End with `## Measurement`.

**Checks.** As CB-N02s2, on `worknotes/cb-n03-seal.md`.

**Not in scope:** editing seeds or code, lab runs, new enemies (CB-N03 forbids adding an enemy for credit), CB-N04 placement.

**Repo rules:** 1, 3, 6, 7, 8 and 9.

---

## CB-N03a — Flank/cover seed fix (conditional)

BlockedDependency on CB-N03s. Exists only if CB-N03s returns **Seed fault**. A new architect pass copies the sealed body from `worknotes/cb-n03-seal.md` into this section, checks its IDs against id-allocation.md, adds the lane checks (`mission_681_686` filters) and dispatches it in W1c.

---

## W1b-INT — Integrate wave 1b into content/cellblock-campaign

| Field | Value |
|---|---|
| Agent | `packet-coder` (ops if it can cherry-pick) |
| Size | About 25k working tokens |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock` (branch `content/cellblock-campaign`) |
| Depends on | Every W1b packet has a reviewer verdict `ship` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\W1b-INT-coder.md` |

1. `git -C <wt> status --porcelain` must be empty and HEAD must be the recorded feature SHA (or a later commit the brief names). Otherwise `blocked`.
2. Cherry-pick, in this order, every commit the brief lists for each packet (packet commit, then its fix commits): CB-N02s2, CB-N03s, CB-N02b, CB-N02c, CB-N02d. Use `git -C <wt> cherry-pick <sha>`. On a conflict, run `git -C <wt> cherry-pick --abort` and report `blocked` with the conflicting paths. Never resolve by hand.
3. Bookkeeping commit `docs(cellblock): record W1b chains 1028, 1036 and 1037`, touching only:
   - `castle_cellblock_chains.sql` header: below the line `--   Mission 638 login restore (2026-10-15): chains 1026-1027, ...` add `--   Mission 638 door-button relog restore (2026-10-16, CB-N02b): chain 1028.`; below `--   Mission 639:  chains 1031-1040` add `--   Mission 639 relog restores (2026-10-16, CB-N02c/d): chains 1036 (vial bit) and 1037 (drone re-arm).`; in the `(next free inside 1001-1111: ...` line change `1028-1030, 1036-1040` to `1029-1030, 1038-1040`.
   - `docs/analysis/four-zone-planning/id-allocation.md`: in the "Chain IDs in use today" Cellblock row change the count `122` to `125` and `1001–1027, 1031–1034` to `1001–1028, 1031–1034, 1036–1037`; in the "CB-N02 restore claims — 2026-10-16" subsection change the status column of all three rows to `In use (W1b-INT)`.
   - `docs/analysis/cellblock-continuation/README.md`: the packet rows for CB-N02b, CB-N02c, CB-N02d, CB-N02s2 and CB-N03s become `Integrated` (CB-N02b/c/d also `UATPending` on CB-U03), each with its integrated SHA.
4. Lane checks (prefix `Set-Location <wt>;`): `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`, then `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content --filter-expr 'test(mission_638) | test(mission_639)'`, then `pwsh tools/build-lane/live-db-test.ps1 mission_63`. All new tests must pass, not skip.
5. Report the integrated SHAs. No push.

Repo rules 1, 2, 3, 6, 9 and 10.

---

## Packets specified on the campaign branch

CB-N05, CB-N05b, CB-N06, CB-N07 and CB-N08 are dispatched from their sections in [cellblock-work-packets.md](../four-zone-planning/cellblock-work-packets.md). CB-N04 has passed its architect check; its packets are in [w2-packets.md](w2-packets.md). Their statuses are in [README.md](README.md#packet-status). Before any of them goes to `packet-coder`, an architect pass checks it against the 40k budget and this file's contract and repo rules. CB-N05 (new mission, six nested missions, dialog, screen, chains 11000-11005) will need splitting.
