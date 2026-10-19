# Dakara_E1 wave 6: missions 1651 to 1654 and the SGC exits

> Type: how-to (work packets). Audience: the packet-coder (or the domain agent) named in each packet, and the read-only wave-6 packet-reviewer (its findings go to a fresh packet-coder).
> Written 2026-10-19 (architect, autonomous) against `main` @ `eb9eaebe7`. Ledger and status: [README](../README.md#packets). Rules and full contract: [work-packets.md](../work-packets.md).

Wave 6 starts when DK-40 and DK-41 are merged, and runs beside wave 5: every packet writes its own seed file and test file. DK-31a is a desk note with no code and no gate: dispatch it now, with wave 4.

**Merge holds.** DK-30b (crossing C2) merges only after SGC packet US-30 has merged, and DK-33 (crossing C3) only after US-31: before that, the crossing lands a player in the SGC with no chain to receive them and no way back (the SGC has no DHD, finding F5). Build, review and open the PRs now; hold the merge. UD-04c tests C2, C2b and C3 once all four are in.

> **Amended 2026-10-19 (owner ruling D-DK24).** The crossings are now real gate travel: the DK-30b and DK-33 sections below are superseded in place by [wave 6c DK-30b-r](wave-6c.md#dk-30b-r-mission-1651-crossing-by-real-gate-travel) and [DK-33-r](wave-6c.md#dk-33-r-mission-1654-crossing-by-real-gate-travel), which ship in the same PRs. The merge holds stand, and DK-30b-r also waits on D-DK40 and D-DK44. Rule 7's "no stargate address grant" is lifted for chains 8313, 8432 and 8433 only (gate 27, never from `player_loaded`). The DHD paragraph and the `teleport SGC H3` row below describe the old design.

**Dormant until 1647 ships (D-DK23).** No player reaches 1651 until 1647 to 1650 exist; these chains are built and tested now.

## Rules for every wave-6 packet

1. PowerShell only. No bash, WSL or Git Bash. Use absolute paths inside your worktree; never `cd` to the root checkout. Prefix each lane call with `Set-Location <worktree>;` and check that the lane log lands under `logs/<worktree>/`.
2. Every compiling `cargo` call goes through the lane, run from the worktree root: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly; never add `--target-dir`. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`. Run lane checks in the foreground.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, or delete a worktree. Commit once, locally, with the subject below and the attribution lines from your brief.
4. Edit only your packet's seed file and test file (DK-31a: only its worknote). Do not edit `db/database.sql`, the linter, `dakara_e1_missions/mod.rs` or another packet's file. Files stay under 700 lines (soft cap 500).
5. The seed files and everything under `docs/` are CRLF; keep them CRLF (check with `[IO.File]::ReadAllText(<abs>).Contains("`r`n")` and that no bare LF is left).
6. Seeds change in `db/resources/` only; never add a `db/scripts/` migration. Every chain has a comment above it: its evidence label (`PROJECT_FINAL_RECONSTRUCTION`: the client holds the missions, steps, objectives and dialogs but no trigger) and the step, objective or bind it serves. The teleport chains also name their crossing row (C2 or C3).
7. No new dialog row, no client patch. Never edit templates 54 or 59 (shared with Harset). No rewards (D-DK7): no XP, cash or reward item. No stargate address grant.
8. Never launch the game client, the launcher or any lab tool.
9. No IPs, credentials, tokens or account names in any file.
10. Report to `<worktree>\.claude\reports\dakara\<packet>-packet-coder.md` (DK-31a: `<packet>-rust-gameserver-dev.md`): files changed, each check's final `status=` line, the commit hash, every deviation and why. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. A context stop past about 90k tokens: commit only passing work (subject suffix `(part N)`), write `docs/analysis/dakara-e1-rebuild/worknotes/<packet>.md` and return `blocked` with the reason `context`.

## Shared contract for waves 5 and 6

**Row shape.** Exactly the C4 example of [work-packets.md](../work-packets.md#c4-chain-row-shape): one `content_chains` insert (`scope_type 'mission'`, `scope_id` the mission, `enabled true`, `priority 0`), one `content_triggers` insert (`scope 'player'`, `once false`, `sort_order 0`), multi-row conditions and actions with `sort_order` from 0 in the order written below. Every action has `delay_ms 0` unless the table says `after 2000 ms`. Descriptions read `'<mission> - <step or state>: <what>'`, with `'` doubled.

**Notation in the chain tables.**

| Written | Row |
|---|---|
| `world 61` | `(id, 'world', 61, NULL, 'eq', NULL, n)` |
| `arch gte 7` / `arch lte 6` / `arch gte 1` | `(id, 'archetype', NULL, NULL, 'gte', '7', n)` and so on |
| `m1651 not_active` | `(id, 'mission_status', 1651, NULL, 'eq', 'not_active', n)`; likewise `active`, `completed` |
| `s1651/4932 active` | `(id, 'step_status', 1651, '4932', 'eq', 'active', n)` |
| `o1653/5641 not completed` | `(id, 'objective_status', 1653, '5641', 'neq', 'completed', n)` |
| `o1653/5641 completed` | `(id, 'objective_status', 1653, '5641', 'eq', 'completed', n)` |
| `display 5843` | `(id, 'display_dialog', 5843, NULL, '{}', 0, n)` |
| `radio 5849` | `(id, 'display_radio_dialog', 5849, NULL, '{}', 0, n)` (DK-06b's verb: a dialog with no speaker entity) |
| `radio 5849 after 2000 ms` | the same row with `delay_ms` 2000 |
| `advance 1651->4930` | `(id, 'advance_step', 1651, '4930', '{}', 0, n)` |
| `objective 1653/5642` | `(id, 'complete_objective', 1653, '5642', '{}', 0, n)` |
| `accept 1651` / `complete 1651` | `(id, 'accept_mission', 1651, NULL, '{}', 0, n)` / `'complete_mission'` |
| `bind 6772@59` / `unbind 6772@59` | `(id, 'add_dialog_set', 6772, NULL, '{"slot": 59}', 0, n)` / `'remove_dialog_set'` |
| `take 6784` | `(id, 'remove_item', 6784, NULL, '{"qty": 1}', 0, n)` |
| `teleport SGC H3` | `(id, 'cross_world_teleport', NULL, 'SGC_W1', '{"x": 172.79, "y": -10.9, "z": 12.0}', 0, n)` (the frozen Union arrival H3 on the Gate Room floor; precedent: chains 8003-8006) |

Triggers: `loaded Dakara_E1` is `('player_loaded', 'Dakara_E1')`, `loaded StoryRm` is `('player_loaded', 'Dakara_E1_StoryRm')`; `click <tag>` is `('interact_tag', '<tag>')`; `region <key>` is `('enter_region', '<key>')`, the key byte-exact as the `point_sets.name`; `choice 5986` is `('dialog_choice', '5986')`; `done 1650` is `('mission_completed', '1650')`.

**How binds behave** (read this before writing a chain). A bind is an entry in the player's cell entity for the world they stand in; a relog or a world change drops every bind, so every bind has a `loaded <world>` restore chain gated on the same state. `add_dialog_set` pushes onto the slot's list, so a restore chain and the chain that first adds the bind must have disjoint conditions, or the NPC gets the dialog twice. Conditions are evaluated before any action runs, so a chain that advances a step in the same world as the next step's NPC either binds that NPC itself or has a co-firing chain gated on the old step. `complete_objective` on a step's last required objective completes the whole mission: leave a step with `advance` (it force-completes the step's objectives), finish with `complete`, and put every `unbind` before `complete` (it fires `mission_completed` chains inline).

**Interaction bits come from the bound dsm (D-DK31).** Every Dakara template is `interaction_type 0`; a player sees the NPC or prop as clickable only while a bound dialog-set map row carries a flag. Some steps' own dsm rows have flags 0, so the packet binds a flagged **carrier** instead, and the click chain displays the step's dialog. A carrier's own dialog is never shown by these chains (an `interact_tag` match skips the bind's auto-display). The carriers in this wave: **6892** (1653's dialog-less world-object row) for the 1653 power supplies; **6775** (1651's active row, bit 24) for Bra'tac at 1653 step 4935. Whether the client shows a carrier's marker as intended is a UAT item (DK-U25).

**A dialog with no NPC is a radio dialog.** `display_dialog` needs an NPC in scope; on a `player_loaded` or `dialog_choice` event with none it sends nothing and logs a WARN. Use `radio N` for those. A radio dialog is recorded as offered, so a `choice N` chain fires when the player closes it, whatever button they press.

**The DHD.** `Dakara_E1_DHD` (spawnlist 38, template 1) carries `INT_DHD`; a right-click is a normal interact. The cell fires `interact_tag` chains **before** the DHD handler and skips the handler when a chain matched (`crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs`, the `if !handled` block; `fire_interact_tag` returns true only when actions resolved). So at a matching step the click runs the chain and the dialling window does not open; at any other state the window opens as before. `cross_world_teleport` needs a `world` condition on its chain.

**Cast** (C6): Bra'tac template 59, tag `Dakara_E1_StoryRm_Bratac`, world 62. DK-41's props on world 61: ring controls 455 `Dakara_E1_RingControl_XMinus` (west) and 462 `Dakara_E1_RingControl_XPlus` (east); power supplies 456 `Dakara_E1_PowerSupply1` (west) and 463 `Dakara_E1_PowerSupply2` (east). The DHD, template 1, `Dakara_E1_DHD`, world 61. Region `Dakara_E1.SuperweaponCourtyard` (2126).

**Tests** (C7). Type: chain replay, live-DB (TESTING.md type 6). File: your submodule of `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/`. Imports: `use super::*;`, `use super::super::assert_no_deferred_actions;`, the `super::super::mission_1324` helpers you need (`ctx_for`, `with_mission`, `with_step`, `fire_player_loaded`, `fire_interact_tag`, `fire_dialog_choice`, `fire_mission_completed`, `display_dialog_count`), `use crate::cell::content::engine_loader::build_engine;`, `use crate::test_support::require_db_or_skip;` (copy the paths `report.rs` or `m1645.rs` uses). `mod.rs` gives you `labels_in`, `with_objective`, `fire_enter_region`, `delays_of`, `radio_count`, the archetype and world consts and your range const. Each test: `#[tokio::test] async fn live_db_<what>()`, `let pool = require_db_or_skip!(); let engine = build_engine(Some(&pool)).await;`. A missing param reads as `not_active`, and `with_step` sets only the step key, so a player "at step S of mission M" is always `with_step(with_mission(ctx, M, "active"), M, S, "active")`; a later mission's state also sets the earlier missions (1645 to 1650 as needed) `completed`. For **every chain**:

- the positive: the matching state gives exactly `labels_in(&r, <YOUR_RANGE>)` equal to the chain's actions in order, in the label format (`"8292:display_dialog(5843)"`, `"8326:remove_item(6784x1)"`, `"8290:accept_mission(1651)"`, `"8292:advance_step(1651->4930)"`, `"8354:complete_objective(1653/5642)"`, `"8290:add_dialog_set(6772@59)"`, `"8292:remove_dialog_set(6772@59)"`, `"8355:display_radio_dialog(5849)"`, `"8310:cross_world_teleport(SGC_W1@172.79,-10.90,12.00)"`, `"8294:complete_mission(1651)"`);
- one negative per condition that a sibling state could pass: the previous and next step, the wrong world, the mission not yet accepted or already completed, the other objective state, the other archetype. Each gives an empty list for that chain;
- on every `click` that displays: `display_dialog_count(&r) == 1` (unfiltered, across the whole engine); on every chain that shows a radio dialog: `radio_count(&r) == 1`;
- `assert_no_deferred_actions(&r, <chain>)` on every chain that advances, completes or teleports and has no delayed action; on a chain with a delayed action, `delays_of(&r, <chain>)` equals the table's delays in order.

Revert proof: every positive asserts a non-empty list, so removing the seed rows fails it.

**Checks, in order, for every wave-5 and wave-6 code packet** (from the worktree root):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine interact_tag_linter`
4. `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1_missions`

**Docs rows owed.** "Game systems, content chains, or content-engine actions": the `docs/content/mission-chains.md` rows are DK-99's, once for the campaign; say so in the report. The crossing rows C2 and C3 are already frozen in the Union plan; change nothing there.

## DK-30a: mission 1651 "Renewed Attack" (Dakara side)

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-30a-m1651` / `dk-30a` |
| Commit subject | `feat(content): Dakara_E1 mission 1651 accept, Bra'tac and return (DK-30a)` |
| Seed file / range | `dakara_e1_m1651_chains.sql`, 8290-8309 (`M1651_CHAINS`); 8290-8294 used |
| Test file | `dakara_e1_missions/m1651.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-30a-packet-coder.md` |
| Size | About 25k tokens |

1651 has no blurb and no Accept dialog, so it is accepted when 1650 is turned in (D-DK26). Step 4932 "Talk to Bra'tac about the status report": Bra'tac's 5843 "You have seen the Hataks?" (dsm 6772, active bit). Step 4930 "Gate to the SGC" is DK-30b's. Steps 4928, 4931 and 4929 are the SGC's (US-30), which sends the player back to the Dakara gate at step 5171 holding item 6784 "Naquadah Generator" (crossing C2b). Step 5171 "Report to Master Bra'tac at the Command Tent": 5847 "Did they send any help?" (dsm 6775, active bit). 6784 stays with the player: 1652 takes it.

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8290 | done 1650 | world 62; m1651 not_active | accept 1651; bind 6772@59 | auto-accept at the 1650 turn-in (in the tent, Bra'tac in scope) |
| 8291 | loaded StoryRm | world 62; s1651/4932 active | bind 6772@59 | restore; disjoint from 8290 (a different event) |
| 8292 | click `Dakara_E1_StoryRm_Bratac` | world 62; s1651/4932 active | display 5843; advance 1651->4930; unbind 6772@59 | the status report; now go to the gate |
| 8293 | loaded StoryRm | world 62; s1651/5171 active | bind 6775@59 | back from the SGC; the player arrives on world 61 and walks in |
| 8294 | click `Dakara_E1_StoryRm_Bratac` | world 62; s1651/5171 active | display 5847; unbind 6775@59; complete 1651 | report; unbind before complete |

**Tests** (`m1651.rs`), besides the shared rules: `fire_mission_completed(.., 1650)` on `STORY_RM` with 1650 `completed` gives `["8290:accept_mission(1651)", "8290:add_dialog_set(6772@59)"]`, and nothing on `DAKARA` or with 1651 active; 8293 gives nothing at 4930, 4928, 4931 or 4929; 8292 and 8294 pass `assert_no_deferred_actions`.

## DK-30b: mission 1651 step 4930, the DHD to the SGC (crossing C2)

> **Superseded 2026-10-19** by [wave 6c DK-30b-r](wave-6c.md#dk-30b-r-mission-1651-crossing-by-real-gate-travel) (D-DK24, real gate travel). Kept as the record of what `34abe129b` built; DK-30b-r replaces chain 8310, deletes 8311 and 8312, and adds 8313.

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-30b-m1651-gate` / `dk-30b` |
| Commit subject | `feat(content): Dakara_E1 mission 1651 gate to the SGC, crossing C2 (DK-30b)` |
| Seed file / range | `dakara_e1_m1651_gate_chains.sql`, 8310-8319 (`M1651_GATE_CHAINS`); 8310-8312 used |
| Test file | `dakara_e1_missions/m1651_gate.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-30b-packet-coder.md` |
| Size | About 25k tokens |
| Merge hold | After SGC US-30 merges (D-DK24) |

Crossing C2, frozen in the Union plan: DK-30 departs from 1651 step 4930 with `advance_step 1651 '4928'`, then `cross_world_teleport 'SGC_W1'` to H3 (172.79, -10.9, 12.0); the SGC's US-30 receives the player at 4928. No address is granted (the SGC has no DHD to dial back with; US-30 sends the player home). Chains 8311 and 8312 are recovery: a player at 4928 or 4931 who is on Dakara (a failed teleport, a GM move) can still reach the SGC.

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8310 | click `Dakara_E1_DHD` | world 61; s1651/4930 active | advance 1651->4928; teleport SGC H3 | C2, in the frozen order: advance, then teleport |
| 8311 | click `Dakara_E1_DHD` | world 61; s1651/4928 active | teleport SGC H3 | recovery at 4928 |
| 8312 | click `Dakara_E1_DHD` | world 61; s1651/4931 active | teleport SGC H3 | recovery at 4931 |

At step 4929 the SGC's own recovery chains (US-30) apply; write none for it.

**Tests** (`m1651_gate.rs`), besides the shared rules: 8310 gives exactly `["8310:advance_step(1651->4928)", "8310:cross_world_teleport(SGC_W1@172.79,-10.90,12.00)"]` and passes `assert_no_deferred_actions`; none of 8310-8312 fires at 4932, 4929 or 5171, on `STORY_RM`, or on another tag (`"Dakara_E1_CommandTerminal"`); with no 1651 state the DHD click resolves no chain in `M1651_GATE_CHAINS` (so the dialling window still opens); `display_dialog_count(&r) == 0` on 8310.

## DK-31a: the Ha'tak sequences for 1652 (desk note, no code)

| Field | Value |
|---|---|
| Role | `rust-gameserver-dev` (evidence only) |
| Branch / worktree | `dakara/dk-31a-hatak-sequences` / `dk-31a` |
| Commit subject | `docs(analysis): Dakara_E1 Ha'tak sequence ids for mission 1652 (DK-31a)` |
| Output | `docs/analysis/dakara-e1-rebuild/worknotes/DK-31a.md` (new, CRLF), nothing else |
| Report | `<worktree>\.claude\reports\dakara\DK-31a-rust-gameserver-dev.md` |
| Size | About 25k tokens |

DK-31b ships 1652 without any sequence. This note decides whether a later one-row-per-chain packet (DK-31c) can make the Ha'taks appear and explode. Answer, with file and line evidence:

1. What `play_sequence`'s `target_id` is on the wire (`ON_SEQUENCE`, `crates/content-engine` and the cell dispatcher): the `event_sets` row id, the event id inside it, or a Kismet sequence id. Calibrate on the working SGC precedent: chains 3028/3030 in `db/resources/Content/Seed/sgc_w1_chains.sql` play 10009/10010 ("Designer 0: Close Doors" / "Designer 1: Open Doors" of `CartersLabDoors`); find those ids in `db/resources/` (`event_sets.sql` and its neighbours) and say which column they are.
2. The audit's facts (`docs/analysis/dakara-e1-rebuild/audit.md` line 135 and finding 4 at line 145): event sets 1194 (`event_sets.sql:1087`) and 1195 (`:73`) hold events 6000 and 6001 on `...GA-Hatak00_Pf0_Seq` and `..._Seq_0`, "Designer 0: Unhide Hatak" and "Designer 1: Hatak Explosion"; Ha'tak meshes at (367.2, 359.6, 136.1) and (-111.7, 353.9, 78.1). Which set is the western Ha'tak (the lower x) and which the eastern?
3. Are the Ha'taks hidden until "Unhide" plays (so a player sees nothing above the staging areas without it)?
4. The exact `play_sequence` rows DK-31c would add, and where: unhide both on 1652 accept and on a `loaded Dakara_E1` restore at steps 4933 and 4934; explode the western one in chain 8325 and the eastern one in 8326 (each after the dialog, `delay_ms` your call). `play_sequence` reaches the acting player only.

Do not use Ghidra, the lab or the client. If the seed and the code cannot answer a question, say "unknown" and what evidence would settle it. Checks: none (no code); verify the note is CRLF.

## DK-31b: mission 1652 "Fireball"

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-31b-m1652` / `dk-31b` |
| Commit subject | `feat(content): Dakara_E1 mission 1652 (DK-31b)` |
| Seed file / range | `dakara_e1_m1652_chains.sql`, 8320-8349 (`M1652_CHAINS`); 8320-8326 used |
| Test file | `dakara_e1_missions/m1652.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-31b-packet-coder.md` |
| Size | About 30k tokens |

Bra'tac offers 1652 after the 1651 report: blurb 5986 (dsm 6776, available bit); 5846 (dsm 6777, flags 0, five Accept screens) is the briefing. Step 4933 "Destroy the Hatak above Ba'al's western staging area": the western ring control (`XMinus`, DK-41's 8018), 6108 "The naquadah bomb will explode in a few seconds..." (dsm 6887, active bit). Step 4934 the eastern one: the eastern ring control (`XPlus`, 8019), 6109 (dsm 6888, active bit). The player carries the generator from the SGC (6784) and spends it on the second bomb (D-DK30). 1652 can be abandoned; an abandon puts it back to `not_active`, so 8321 offers it again on the next tent entry.

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8320 | done 1651 | world 62; m1652 not_active | bind 6776@59 | the offer, at the 1651 report |
| 8321 | loaded StoryRm | world 62; m1651 completed; m1652 not_active | bind 6776@59 | restore the offer |
| 8322 | choice 5986 | world 62; m1651 completed; m1652 not_active | accept 1652; display 5846; unbind 6776@59 | accept and brief |
| 8323 | loaded Dakara_E1 | world 61; s1652/4933 active | bind 6887@455 | the western control (the accept is in world 62) |
| 8324 | loaded Dakara_E1 | world 61; s1652/4934 active | bind 6888@462 | restore the eastern control |
| 8325 | click `Dakara_E1_RingControl_XMinus` | world 61; s1652/4933 active | display 6108; advance 1652->4934; unbind 6887@455; bind 6888@462 | the western bomb; the eastern control lights in the same world |
| 8326 | click `Dakara_E1_RingControl_XPlus` | world 61; s1652/4934 active | display 6109; take 6784; unbind 6888@462; complete 1652 | the eastern bomb; unbind before complete (DK-32a's accept of 1653 fires inline) |

**Tests** (`m1652.rs`), besides the shared rules: 8325 and 8326 each pass `assert_no_deferred_actions` and `display_dialog_count(&r) == 1`; the eastern control at 4933 and the western at 4934 resolve nothing; with 1652 completed neither control resolves anything. Assert only your own range (DK-32a's 8350 co-fires on 1652's completion in the live engine, but the replay of a click does not fire `mission_completed`).

## DK-32a: mission 1653 "Superweapon" (no fight)

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-32a-m1653` / `dk-32a` |
| Commit subject | `feat(content): Dakara_E1 mission 1653 (DK-32a)` |
| Seed file / range | `dakara_e1_m1653_chains.sql`, 8350-8389 (`M1653_CHAINS`); 8350-8363 used |
| Test file | `dakara_e1_missions/m1653.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-32a-packet-coder.md` |
| Size | About 40k tokens |

1653 has no blurb and no Accept, so it is accepted when 1652 completes (D-DK27; the player stands at the eastern ring control on world 61). Step 4936 "Get to the Superweapon Courtyard!" (O5958): the region. Step 4938 "Disable the cannons": O5641 (eastern) and O5642 (western) required, O5643 (the vocuum) optional and not built here. The power supplies are lit through the carrier 6892 (D-DK31) and show no dialog: the objective tick and the marker going out are the feedback. Step 4937 "Use the radio to contact Bra'tac" (O5959): Bra'tac's "What news?" 5849 as a radio call. Step 4935 "Report back to Bra'tac" (O5960): 5848 in person (dsm 6778 has flags 0, so Bra'tac carries 6775). The guards in the courtyard are DK-32b's (wave 7, blocked on D-DK5); without them this is the peaceful path.

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8350 | done 1652 | world 61; m1653 not_active | accept 1653 | auto-accept |
| 8351 | region `Dakara_E1.SuperweaponCourtyard` | world 61; s1653/4936 active | advance 1653->4938; bind 6892@456; bind 6892@463 | in the courtyard; both supplies light |
| 8352 | loaded Dakara_E1 | world 61; s1653/4938 active; o1653/5642 not completed | bind 6892@456 | restore the western supply |
| 8353 | loaded Dakara_E1 | world 61; s1653/4938 active; o1653/5641 not completed | bind 6892@463 | restore the eastern supply |
| 8354 | click `Dakara_E1_PowerSupply1` | world 61; s1653/4938 active; o1653/5642 not completed; o1653/5641 not completed | objective 1653/5642; unbind 6892@456 | western first (5641 still open, so the mission does not complete) |
| 8355 | click `Dakara_E1_PowerSupply1` | world 61; s1653/4938 active; o1653/5642 not completed; o1653/5641 completed | unbind 6892@456; advance 1653->4937; radio 5849 | western second: leave the step with `advance`, never the last `objective` |
| 8356 | click `Dakara_E1_PowerSupply2` | world 61; s1653/4938 active; o1653/5641 not completed; o1653/5642 not completed | objective 1653/5641; unbind 6892@463 | eastern first |
| 8357 | click `Dakara_E1_PowerSupply2` | world 61; s1653/4938 active; o1653/5641 not completed; o1653/5642 completed | unbind 6892@463; advance 1653->4937; radio 5849 | eastern second |
| 8358 | choice 5849 | s1653/4937 active | advance 1653->4935 | closing the call; no world condition (a radio window can be closed after a flap trip) |
| 8359 | choice 5849 | world 62; s1653/4937 active | bind 6775@59 | co-fires with 8358 when the call is closed in the tent |
| 8360 | loaded Dakara_E1 | world 61; s1653/4937 active | radio 5849 after 2000 ms | recovery: the call was never closed |
| 8361 | loaded StoryRm | world 62; s1653/4937 active | radio 5849 after 2000 ms | the same in the tent |
| 8362 | loaded StoryRm | world 62; s1653/4935 active | bind 6775@59 | Bra'tac, report |
| 8363 | click `Dakara_E1_StoryRm_Bratac` | world 62; s1653/4935 active | display 5848; unbind 6775@59; complete 1653 | report; unbind before complete (DK-33's offer binds inline) |

A player already inside the courtyard when 1653 is accepted must step out and back in: `enter_region` fires on entry only. That is a UAT note (DK-U31), not a chain.

**Tests** (`m1653.rs`), besides the shared rules: 8351 through `fire_enter_region(&engine, &ctx, "Dakara_E1.SuperweaponCourtyard")` at 4936 gives `["8351:advance_step(1653->4938)", "8351:add_dialog_set(6892@456)", "8351:add_dialog_set(6892@463)"]` and nothing at 4938 or with `"Dakara_E1.MohkatanTent"`; one test walks the supplies in both orders (west then east, east then west) through `with_objective` and asserts exactly one supply chain per click with the right labels; a supply whose objective is completed resolves nothing; 8355 and 8357 give `radio_count(&r) == 1` and `assert_no_deferred_actions`; 8358 fires with no world and with `STORY_RM`, and 8359 only on `STORY_RM`; 8360 and 8361 give `delays_of == [2000]`; 8363 passes `assert_no_deferred_actions`.

## DK-33: mission 1654 "Aftermath" and the DHD to the SGC (crossing C3)

> **Partly superseded 2026-10-19** by [wave 6c DK-33-r](wave-6c.md#dk-33-r-mission-1654-crossing-by-real-gate-travel) (D-DK24): chains 8432 and 8433 also grant gate 27, 8434 completes 5961 on `stargate_crossed SGC_W1` instead of a DHD click and teleport, and 8435 is deleted. 8430 and 8431 stand as written.

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-33-m1654` / `dk-33` |
| Commit subject | `feat(content): Dakara_E1 mission 1654 offer and gate to the SGC, crossing C3 (DK-33)` |
| Seed file / range | `dakara_e1_m1654_chains.sql`, 8430-8449 (`M1654_CHAINS`); 8430-8435 used |
| Test file | `dakara_e1_missions/m1654.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-33-packet-coder.md` |
| Size | About 30k tokens |
| Merge hold | After SGC US-31 merges |

Bra'tac offers 1654 after the 1653 report: blurb 5987 (dsm 6780, available bit). The briefing depends on who you are: 5850 (dsm 6781, four Accept screens) for the Tau'ri archetypes 1 to 6, 5851 (dsm 6782, six Accept screens) for the Free Jaffa archetype 7 and up. Step 4939 "Report back to Gen. Hammond at the SGC" has two required objectives, O5961 and O5962. Crossing C3, frozen: DK-33 completes O5961 with `complete_objective 1654 '5961'` (never `advance_step`, which would close O5962 too) and teleports to H3; the SGC's US-31 completes O5962 at Hammond (5852 Human, 5853 Jaffa).

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8430 | done 1653 | world 62; m1654 not_active | bind 6780@59 | the offer, at the 1653 report |
| 8431 | loaded StoryRm | world 62; m1653 completed; m1654 not_active | bind 6780@59 | restore the offer |
| 8432 | choice 5987 | world 62; m1653 completed; m1654 not_active; arch gte 1; arch lte 6 | accept 1654; unbind 6780@59; display 5850 | Tau'ri briefing |
| 8433 | choice 5987 | world 62; m1653 completed; m1654 not_active; arch gte 7 | accept 1654; unbind 6780@59; display 5851 | Free Jaffa briefing |
| 8434 | click `Dakara_E1_DHD` | world 61; s1654/4939 active; o1654/5961 not completed | objective 1654/5961; teleport SGC H3 | C3: complete 5961 only, then teleport |
| 8435 | click `Dakara_E1_DHD` | world 61; s1654/4939 active; o1654/5961 completed | teleport SGC H3 | recovery: 5961 done but the player is still on Dakara |

**Tests** (`m1654.rs`), besides the shared rules: on `fire_dialog_choice(.., 5987)` a `TAURI` gets only 8432 and a `FREE_JAFFA` only 8433 (`display_dialog_count(&r) == 1` each); 8434 gives exactly `["8434:complete_objective(1654/5961)", "8434:cross_world_teleport(SGC_W1@172.79,-10.90,12.00)"]`, contains no `advance_step` label, and passes `assert_no_deferred_actions`; 8434 and 8435 are exclusive over O5961; neither fires on `STORY_RM`, with 1654 completed, or with 1651 at 4930 (so DK-30b's 8310 and these never both match).
