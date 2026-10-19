# Dakara_E1 wave 5: missions 1648, 1649 and 1650

> Type: how-to (work packets). Audience: the packet-coder named in each packet, and the read-only wave-5 packet-reviewer (no packet-review-fixer: the reviewer's findings go to a fresh packet-coder).
> Written 2026-10-19 (architect, autonomous) against `main` @ `eb9eaebe7`. Ledger and status: [README](../README.md#packets). Rules and full contract: [work-packets.md](../work-packets.md).

Wave 5 starts when DK-40 and DK-41 are merged. Its four packets write four different seed files and four different test files that DK-40 created, so all four run in parallel, each in its own worktree. Wave 6 can run beside it (different files).

**Dormant until 1647 ships (D-DK23).** Mission 1648 is offered when 1647 is completed, and 1647 is blocked on D-DK5 (wave 7). These chains are correct and tested now, but no player reaches them until 1647 exists. Build order is not story order.

## Rules for every wave-5 packet

1. PowerShell only. No bash, WSL or Git Bash. Use absolute paths inside your worktree; never `cd` to the root checkout. Prefix each lane call with `Set-Location <worktree>;` and check that the lane log lands under `logs/<worktree>/`.
2. Every compiling `cargo` call goes through the lane, run from the worktree root: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly; never add `--target-dir`. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`. Run lane checks in the foreground.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, or delete a worktree. Commit once, locally, with the subject below and the attribution lines from your brief.
4. Edit only your packet's seed file and test file. Do not edit `db/database.sql`, the linter, `dakara_e1_missions/mod.rs` or another packet's file. Files stay under 700 lines (soft cap 500).
5. The seed files are CRLF; keep them CRLF (check with `[IO.File]::ReadAllText(<abs>).Contains("`r`n")` and that no bare LF is left).
6. Seeds change in `db/resources/` only; never add a `db/scripts/` migration. Every chain has a comment above it: its evidence label (`PROJECT_FINAL_RECONSTRUCTION`: the client holds the missions, steps, objectives and dialogs but no trigger) and the step, objective or bind it serves.
7. No new dialog row, no client patch. Never edit templates 54 or 59 (shared with Harset). No rewards (D-DK7): no XP, cash or reward item.
8. Never launch the game client, the launcher or any lab tool.
9. No IPs, credentials, tokens or account names in any file.
10. Report to `<worktree>\.claude\reports\dakara\<packet>-packet-coder.md`: files changed, each check's final `status=` line, the commit hash, every deviation and why. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. A context stop past about 90k tokens: commit only passing work (subject suffix `(part N)`), write `docs/analysis/dakara-e1-rebuild/worknotes/<packet>.md` and return `blocked` with the reason `context`.

## Shared contract for waves 5 and 6

**Row shape.** Exactly the C4 example of [work-packets.md](../work-packets.md#c4-chain-row-shape): one `content_chains` insert (`scope_type 'mission'`, `scope_id` the mission, `enabled true`, `priority 0`), one `content_triggers` insert (`scope 'player'`, `once false`, `sort_order 0`), multi-row conditions and actions with `sort_order` from 0 in the order written below. Every action has `delay_ms 0` unless the table says `after 2000 ms`. Descriptions read `'<mission> - <step or state>: <what>'`, with `'` doubled.

**Notation in the chain tables.**

| Written | Row |
|---|---|
| `world 61` | `(id, 'world', 61, NULL, 'eq', NULL, n)` |
| `arch gte 7` / `arch lte 6` / `arch gte 1` | `(id, 'archetype', NULL, NULL, 'gte', '7', n)` and so on |
| `m1648 not_active` | `(id, 'mission_status', 1648, NULL, 'eq', 'not_active', n)`; likewise `active`, `completed` |
| `s1648/4919 active` | `(id, 'step_status', 1648, '4919', 'eq', 'active', n)` |
| `o1649/5639 not completed` | `(id, 'objective_status', 1649, '5639', 'neq', 'completed', n)` |
| `o1649/5638 completed` | `(id, 'objective_status', 1649, '5638', 'eq', 'completed', n)` |
| `display 5829` | `(id, 'display_dialog', 5829, NULL, '{}', 0, n)` |
| `radio 5833` | `(id, 'display_radio_dialog', 5833, NULL, '{}', 0, n)` (DK-06b's verb: a dialog with no speaker entity) |
| `radio 5833 after 2000 ms` | the same row with `delay_ms` 2000 |
| `advance 1648->4918` | `(id, 'advance_step', 1648, '4918', '{}', 0, n)` |
| `objective 1649/5638` | `(id, 'complete_objective', 1649, '5638', '{}', 0, n)` |
| `accept 1648` / `complete 1648` | `(id, 'accept_mission', 1648, NULL, '{}', 0, n)` / `'complete_mission'` |
| `bind 6754@440` / `unbind 6754@440` | `(id, 'add_dialog_set', 6754, NULL, '{"slot": 440}', 0, n)` / `'remove_dialog_set'` |
| `give 6781` / `take 6781` | `(id, 'add_item', 6781, NULL, '{"container": 0, "qty": 1}', 0, n)` / `(id, 'remove_item', 6781, NULL, '{"qty": 1}', 0, n)` |
| `spawn hologram` | `(id, 'spawn_entity', 443, 'Dakara_E1_StoryRm_BaalHologram', '{"x": 66.2, "y": -0.12, "z": 34.3, "heading": 1.5708, "is_stationary": true}', 0, n)` (placement B-07; world 62 is instanced, so no `allow_shared`) |
| `despawn hologram` | `(id, 'despawn_entity', NULL, 'Dakara_E1_StoryRm_BaalHologram', '{}', 0, n)` |
| `teleport SGC H3` | `(id, 'cross_world_teleport', NULL, 'SGC_W1', '{"x": 172.79, "y": -10.9, "z": 12.0}', 0, n)` (the frozen Union arrival H3; precedent: chains 8003-8006) |

Triggers: `loaded Dakara_E1` is `('player_loaded', 'Dakara_E1')`, `loaded StoryRm` is `('player_loaded', 'Dakara_E1_StoryRm')`; `click <tag>` is `('interact_tag', '<tag>')`; `region <key>` is `('enter_region', '<key>')`, the key byte-exact as the `point_sets.name`; `choice 5983` is `('dialog_choice', '5983')`; `done 1647` is `('mission_completed', '1647')`.

**How binds behave** (read this before writing a chain). A bind is an entry in the player's cell entity for the world they stand in; a relog or a world change drops every bind, so every bind has a `loaded <world>` restore chain gated on the same state. `add_dialog_set` pushes onto the slot's list, so a restore chain and the chain that first adds the bind must have disjoint conditions, or the NPC gets the dialog twice. Conditions are evaluated before any action runs, so a chain that advances a step in the same world as the next step's NPC either binds that NPC itself or has a co-firing chain gated on the old step. `complete_objective` on a step's last required objective completes the whole mission: leave a step with `advance` (it force-completes the step's objectives), finish with `complete`, and put every `unbind` before `complete` (it fires `mission_completed` chains inline).

**Interaction bits come from the bound dsm (D-DK31).** Every Dakara template is `interaction_type 0`; a player sees the NPC or prop as clickable only while a bound dialog-set map row carries a flag. Some steps' own dsm rows have flags 0, so the packet binds a flagged **carrier** instead, and the click chain displays the step's dialog. A carrier's own dialog is never shown by these chains. The carriers: **6760** (mission 1649's dialog-less world-object row, bit 30) for the 1649 Command Terminal, the 1649 drops and the 1650 terminal; **6892** (1653's dialog-less world-object row) for the 1653 power supplies; **6775** (1651's active row, bit 24) for Bra'tac at 1653 step 4935. Whether the client shows a carrier's marker as intended is a UAT item (DK-U25).

**A dialog with no NPC is a radio dialog.** `display_dialog` needs an NPC in scope; on a `player_loaded` or `dialog_choice` event with none it sends nothing and logs a WARN. Use `radio N` for those. A radio dialog is recorded as offered, so a `choice N` chain fires when the player closes it, whatever button they press (any close accepts an offer).

**Cast** (C6): Bra'tac template 59, tag `Dakara_E1_StoryRm_Bratac`, world 62. Moh'katan 54, `Dakara_E1_StoryRm_Mohkatan`, world 62. Loth'ta 440, `Dakara_E1_Lothta`, world 61. Drops 449, 458-461, `Dakara_E1_Drop1` to `Drop5`, world 61. DK-41's props: Command Terminal 453, `Dakara_E1_CommandTerminal`, world 61; Moh'katan's Terminal 454, `Dakara_E1_StoryRm_MohkatanTerminal`, world 62; ring controls 455 `Dakara_E1_RingControl_XMinus` (west) and 462 `Dakara_E1_RingControl_XPlus` (east), world 61; power supplies 456 `Dakara_E1_PowerSupply1` (west) and 463 `Dakara_E1_PowerSupply2` (east), world 61; the DHD, template 1, `Dakara_E1_DHD`, world 61. Regions: `Dakara_E1.MohkatanTent` (2124) and `Dakara_E1.SuperweaponCourtyard` (2126).

**Tests** (C7). Type: chain replay, live-DB (TESTING.md type 6). File: your submodule of `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/`. Imports: `use super::*;`, `use super::super::assert_no_deferred_actions;`, the `super::super::mission_1324` helpers you need (`ctx_for`, `with_mission`, `with_step`, `fire_player_loaded`, `fire_interact_tag`, `fire_dialog_choice`, `fire_mission_completed`, `display_dialog_count`), `use crate::cell::content::engine_loader::build_engine;`, `use crate::test_support::require_db_or_skip;` (copy the paths `report.rs` or `m1645.rs` uses). `mod.rs` gives you `labels_in`, `with_objective`, `fire_enter_region`, `delays_of`, `radio_count`, the archetype and world consts and your range const. Each test: `#[tokio::test] async fn live_db_<what>()`, `let pool = require_db_or_skip!(); let engine = build_engine(Some(&pool)).await;`. A missing param reads as `not_active`, and `with_step` sets only the step key, so a player "at step S of mission M" is always `with_step(with_mission(ctx, M, "active"), M, S, "active")`; a later mission's state also sets the earlier missions `completed`. For **every chain**:

- the positive: the matching state gives exactly `labels_in(&r, <YOUR_RANGE>)` equal to the chain's actions in order, in the label format (`"8184:display_dialog(5829)"`, `"8184:add_item(6781x1)"`, `"8186:remove_item(6781x1)"`, `"8182:accept_mission(1648)"`, `"8184:advance_step(1648->4918)"`, `"8223:complete_objective(1649/5638)"`, `"8183:add_dialog_set(6754@440)"`, `"8184:remove_dialog_set(6754@440)"`, `"8204:display_radio_dialog(5833)"`, `"8254:spawn_entity(443#Dakara_E1_StoryRm_BaalHologram)"`, `"8258:despawn_entity(Dakara_E1_StoryRm_BaalHologram)"`, `"8310:cross_world_teleport(SGC_W1@172.79,-10.90,12.00)"`);
- one negative per condition that a sibling state could pass: the previous and next step, the wrong world, the mission not yet accepted or already completed, the other objective state. Each gives an empty list for that chain;
- on every `click` that displays: `display_dialog_count(&r) == 1` (unfiltered, across the whole engine); on every chain that shows a radio dialog: `radio_count(&r) == 1`;
- `assert_no_deferred_actions(&r, <chain>)` on every chain that advances or completes and has no delayed action; on a chain with a delayed action, `delays_of(&r, <chain>)` equals the table's delays in order (for example `[2000, 0]`).

Revert proof: every positive asserts a non-empty list, so removing the seed rows fails it.

**Checks, in order, for every wave-5 and wave-6 packet** (from the worktree root):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine interact_tag_linter`
4. `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1_missions`

**Docs rows owed.** "Game systems, content chains, or content-engine actions": the `docs/content/mission-chains.md` rows are DK-99's, once for the campaign; say so in the report. Nothing else.

## DK-13: mission 1648 "Loth'ta's Withdrawal"

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-13-m1648` / `dk-13` |
| Commit subject | `feat(content): Dakara_E1 mission 1648 (DK-13)` |
| Seed file / range | `dakara_e1_m1648_chains.sql`, 8180-8199 (`M1648_CHAINS`); 8180-8186 used |
| Test file | `dakara_e1_missions/m1648.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-13-packet-coder.md` |
| Size | About 30k tokens |

Bra'tac offers 1648 when 1647 is done: his blurb 5983 (dsm 6752, available bit) has Accept and More Info. Step 4919 "Speak to Loth'ta at the Naquadah Repository" (objective 5944): Loth'ta's 5829 (dsm 6754, active bit), and he hands over item 6781 "Message to Bra'tac". Step 4918 "Deliver Loth'ta's refusal to Bra'tac" (5945): Bra'tac's 5830 (dsm 6755, turn-in bit). 5828 (Bra'tac's briefing, dsm 6753) has flags 0, so it is shown on accept and never bound.

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8180 | done 1647 | world 62; m1648 not_active | bind 6752@59 | the offer, when 1647 completes in the tent (if it completes on world 61, 8181 binds it on entry) |
| 8181 | loaded StoryRm | world 62; m1647 completed; m1648 not_active | bind 6752@59 | restore the offer; disjoint from 8180 (a different event) |
| 8182 | choice 5983 | world 62; m1647 completed; m1648 not_active | accept 1648; display 5828; unbind 6752@59 | accept and brief |
| 8183 | loaded Dakara_E1 | world 61; s1648/4919 active | bind 6754@440 | Loth'ta (the accept is in world 62, so this chain is his only bind) |
| 8184 | click `Dakara_E1_Lothta` | world 61; s1648/4919 active | display 5829; give 6781; advance 1648->4918; unbind 6754@440 | Loth'ta refuses; leave 4919 |
| 8185 | loaded StoryRm | world 62; s1648/4918 active | bind 6755@59 | Bra'tac, turn-in |
| 8186 | click `Dakara_E1_StoryRm_Bratac` | world 62; s1648/4918 active | display 5830; take 6781; unbind 6755@59; complete 1648 | turn-in; unbind before complete |

**Tests** (`m1648.rs`), besides the shared rules: `fire_mission_completed(&engine, &with_mission(ctx_for(FREE_JAFFA, Some(STORY_RM)), 1647, "completed"), 1647)` gives `["8180:add_dialog_set(6752@59)"]`, and the same on `Some(DAKARA)` gives an empty list; a Tau'ri (`TAURI`) gets the same chains as a Free Jaffa (1648 has no archetype split); 8184 and 8186 each pass `assert_no_deferred_actions`.

## DK-14a: mission 1649 offer, Bra'tac, the radio call to Teal'c, and the turn-in

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-14a-m1649-offer` / `dk-14a` |
| Commit subject | `feat(content): Dakara_E1 mission 1649 offer, radio and turn-in (DK-14a)` |
| Seed file / range | `dakara_e1_m1649_offer_chains.sql`, 8200-8219 (`M1649_OFFER_CHAINS`); 8200-8208 used |
| Test file | `dakara_e1_missions/m1649_offer.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-14a-packet-coder.md` |
| Size | About 35k tokens |

1649 "Moh'katan" has no offering NPC: its blurb 5984 (dsm 6756) has flags 0, and its only Accept is 5831, a narrator text ("Something isn't sitting right with you about Moh'katan...", speaker 0, one Accept button). So the offer is a radio dialog two seconds after the player next enters world 61 once 1648 is done (D-DK25). Steps: 4923 "Talk to Bra'tac about his daughter" (5832, dsm 6758, active bit, five screens, no buttons); 4921 "Use your radio to contact Teal'c" (5833, a radio dialog, speaker Teal'c); 4922 "Investigate" (DK-14b); 4920 "Present the evidence to Bra'tac" (5838, dsm 6765, turn-in bit).

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8200 | loaded Dakara_E1 | world 61; m1648 completed; m1649 not_active | radio 5831 after 2000 ms | the offer (D-DK25); repeats on each entry until accepted |
| 8201 | choice 5831 | m1648 completed; m1649 not_active | accept 1649 | closing the offer accepts it; no world condition, since a radio window can be closed after a flap trip |
| 8202 | loaded StoryRm | world 62; s1649/4923 active | bind 6758@59 | Bra'tac (the accept is in world 61) |
| 8203 | click `Dakara_E1_StoryRm_Bratac` | world 62; s1649/4923 active | display 5832; advance 1649->4921; unbind 6758@59 | Bra'tac on his daughter; leave 4923 |
| 8204 | choice 5832 | world 62; s1649/4921 active | radio 5833; advance 1649->4922 | closing Bra'tac's talk makes the radio call to Teal'c; leave 4921 |
| 8205 | loaded Dakara_E1 | world 61; s1649/4921 active | radio 5833 after 2000 ms; advance 1649->4922 | recovery: the player left before closing 5832. DK-14b's 8222 co-fires to bind the terminal and drops |
| 8206 | loaded StoryRm | world 62; s1649/4921 active | radio 5833 after 2000 ms; advance 1649->4922 | the same recovery in the tent |
| 8207 | loaded StoryRm | world 62; s1649/4920 active | bind 6765@59 | Bra'tac, turn-in (the advance to 4920 is DK-14b's, in world 61) |
| 8208 | click `Dakara_E1_StoryRm_Bratac` | world 62; s1649/4920 active | display 5838; take 6782; take 6783; unbind 6765@59; complete 1649 | the evidence: Moh'katan's orders (6782) and the decoded message (6783) |

The radio and the advance in 8205 and 8206 are separate actions: the advance runs at once (so the step gate closes) and only the radio waits. 6780 (the encrypted message) is taken by DK-14b when it is decoded.

**Tests** (`m1649_offer.rs`), besides the shared rules: 8200 gives `["8200:display_radio_dialog(5831)"]` with `delays_of(&r, 8200) == [2000]`, and nothing on `STORY_RM` or with 1648 active; 8201 fires on `fire_dialog_choice(.., 5831)` with no world set and not with 1649 active; 8204 gives `["8204:display_radio_dialog(5833)", "8204:advance_step(1649->4922)"]` with `radio_count(&r) == 1` and `assert_no_deferred_actions`; 8205 and 8206 give `delays_of == [2000, 0]`; 8203 and 8208 pass `assert_no_deferred_actions`.

## DK-14b: mission 1649 step 4922, the Command Terminal and the drops

> **Amended 2026-10-19 (owner ruling D-DK31).** The terminal hack is now the Livewire minigame: [wave 6c DK-14c](wave-6c.md#dk-14c-livewire-on-the-1649-command-terminal) turns chain 8223 into the launcher and moves its three actions to victory chain 8231. Chains 8224 and 8225 stay plain clicks (D-DK42). Merged as written in #1905.

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-14b-m1649-terminal` / `dk-14b` |
| Commit subject | `feat(content): Dakara_E1 mission 1649 step 4922 terminal and drops (DK-14b)` |
| Seed file / range | `dakara_e1_m1649_terminal_chains.sql`, 8220-8249 (`M1649_TERMINAL_CHAINS`); 8220-8230 used |
| Test file | `dakara_e1_missions/m1649_terminal.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-14b-packet-coder.md` |
| Size | About 40k tokens |

Step 4922 "Investigate Moh'katan's behavior" has three required objectives: 5638 "Hack into a terminal in the command center and review Moh'katan's recent orders", 5639 "Search the drop locations ... to see if any of her orders are still there", 5640 "Decode Moh'katan's orders and read their contents". The dialogs are narrator texts with flags 0: 5834 "You download Moh'katan's recent orders...", 5835 "You find one of Moh'katan's coded messages, still in its hiding spot.", 5836 "The message you placed here has already been taken.", 5837 "You decode the message...". Items: 6782 "Copy of Moh'katan's Orders", 6780 "Encrypted Message", 6783 "Decoded Message". The props are clickable only through the carrier 6760 (D-DK31). Drop 5 holds the message (RECONSTRUCTION: the client names no drop); drops 1-4 say it was taken. Decoding is a second terminal use once the message is found; 5640 is never completed by hand (the advance completes it), so neither 5638 nor 5639 is ever the step's last open objective.

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8220 | loaded Dakara_E1 | world 61; s1649/4922 active | bind 6760@453 | the terminal's carrier, on entry and after a relog |
| 8221 | loaded Dakara_E1 | world 61; s1649/4922 active; o1649/5639 not completed | bind 6760@449; bind 6760@458; bind 6760@459; bind 6760@460; bind 6760@461 | the five drops until the message is found |
| 8222 | loaded Dakara_E1 | world 61; s1649/4921 active | bind 6760@453; bind 6760@449; bind 6760@458; bind 6760@459; bind 6760@460; bind 6760@461 | co-fires with DK-14a's 8205, which advances 4921->4922 in this event (conditions are read before it runs); disjoint from 8220/8221 by step |
| 8223 | click `Dakara_E1_CommandTerminal` | world 61; s1649/4922 active; o1649/5638 not completed | display 5834; give 6782; objective 1649/5638 | the hack: download the orders |
| 8224 | click `Dakara_E1_CommandTerminal` | world 61; s1649/4922 active; o1649/5638 completed; o1649/5639 completed | display 5837; take 6780; give 6783; advance 1649->4920; unbind 6760@453 | decode the message; leave 4922 (completes 5640) |
| 8225 | click `Dakara_E1_CommandTerminal` | world 61; s1649/4922 active; o1649/5638 completed; o1649/5639 not completed | display 5834 | feedback: the orders again, nothing to decode yet |
| 8226 | click `Dakara_E1_Drop5` | world 61; s1649/4922 active; o1649/5639 not completed | display 5835; give 6780; objective 1649/5639; unbind 6760@449; unbind 6760@458; unbind 6760@459; unbind 6760@460; unbind 6760@461 | the coded message |
| 8227 | click `Dakara_E1_Drop1` | world 61; s1649/4922 active; o1649/5639 not completed | display 5836 | already taken |
| 8228 | click `Dakara_E1_Drop2` | same as 8227 | display 5836 | already taken |
| 8229 | click `Dakara_E1_Drop3` | same as 8227 | display 5836 | already taken |
| 8230 | click `Dakara_E1_Drop4` | same as 8227 | display 5836 | already taken |

Mission 1646's own drop chains (DK-12c, 8140-8159) gate on 1646 state, so they stay quiet here; check that the shared-rule `display_dialog_count == 1` holds on each drop click with 1646 `completed`.

**Tests** (`m1649_terminal.rs`), besides the shared rules (build every 4922 context with 1648 `completed` and 1646 `completed`): 8222 resolves on a 4921 load and 8220/8221 do not; with `with_objective(.., 1649, 5639, "completed")` 8221 and 8226-8230 give nothing; 8223, 8224 and 8225 are mutually exclusive: one test walks the four combinations of 5638 and 5639 and asserts exactly one terminal chain each time (both open: 8223; 5638 open, 5639 done: 8223; 5638 done, 5639 open: 8225; both done: 8224); 8224 passes `assert_no_deferred_actions`. Assert only your own range (DK-14a may not be merged yet).

## DK-15: mission 1650 "Confront Moh'katan"

> **Amended 2026-10-19 (owner ruling D-DK31).** Moh'katan's terminal now runs Livewire: [wave 6c DK-15b](wave-6c.md#dk-15b-livewire-on-mohkatans-terminal) turns chain 8258 into the launcher and moves its five actions to victory chain 8261 (D-DK42: the owner may drop DK-15b). Merged as written in #1906.

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-15-m1650` / `dk-15` |
| Commit subject | `feat(content): Dakara_E1 mission 1650 (DK-15)` |
| Seed file / range | `dakara_e1_m1650_chains.sql`, 8250-8289 (`M1650_CHAINS`); 8250-8260 used |
| Test file | `dakara_e1_missions/m1650.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-15-packet-coder.md` |
| Size | About 40k tokens |

Bra'tac offers 1650 right after the 1649 turn-in: blurb 5985 (dsm 6767, available bit); his 5839 ("You will meet me at her tent", flags 0) is the briefing. Step 4927 "Meet Bra'tac at Moh'katan's tent": the region `Dakara_E1.MohkatanTent` on world 61 completes it (D-DK28). Step 4924 "Confront Moh'katan": inside the tent (world 62) Ba'al's hologram appears on its platform (template 443, placement B-07; a per-player spawn, since world 62 is instanced) and Moh'katan's 5840 (dsm 6768, active bit, eleven screens: Ba'al, Moh'katan, Bra'tac) plays. Step 4926 "Search Moh'katan's chambers": closing 5840 brings Bra'tac's 5841 "Search the room..." as a radio dialog (dsm 6769 has flags 0, and Bra'tac is not the one clicked), and the terminal (DK-41's 8017) lights through the carrier 6760; using it gives 6787 "Downloaded File" (6770, the original row for it, has no dialog and flags 0). Step 4925 "Speak to Bra'tac about his daughter's betrayal": 5842 (dsm 6771, turn-in bit).

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8250 | done 1649 | world 62; m1650 not_active | bind 6767@59 | the offer, after the 1649 turn-in in the tent |
| 8251 | loaded StoryRm | world 62; m1649 completed; m1650 not_active | bind 6767@59 | restore the offer |
| 8252 | choice 5985 | world 62; m1649 completed; m1650 not_active | accept 1650; display 5839; unbind 6767@59 | accept and brief |
| 8253 | region `Dakara_E1.MohkatanTent` | world 61; s1650/4927 active | advance 1650->4924 | at Moh'katan's tent (D-DK28) |
| 8254 | loaded StoryRm | world 62; s1650/4924 active | bind 6768@54; spawn hologram | Moh'katan and Ba'al's image, on entry and after a relog |
| 8255 | click `Dakara_E1_StoryRm_Mohkatan` | world 62; s1650/4924 active | display 5840; advance 1650->4926; unbind 6768@54; bind 6760@454 | the confrontation; the terminal lights in the same room |
| 8256 | choice 5840 | world 62; s1650/4926 active | radio 5841 | Bra'tac: search the room |
| 8257 | loaded StoryRm | world 62; s1650/4926 active | bind 6760@454; spawn hologram | restore after a relog |
| 8258 | click `Dakara_E1_StoryRm_MohkatanTerminal` | world 62; s1650/4926 active | give 6787; advance 1650->4925; unbind 6760@454; despawn hologram; bind 6771@59 | the file; the image fades; Bra'tac is in the same room |
| 8259 | loaded StoryRm | world 62; s1650/4925 active | bind 6771@59 | restore Bra'tac's turn-in |
| 8260 | click `Dakara_E1_StoryRm_Bratac` | world 62; s1650/4925 active | display 5842; take 6787; unbind 6771@59; complete 1650 | turn-in |

The spawn's tag guard refuses a second hologram while one is live, so 8254 and 8257 are safe to fire again in the same instance. The terminal click shows no dialog (none exists for it); the item grant and the step change are its feedback.

**Tests** (`m1650.rs`), besides the shared rules: 8253 through `fire_enter_region(&engine, &ctx, "Dakara_E1.MohkatanTent")` gives `["8253:advance_step(1650->4924)"]` at 4927 and nothing at 4924 or with the key `"Dakara_E1.CommandTent"`; 8254 gives `["8254:add_dialog_set(6768@54)", "8254:spawn_entity(443#Dakara_E1_StoryRm_BaalHologram)"]`; 8256 gives `radio_count(&r) == 1`; 8258's labels include `"8258:despawn_entity(Dakara_E1_StoryRm_BaalHologram)"` before `"8258:add_dialog_set(6771@59)"`; also assert the spawn's position from the resolved action (match `Action::SpawnEntity { .. }` and check its position is `[66.2, -0.12, 34.3]` within 0.01, its heading `1.5708`, and that it is not shared; read the variant in `crates/content-engine/src/actions.rs` and use its real field names); 8255, 8258 and 8260 pass `assert_no_deferred_actions`.
