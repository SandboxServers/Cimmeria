# Dakara_E1 wave 6b: the Ha'tak sequences and the wave-6 nits

> Type: how-to (work packets). Audience: the packet-coder named in each packet, and the read-only wave-6b packet-reviewer (its findings go to a fresh packet-coder).
> Written 2026-10-19 (architect, autonomous) against `main` @ `5228c72a4`. Ledger and status: [README](../README.md#packets). Rules and full contract: [work-packets.md](../work-packets.md). Evidence: [DK-31a desk note](../worknotes/DK-31a.md).

Wave 6b has three small packets, each in its own worktree. None needs the lab, the client or an owner answer.

| Packet | Worktree, branch | Size | Gate |
|---|---|---|---|
| [DK-31c](#dk-31c-the-hatak-sequences-of-mission-1652) | `dk-31c`, `dakara/dk-31c-hatak-sequences` (new, from `main`) | about 25k tokens | none: DK-31a (#1900) and DK-31b (#1909) are merged. **Ready** |
| [DK-30b-n](#dk-30b-n-two-review-nits-in-dk-30b) | `dk-30b` (existing, branch as created; do not make a new one) | about 8k tokens | none. **Ready**; the PR still waits for US-30 |
| [DK-33-n](#dk-33-n-two-review-nits-in-dk-33) | `dk-33` (existing, branch as created; do not make a new one) | about 10k tokens | none. **Ready**; the PR still waits for US-31 |

## Rules for every wave-6b packet

1. PowerShell only. No bash, WSL or Git Bash. Use absolute paths inside your worktree; never `cd` to the root checkout. Prefix each lane call with `Set-Location <worktree>;` and check that the lane log lands under `logs/<worktree>/`.
2. Every compiling `cargo` call goes through the lane, run from the worktree root: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly; never add `--target-dir`. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`. Run lane checks in the foreground, one at a time.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, `git rebase`, or delete a worktree. Commit once, locally, with the subject below and the attribution lines from your brief.
4. Edit only the files your packet names. Do not edit `db/database.sql`, the linter, `dakara_e1_missions/mod.rs` or another packet's file. Files stay under 700 lines (soft cap 500).
5. The seed files and everything under `docs/` are CRLF; keep them CRLF. Check with `[IO.File]::ReadAllText('<absolute path>')`: it must contain "`r`n" and no "`n" without a "`r" before it.
6. Seeds change in `db/resources/` only; never add a `db/scripts/` migration. Every chain has a comment above it with its evidence label and what it serves.
7. No new dialog row, no new sequence row, no client patch. No rewards.
8. Never launch the game client, the launcher or any lab tool.
9. No IPs, credentials, tokens or account names in any file.
10. Report to `<worktree>\.claude\reports\dakara\<packet>-packet-coder.md`: files changed, each check's final `status=` line, the commit hash, every deviation and why. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. A context stop past about 90k tokens: commit only passing work (subject suffix `(part N)`), write `docs/analysis/dakara-e1-rebuild/worknotes/<packet>.md` and return `blocked` with the reason `context`.

**Checks, in order, for every wave-6b packet** (from the worktree root; each must end `status=ok` or with every test passing):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine interact_tag_linter`
4. `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1_missions`

A failing check gets at most five fix rounds; then commit nothing, report, and return `failed`.

**Docs rows owed.** None in these packets. The `docs/content/mission-chains.md` row for 1652 is DK-99b's, once for the campaign; say so in the report.

## DK-31c: the Ha'tak sequences of mission 1652

**Goal.** While a player is on mission 1652 in world 61, both Ha'tak ships are shown; clicking the western ring control explodes the western ship, and clicking the eastern one explodes the eastern ship. Four new chains, appended to DK-31b's seed file. The existing chains 8320-8326 are not changed.

**Files (only these two):**

- `db/resources/Content/Seed/dakara_e1_m1652_chains.sql`: append chains 8327-8330 after chain 8326.
- `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/m1652.rs`: update three tests' expected vectors and add one test.

### Contract excerpt you need

- **`play_sequence`.** Row: `(chain, 'play_sequence', <sequence_id>, NULL, '{}', <delay_ms>, <sort_order>)`. `target_id` is `sequences.sequence_id`, not an event-set id and not an event id (DK-31a § 1). The server sends `onSequence` to the acting player only. Precedent: `db/resources/Content/Seed/sgc_w1_chains.sql`, chain 3030 (`(3030, 'play_sequence', 10010, NULL, '{}', 3000, 1)`), which co-fires with chain 3028 on the same `interact_tag`.
- **Row shape** (work-packets C4): one `content_chains` insert (`scope_type 'mission'`, `scope_id 1652`, `enabled true`, `priority 0`), one `content_triggers` insert (`scope 'player'`, `once false`, `sort_order 0`), multi-row conditions and actions with `sort_order` from 0. Descriptions read `'1652 - <state>: <what>'` with every `'` doubled (`Ha''tak`).
- **Every matching chain fires, and conditions are read before any action runs.** So chain 8329 (click west at step 4933) fires beside 8325 even though 8325 advances the step to 4934 in the same click; 8330 fires beside 8326 the same way.
- **A delayed action** (`delay_ms` > 0) does not re-check its chain's conditions and is dropped if the player logs out first (finding F27).
- **Chain order in a result.** The engine loads chains `ORDER BY chain_id` and keeps that order between equal priorities, so `labels_in` lists 8323's actions before 8327's, and 8325's before 8329's.

### The sequence ids (one swap point)

| Use | Sequence id | Kismet root (`sequences.sql`) | Event |
|---|---|---|---|
| West unhide | **2392** | `Dakara_E1.Main_Sequence.Prefabs.GA-Hatak00_Pf0_Seq_0` | 6000, "Designer 0: Unhide Hatak" |
| West explosion | **2393** | `..._Seq_0` | 6001, "Designer 1: Hatak Explosion" |
| East unhide | **2390** | `Dakara_E1.Main_Sequence.Prefabs.GA-Hatak00_Pf0_Seq` | 6000 |
| East explosion | **2391** | `..._Seq` | 6001 |

West is the Ha'tak at (-111.710, 353.916, 78.091), east the one at (367.160, 359.597, 136.130) (DK-31a § 2). The mapping is **settled** by an offline read of the cooked map ([DK-31a addendum](../worknotes/DK-31a.md#addendum-2026-10-19-offline-map-read), D-DK35): each Ha'tak's `PrefabInstance` names its Kismet root, and the roots' Designer 0 and 1 events drive "Unhide" and "Explosion" exactly as the table says. A level-load event in each root hides both ships and turns the emitters off, so the ships are hidden until an unhide plays, on every map load. Use exactly the ids in this table. If the owner's DK-U27 run still shows the wrong ship exploding, the fix is to swap 2390 with 2392 and 2391 with 2393 in this file and in m1652.rs, nothing else.

### The four chains

Write them exactly like this, after chain 8326, with one blank line between blocks (the same layout as 8320-8326). The comment lines are part of the packet.

```sql
-- Chain 8327 PROJECT_FINAL_RECONSTRUCTION: step 4933 on Dakara_E1, show both Ha'taks
-- (unhide sequences 2392 west and 2390 east, ORIGINAL_DATA Kismet, DK-31a). Delayed 3 s so
-- the map has loaded (precedent: chain 3030). Co-fires with 8323; no accept-time unhide,
-- because the accept (8322) is in world 62, which has no Ha'tak (D-DK33).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8327, '1652 - Step 4933 on Dakara_E1: show both Ha''taks (play 2392 west, 2390 east)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8327, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8327, 'world', 61, NULL, 'eq', NULL, 0),
  (8327, 'step_status', 1652, '4933', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8327, 'play_sequence', 2392, NULL, '{}', 3000, 0),
  (8327, 'play_sequence', 2390, NULL, '{}', 3000, 1);

-- Chain 8328 PROJECT_FINAL_RECONSTRUCTION: step 4934 on Dakara_E1, show the eastern Ha'tak
-- only (unhide 2390). The map hides both ships on every load, so the western one, destroyed
-- at 4933, stays gone; there is no explosion replay (D-DK34). Co-fires with 8324.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8328, '1652 - Step 4934 on Dakara_E1: show the eastern Ha''tak (play 2390)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8328, 'player_loaded', 'Dakara_E1', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8328, 'world', 61, NULL, 'eq', NULL, 0),
  (8328, 'step_status', 1652, '4934', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8328, 'play_sequence', 2390, NULL, '{}', 3000, 0);

-- Chain 8329 PROJECT_FINAL_RECONSTRUCTION: click the western control at 4933, explode the
-- western Ha'tak (2393) 3 s after the bomb dialog 6108. Co-fires with 8325, which advances
-- the step; conditions are read before either chain acts.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8329, '1652 - Step 4933 click Dakara_E1_RingControl_XMinus: explode the western Ha''tak (play 2393)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8329, 'interact_tag', 'Dakara_E1_RingControl_XMinus', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8329, 'world', 61, NULL, 'eq', NULL, 0),
  (8329, 'step_status', 1652, '4933', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8329, 'play_sequence', 2393, NULL, '{}', 3000, 0);

-- Chain 8330 PROJECT_FINAL_RECONSTRUCTION: click the eastern control at 4934, explode the
-- eastern Ha'tak (2391) 3 s after the bomb dialog 6109. Co-fires with 8326, which completes
-- 1652; a delayed action survives the completion (it is dropped only on logout).
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8330, '1652 - Step 4934 click Dakara_E1_RingControl_XPlus: explode the eastern Ha''tak (play 2391)', 'mission', 1652, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8330, 'interact_tag', 'Dakara_E1_RingControl_XPlus', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8330, 'world', 61, NULL, 'eq', NULL, 0),
  (8330, 'step_status', 1652, '4934', 'eq', 'active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8330, 'play_sequence', 2391, NULL, '{}', 3000, 0);
```

Also change line 2 of the file's header from `-- Campaign: docs/analysis/dakara-e1-rebuild/ (packet DK-31b).` to `-- Campaign: docs/analysis/dakara-e1-rebuild/ (packets DK-31b, DK-31c).` Nothing else in the file changes.

### Tests (m1652.rs)

Type: chain replay, live-DB (TESTING.md type 6). Each existing test keeps its negatives unchanged.

1. `delays_of` comes from `use super::*;` already; add no import. Append to the module doc comment (the `//!` block at the top), as `//!` lines wrapped under 100 columns: `The Ha'tak sequences (8327-8330, DK-31c) co-fire: both ships are shown on entering Dakara at 4933, the eastern one at 4934, and each click explodes its ship 3 s later.`
2. `live_db_entering_dakara_binds_the_control_of_the_current_step`: the western vector becomes `vec!["8323:add_dialog_set(6887@455)", "8327:play_sequence(2392)", "8327:play_sequence(2390)"]`; the eastern vector becomes `vec!["8324:add_dialog_set(6888@462)", "8328:play_sequence(2390)"]`. After each `assert_eq!` add `assert_eq!(delays_of(&western, 8327), vec![3000, 3000]);` and `assert_eq!(delays_of(&eastern, 8328), vec![3000]);`. The story-room negative stays.
3. `live_db_clicking_the_western_control_bombs_it_and_lights_the_east`: append `"8329:play_sequence(2393)"` as the last element of the expected vector, keep `assert_eq!(display_dialog_count(&r), 1);` and `assert_no_deferred_actions(&r, 8325);`, and add `assert_eq!(delays_of(&r, 8329), vec![3000]);` after them.
4. `live_db_clicking_the_eastern_control_spends_the_generator_and_completes`: append `"8330:play_sequence(2391)"` as the last element, keep the two existing asserts, add `assert_eq!(delays_of(&r, 8330), vec![3000]);`.
5. Add this test at the end of the file:

```rust
#[tokio::test]
async fn live_db_the_hatak_sequences_play_only_on_dakara_during_1652() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let is_sequence = |l: &String| l.contains(":play_sequence(");

    // Not accepted yet, already completed, or loaded into the story room: no sequence.
    let offered = offer_state(Some(DAKARA));
    let completed = with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 1652, "completed");
    for ctx in [&offered, &completed] {
        let r = fire_player_loaded(&engine, ctx, DAKARA_NAME);
        assert!(!labels_in(&r, M1652_CHAINS).iter().any(is_sequence));
    }
    let story_room = fire_player_loaded(&engine, &at_step(Some(STORY_RM), 4933), STORY_RM_NAME);
    assert!(!labels_in(&story_room, M1652_CHAINS).iter().any(is_sequence));

    // At 4934 the western ship is never shown or exploded again.
    let eastern = fire_player_loaded(&engine, &at_step(Some(DAKARA), 4934), DAKARA_NAME);
    let labels = labels_in(&eastern, M1652_CHAINS);
    assert!(!labels.iter().any(|l| l.contains("(2392)") || l.contains("(2393)")));

    // The accept in the story room plays no sequence (D-DK33).
    let accept = fire_dialog_choice(&engine, &offer_state(Some(STORY_RM)), 5986);
    assert!(!labels_in(&accept, M1652_CHAINS).iter().any(is_sequence));
}
```

`labels_in` returns `Vec<String>`, so `.iter()` yields `&String` and the capture-free closure is `Copy`, which lets it be passed to `any` more than once. If the compiler or clippy still objects, inline the closure at each call; never add an `#[allow]`.

**Revert proof.** Removing chains 8327-8330 makes the three updated vectors and the three `delays_of` asserts fail; removing only one chain fails its vector. The new test fails if a sequence is moved onto the accept (8322) or onto the step-4934 restore with a western id.

**Commit subject:** `feat(content): Dakara_E1 mission 1652 Ha'tak sequences (DK-31c)`.

## DK-30b-n: two review nits in DK-30b

**Worktree:** the existing `dk-30b` (`C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dk-30b`). It holds DK-30b's reviewed commit `34abe129b`; add one commit on top. Review: `.claude\reports\dakara\dk-30b-review.md` in that worktree.

1. `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/m1651_gate.rs`, test `live_db_dhd_without_1651_state_resolves_no_gate_chain` (about lines 78-79). Replace the comment
   `// No 1651 state at all: the DHD click resolves nothing here, so the`
   `// dialling window still opens.`
   with
   `// No 1651 state at all: no 1651 gate chain resolves for the DHD click.`
   `// (The cell then opens the dialling window; that part is DK-U26, not this test.)`
2. `db/resources/Content/Seed/dakara_e1_m1651_gate_chains.sql`: add one blank line after the `SET search_path = resources, pg_catalog;` line, so the first chain comment is separated from it as in the other Dakara seed files. Nothing else changes.

Checks: the four wave-6b checks. **Commit subject:** `test(content): DK-30b review nits (comment, blank line) (DK-30b-n)`.

## DK-33-n: two review nits in DK-33

**Worktree:** the existing `dk-33` (`C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dk-33`). It holds DK-33's reviewed commit `41686b50c`; add one commit on top. Review: `.claude\reports\dakara\dk-33-review.md` in that worktree. File: `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/m1654.rs` only.

1. Delete the test `live_db_dhd_chains_are_exclusive_over_5961` (about lines 153-165) whole: the exact label vectors in `live_db_dhd_completes_5961_then_teleports_to_sgc` and `live_db_dhd_recovery_teleports_once_5961_is_done` already prove the two chains never co-fire.
2. Cover the archetype boundary of chain 8432 (`archetype lte 6`). In `live_db_tauri_offer_takes_1654_with_the_tauri_briefing`, wrap the body after `let engine = ...;` in `for archetype in [TAURI, 6] { ... }` and use `offer(archetype)` in place of `offer(TAURI)`, the same shape as the Free Jaffa test (archetype 7 is `FREE_JAFFA`, already covered there). Add a one-line comment: `// 6 is the highest Tau'ri archetype (lte 6), so it is the boundary.`

Checks: the four wave-6b checks. **Commit subject:** `test(content): DK-33 review nits (redundant test, archetype 6) (DK-33-n)`.

## Review

One read-only `packet-reviewer` for the three packets (owner rule, 2026-10-18), with each worktree, its commit range and this file. Findings go to a fresh packet-coder in the same worktree. DK-31c then ships through `ops` like any packet; DK-30b and DK-33 open their PRs and merge only after US-30 and US-31 respectively.
