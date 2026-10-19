# Dakara_E1 wave 3: missions 1570 (Dakara half), 1645 and 1646

> Type: how-to (work packets). Audience: the packet-coder named in each packet, and the read-only wave-3 packet-reviewer (no packet-review-fixer: the reviewer's findings go to a fresh packet-coder).
> Written 2026-10-18 against `main` @ `d1bba0400`. Ledger and status: [README](../README.md#packets). Rules and full contract: [work-packets.md](../work-packets.md).

Wave 3 starts when DK-10a, DK-09 and DK-09b are merged (DK-10fc also needs DK-10fs). DK-10d is split: DK-10d-B runs as wave 3b with DK-06b, and DK-10d-A runs as wave 3c after both merge (see [DK-10d](#dk-10d-1570-steps-4905-and-4903-split-into-dk-10d-b-and-dk-10d-a)). The seven packets write seven different seed files and seven different test files that DK-10a created, so all seven run in parallel, each in its own worktree. The reviewer may split the wave into two halves if one context cannot hold seven. Every packet carries its own rules; you do not need CLAUDE.md.

**Status of the sections (architect, 2026-10-18).** DK-10c and DK-12c now carry the rows DK-08 ruled, with the 4902 triggers from D-DK18 (three client-hinted regions that DK-09 seeds) and the ranges from D-DK19 (DK-10c 8060-8074, DK-10d 8160-8179). D-DK10 is ruled (option B, server-only: the `display_radio_dialog` verb from DK-06b), and DK-10d is split: DK-10d-B (step 4903) is complete and dispatchable now; DK-10d-A (step 4905) is complete and dispatchable once DK-06b and DK-10d-B have merged. DK-10b, DK-10c, DK-11, DK-12b and DK-12c are complete. Under D-DK1 **C**, DK-10b to DK-10d are dropped and DK-11's offer gate changes (see DK-11).

**Architect amendment (2026-10-18, D-DK1 option F, recommended).** A Free Jaffa now starts with the authored mission 63100 "Home to Dakara" (DK-10fs's catalog entry and seed; DK-10fc's chains below). Talking to Rak'nor completes it, and DK-10a's chain 8040 then starts 1570 at 4906 with Rak'nor's optional 5936 already completed. So DK-10b has no 8057 (the bridge binds Rak'nor), and its Rak'nor chains 8045-8048 serve only players who reach 4906 with 5936 open (Tau'ri from the SGC route, a Praxis Jaffa). Under fallback **B**, DK-10fc is not dispatched and DK-10b keeps 8057 (its "Fallback B" note).

## Rules for every wave-3 packet

1. PowerShell only. No bash, WSL or Git Bash. Use absolute paths inside your worktree; never `cd` to the root checkout. Prefix each lane call with `Set-Location <worktree>;` and check that the lane log lands under `logs/<worktree>/`.
2. Every compiling `cargo` call goes through the lane, run from the worktree root: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly; never add `--target-dir`. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`. Run lane checks in the foreground.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, or delete a worktree. Commit once, locally, with the subject below and the attribution lines from your brief.
4. Edit only your packet's seed file and test file (C3 and C7 of work-packets.md). Do not edit `db/database.sql`, the linter, `dakara_e1_missions/mod.rs` or another packet's file. Files stay under 700 lines (soft cap 500).
5. The seed files are CRLF; keep them CRLF (check with `[IO.File]::ReadAllText(<abs>).Contains("`r`n")`).
6. Seeds change in `db/resources/` only; never add a `db/scripts/` migration. Every chain has a comment above it: its evidence label (these are all `PROJECT_FINAL_RECONSTRUCTION`: the client holds the missions, steps, objectives and dialogs but no trigger; DK-10fc's chains are `NEW CONTENT`, because mission 63100 is) and the step, objective or bind it serves.
7. No new dialog row, no client patch. Never edit templates 54 or 59 (shared with Harset). No rewards (D-DK7): no XP, cash or reward item.
8. Never launch the game client, the launcher or any lab tool.
9. No IPs, credentials, tokens or account names in any file.
10. Report to `<worktree>\.claude\reports\dakara\<packet>-packet-coder.md`: files changed, each check's final `status=` line, the commit hash, every deviation and why. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. A context stop past about 90k tokens: commit only passing work (subject suffix `(part N)`), write `docs/analysis/dakara-e1-rebuild/worknotes/<packet>.md` and return `blocked` with the reason `context`.

## Shared contract for wave 3

**Row shape.** Exactly the C4 example of [work-packets.md](../work-packets.md#c4-chain-row-shape): one `content_chains` insert (`scope_type 'mission'`, `scope_id` the mission, `enabled true`, `priority 0`), one `content_triggers` insert (`scope 'player'`, `once false`, `sort_order 0`), multi-row conditions and actions with `sort_order` from 0 in the order written below. Every action has `delay_ms 0`. Descriptions read `'<mission> - <step or state>: <what>'`, with `'` doubled.

**Notation in the chain tables below.**

| Written | Row |
|---|---|
| `world 61` | `(id, 'world', 61, NULL, 'eq', NULL, n)` |
| `arch gte 7` / `arch lte 6` / `arch eq 7` | `(id, 'archetype', NULL, NULL, 'gte', '7', n)` and so on |
| `m1570 not_active` | `(id, 'mission_status', 1570, NULL, 'eq', 'not_active', n)`; likewise `active`, `completed` |
| `s1570/4906 active` | `(id, 'step_status', 1570, '4906', 'eq', 'active', n)` |
| `o1570/5936 not completed` | `(id, 'objective_status', 1570, '5936', 'neq', 'completed', n)` |
| `o1570/5630 completed` | `(id, 'objective_status', 1570, '5630', 'eq', 'completed', n)` |
| `ctr sg18_louis eq 0` / `ctr sg18_remains gte 2` | `(id, 'counter', NULL, 'sg18_louis', 'eq', '0', n)` / `(id, 'counter', NULL, 'sg18_remains', 'gte', '2', n)`; likewise `lt` |
| `display 6110` | `(id, 'display_dialog', 6110, NULL, '{}', 0, n)` |
| `advance 1570->4904` | `(id, 'advance_step', 1570, '4904', '{}', 0, n)` |
| `objective 1570/5936` | `(id, 'complete_objective', 1570, '5936', '{}', 0, n)` |
| `accept 1645` / `complete 1645` | `(id, 'accept_mission', 1645, NULL, '{}', 0, n)` / `'complete_mission'` |
| `bind 6890@441` / `unbind 6890@441` | `(id, 'add_dialog_set', 6890, NULL, '{"slot": 441}', 0, n)` / `'remove_dialog_set'` |
| `give 6779` / `take 6779` | `(id, 'add_item', 6779, NULL, '{"container": 0, "qty": 1}', 0, n)` / `(id, 'remove_item', 6779, NULL, '{"qty": 1}', 0, n)` |
| `inc sg18_remains` / `reset sg18_remains` | `(id, 'increment_counter', NULL, 'sg18_remains', '{"amount": 1}', 0, n)` / `(id, 'reset_counter', NULL, 'sg18_remains', '{}', 0, n)` |

Triggers: `loaded Dakara_E1` is `('player_loaded', 'Dakara_E1')`; `click <tag>` is `('interact_tag', '<tag>')`; `region <key>` is `('enter_region', '<key>')`, the key byte-exact as the `point_sets.name`; `choice 5980` is `('dialog_choice', '5980')`; `done 1570` is `('mission_completed', '1570')`. Every mission form takes any mission id, for example `m63100 not_active`, `s63100/63101 active`, `complete 63100`.

Counters (DK-10c, DK-12c) follow the Castle precedent (`castle_cellblock_chains.sql` chains 1085-1087) but add **no** `content_counters` row: only the admin editor reads that table, the engine does not. Counters live in memory and reset on relog and on every world change; a missing counter reads 0.

**How binds behave** (read this before writing a chain). A bind is an entry in the player's cell entity for the world they stand in; a relog or a world change drops every bind, so every bind has a `loaded <world>` restore chain gated on the same state. A bind for an NPC in the other world never needs removing. `add_dialog_set` pushes onto the slot's list, so a restore chain and the chain that first adds the bind must have disjoint conditions, or the NPC gets the dialog twice. `complete_objective` on a step's last required objective completes the whole mission: leave a step with `advance`, finish with `complete`, and put every `unbind` before `complete` (it fires `mission_completed` chains inline).

**Cast** (C6): Bra'tac template 59, tag `Dakara_E1_StoryRm_Bratac`, world 62. Moh'katan 54, `Dakara_E1_StoryRm_Mohkatan`, world 62. Rak'nor 441, `Dakara_E1_Raknor`, world 61. Loth'ta 440, `Dakara_E1_Lothta`, world 61. Remains 450-452, `Dakara_E1_SG18_Louis`, `_Nguyn`, `_Waters`, world 61. Drops 449, 458-461, `Dakara_E1_Drop1` to `Drop5`, world 61.

**Tests** (C7). Type: chain replay, live-DB (TESTING.md type 6). File: your submodule of `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/`. Imports as DK-10a's `entry.rs` (`use super::*;`, the `mission_1324` helpers you need, `build_engine`, `require_db_or_skip`, `assert_no_deferred_actions`). Each test: `#[tokio::test] async fn live_db_<what>()`, `let pool = require_db_or_skip!(); let engine = build_engine(Some(&pool)).await;`. A missing param reads as `not_active`, and `with_step` sets only the step key, so a player "at step S of mission M" is always `with_step(with_mission(ctx, M, "active"), M, S, "active")`; a later mission's state also sets the earlier missions `completed`. For **every chain**:

- the positive: the matching state gives exactly `labels_in(&r, <YOUR_RANGE>)` equal to the chain's actions in order, in the label format (`"8045:display_dialog(6110)"`, `"8045:complete_objective(1570/5936)"`, `"8045:remove_dialog_set(6890@441)"`, `"8082:add_item(6779x1)"`, `"8084:remove_item(6779x1)"`, `"8082:accept_mission(1645)"`, `"8051:advance_step(1570->4904)"`, `"8065:increment_counter(sg18_remains+1)"`, `"8066:reset_counter(sg18_louis)"`);
- one negative per condition that a sibling state could pass: the other archetype branch, the previous and next step, the wrong world, the mission already completed. Each gives an empty list for that chain;
- on every `click` that displays: `display_dialog_count(&r) == 1` (unfiltered, across the whole engine);
- `assert_no_deferred_actions(&r, <chain>)` on every chain that advances or completes.

Revert proof: every positive asserts a non-empty list, so removing the seed rows fails it. Run: `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1_missions`.

**Checks, in order, for every wave-3 packet** (from the worktree root):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine interact_tag_linter`
4. `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1_missions`

**Docs rows owed.** "Game systems, content chains, or content-engine actions": the `docs/content/mission-chains.md` rows are DK-99's, once for the campaign; say so in the report. Nothing else.

## DK-10b: 1570 steps 4906 and 4904

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-10b-1570-tent` / `dk-10b` |
| Commit subject | `feat(content): Dakara_E1 mission 1570 steps 4906 and 4904 (DK-10b)` |
| Seed file / range | `dakara_e1_m1570_tent_chains.sql`, 8045-8059 (`TENT_CHAINS`) |
| Test file | `dakara_e1_missions/tent.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-10b-packet-coder.md` |
| Size | About 40k tokens |

Step 4906 "Speak to Bra'tac at his command tent": optional 5936 (Rak'nor), optional 5629, required 5627 (Bra'tac). Step 4904 "Speak to Loth'ta": 5937 and 5928, both seeded required (F17); the step is left with `advance`, so neither is completed by hand. Jaffa (`arch gte 7`) and Tau'ri (`arch lte 6`) get different dialogs.

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8045 | click `Dakara_E1_Raknor` | world 61; arch gte 7; s1570/4906 active; o1570/5936 not completed | display 6110; objective 1570/5936; unbind 6890@441 | Rak'nor, Jaffa, optional 5936 |
| 8046 | click `Dakara_E1_Raknor` | world 61; arch lte 6; s1570/4906 active; o1570/5936 not completed | display 6111; objective 1570/5936; unbind 6889@441 | Rak'nor, Tau'ri |
| 8047 | loaded `Dakara_E1` | world 61; arch gte 7; s1570/4906 active; o1570/5936 not completed | bind 6890@441 | restore Rak'nor, Jaffa |
| 8048 | loaded `Dakara_E1` | world 61; arch lte 6; s1570/4906 active; o1570/5936 not completed | bind 6889@441 | restore Rak'nor, Tau'ri (a Tau'ri arrives from the SGC at 4906) |
| 8049 | loaded `Dakara_E1_StoryRm` | world 62; arch gte 7; s1570/4906 active | bind 6727@59 | Bra'tac, Jaffa |
| 8050 | loaded `Dakara_E1_StoryRm` | world 62; arch lte 6; s1570/4906 active | bind 6726@59 | Bra'tac, Tau'ri |
| 8051 | click `Dakara_E1_StoryRm_Bratac` | world 62; arch gte 7; s1570/4906 active | display 5811; advance 1570->4904; unbind 6727@59 | Bra'tac, Jaffa: leave 4906 |
| 8052 | click `Dakara_E1_StoryRm_Bratac` | world 62; arch lte 6; s1570/4906 active | display 5810; advance 1570->4904; unbind 6726@59 | Bra'tac, Tau'ri |
| 8053 | loaded `Dakara_E1` | world 61; arch gte 7; s1570/4904 active | bind 6729@440 | Loth'ta, Jaffa |
| 8054 | loaded `Dakara_E1` | world 61; arch lte 6; s1570/4904 active | bind 6728@440 | Loth'ta, Tau'ri |
| 8055 | click `Dakara_E1_Lothta` | world 61; arch gte 7; s1570/4904 active | display 5813; advance 1570->4902; unbind 6729@440 | Loth'ta, Jaffa: leave 4904 |
| 8056 | click `Dakara_E1_Lothta` | world 61; arch lte 6; s1570/4904 active | display 5812; advance 1570->4902; unbind 6728@440 | Loth'ta, Tau'ri |

Under D-DK1 F there is no 8057: DK-10fc's bridge chains bind Rak'nor before 1570 starts, and 8040 completes 5936 itself, so 8045 and 8047 stay quiet for a Free Jaffa. 8057 stays free.

**Fallback B only** (the brief says so): add `| 8057 | loaded Dakara_E1 | world 61; arch eq 7; m1570 not_active; m1645 not_active | bind 6890@441 | co-fires with DK-10a's 8040 (same trigger and conditions): Rak'nor's bind on the Free Jaffa's first arrival, when 4906 is not active yet |`.

Notes: Loth'ta's 4904 click leads into 4902. Step 4902 has no binds (D-DK18: its objectives are regions), so no chain co-fires on that click. Bra'tac and Loth'ta are in different worlds from the chain that moves the player to their step, so their binds come from 8049/8050 and 8053/8054 only.

**Tests** (`tent.rs`), besides the shared rules: a Free Jaffa first arrival (`ctx_for(FREE_JAFFA, Some(DAKARA))`, nothing set) gives an empty list in `TENT_CHAINS`; a player at 1570 step 4906 (mission and step both `active`, see the shared tests) with 5936 open gives `["8047:add_dialog_set(6890@441)"]`; with `with_objective(ctx, 1570, 5936, "completed")` (the Free Jaffa's state after 8040) the Rak'nor restore and click give nothing in `TENT_CHAINS`; a `PRAXIS_JAFFA` at 4906 gets the Jaffa chains; `TAURI` at 4906 on Dakara gets 8048. Fallback B: the first arrival gives `["8057:add_dialog_set(6890@441)"]` instead, and 8057 and 8047 never both fire.

## DK-10fc: the bridge mission 63100's chains

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Gate | D-DK1 answered **F**; DK-10a, DK-10fs and DK-10e merged, and the worktree based on a `main` that has all three. Not dispatched under any other answer. |
| Branch / worktree | `dakara/dk-10fc-bridge-chains` / `dk-10fc` |
| Commit subject | `feat(content): Dakara_E1 bridge mission 63100 chains (DK-10fc)` |
| Seed file / range | `dakara_e1_bridge_chains.sql`, 8075-8079 (`BRIDGE_CHAINS`); 8075-8077 used, 8078-8079 spare |
| Test file | `dakara_e1_missions/bridge.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-10fc-packet-coder.md` |
| Size | About 33k tokens |

Mission 63100 "Home to Dakara" (`NEW CONTENT`, level 1): one step 63101 "Speak with Rak'nor at the Stargate.", one required objective 63102, one task 63103. DK-10fs put it in the client's mission catalog and in `resources.missions` / `mission_steps` / `mission_objectives`. A Free Jaffa who arrives on Dakara with no Dakara mission yet gets it, Rak'nor is bound to his Jaffa dialog 6110 (dsm 6890, slot 441), and the click completes it. `complete_mission` fires `mission_completed` inline, so DK-10a's 8040 (`done 63100`) then accepts 1570 at 4906 in the same click. Free Jaffa only (`arch eq 7`): a Praxis Jaffa or a Tau'ri never sees 63100.

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8075 | loaded `Dakara_E1` | world 61; arch eq 7; m63100 not_active; m1570 not_active; m1645 not_active | accept 63100; bind 6890@441 | first arrival: the bridge mission and Rak'nor's bind (never again once any of the three is active or completed) |
| 8076 | loaded `Dakara_E1` | world 61; arch eq 7; s63100/63101 active | bind 6890@441 | restore Rak'nor after a relog or a tent trip; disjoint from 8075 (63100 active vs not active) |
| 8077 | click `Dakara_E1_Raknor` | world 61; arch eq 7; s63100/63101 active | display 6110; unbind 6890@441; complete 63100 | the conversation; unbind before complete (C5 rule 5) |

Each chain: `scope_type 'mission'`, `scope_id 63100`; descriptions `'63100 - first arrival: accept Home to Dakara, bind Rak''nor'`, `'63100 - 63101: restore Rak''nor'`, `'63100 - 63101: Rak''nor 6110, complete'`; a comment above each naming `NEW CONTENT` (DK-10fc, D-DK1 F) and what it serves. 8077 shows the dialog itself, so the click never falls back to the dsm dialog (one display). 8045 (DK-10b) needs 1570 at 4906, which a Free Jaffa only reaches after 8077, so the two Rak'nor clicks never both fire.

**Tests** (`bridge.rs`), besides the shared rules. "With 63100 at 63101" is `with_step(with_mission(ctx, 63100, "active"), 63100, 63101, "active")`. Imports as `entry.rs`, plus `fire_interact_tag`, `fire_mission_completed`, `with_step`, `display_dialog_count`.

1. `live_db_free_jaffa_first_arrival_accepts_home_to_dakara`: `fire_player_loaded(&engine, &ctx_for(FREE_JAFFA, Some(DAKARA)), DAKARA_NAME)` gives `["8075:accept_mission(63100)", "8075:add_dialog_set(6890@441)"]` in `BRIDGE_CHAINS`; `assert_no_deferred_actions(&r, 8075)`.
2. `live_db_home_to_dakara_restores_raknor`: the same load with 63100 at 63101 gives `["8076:add_dialog_set(6890@441)"]` only (8075 and 8076 never both fire).
3. `live_db_raknor_completes_home_to_dakara`: `fire_interact_tag(&engine, &ctx, "Dakara_E1_Raknor")` with 63100 at 63101 gives `["8077:display_dialog(6110)", "8077:remove_dialog_set(6890@441)", "8077:complete_mission(63100)"]`; `display_dialog_count(&r) == 1`; `assert_no_deferred_actions(&r, 8077)`.
4. `live_db_home_to_dakara_hands_over_to_1570`: `fire_mission_completed(&engine, &with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 63100, "completed"), 63100)` gives DK-10a's four 8040 labels in `ENTRY_CHAINS` and nothing in `BRIDGE_CHAINS`.
5. `live_db_home_to_dakara_is_free_jaffa_only_and_once`: each of these gives an empty `BRIDGE_CHAINS` list on the load and on the Rak'nor click: `PRAXIS_JAFFA` and `TAURI` on Dakara; a Free Jaffa in `Some(STORY_RM)` firing `STORY_RM_NAME`; 63100 `completed`; 1570 `active`; 1570 `completed`; 1645 `active`. A Free Jaffa with nothing set clicking Rak'nor also gives an empty list.
6. `live_db_free_jaffa_first_arrival_resolves_exactly_the_space_and_bridge_chains`: the whole-seed guard, unfiltered by range. Add a private helper `fn sorted_ids(r: &ResolvedActions) -> Vec<i64>` (every `r.actions` chain id, sorted; import `ResolvedActions` from `cimmeria_content_engine::chain` if `use super::*;` does not bring it). `fire_player_loaded(&engine, &ctx_for(FREE_JAFFA, Some(DAKARA)), DAKARA_NAME)` gives `vec![8001, 8002, 8007, 8075, 8075]`. The same call with `ctx_for(FREE_JAFFA, None)` (no world id, the context `dakara_e1_space.rs`'s `arrive()` builds) gives `vec![8001, 8002, 8007]`. Its doc comment says, in two sentences, that this is the deliberate addition of 8075 to a Free Jaffa's arrival, and that the second list is why `dakara_e1_space`'s resolve tests do not see it. If the run shows another chain id in either list, do not adjust the list: paste it into the report and return `blocked` with the reason `arrival-set`.

**Ruling on the `space-test` block (architect, 2026-10-18).** The first DK-10fc run blocked before writing anything, saying 8075 breaks three `dakara_e1_space.rs` tests. It does not, and neither option the orchestrator offered applies: `dakara_e1_space.rs` is **not edited** and 8075 gets **no extra condition**. DK-10fc's files stay its seed file and `bridge.rs`. The reasons:

- **The two resolve tests** (`..._resolves_exactly_the_grant_...` and `..._fire_for_no_other_archetype_and_on_no_other_world`) build their context with `arrive()`. It sets the `world_name` param, which matches the trigger key `Dakara_E1`, but never the typed `ExecutionContext::world_id`. 8075's `world 61` condition reads only `world_id` and fails closed on `None` (`crates/content-engine/src/conditions/mod.rs`, `Condition::World`). 8001, 8002 and 8007 have no `world` condition, so they still resolve and the exact lists hold.
- **The two executed tests** (`..._grants_omega_site_once_and_requests_the_notice`, renamed by DK-10e to name the mails, and `..._sends_a_visitor_nothing`) run the real `fire_player_loaded` on a bare `SpaceManager::new(1)` that has only the stargate cache: no world ids stamped from `resources.worlds`, no `mission_defs`, no `dialog_set_maps`. So 8075 either fails its `world` condition or, if it resolves, its `accept_mission` only logs the "No mission_defs entry" WARN and its `add_dialog_set` finds no map. Neither sends a `CellToBaseMsg`, so the exact message destructure and the relog's mails-only assertion hold.
- The bridge is guarded in `bridge.rs`, with a world id (tests 1-5), and test 6 pins the whole arrival set on purpose, as `dakara_e1_space.rs`'s module doc asks of any new arrival chain.

**Before re-dispatch (orchestrator, through ops).** The `dk-10fc` worktree has no commit and sits on `3840e55ae`, before DK-10e (`a97b61e25`, #1876). Move it to current `main` first, so check 5 runs DK-10e's three-mail tests and test 6 sees chain 8007.

**Checks:** the four shared checks, then 5. `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1_space`. Expected: all four tests pass unchanged; copy the final `status=` line and the pass count into the report. If check 5 fails, do **not** edit `dakara_e1_space.rs`, weaken a test or add a condition to 8075: paste the failing test's output into the report and return `blocked` with the reason `space-test`.

**Revert proof.** Tests 1-3 assert non-empty lists, so removing the seed rows fails them; test 5 fails if the archetype, world or mission gates are dropped. Test 6's first list fails if 8075 is removed or another chain starts firing on a Free Jaffa's first arrival; its second list fails if 8075 loses its `world 61` condition, which is the change that would also break the `dakara_e1_space` tests.

**Docs rows owed.** The shared rule (DK-99 owns `mission-chains.md`). The UAT rows are DK-U11 to DK-U16 in the README already; name them in the report.

## DK-10c: 1570 steps 4902 and 4901

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-10c-1570-search` / `dk-10c` |
| Commit subject | `feat(content): Dakara_E1 mission 1570 steps 4902 and 4901 (DK-10c)` |
| Seed file / range | `dakara_e1_m1570_search_chains.sql`, 8060-8074 (`SEARCH_CHAINS`); 8060-8071 used, 8072-8074 spare |
| Test file | `dakara_e1_missions/search.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-10c-packet-coder.md` |
| Size | About 40k tokens |

Step 4902 "Search the locations Loth'ta indicated": 5632 required, 5631 optional, 5630 required. Step 4901 "Collect the dog tags": 5930 required and hidden, 5929 required; remains 450-452 with dsm 7153, 7154, 7155 (dialogs 6224, 6225, 6226); dog tags item 6778 (stacks to 3, so the three pickups fit one stack). After 4901 comes 4905 (DK-10d).

**The ruling** (DK-08 worknote, D-DK18, D-DK19). Each 4902 objective is a client-hinted region DK-09 seeds around one remains site: `Dakara_E1.SG18Louis` (5632), `Dakara_E1.SG18Nguyn` (5630) and `Dakara_E1.SG18Waters` (5631, optional), each radius 8 m. Walking into the second required region shows 5814 ("weak radio signal", speaker 0) and moves the player to 4901, which binds the three remains on the spot. Step 4902 has no binds and dsm 6730 is not used. The two required objectives use a sibling pair: "not last" completes the objective and "last" advances, kept disjoint by the other objective's status. Step 4901 uses the counter `sg18_remains` and one visited marker per remains (`sg18_louis`, `sg18_nguyn`, `sg18_waters`); without the marker, three clicks on one remains would advance the step. Every remains chain shows its own dialog, because a chain that handles a click stops the dsm fallback dialog. The "last" chains use `gte`, so a drifted counter cannot strand the step. No chain needs an archetype condition (every dialog here is speaker 0, one variant).

Every chain's conditions start with `world 61` and the step: `s1570/4902 active` for 8060-8064, `s1570/4901 active` for 8065-8071. The table lists the further conditions in the order they follow.

| Chain | Trigger | Further conditions | Actions | Why |
|---|---|---|---|---|
| 8060 | region `Dakara_E1.SG18Louis` | o1570/5632 not completed; o1570/5630 not completed | objective 1570/5632 | Louis site first |
| 8061 | region `Dakara_E1.SG18Louis` | o1570/5632 not completed; o1570/5630 completed | display 5814; advance 1570->4901; bind 7153@450; bind 7154@451; bind 7155@452 | Louis site last: leave 4902 |
| 8062 | region `Dakara_E1.SG18Nguyn` | o1570/5630 not completed; o1570/5632 not completed | objective 1570/5630 | Nguyn site first |
| 8063 | region `Dakara_E1.SG18Nguyn` | o1570/5630 not completed; o1570/5632 completed | display 5814; advance 1570->4901; bind 7153@450; bind 7154@451; bind 7155@452 | Nguyn site last: leave 4902 |
| 8064 | region `Dakara_E1.SG18Waters` | o1570/5631 not completed | objective 1570/5631 | optional site; `advance_step` checks it off if skipped |
| 8065 | click `Dakara_E1_SG18_Louis` | ctr sg18_louis eq 0; ctr sg18_remains lt 2 | display 6224; give 6778; objective 1570/5930; inc sg18_remains; inc sg18_louis; unbind 7153@450 | first or second pickup |
| 8066 | click `Dakara_E1_SG18_Louis` | ctr sg18_louis eq 0; ctr sg18_remains gte 2 | display 6224; give 6778; advance 1570->4905; reset sg18_remains; reset sg18_louis; reset sg18_nguyn; reset sg18_waters; unbind 7153@450 | third pickup: leave 4901 |
| 8067 | click `Dakara_E1_SG18_Nguyn` | ctr sg18_nguyn eq 0; ctr sg18_remains lt 2 | display 6225; give 6778; objective 1570/5930; inc sg18_remains; inc sg18_nguyn; unbind 7154@451 | as 8065 |
| 8068 | click `Dakara_E1_SG18_Nguyn` | ctr sg18_nguyn eq 0; ctr sg18_remains gte 2 | display 6225; give 6778; advance 1570->4905; reset sg18_remains; reset sg18_louis; reset sg18_nguyn; reset sg18_waters; unbind 7154@451 | as 8066 |
| 8069 | click `Dakara_E1_SG18_Waters` | ctr sg18_waters eq 0; ctr sg18_remains lt 2 | display 6226; give 6778; objective 1570/5930; inc sg18_remains; inc sg18_waters; unbind 7155@452 | as 8065 |
| 8070 | click `Dakara_E1_SG18_Waters` | ctr sg18_waters eq 0; ctr sg18_remains gte 2 | display 6226; give 6778; advance 1570->4905; reset sg18_remains; reset sg18_louis; reset sg18_nguyn; reset sg18_waters; unbind 7155@452 | as 8066 |
| 8071 | loaded `Dakara_E1` | none | bind 7153@450; bind 7154@451; bind 7155@452 | restore the remains after a relog or a tent trip (the counters reset too, so all three re-arm) |

Notes: 5930 is completed again on the second pickup; `complete_objective` is idempotent, so that is harmless. The third pickup never completes 5929: `advance_step` does. 8061/8063 add the remains binds in the same world, so no co-firing chain is needed; 8071 has no other trigger, so it never doubles a bind. Use the chain description shape `'1570 - 4902: Louis site, last required search region'` and a comment above each chain with its evidence label and the objective or bind it serves.

**Tests** (`search.rs`), besides the shared rules. Use `fire_enter_region(&engine, &ctx, "Dakara_E1.SG18Louis")` (from `super`) for the regions, `fire_interact_tag` for the remains and `with_counter(ctx, "sg18_remains", 2)` (from `super`) for counter states. "At 4902" is `with_step(with_mission(ctx, 1570, "active"), 1570, 4902, "active")`.

1. 8060 fires with 5632 and 5630 unset; nothing fires with 5632 completed; with 5630 completed only 8061 fires. 8061: the five labels in order, `display_dialog_count(&r) == 1`, `assert_no_deferred_actions(&r, 8061)`. Mirror both for 8062/8063.
2. 8064 fires with 5631 unset whatever the pair's state; nothing with 5631 completed.
3. Every region chain resolves nothing in world 62 (`ctx_for(.., Some(STORY_RM))`), at step 4904, and at step 4901.
4. Relog at 4902: with 5630 completed, a fresh context (no counters) entering the Louis region still gives only 8061 (completed objectives survive a relog; the DK-08 worknote, "Facts").
5. 8065/8067/8069 fire with their own marker 0 and `sg18_remains` 0, and again with 1; each with `display_dialog_count == 1`. None fires with `sg18_remains` 2 or with its own marker 1.
6. 8066/8068/8070 fire with their own marker 0 and `sg18_remains` 2; not with 1. Revert proof for the marker: own marker 1 with `sg18_remains` 2 resolves nothing in `SEARCH_CHAINS`.
7. 8071 fires on `fire_player_loaded(.., DAKARA_NAME)` at 4901 and not at 4905 or 4902.
8. Relog at 4901: a fresh context (all counters 0) gives 8071's three binds, and the first click on any remains gives the 8065-shape chain, not the advancing one.

Dog tags stack to 3 (`max_stack_size` 3, container set 2), so no stacking check is needed; say so in the report.

## DK-10d: 1570 steps 4905 and 4903 (split into DK-10d-B and DK-10d-A)

Step 4905 "Use your radio to contact Gen. Hammond": 5934 and 5933, both required; dialogs 5815 (Tau'ri) and 5816 (Jaffa), dsm 6731 / 6732, no NPC. Step 4903 "Speak with Bra'tac to arrange transport": 5935; dsm 6733 (dialog 5817, turn-in). It is the mission's last step.

**Split (architect, 2026-10-18).** DK-10d is two packets that share the seed file `dakara_e1_m1570_report_chains.sql` (`REPORT_CHAINS` 8160-8179; D-DK19 moved it from 8070-8079) and the test file `dakara_e1_missions/report.rs`. **DK-10d-B** (step 4903, Bra'tac's turn-in) needs no radio and is dispatchable now. **DK-10d-A** (step 4905, the radio) needs DK-06b's verb, so it is dispatched only after DK-06b and DK-10d-B have both merged, and it appends to DK-10d-B's files. Until DK-10d-A merges, 1570 stops at step 4905 in play, so 1645 (DK-11 fires on `done 1570`) and 1646 cannot be reached by playing; their chain-replay tests do not depend on it.

### DK-10d-B: 1570 step 4903, Bra'tac's turn-in

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-10d-b-1570-turn-in` / `dk-10d-b` |
| Commit subject | `feat(content): Dakara_E1 mission 1570 Bra'tac turn-in, step 4903 (DK-10d-B)` |
| Seed file / chains | `dakara_e1_m1570_report_chains.sql`, chains 8164-8165 only (8160-8163 and 8166-8179 are DK-10d-A's) |
| Test file | `dakara_e1_missions/report.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-10d-B-packet-coder.md` |
| Size | About 15k tokens |
| Dispatch gate | DK-10a merged (met, #1871). Dispatch now; reviewed with DK-06b as wave 3b. |
| Merge dependency | None. DK-10d-A needs this packet merged. |

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8164 | loaded `Dakara_E1_StoryRm` | world 62; s1570/4903 active | bind 6733@59 | Bra'tac's turn-in; one dialog for both archetypes |
| 8165 | click `Dakara_E1_StoryRm_Bratac` | world 62; s1570/4903 active | display 5817; unbind 6733@59; complete 1570 | turn-in: unbind before complete |

The player reaches 4903 out on Dakara (the radio), then walks into the tent, so 8164 binds Bra'tac on that entry. A player who is already in the tent when 4903 begins gets the bind from DK-10d-A's chains 8170/8171, not from 8164.

Write the two chains into the existing file after its `SET search_path` line, each with its evidence comment (`PROJECT_FINAL_RECONSTRUCTION`, step 4903).

**Tests** (`report.rs`, below its `//!` line; the shared rules above apply):

1. 8164: `fire_player_loaded(.., STORY_RM_NAME)` at 1570 step 4903 in world 62 gives exactly `["8164:add_dialog_set(6733@59)"]`, for `FREE_JAFFA` and for `TAURI`. At step 4905: no `8164:` label (filter `8164..=8164`, not all of `REPORT_CHAINS`: DK-10d-A's 8170/8171 fire on this load at 4905 by design, so a `REPORT_CHAINS` negative there breaks the day DK-10d-A merges; wave-3b review finding 1, the shipped test does this). Nothing in `REPORT_CHAINS` at step 4906, with 1570 `completed`, or in world 61.
2. 8165: a Bra'tac click at 4903 in world 62 gives exactly `["8165:display_dialog(5817)", "8165:remove_dialog_set(6733@59)", "8165:complete_mission(1570)"]`, `display_dialog_count(&r) == 1` and `assert_no_deferred_actions(&r, 8165)`. Nothing in `REPORT_CHAINS` at 4906 (DK-10b's 8051/8052 own that state), at 4905, or in world 61.

Leave the `#[allow(dead_code)]` on `STORY_RM_NAME`: `mod.rs` is not yours (rule 4).

**Checks:** the four shared checks. Name `docs/content/mission-chains.md` as DK-99's in the report.

### DK-10d-A: 1570 step 4905, the radio report

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-10d-a-1570-radio` / `dk-10d-a` |
| Commit subject | `feat(content): Dakara_E1 mission 1570 radio report, step 4905 (DK-10d-A)` |
| Seed file / chains | `dakara_e1_m1570_report_chains.sql`, chains 8160-8163 and 8166-8171; 8172-8179 stay spare |
| Test file | `dakara_e1_missions/report.rs` (append below DK-10d-B's tests; do not change them) |
| Report | `<worktree>\.claude\reports\dakara\DK-10d-A-packet-coder.md` |
| Size | About 34k tokens (amended 2026-10-19 with tests 8 and 9 from the wave-3b review) |
| Dispatch gate | DK-06b merged (the verb, its loader arm and the `labels_in` arm) **and** DK-10d-B merged (the shared files). Create the worktree from `main` after both merges. DK-10c need not be merged: its file and range are disjoint. Wave-3b review (2026-10-18): both verdict `ship`, merge DK-06b first, then DK-10d-B (disjoint files, clean `merge-tree`). |
| Merge dependency | None after the gate. M2 (the owner's playtest) needs DK-10c and DK-10d-A both merged. |

**The ruling this packet carries (D-DK10, option B).** No NPC speaks the radio call, so the chains use DK-06b's verb `display_radio_dialog`, which sends `onDialogDisplay` with `EntityId` 0: the window opens with Hammond's text and no portrait. Two notation rows join the shared table:

| Written | Row |
|---|---|
| `radio 5816` | `(id, 'display_radio_dialog', 5816, NULL, '{}', 0, n)` |
| `radio 5816 after 2000 ms` | `(id, 'display_radio_dialog', 5816, NULL, '{}', 2000, n)` |

The rows show 5816 for `arch gte 7` (Jaffa) and 5815 for `arch lte 6` (Tau'ri), and leave 4905 with `advance 1570->4903`, never with `objective` on 5934 or 5933.

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8160 | choice 6224 | arch gte 7; s1570/4905 active | radio 5816; advance 1570->4903 | Louis's dog-tag dialog closed after the third pickup |
| 8161 | choice 6224 | arch lte 6; s1570/4905 active | radio 5815; advance 1570->4903 | as 8160, Tau'ri |
| 8162 | choice 6225 | arch gte 7; s1570/4905 active | radio 5816; advance 1570->4903 | Nguyen's |
| 8163 | choice 6225 | arch lte 6; s1570/4905 active | radio 5815; advance 1570->4903 | as 8162, Tau'ri |
| 8166 | choice 6226 | arch gte 7; s1570/4905 active | radio 5816; advance 1570->4903 | Waters's |
| 8167 | choice 6226 | arch lte 6; s1570/4905 active | radio 5815; advance 1570->4903 | as 8166, Tau'ri |
| 8168 | loaded `Dakara_E1` | world 61; arch gte 7; s1570/4905 active | radio 5816 after 2000 ms; advance 1570->4903 | restore after a relog or world change at 4905 |
| 8169 | loaded `Dakara_E1` | world 61; arch lte 6; s1570/4905 active | radio 5815 after 2000 ms; advance 1570->4903 | as 8168, Tau'ri |
| 8170 | loaded `Dakara_E1_StoryRm` | world 62; arch gte 7; s1570/4905 active | radio 5816 after 2000 ms; advance 1570->4903; bind 6733@59 | restore in the tent; binds Bra'tac too |
| 8171 | loaded `Dakara_E1_StoryRm` | world 62; arch lte 6; s1570/4905 active | radio 5815 after 2000 ms; advance 1570->4903; bind 6733@59 | as 8170, Tau'ri |

Ids 8164-8165 are DK-10d-B's, so this packet's ids skip them. Append the ten chains after DK-10d-B's, each with its evidence comment (`PROJECT_FINAL_RECONSTRUCTION`, step 4905, and "radio: D-DK10 option B").

**Why these triggers** (put the short form in the chain comments):

- DK-10c's chains 8066, 8068 and 8070 show the dog-tag dialog (6224, 6225 or 6226) and advance to 4905 in the same chain. Those dialogs have no buttons, so closing one fires `dialog_choice` with button -1, and on the third pickup 4905 is already active. A close after the first or second pickup happens at step 4901 and matches nothing.
- Progress never waits on the radio dialog's own close: the advance runs with the radio, so the step works even if the client never reports a close for an `EntityId` 0 dialog.
- No world condition on 8160-8167: the player may close the dog-tag dialog after walking into the tent.
- 8168-8171 cover a relog or world change at 4905 before the close (the #479 offer gate forgets offers across a world change). The 2000 ms delay lets the client finish loading before the window opens; it is the one deliberate exception to "every action has `delay_ms 0`" in this wave (precedent: Castle chain 1172). The advance has no delay, so the step gate closes at once and the chain cannot fire twice.
- 8170/8171 bind Bra'tac themselves: conditions are read before any action runs, so 8164 (`s1570/4903`) does not fire on the same load, and 8164 and 8170/8171 never both fire.
- If dog-tag dialogs from earlier pickups are closed late, a second radio window may open; it is harmless and the step has already moved.

**Tests** (`report.rs`, appended). Add two local helpers: `radio_count(&r)`, the number of `Action::DisplayRadioDialog` across the whole engine (as `display_dialog_count` does for `DisplayDialog`), and `delays_of(&r, chain) -> Vec<i32>`, the `action_delays` of that chain's actions in order. The labels read `"8160:display_radio_dialog(5816)"` (DK-06b's `labels_in` arm).

1. For each of `fire_dialog_choice(.., 6224)`, `6225` and `6226` at 1570 step 4905 in world 61: `FREE_JAFFA` and `PRAXIS_JAFFA` give exactly `["<chain>:display_radio_dialog(5816)", "<chain>:advance_step(1570->4903)"]`; `TAURI` gives the 5815 pair from the odd chain. Each with `radio_count == 1`, `display_dialog_count == 0` and `assert_no_deferred_actions(&r, <chain>)`.
2. The same choice at 4905 in world 62 also fires (8160 for 6224 with `FREE_JAFFA`).
3. `fire_dialog_choice(.., 6224)` at step 4901, at step 4903, and with 1570 `completed`: nothing in `REPORT_CHAINS`.
4. `fire_dialog_choice(.., 5816)` and `(.., 5815)` at 4903: nothing in `REPORT_CHAINS` (the radio's own close does nothing).
5. 8168/8169: `fire_player_loaded(.., DAKARA_NAME)` at 4905 in world 61 gives the radio and advance labels, `delays_of == [2000, 0]`, `radio_count == 1`. Nothing in `REPORT_CHAINS` at 4901 or 4903.
6. 8170/8171: `fire_player_loaded(.., STORY_RM_NAME)` at 4905 in world 62 gives `[radio, advance, "<chain>:add_dialog_set(6733@59)"]` and `delays_of == [2000, 0, 0]`, with no `8164:` label. At 4903 the same load gives only `["8164:add_dialog_set(6733@59)"]`.
7. DK-10d-B's tests stay unchanged and pass.
8. **Seed guard (type 3, live-DB; replaces the old `dialog_button_linter` check).** `live_db_report_radio_rows_and_dialog_shape`, with `sqlx::query_as` / `query_scalar` on `pool` as `dakara_e1_space.rs` does: (a) `SELECT chain_id, target_id FROM resources.content_actions WHERE chain_id BETWEEN 8160 AND 8179 AND action_type::text = 'display_radio_dialog' ORDER BY chain_id` returns exactly the ten rows `(8160, 5816), (8161, 5815), (8162, 5816), (8163, 5815), (8166, 5816), (8167, 5815), (8168, 5816), (8169, 5815), (8170, 5816), (8171, 5815)`; (b) `SELECT count(*) FROM resources.dialog_screen_buttons b JOIN resources.dialog_screens s ON s.screen_id = b.screen_id WHERE s.dialog_id IN (5815, 5816, 6224, 6225, 6226)` is 0 (the radio and dog-tag dialogs have no buttons, so their only response is the close, button -1, which is what 8160-8167 and test 4 rely on; checked 2026-10-19: none has a button); (c) dialog 5815 has 7 screens and 5816 has 10 (`SELECT dialog_id, count(*) ... GROUP BY dialog_id`). Revert proof: (a) fails when this packet's rows are removed. Why not the linter: `dialog_button_linter` scans only five Castle and debug-hub chain files (`crates/content-engine/tests/it/dialog_button_linter/seed_model.rs`, `CHAIN_FILES`) and only `display_dialog` actions, so it can never fail on these chains (wave-3b review, DK-06b finding 2). Do not edit the linter.
9. **8165 for a Tau'ri (wave-3b review, DK-10d-B finding 2).** A new test, `live_db_report_turn_in_tauri`: the 8165 positive of DK-10d-B's test 2 for `TAURI` (same three labels, `display_dialog_count == 1`, `assert_no_deferred_actions(&r, 8165)`). DK-10d-B tested only `FREE_JAFFA`; the chain has no archetype row, and this catches one being added. Do not edit DK-10d-B's test.

`assert_no_deferred_actions` fails by design on 8168-8171; use `delays_of` there instead. DK-06b ships `Action::DisplayRadioDialog`, the `display_radio_dialog(<id>)` label and `action_delays` (parallel to `actions`), which `radio_count` and `delays_of` read.

**Checks:** the four shared checks only. There is no fifth check: `dialog_button_linter` does not scan the Dakara files (test 8 replaces it). Name `docs/content/mission-chains.md` as DK-99's in the report; the radio row of the UAT checklist (DK-U17) is in the README already. Size with tests 8 and 9: about 34k tokens.

## DK-11: mission 1645, Loth'ta's withdrawal orders

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-11-m1645` / `dk-11` |
| Commit subject | `feat(content): Dakara_E1 mission 1645 (DK-11)` |
| Seed file / range | `dakara_e1_m1645_chains.sql`, 8080-8119 (`M1645_CHAINS`) |
| Test file | `dakara_e1_missions/m1645.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-11-packet-coder.md` |
| Size | About 30k tokens |

Moh'katan offers 1645 in the tent (dsm 6736, dialog 5980, "available"). Accepting lands on step 5168 "Take Loth'ta's orders from Moh'katan" (objective 5938); the accept scene 5807 (Moh'katan and Bra'tac) is that step, so the accept chain moves straight on to 4907 "Deliver the withdrawal orders to Loth'ta" (5634 optional, 5633 required) and hands over item 6779. Loth'ta takes the turn-in (dsm 6735, dialog 5808). The offer has **no** `click` chain on Moh'katan: a click chain would skip `handle_interact`, the only place that pins Moh'katan as the speaker for the accept chain's `display 5807` (the Harset 1324 lesson, `chain_replay_tests/mission_1324.rs` lines 17-28).

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8080 | done `1570` | world 62; m1645 not_active | bind 6736@54 | 1570's turn-in is in the tent beside Moh'katan, so the offer appears at once |
| 8081 | loaded `Dakara_E1_StoryRm` | world 62; m1570 completed; m1645 not_active | bind 6736@54 | restore the offer |
| 8082 | choice `5980` | world 62; m1570 completed; m1645 not_active | accept 1645; display 5807; give 6779; advance 1645->4907; unbind 6736@54 | accept, the orders scene, leave 5168 |
| 8083 | loaded `Dakara_E1` | world 61; s1645/4907 active | bind 6735@440 | Loth'ta's turn-in |
| 8084 | click `Dakara_E1_Lothta` | world 61; s1645/4907 active | display 5808; take 6779; unbind 6735@440; complete 1645 | turn-in: unbind before complete |

**Under D-DK1 C** (the architect confirms before dispatch): 8080 is dropped; 8081 and 8082 replace `m1570 completed` with `arch eq 7`. Under F or B the table stands as written: 1645 still follows 1570.

**Tests** (`m1645.rs`): the shared rules; `fire_mission_completed(&engine, &with_mission(ctx, 1570, "completed"), 1570)` in world 62 gives `["8080:add_dialog_set(6736@54)"]`; with 1645 `active` it gives nothing; `fire_interact_tag` on `Dakara_E1_StoryRm_Mohkatan` with 1570 completed and 1645 not active resolves **no chain at all** in the whole engine (`r.actions.is_empty()`); `fire_dialog_choice(&engine, &ctx, 5980)` gives the five 8082 labels in order; the Loth'ta click at 1645/4907 gives `display_dialog_count == 1`.

## DK-12b: mission 1646 offer, accept and turn-in

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-12b-m1646-offer` / `dk-12b` |
| Commit subject | `feat(content): Dakara_E1 mission 1646 offer and turn-in (DK-12b)` |
| Seed file / range | `dakara_e1_m1646_offer_chains.sql`, 8120-8139 (`M1646_OFFER_CHAINS`) |
| Test file | `dakara_e1_missions/m1646_offer.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-12b-packet-coder.md` |
| Size | About 25k tokens |

Moh'katan offers 1646 (dsm 6737, dialog 5981, "available") once 1645 is completed. 1645 ends out at Loth'ta, so the offer appears when the player next enters the tent (8120). Accepting lands on step 4908 (the five drops, DK-12c) and hands over the coded orders, item 6780. Item 6780 is **not** taken back: mission 1649 decodes it later (DK-14b). Step 4909 "Report back to Moh'katan" (5943) is the turn-in, dsm 6744 (dialog 5820). The offer has no `click` chain on Moh'katan (see DK-11).

| Chain | Trigger | Conditions | Actions | Why |
|---|---|---|---|---|
| 8120 | loaded `Dakara_E1_StoryRm` | world 62; m1645 completed; m1646 not_active | bind 6737@54 | the offer, and its restore |
| 8121 | choice `5981` | world 62; m1645 completed; m1646 not_active | accept 1646; display 5819; give 6780; unbind 6737@54 | accept and the briefing |
| 8122 | loaded `Dakara_E1_StoryRm` | world 62; s1646/4909 active | bind 6744@54 | the turn-in, on entering the tent after the last drop |
| 8123 | click `Dakara_E1_StoryRm_Mohkatan` | world 62; s1646/4909 active | display 5820; unbind 6744@54; complete 1646 | turn-in: unbind before complete |

**Tests** (`m1646_offer.rs`): the shared rules; with 1645 completed and 1646 not active, a Moh'katan click resolves no chain in the whole engine; `fire_dialog_choice(..., 5981)` gives the four 8121 labels in order; with 1645 `active` the offer restore gives nothing; the Moh'katan click at 1646/4909 gives `display_dialog_count == 1`.

## DK-12c: mission 1646 step 4908, the five drops

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Branch / worktree | `dakara/dk-12c-m1646-drops` / `dk-12c` |
| Commit subject | `feat(content): Dakara_E1 mission 1646 drop locations (DK-12c)` |
| Seed file / range | `dakara_e1_m1646_drop_chains.sql`, 8140-8159 (`M1646_DROP_CHAINS`) |
| Test file | `dakara_e1_missions/m1646_drops.rs` |
| Report | `<worktree>\.claude\reports\dakara\DK-12c-packet-coder.md` |
| Size | About 40k tokens |

Step 4908 "Hide the coded orders at the drop sites": objectives 5635, 5939, 5940, 5941, 5942 ("Drop Location #1" to "#5"), all required, any order. Drop *n* is template 449, 458, 459, 460, 461 with dsm 6739 to 6743 (dialog 5821, "You hide the coded orders", speaker 0), tags `Dakara_E1_Drop1` to `Dakara_E1_Drop5`, spawns 8011-8015 (DK-09). The player accepts 1646 in the tent (world 62) and the drops stand in world 61, so the drop binds come only from the `loaded Dakara_E1` restores (C5 rule 6; no co-firing chain). The step is left with `advance 1646->4909`; DK-12b binds Moh'katan for 4909.

**The ruling** (DK-08 worknote). Five required objectives cannot be kept disjoint with AND-only conditions, so a counter `drops_4908` counts the drops done; each drop guards itself with its own objective status. The counter resets on relog and on every world change (a tent trip too), so a rebuild chain per completed drop adds it back on entering world 61. The advancing chain never completes the fifth objective: `advance_step` does. No archetype condition (5821 is speaker 0).

| Drop k | Tag | Objective O | Template / slot T | dsm D |
|---|---|---|---|---|
| 1 | `Dakara_E1_Drop1` | 5635 | 449 | 6739 |
| 2 | `Dakara_E1_Drop2` | 5939 | 458 | 6740 |
| 3 | `Dakara_E1_Drop3` | 5940 | 459 | 6741 |
| 4 | `Dakara_E1_Drop4` | 5941 | 460 | 6742 |
| 5 | `Dakara_E1_Drop5` | 5942 | 461 | 6743 |

Four chains per drop, 20 in all. Every chain's conditions start with `world 61; s1646/4908 active`; the table lists the further conditions.

| Chain | Trigger | Further conditions | Actions | Why |
|---|---|---|---|---|
| 8140 + 2(k-1) | click `Dakara_E1_Drop<k>` | o1646/O not completed; ctr drops_4908 lt 4 | display 5821; objective 1646/O; inc drops_4908; unbind D@T | drops 1-4 in any order |
| 8141 + 2(k-1) | click `Dakara_E1_Drop<k>` | o1646/O not completed; ctr drops_4908 gte 4 | display 5821; advance 1646->4909; reset drops_4908; unbind D@T | the fifth drop: leave 4908 |
| 8150 + (k-1) | loaded `Dakara_E1` | o1646/O not completed | bind D@T | restore the drops not yet done |
| 8155 + (k-1) | loaded `Dakara_E1` | o1646/O completed | inc drops_4908 | rebuild the counter |

So drop 1 is 8140, 8141, 8150, 8155; drop 5 is 8148, 8149, 8154, 8159. Write all 20 chains out in full in the seed file, in that drop order (k = 1 first), each with its own comment. Description shape: `'1646 - 4908: drop 3, not last'`.

**Tests** (`m1646_drops.rs`), besides the shared rules. "At 4908" is `with_step(with_mission(ctx, 1646, "active"), 1646, 4908, "active")` with 1570 and 1645 `completed`; use `with_counter` and `with_objective` from `super`.

1. For each k: the count chain fires with O unset and `drops_4908` 0 (and with 3), with `display_dialog_count == 1`; nothing fires with O completed.
2. For each k: the last chain fires with O unset and `drops_4908` 4 (`assert_no_deferred_actions`), and not with 3; nothing fires with O completed and 4.
3. First entry after the accept (no objective completed): `fire_player_loaded(.., DAKARA_NAME)` gives the five binds 8150-8154 and no rebuild.
4. Relog with drops 1 and 2 completed: the load gives the `8152`, `8153`, `8154` binds and the `8155`, `8156` increments, nothing else (sort both label lists before comparing; the order across chains is not part of the contract); then drop 3 with `drops_4908` 2 and drop 4 with 3 give count chains, and drop 5 with 4 gives 8149.
5. Every drop chain resolves nothing in world 62 and at step 4909.
