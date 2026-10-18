# Cellblock continuation: wave 4 packets and close-out

> **Type:** Work packets (canonical, on `content/cellblock-campaign`)
> **Audience:** The Cellblock orchestrator, and the agents it dispatches for CB-N03t, CB-N07s, CB-N07m, CB-N07a, W4-INT, CB-N08a, CB-N08b, CB-N08-CI and CB-N08-R
> **Last updated:** 2026-10-17
> **Companions:** [work-packets.md](work-packets.md) (dispatch rules, waves), [w3-packets.md](w3-packets.md) (wave 3, runs first), [README.md](README.md) (status, D-CC decisions), [owner-lab-checklist.md](owner-lab-checklist.md), [CB-N07 contract](../four-zone-planning/cellblock-work-packets.md#cb-n07--one-rooms-authored-encounter-response-conditional), [CB-N08 contract](../four-zone-planning/cellblock-work-packets.md#cb-n08--full-escape-acceptance-and-campaign-closeout), [execution contract](../four-zone-planning/execution-contract.md)

This file pre-writes every packet left in the campaign, so that what remains is coding, review and the owner's lab rows.

- **No packet drives the lab** (owner directive, 2026-10-17). No packet takes a lab lease or dispatches `lab-driver`. Lab evidence comes only from the owner's rows in [owner-lab-checklist.md](owner-lab-checklist.md).
- **CB-N07 ships a default-safe tier (D-CC16).** CB-N07a adds one gated, fire-once NID Guard alarm line in the barracks. It adds no spawn, aggression, threat, objective, death risk or time, so it needs no D-CC11 answer. Lab-dependent inputs are stated as assumptions A1-A4 and checked by the owner in the new row CB-U11. Anything that changes difficulty (Tier 1) stays held as CB-N07b.
- **CB-N03t** is an optional static test: the other three cover guards hold, defend and release like `MessHall_Guard1`. It strengthens CB-N03's evidence before CB-U05; it does not replace CB-U05.
- **Held, not pre-writable:** CB-N07b (D-CC11 plus CB-U05/U09), CB-N06 build children (D-CC15 names the quest and prop), CB-N05b (HA-1360). See [§ Held](#held-packets).
- **CB-N08** is split into four parts under the 40k cap: CB-N08a (UAT reconciliation), CB-N08b (close-out docs), CB-N08-CI (full local CI) and CB-N08-R (campaign-level review). Ops then ships one PR.

## Dispatch plan

| Step | Packet | Agent | Worktree / branch | Base | Runs after |
|---|---|---|---|---|---|
| E1 | [CB-N03t](#cb-n03t--cover-guard-seed-test-optional) | `packet-coder` | `cellblock-cb-n03t` / `cellblock/cb-n03t-cover-guards` | W4 architect commit | dispatch; may run beside W3 |
| E1 | [CB-N07s](#cb-n07s--barracks-response-seal) | `npc-ai-spawn-advisor` | `cellblock-cb-n07s` / `cellblock/cb-n07s-barracks-seal` | W4 architect commit | dispatch; may run beside W3 |
| E2 | [CB-N07m](#cb-n07m--mission-sign-off) | `mission-systems-advisor` | `cellblock-cb-n07s` (same worktree) | CB-N07s tip | CB-N07s `done` with verdict `seal` |
| 1 | [CB-N07a](#cb-n07a--barracks-alarm-bark) | `packet-coder` | `cellblock-cb-n07a` / `cellblock/cb-n07-barracks` | **W3-INT commit** | W3-INT `done` and CB-N07m `approve` or `change` |
| 2 | W4a review | `packet-reviewer` (read-only), one for all four packets | the three worktrees | - | CB-N03t, CB-N07m and CB-N07a returned |
| 3 | Fixes, then re-review | `packet-coder` in the owning worktree, then a fresh `packet-reviewer` | owning worktree | - | only on `fix`; at most 3 rounds (D-CC6) |
| 4 | [W4-INT](#w4-int--integrate-wave-4-and-bookkeeping) | `packet-coder` | `zone-cellblock` / `content/cellblock-campaign` | branch tip | every W4a packet `ship` |
| - | CB-U03, U05, U08, U09, U10, U11 | **the owner** | lab | - | after W4-INT; never dispatched by an agent |
| - | D-CC15 (and D-CC11 only if CB-N07b is wanted) | orchestrator asks the owner | - | - | after the owner's CB-U09/U10 results |
| 5 | [CB-N08a](#cb-n08a--uat-reconciliation) | `testing-validation-engineer` | `zone-cellblock` | W4-INT tip | owner lab results recorded, or O14-09 documented unavailability |
| 6 | [CB-N08b](#cb-n08b--close-out-docs) | `documentation-writer` | `zone-cellblock` | CB-N08a tip | CB-N08a `done` |
| 7 | [CB-N08-CI](#cb-n08-ci--full-local-ci) | `packet-coder` | `zone-cellblock` | CB-N08b tip | CB-N08b `done` |
| 8 | [CB-N08-R](#cb-n08-r--campaign-review) | `packet-reviewer`, fixes by `packet-coder` | `zone-cellblock` | CB-N08-CI tip | CB-N08-CI `done`; at most 3 rounds, CI rerun after any fix |
| 9 | [Ship](#ship-merge-and-retire) | `ops` | `zone-cellblock` | - | CB-N08-R `ship` |

E1 and E2 touch only new files (one test file plus one module line, and one seal note), so they may start as soon as this commit lands, beside wave 3. CB-N07a edits the same seed files as CB-N05-2 and CB-N05-3, so it waits for W3-INT and branches from it. It reads the seal from the `cellblock-cb-n07s` worktree by absolute path; W4-INT cherry-picks the seal commits too. Steps 5-9 run in the one integration worktree, one at a time.

**Stop rules (every packet).**

- Any needed file outside the packet's **Writes** list: stop, report `blocked`.
- A seed loader that refuses the row, or a test that fails on unmodified code where the packet says it should pass: stop, report `blocked` with the observed result. Never weaken an assertion or edit an unrelated seed row.
- CB-N07s or CB-N07m returns `reject`: CB-N07a is not dispatched; CB-N07 is recorded `Done / omitted` by CB-N08b with the seal as evidence.

## Contract

| Item | Value |
|---|---|
| Feature branch | `content/cellblock-campaign` (worktree `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock`) |
| Base SHA | E1 packets: **the W4 architect commit** (the newest commit that adds this file; the main-checkout overlay `RESUME.md` names it). CB-N07a: **the W3-INT commit** (W3-INT's return line names it). Steps 5-9: the zone-cellblock tip left by the step before. |
| Worktree recipe | 1. Ops: `pwsh tools/build-lane/mk-worktree.ps1 <branch> <wt-name>` (branches from `main`). 2. Ops: `git -C <wt> rebase --onto <base> HEAD <branch>`. 3. Verify `git -C <wt> rev-parse HEAD` = `<base>` and `git -C <wt> status --porcelain` is empty. Stop on any mismatch; never reset. |
| Report path | `<wt>\.claude\reports\cellblock-continuation\<packet>-<role>.md` (role `coder`, `reviewer`, or the agent name) |
| Return line | Exactly one line: `<packet> <done\|blocked\|failed> <commit-sha\|-> <report-path>` |
| Measurement | Every report ends with `## Measurement`: working tokens (or `not exposed`), wall-clock start and end (UTC, `Get-Date -AsUTC`), lane runs, review rounds. |
| Claimed IDs (CB-N07) | Chain **11020**; dialog screen **200170**, under the existing holder dialog **60160** (reused, not claimed). [id-allocation.md § CB-N07 claims](../four-zone-planning/id-allocation.md#cb-n07-claims--2026-10-17). Never pick another ID. |
| Bark shape (precedent 11010) | Trigger `enter_region`, scope `player`, `once` true. One condition `step_status`, value `active`. One action `npc_bark` with params `{"screen_id": N, "speaker": "...", "channel": "say"}`. Text lives in `dialog_screens` under holder dialog 60160, which is never displayed. Context key format: `mission_<m>_step_<s>_status`; a missing key reads as `not_active`. The H52 step-activation replay re-fires mission-gated `enter_region` chains for a player already inside the region; `once` stops a second line in one visit. |
| Review (D-CC6) | One read-only `packet-reviewer` per wave; fixes by `packet-coder` (or the packet's own agent) in the owning worktree; a fresh `packet-reviewer` re-reviews; at most 3 rounds. **No `packet-review-fixer`.** Haiku agents stay under 100k tokens each. |
| Telemetry | Existing only: `content.npc_bark` span and `Content: npc bark` INFO with `chain_id` and `screen_id`. No new telemetry code. |

### Repo rules every packet carries

1. **PowerShell only**, no bash. Begin every command with `Set-Location <wt>;` and use absolute paths. `[IO.File]` calls take absolute paths only.
2. **Never run `cargo` directly.** Every fmt, compile, test or clippy goes through `pwsh tools/build-lane/lane.ps1 cargo ...` (fmt too: format new Rust with `pwsh tools/build-lane/lane.ps1 cargo fmt -p <crate>`, then check with `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`; the code shapes below are not rustfmt-formatted). Live-DB tests go through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads this worktree's own `sgw_<wt>` database from seeds first. Run lanes in the foreground; read their logs under `logs/<worktree-name>/`.
3. **No `git stash`, no `git worktree prune`, no `git reset`, no push.** Commit locally once the checks pass.
4. **Seeds are the source of truth.** Edit `db/resources/` only. Never add `db/scripts/*.sql`.
5. **File caps**: soft 500 lines, hard 700, for Rust and new docs. Check `(Get-Content <file>).Count` before and after. Seed SQL and `docs/content/mission-chains.md` are exempt (append only).
6. **Line endings.** Docs under `docs/` are CRLF: after writing one, run `$p='<abs path>'; $t=[IO.File]::ReadAllText($p) -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($p,$t)`; after `git add`, `git -C <wt> ls-files --eol <path>` must show `w/crlf`. Stored `i/crlf`: `castle_cellblock_chains.sql`, `docs/content/mission-chains.md`. Stored `i/lf w/crlf`: `dialog_screens.sql`. For every seed file, `git -C <wt> diff --stat <path>` must count only your changed lines; a whole-file count means the endings broke. Rust files are `i/lf`; the Edit tool keeps them.
7. **Edit only the files your packet lists.** Never edit the chain seed's top-of-file range header or `id-allocation.md` (D-CC8: W4-INT owns them). Never hand-edit a `<!-- gen:NAME -->` block.
8. **IDs come only from the contract's claimed list**; copy and SQL only from this file or the seal, byte for byte.
9. **Every test must fail when its change is reverted.** Each packet names its revert or mutation proof; do it once (change, run, see the failure, undo with the Edit tool, rerun green) and record the failing assertion in the report.
10. Run `git -C <wt> diff --check` before committing. If a lane check fails twice for the same reason, stop and report `failed` with the log path. Do not widen the change to make it pass.

---

## CB-N03t — Cover-guard seed test (optional)

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 30k working tokens |
| Branch / worktree | `cellblock/cb-n03t-cover-guards` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n03t` |
| Writes | `crates/cell/src/cell/service/tests/npc_ai_cover_seed_guards.rs` (new, under 250 lines); `crates/cell/src/cell/service/tests/mod.rs` (one line) |
| Commit | `test(cellblock): pin the other three cover guards' hold and flank release` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n03t\.claude\reports\cellblock-continuation\CB-N03t-coder.md` |

**Why.** `npc_ai_cover_seed.rs` proves only `MessHall_Guard1` (spawn 29) holds its marker, takes Cover Stance and releases when flanked, on the real navmesh and the real world-12 cover seed. CB-U05 covers `MessHall_Guard2` and both `Hallway05` guards. This test gives them the same static proof. Optional: skip it if the orchestrator wants to save the tokens; CB-U05 still decides CB-N03.

**Read first:** `crates/cell/src/cell/service/tests/npc_ai_cover_seed.rs` (whole file, 381 lines). It exports `pub(super)`: `NPC`, `PLAYER`, `Seeded`, `seeded()`, `node_pos()`, `mess_hall_guard()`, `engage()`, `held()`, `ai_tick()`. Its `along_facing` is private and pinned to spawn 29's slot, so this file has its own helper.

**Step 1.** `tests/mod.rs`: add `mod npc_ai_cover_seed_guards;` directly after `mod npc_ai_cover_seed;` (line 27).

**Step 2.** New file, this shape:

```rust
//! CB-N03t — the other three cover guards of world 12 (`MessHall_Guard2`,
//! `Hallway05_Guard1`, `Hallway05_Guard2`) spawn holding their marker,
//! take Cover Stance against a threat in front, and release when flanked,
//! on the real navmesh and cover seed. `npc_ai_cover_seed.rs` proves the
//! same for `MessHall_Guard1`. Static evidence for CB-N03; the owner's
//! CB-U05 lab row is still the acceptance.

use cimmeria_common::Vector3;

use super::npc_ai_cover_behaviour::cover_defense;
use super::npc_ai_cover_seed::{
    ai_tick, engage, held, mess_hall_guard, node_pos, seeded, Seeded, NPC, PLAYER,
};
use crate::cell::cover::{is_flanked, CoverSlotKey};

struct Guard {
    spawn_id: i32,
    tag: &'static str,
    pos: [f32; 3],
    heading: f32,
    slot: CoverSlotKey,
}

/// `spawnlist.sql` rows 28, 32, 33 (template 24) and the cover marker each
/// spawns on.
const GUARDS: &[Guard] = &[
    Guard { spawn_id: 28, tag: "MessHall_Guard2", pos: [-95.89, 34.591, -98.808], heading: 2.094_267_4, slot: CoverSlotKey { chunk_id: 1200053, node_id: 0 } },
    Guard { spawn_id: 32, tag: "Hallway05_Guard1", pos: [-100.16, 24.67, -43.895], heading: 1.570_796_4, slot: CoverSlotKey { chunk_id: 1200056, node_id: 1 } },
    Guard { spawn_id: 33, tag: "Hallway05_Guard2", pos: [-101.5, 24.67, -51.3], heading: 1.570_796_4, slot: CoverSlotKey { chunk_id: 1200044, node_id: 3 } },
];
```

- `fn record(g: &Guard) -> SpawnRecord`: `let mut r = mess_hall_guard();` then set `spawn_id`, `x`, `y`, `z`, `heading`, `tag = Some(g.tag.to_string())`. Import `crate::cell::spawner::SpawnRecord`.
- `fn along(s: &Seeded, slot: CoverSlotKey, dist: f32) -> Vector3`: the body of `along_facing` with `slot` in place of `GUARD_SLOT`.
- `fn front(s, slot) -> Vector3`: the first of `along(s, slot, d)` for `d` in `[12.0, 10.0, 8.0, 6.0]` that is **not** `is_flanked(node.pos, node.orient, p)`; otherwise `panic!("no front point for {slot:?}")`.
- `fn behind(s, slot) -> Vector3`: the first of `d` in `[-8.0, -6.0, -4.0]` that **is** flanked; otherwise panic the same way.

**Step 3.** Three `#[tokio::test]`s. Each loops over `GUARDS` and builds a **fresh** `seeded()` per guard (`let Some(mut s) = seeded() else { return; };`), then `s.mgr.spawn_npc_from_record(NPC, &record(g)).unwrap();`. Every assertion message names `g.tag`.

- `other_cover_guards_spawn_holding_their_marker`: `assert_eq!(held(&s.mgr), Some(g.slot))`.
- `other_cover_guards_take_cover_stance_against_a_threat_in_front`: `engage(&mut s.mgr, front(&s, g.slot));` `ai_tick(&mut s.mgr).await;` then `held == Some(g.slot)` and `cover_defense(&s.mgr, NPC) == 100`.
- `other_cover_guards_release_when_flanked`: as above, then `let b = behind(&s, g.slot);` `s.mgr.update_entity_position(PLAYER, [b.x, b.y, b.z], [0, 0, 0], [0.0; 3]);` `ai_tick(&mut s.mgr).await;` then `assert_ne!(held(&s.mgr), Some(g.slot))` and `cover_defense(&s.mgr, NPC) == 0`. This mirrors `flanking_the_guard_releases_its_slot_and_stance` (lines 357-381 of `npc_ai_cover_seed.rs`).

**Step 4.** **Slot fallback.** If spawn 33 holds `1200044/2` instead of `/3` in the first test, use `/2` (both nodes sit at the same marker) and say so in the report. Any other unexpected slot, or a failure of the stance or release test on unmodified code, is evidence for CB-U05: stop and report `blocked` with the observed slot or values. Do not tune distances beyond the lists above.

**Step 5.** **Mutation proof.** In `crates/cell-cover/src/cell/cover/ai_integration.rs`, change `let reason = if flanked {` (about line 351) to `let reason = if false && flanked {`. Run the nextest lane: `other_cover_guards_release_when_flanked` must fail on `assert_ne!(held ...)`. Undo with the Edit tool, `git -C <wt> diff --stat` shows only this packet's two files, rerun green.

Lanes: `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`; `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell --all-targets -- -D warnings`; `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell --filter-expr 'test(npc_ai_cover_seed)'`. The filter also runs the original three tests; all six must pass, not skip (`data/spaces/castle_cellblock.nav` must exist; if `seeded()` returns `None`, report `blocked`).

Repo rules 1, 2, 3, 5, 7, 9, 10.

---

## CB-N07s — Barracks response seal

| Field | Value |
|---|---|
| Agent | `npc-ai-spawn-advisor` |
| Size | About 30k working tokens. Static: seeds and server code. No lab, no Ghidra, no code changes. |
| Branch / worktree | `cellblock/cb-n07s-barracks-seal` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n07s` |
| Writes | `docs/analysis/cellblock-continuation/worknotes/cb-n07-seal.md` (new, under 200 lines) only |
| Commit | `docs(cellblock): seal the barracks escape response` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n07s\.claude\reports\cellblock-continuation\CB-N07s-npc-ai-spawn-advisor.md` |

**Contract (CB-N07, excerpt).** One room's authored response, one of med bay, mess hall, or barracks/armory. Reuse current triggers and AI/content primitives for one reactive beat. Keep it optional or inside existing objective counts; suppression prevents replay after its beat. Accept: correct event ordering on first trigger and after relog, no duplicate spawns or rewards, no aggro at tutorial spawn, no added unavoidable fight or unreachable mission state. No new engine verb or blanket spawn multiplier. Omit if the baseline has enough life or the beat harms pacing.

**Proposal to verify (architect, 2026-10-17).** The room is the barracks (mission 687 step 2355, "Eliminate the guards in the barracks."). Chain 11020 speaks one NID Guard alarm line on the first entry to `Castle_Cellblock.Region12` while step 2355 is active. The mess hall already has a bark (chain 1177), and the med bay belongs to the 639 tutorial beats.

| Fact | Value (architect, from seeds) |
|---|---|
| Region12 | point set 2043, `AreaSet`, `BoundingBox`, world 12: x -148.81 to -115.18, z -146.72 to -93.80, y 24.64 to 31.90 |
| Barracks guards (template 24) | spawn 25 `Barracks_Guard1` (-118.41, 24.67, -118.35); 26 `Barracks_Guard2` (-131.48, 24.67, -116.97); 36 `Barracks_Guard3` (-136.38, 24.67, -135.67). All three inside Region12. |
| Pack spacing | 1-2: 13.1 m; 2-3: 19.3 m; 1-3: 24.96 m. All within assist radius 26, so the pack already pulls as one. Aggro radius 18. |
| Crate (687 step 2354) | spawn 8 `Cellblock_WoodenCrate` (-130.45, 24.67, -92.07), 1.7 u north of Region12's north edge |
| 687 flow | 2354 -> 2355 by chains 1098, 1099, 1192-1195 (`advance_step 2355`); 2355 objectives by 1100-1103; login restore 1104 |
| Existing Region12 chains | none in any seed (`git grep "Region12" -- db/resources/Content` is empty) |

Assumptions the seal must check and record (each `holds`, `fails` with evidence, or `owner lab` if only CB-U11 can tell):

- **A1.** `enter_region` events fire for Region12 (the xz containment loader handles `BoundingBox` sets the way it does Region2 for 11010).
- **A2.** A player who searches the crate from inside Region12 is already in the region when 2355 activates. Confirm from the H52 replay code that the step activation re-fires `enter_region` for a player already inside, so the line still plays once. If it does not, record that the line plays only on a later entry, and say whether that is acceptable.
- **A3.** CB-U05 is not needed: the barracks has no flank objective and the bark changes no AI state.
- **A4.** It adds zero pacing time: no objective, no movement lock, no dialog window.

The seal also checks: the engine skips a chain with `enabled = false` (the rollback); the bark changes no aggression, threat, faction or spawn state (read the `npc_bark` executor); no other bark can co-fire on the same entry (chains 1176-1178 and 11010 have different gates); `dialog_screens` accepts a second screen under holder 60160 at `index` 1 and the bark screen cache loads it (no `screen_not_cached`); the `dialog_screens_screen_id_seq` setval must become 200170 (the highest authored screen); and a fresh ID collision recheck of chain 11020 and screen 200170 across all local branches (`git for-each-ref refs/heads` plus `git grep` of `VALUES (11020,` and `200170` under `db/resources`), with a control hit (chain 11010, screen 200160) proving the search works.

**Seal file sections:** `## Verdict` (`seal`, `change` or `reject` with one line why); `## Values` (the table above, corrected if any value is wrong); `## Assumptions` (A1-A4 with result); `## Combat impact` (none, with the code lines that prove it); `## Copy` (the line below, under 120 characters, speaker `NID Guard`, no named character, no new lore); `## SQL` (the CB-N07a blocks below, corrected only on `change`); `## IDs` (the recheck output); `## CB-U11` (the owner's steps, copied from [owner-lab-checklist.md § CB-U11](owner-lab-checklist.md#cb-u11--barracks-alarm-bark-chain-11020), corrected if needed).

Proposed copy: `Intruder in the barracks! One of the prisoners got loose. Weapons free!`

`reject` when the bark would change combat, the region does not fire, or the advisor judges the barracks already has enough life. Repo rules 1, 3, 6, 7.

---

## CB-N07m — Mission sign-off

| Field | Value |
|---|---|
| Agent | `mission-systems-advisor` |
| Size | About 20k working tokens. Static. |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n07s` (on the CB-N07s commit) |
| Writes | `docs/analysis/cellblock-continuation/worknotes/cb-n07-seal.md` only (appends `## Mission sign-off`; edits `## Values`, `## Copy` or `## SQL` only on `change`) |
| Commit | `docs(cellblock): mission sign-off on the barracks response seal` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n07s\.claude\reports\cellblock-continuation\CB-N07m-mission-systems-advisor.md` |

Check against the seeds in `castle_cellblock_chains.sql` (687 block, about lines 2206-2549) and `mission_steps.sql`:

1. The gate: `step_status(687, '2355') = 'active'` is true only between the crate search and the barracks clear; `not_active` during 2354; `completed` after. The context key `mission_687_step_2355_status` is the one the engine fills.
2. Suppression: `once` stops a repeat in one visit; the step gate stops it after the fight; a relog during 2355 can give at most one more line (the same accepted behaviour as 11010).
3. No objective, step or reward changes; mission completion still runs through chains 1100-1103.
4. The line does not contradict 687's text or Marsh's barks (1176-1178).
5. The A2 result from the seal is acceptable for the beat.

Verdict `approve`, `change` (make the edit in the seal and say what), or `reject`. Repo rules 1, 3, 6, 7.

---

## CB-N07a — Barracks alarm bark

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 35k working tokens: two seed blocks, one test file, one module line, one docs section, one worknote |
| Branch / worktree | `cellblock/cb-n07-barracks` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n07a` (on the **W3-INT** commit) |
| Writes | `db/resources/Content/Seed/castle_cellblock_chains.sql` (append); `db/resources/Dialogs/Seed/dialog_screens.sql` (one row, one setval value); `crates/cell-content/src/cell/content/chain_replay_tests/cellblock_barracks_response.rs` (new, about 150 lines); `crates/cell-content/src/cell/content/chain_replay_tests/mod.rs` (one line); `docs/content/mission-chains.md` (one section); `docs/analysis/cellblock-continuation/worknotes/cb-n07a.md` (new, under 60 lines) |
| Commit | `feat(cellblock): stage barracks escape response` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n07a\.claude\reports\cellblock-continuation\CB-N07a-coder.md` |

**Read first:** the seal at `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n07s\docs\analysis\cellblock-continuation\worknotes\cb-n07-seal.md`. If its `## SQL` or `## Copy` differs from this section (CB-N07m `change`), the seal wins; copy it byte for byte. If the seal's verdict is `reject`, stop and report `blocked`.

**Step 1.** **Chains seed** (`i/crlf`): append at the end of the file, after a blank line:

```sql
-- =====================================================================
-- CB-N07a (2026-10-17): Cellblock barracks escape response
-- =====================================================================
-- One NID Guard alarm line when the escaping player first enters the
-- barracks (Castle_Cellblock.Region12) while mission 687 step 2355
-- (eliminate the guards in the barracks) is active. It adds no spawn,
-- aggression, threat or objective: the three barracks guards already pull
-- as one pack. once = true stops the H52 step-activation replay speaking
-- twice in a visit; the step gate stops it after the fight. Seal:
-- docs/analysis/cellblock-continuation/worknotes/cb-n07-seal.md. Holder
-- dialog 60160 is never displayed. Rollback: enabled = false on 11020.

-- Chain 11020: Region12 entry during 687/2355 -> NID guard alarm bark
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11020, 'CB-N07a - barracks response: enter Region12 during 687 step 2355 -> NID guard alarm bark', 'mission', 687, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11020, 'enter_region', 'Castle_Cellblock.Region12', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES (11020, 'step_status', 687, '2355', 'eq', 'active', 0);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11020, 'npc_bark', NULL, NULL,
 '{"screen_id": 200170, "speaker": "NID Guard", "channel": "say"}', 0, 0);
```

**Step 2.** **`dialog_screens.sql`** (`i/lf w/crlf`): directly after the line containing `VALUES (60160, 200160,`, add a blank line and:

```sql
-- Cellblock continuation CB-N07a: barracks alarm bark (CIMMERIA_AUTHORING).
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (60160, 200170, 'Intruder in the barracks! One of the prisoners got loose. Weapons free!', 0, 1);
```

Then change `SELECT pg_catalog.setval('dialog_screens_screen_id_seq', 200160, true);` to `200170`. W3's 60150 row (screen 96110) follows yours and stays untouched; 96110 is below 200160, so the setval was still 200160 before your change. `git diff --stat` on this file must show `4 insertions(+), 1 deletion(-)`.

**Step 3.** **`chain_replay_tests/mod.rs`:** add `mod cellblock_barracks_response;` directly after `mod cellblock_ambience;`.

**Step 4.** **Test file.** Copy the imports, `type Params`, `engine_for` and `resolve_region_enter` from `cellblock_ambience.rs` in the same folder (drop the `region_contains_xz` import). Constants:

```rust
//! CB-N07a — the barracks escape response: chain 11020 in
//! `castle_cellblock_chains.sql` speaks one NID Guard alarm line (screen
//! 200170) on the first entry to `Castle_Cellblock.Region12` while mission
//! 687 step 2355 ("Eliminate the guards in the barracks.") is active.
//! Sealed in `docs/analysis/cellblock-continuation/worknotes/cb-n07-seal.md`.

const CHAIN_ID: i32 = 11020;
const REGION_KEY: &str = "Castle_Cellblock.Region12";
/// Another Cellblock room (chain 1084 accepts 687 there).
const OTHER_REGION_KEY: &str = "Castle_Cellblock.Region6";
const SCREEN_ID: i32 = 200170;
const SPEAKER: &str = "NID Guard";
/// `CHAN_say`; the loader accepts no other channel.
const CHAN_SAY: u8 = 0;
const STEP_KEY: &str = "mission_687_step_2355_status";
```

Three `#[tokio::test]`s, each starting `let pool = require_db_or_skip!();` and `let engine = engine_for(&pool, CHAIN_ID).await;` with `let id = i64::from(CHAIN_ID);`:

- `live_db_cellblock_barracks_response_barks_once_while_2355_is_active`: `assert!(engine.is_once(id))`; resolve `REGION_KEY` with `&[(STEP_KEY, "active")]`; exactly one `Action::NpcBark` for chain `id` with `screen_id == SCREEN_ID`, `speaker == SPEAKER`, `channel == CHAN_SAY` (same match block as `cellblock_ambience.rs` lines 121-147); then `assert_no_deferred_actions(&resolved, id)`.
- `live_db_cellblock_barracks_response_refuses_the_crate_and_cleared_phases`: for `("step 2354 (the crate) is still active", "not_active")` and `("the barracks are cleared", "completed")`, resolve `REGION_KEY` with `&[(STEP_KEY, value)]` and assert no action has chain id `id`.
- `live_db_cellblock_barracks_response_refuses_another_room`: resolve `OTHER_REGION_KEY` with `&[(STEP_KEY, "active")]`; assert no action has chain id `id`.

Screen existence and the speaker are already guarded for every seeded bark by `marsh_barks::seed_shape::live_db_every_seeded_bark_names_a_real_screen_and_a_named_speaker`; run it (below), do not duplicate it.

**Step 5.** **`docs/content/mission-chains.md`** (`i/crlf`): insert directly before the line `### Castle (Persistent Zone)`:

```markdown
#### Cellblock barracks response (CB-N07a)

| Chain | Trigger | Conditions | Actions |
|---|---|---|---|
| 11020 | `enter_region('Castle_Cellblock.Region12')` (once) | `step_status(687, 2355) = 'active'` | `npc_bark(200170)`: one line in chat, speaker `NID Guard` |

One alarm line as the player enters the barracks to fight its three guards (spawns 25, 26, 36). It changes no AI, spawn or objective state. Sealed in [cb-n07-seal.md](../analysis/cellblock-continuation/worknotes/cb-n07-seal.md).

```

**Step 6.** **`worknotes/cb-n07a.md`** (new, CRLF): what was added, the three test names, the mutation proofs and their failing assertions, the lanes run, and that CB-U11 is owner-run UATPending.

**Mutation proofs** (rule 9; one at a time, in the seed, then `live-db-test.ps1 cellblock_barracks_response`): (a) the condition value `'active'` to `'completed'`: the fire test fails; (b) the trigger key `Region12` to `Region6`: the fire test and the other-room test fail. Restore each with Edit; `diff --stat` back to step 1's count.

Lanes, in order: `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`; `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`; `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content --filter-expr 'test(cellblock_barracks_response)'`; `pwsh tools/build-lane/live-db-test.ps1 cellblock_barracks_response` (3 pass, none skipped); `pwsh tools/build-lane/live-db-test.ps1 marsh_barks`; `pwsh tools/build-lane/live-db-test.ps1 cellblock_ambience`.

Repo rules 1-10.

---

## W4-INT — Integrate wave 4 and bookkeeping

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 30k working tokens |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock` |
| Writes | cherry-picks, then one commit touching `db/resources/Content/Seed/castle_cellblock_chains.sql` (header line only), `docs/analysis/four-zone-planning/id-allocation.md`, `docs/analysis/cellblock-continuation/README.md` |
| Commit | `docs(cellblock): W4 integrate CB-N03t and CB-N07` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\W4-INT-coder.md` |

1. `git -C <zone-cellblock> status --porcelain` shows only the untracked `lab-server*.log` files (leave them; never add them). HEAD is the W3-INT bookkeeping commit or later.
2. `git -C <zone-cellblock> cherry-pick -x`, in order: CB-N03t (if run), CB-N07s, CB-N07m, CB-N07a, each followed by its fix commits. On a conflict: `git cherry-pick --abort`, report `blocked` with the paths.
3. Bookkeeping commit:
   - Chain seed header: one line directly before the line starting `--   (next free inside 1001-1111:`: `--   Cellblock barracks response CB-N07a (2026-10-17): chain 11020 (id-allocation.md, CB-N07 claims).` `diff --stat` must show 1 line.
   - `id-allocation.md` § CB-N07 claims: every `Status` cell becomes `In use (W4-INT)`. In "Chain IDs in use today", the `castle_cellblock_chains.sql` row: count +1, and append `, 11020` after the 11000-11005/11010 entries in the same style. **Last updated** stays or becomes the commit date.
   - Branch `README.md`: CB-N03t -> `Integrated` (or `Done / skipped` if not run); CB-N07s, CB-N07m -> `Integrated`; CB-N07a -> `Integrated, UATPending` (CB-U11); each with its integrated SHA. Nothing else.
   - CRLF for all docs (rule 6).
4. Lanes from zone-cellblock: `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`; `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell -p cimmeria-cell-content --all-targets -- -D warnings`; `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell -p cimmeria-cell-content`; `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests`.
5. Return the tip SHA. The owner's lab rows run on it.

Repo rules 1, 2, 3, 6, 10.

---

## CB-N08a — UAT reconciliation

| Field | Value |
|---|---|
| Agent | `testing-validation-engineer` |
| Size | About 40k working tokens. Static: docs, seeds and test inventory. No lab. |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock` (W4-INT tip) |
| Writes | `docs/analysis/cellblock-continuation/worknotes/cb-n08-acceptance.md` (new, under 300 lines); `docs/guides/uat-specs/castle-cellblock.toml` only if step 2 finds a stale literal |
| Commit | `docs(cellblock): reconcile escape acceptance rows` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\CB-N08a-testing-validation-engineer.md` |

**Input from the orchestrator (in the brief):** the owner's result for each of CB-U03, U05, U08, U09, U10 and U11 (pass, fail with what was seen, skipped), or the O14-09 note that the lab was unavailable, and the D-CC15 answer.

1. **Acceptance matrix.** One table row per scenario the CB-N08 contract lists: fresh Human and Jaffa full runs, another archetype where class branches differ, skip both optional stories, accept each story, relog and death before and after key transitions, two-client AoI, boots Livewire, Marsh escort, combat/flank, first-press feedback, Castle boundary. Plus the presentation rows CA-U01-21 ([castle-cellblock-rebuild uat-guide](../castle-cellblock-rebuild/uat-guide.md)), class-start and ring travel ([presentation-handoff.md](presentation-handoff.md)). Columns: scenario, covering owner row or test, evidence (client, server log, test name), state (`pass`, `fail`, `UNTESTED: <reason>`). Never infer a pass from logs or tests alone where the row needs the client.
2. **Spec check.** `castle-cellblock.toml` has one row, T01/T02. Verify its literals against the seeds (dialog 2982, mission 622 step 2113). Change it only if a literal is stale. Add no lab rows (owner directive).
3. **Route-baseline rows** CB-U01, U02, U04, U06, U07: the owner's result if given, else `UNTESTED: owner lab not run`.
4. **Issue #1667** (first-guard pistol damage): a known blocker; record its state from `gh issue view 1667 --json state,title` (read-only).
5. **Blockers.** Any CB-U10 blocker (blank dialog, crash, missing first-press response, lost progress, changed Marsh briefing), any mandatory route blocker, duplicated reward, impossible branch or uncontrolled aggro: list it under `## Blockers` and return `blocked`. The orchestrator then opens a fix packet through the architect.
6. **Test inventory.** List the campaign's new tests by file with counts (`git -C <wt> diff --stat main...HEAD -- crates`), and confirm each live-DB test name contains `live_db`.

Repo rules 1, 3, 6, 7, 10.

---

## CB-N08b — Close-out docs

| Field | Value |
|---|---|
| Agent | `documentation-writer` |
| Size | About 40k working tokens |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock` (CB-N08a tip) |
| Writes | `docs/analysis/cellblock-continuation/{README.md,decision-log.md,owner-lab-checklist.md,worknotes/cb-n08.md}`; `docs/guides/unified-uat.md` (Castle Cellblock section only); `docs/content/mission-chains.md` and `docs/content/README.md` only if stale; `docs/architecture/mission-pak-overrides.md` (one stale sentence); one new file under `.claude/agent-memory/main-session/` plus its `MEMORY.md` index line |
| Commit | `docs(cellblock): close escape campaign with client evidence` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\CB-N08b-documentation-writer.md` |

Status docs (`docs/gap-analysis*`, `docs/project-status.md`, `docs/content/zone-audit.md`) belong to FZ-99: do not touch them.

1. **README.md:** every packet row reaches a final state: `Done`, `Integrated, UATPending (<row>)`, `Done / omitted` (with evidence), or `BlockedDependency` (CB-N05b, HA-1360). CB-N06: `Done / omitted` unless D-CC15 was answered yes (then `BlockedDecision`, a later campaign). CB-N07b: `Done / omitted` unless D-CC11 asked for it. CB-N03 and CB-N03a: from the CB-U05 result. Add `## FZ-99 handoff`: what changed in world 12 (chains, missions, spawns, dialogs), the open UATPending rows, known blockers (#1667), and the rows FZ-99 should write into the status docs.
2. **decision-log.md:** one dated entry per CB-D or D-CC decision taken since its last entry; link the README table rather than copying it.
3. **owner-lab-checklist.md:** record each owner result next to its row; keep the steps.
4. **worknotes/cb-n08.md** (new): the close-out summary, linking `cb-n08-acceptance.md`, with what was tested versus never tested stated up front.
5. **unified-uat.md**, section `## Castle Cellblock tutorial` only: add rows for the continuation's tester-facing behaviour with new ids `CB1`-`CB6` (relog restores 1026-1028/1036/1037, ambient cast and bark 11010, *The Remaining Pods* 62050, barracks bark 11020, cover flank in Mess Hall/Hallway05, core pacing), each linking the matching [owner-lab-checklist.md](../analysis/cellblock-continuation/owner-lab-checklist.md) section. Keep the existing T rows and ids. Do not edit a `<!-- gen:... -->` block.
6. **mission-pak-overrides.md:** the sentence "`MISSION_ADDITIONS` stays empty until an authored content packet seals production XML." becomes a statement that mission 62050 (*The Remaining Pods*, CB-N05) is the first production entry, with a link to `worknotes/cb-n05-seal.md`.
7. **Project memory:** `.claude/agent-memory/main-session/project_cellblock_continuation_closeout.md` (frontmatter as in the sibling `reference_cellblock_*` files): durable facts only (ID claims, the bark pattern, the owner-run lab rule, where the ledger is). No IPs, credentials or account names. Add its one-line entry to `.claude/agent-memory/main-session/MEMORY.md`.
8. **Doc-update map:** walk [doc-update-map.md](../../agents/doc-update-map.md) and list the rows touched in the report (expected: content chains, complete authored mission catalog entries, UAT steps, project memory). The ship step copies the list into the PR body.
9. Checks: `pwsh tools/lint-md.ps1` on the changed docs (warn-only, fix what is yours); `python tools/docs-gen/regen.py --check`; `git -C <wt> diff --check`; CRLF on every changed `docs/` file.

Repo rules 1, 3, 5, 6, 7, 10.

---

## CB-N08-CI — Full local CI

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 20k working tokens; long wall-clock |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock` (CB-N08b tip) |
| Writes | nothing (a fix needs a new packet) |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\CB-N08-CI-coder.md` |

Run in order, each in the foreground; stop at the first failure and report `failed` with the log path. `$EX = '--exclude cimmeria-app --exclude cimmeria-content-editor --exclude cimmeria-scene-editor --exclude cimmeria-client-telemetry --exclude cimmeria-client-patches --exclude cimmeria-lab'` (type the flags out; do not rely on variable splitting).

1. `git -C <wt> fetch origin main`, then `git -C <wt> merge-base --is-ancestor origin/main HEAD`. If main has moved past the branch, report `blocked`: ops rebases (or merges) main into the campaign branch first, and this packet reruns.
2. `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`
3. `pwsh tools/build-lane/lane.ps1 --exclusive cargo hakari generate --diff` and `pwsh tools/build-lane/lane.ps1 --exclusive cargo hakari manage-deps --dry-run`
4. `pwsh tools/build-lane/lane.ps1 --exclusive cargo clippy --workspace <EX> --all-targets -- -D warnings`
5. `pwsh tools/build-lane/lane.ps1 --exclusive cargo build --workspace <EX> --all-targets`
6. `pwsh tools/build-lane/lane.ps1 --exclusive cargo nextest run --profile=ci --workspace <EX>`
7. `pwsh tools/build-lane/lane.ps1 cargo test --doc -p cimmeria-commands`
8. `pwsh tools/build-lane/live-db-test.ps1 live_db` (reloads `sgw_zone_cellblock` from seeds, then every live-DB test)
9. `python tools/ability_mechanics/effect_nvps_from_desc.py --check`; `python tools/ability_mechanics/weapon_requirement_audit.py --check`; `python -m unittest discover -s tools/ability_mechanics -p "test_*.py"`
10. `python tools/docs-gen/regen.py --check`; `pwsh tools/lint-md.ps1` (warn-only; list warnings in the report)

Report each step's result, test counts, and the log paths. Repo rules 1, 2, 3, 10.

---

## CB-N08-R — Campaign review

| Field | Value |
|---|---|
| Agent | `packet-reviewer` (fresh, read-only); fixes by `packet-coder` |
| Size | About 60k (reviewer, not Haiku); fixes about 20k each |
| Scope | `git -C <zone-cellblock> diff main...HEAD` (the whole campaign) against the [execution contract](../four-zone-planning/execution-contract.md), this file, w1b/w2/w3 packets and the seals |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\CB-N08-R<round>-reviewer.md` |

Review for: correctness bugs, spec drift, theatre tests (a test that passes with its change reverted), repo-rule breaches (file caps, CRLF, generated blocks, `db/scripts`, IDs outside the claims, committed IPs or names), and missing docs rows. Verdict `ship` or `fix` with verified findings. On `fix`: one `packet-coder` per finding group, in zone-cellblock, commit `fix(cellblock): campaign review round <n>`; rerun CB-N08-CI steps 2, 4, 6 and 8; then a fresh reviewer. At most 3 rounds; then back to the architect.

---

## Ship, merge and retire

`ops`, one operation per dispatch, after CB-N08-R `ship`:

1. From the main checkout (the branch's own copy of `tools/build-lane/` predates `ship.py`): `python C:\Users\Steve\source\projects\Cimmeria\tools\build-lane\ship.py pr -C C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock -m "feat(cellblock): relog restores, ambient cast, The Remaining Pods, barracks response" --body-file <body.md>`. The body names the UATPending owner rows, #1667, the doc-update-map rows from CB-N08b, and ends with the attribution the orchestrator gives.
2. After the owner's go: `python C:\Users\Steve\source\projects\Cimmeria\tools\build-lane\ship.py merge <PR> --retire zone-cellblock` (it waits for CI and the merge queue).
3. Retire every packet worktree, one `rm-worktree.ps1 --force <name>` each (all NO_PR, integrated by cherry-pick): `cellblock-cb-n02a`, `cellblock-cb-n02b`, `cellblock-cb-n02c`, `cellblock-cb-n02d`, `cellblock-cb-n02s`, `cellblock-cb-n02s2`, `cellblock-cb-n03s`, `cellblock-cb-n04s`, `cellblock-cb-n04a2`, `cellblock-cb-n05-1` to `cellblock-cb-n05-7`, `cellblock-cb-n03t`, `cellblock-cb-n07s`, `cellblock-cb-n07a`. **`cellblock-cb-n04a`** (branch `cellblock/cb-n04-region2`, dirty, stale) only on the owner's word (salvage or retire). Never `git worktree prune`.
4. The orchestrator then empties `.claude/state/active-campaign` and posts the board handoff through ops.

---

## Held packets

| Packet | Status | Blocked on | What unblocks it | Default at close-out |
|---|---|---|---|---|
| CB-N07b (Tier 1: difficulty, aggression or reinforcement) | BlockedDecision D-CC11, BlockedEvidence CB-U05/U09 | The barracks pack already pulls as one (spacing 13.1-24.96 m, assist radius 26), so a scripted aggression adds nothing; a reinforcement spawn is an added unavoidable fight, which the contract forbids. A real difficulty change belongs in a room with a flank objective (likely the Mess Hall) and needs CB-U05 and measured pacing. | Owner answers D-CC11 after CB-U05/U09; a new architect pass seals it | `Done / omitted` |
| CB-N06 (second optional prisoner-aid choice) | BlockedDecision D-CC15, BlockedEvidence CB-U09/U10 | The owner must say whether it exists and name the quest and the prop; the contract freezes branches, copy, IDs, item and reward at seal time. | D-CC15 yes: dispatch a seal packet CB-N06s (`mission-systems-advisor` with `items-systems-advisor` as second reader, writes `worknotes/cb-n06-seal.md`, about 40k), then an architect pass splits the build | `Done / omitted` with evidence |
| CB-N05b (Castle evidence hand-in) | BlockedDependency | Harset HA-1360's atomic possession, consume and completion primitive is not on `main` | A later campaign after HA-1360 merges | Recorded as deferred, outside this PR |
| CB-N03a/b | BlockedEvidence | Exists only if CB-U05 (or CB-N03t) shows a seed fault | A new architect pass | `Done / not needed` on a CB-U05 pass |
