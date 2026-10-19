# Dakara_E1 wave 6c: Livewire terminals and real gate travel to the SGC

> Type: how-to (work packets). Audience: the packet-coder named in each packet, and the read-only wave-6c packet-reviewer (its findings go to a fresh packet-coder).
> Written 2026-10-19 (architect) for the owner's rulings of 2026-10-19 on D-DK24 (real dialling to the SGC, grant the SGC address) and D-DK31 (the Livewire hack is required on the terminal steps), against `origin/main` @ `12edca3e4`. Ledger and status: [README](../README.md#packets). Rules and full contract: [work-packets.md](../work-packets.md). Findings: F40 to F45; decisions: D-DK24, D-DK31, D-DK40 to D-DK44.

Wave 6c has five packets in two steps. Every packet is seed and test code only. None needs the lab, the client or an owner answer to **build**; two need owner and cross-campaign answers to **merge**.

| Step | Packet | Worktree, branch | Size | Gate |
|---|---|---|---|---|
| 1 | [DK-40b](#dk-40b-replay-labels-and-helpers-for-wave-6c) | `dk-40b`, `dakara/dk-40b-replay-helpers` (new, from `main`) | about 15k tokens | none. **Ready**. Review, PR and merge before step 2 |
| - | `ops` rebase | `dk-30b` and `dk-33` | - | `ops` rebases `dakara/dk-30b` and `dakara/dk-33` onto the `main` SHA that contains DK-40b, and records it. Packet coders never rebase |
| 2 | [DK-14c](#dk-14c-livewire-on-the-1649-command-terminal) | `dk-14c`, `dakara/dk-14c-livewire-terminal` (new, from `main` after DK-40b) | about 20k tokens | DK-40b merged |
| 2 | [DK-15b](#dk-15b-livewire-on-mohkatans-terminal) | `dk-15b`, `dakara/dk-15b-livewire-terminal` (new, from `main` after DK-40b) | about 20k tokens | DK-40b merged |
| 2 | [DK-30b-r](#dk-30b-r-mission-1651-crossing-by-real-gate-travel) | `dk-30b` (existing; branch as created) | about 25k tokens | DK-40b merged and the `ops` rebase done. **Merge hold:** US-30, D-DK40, D-DK44 |
| 2 | [DK-33-r](#dk-33-r-mission-1654-crossing-by-real-gate-travel) | `dk-33` (existing; branch as created) | about 20k tokens | DK-40b merged and the `ops` rebase done. **Merge hold:** US-31, D-DK44 |

One read-only `packet-reviewer` reviews DK-40b alone (step 1), and one reviews the four step-2 packets together; its findings go to a fresh `packet-coder`. DK-30b-r ships in one PR with DK-30b (`34abe129b`) and DK-30b-n (`8b1ac4673`); DK-33-r ships in one PR with DK-33 (`41686b50c`) and DK-33-n (`ffec26d33`).

**Dormant by design (D-DK23).** Every chain here belongs to missions 1649 to 1654, which stay unreachable in play until wave 7 ships 1647. Do not add a GM shortcut, a `player_loaded` offer or any other way in.

## Rules for every wave-6c packet

1. PowerShell only. No bash, WSL or Git Bash. Use absolute paths inside your worktree; never `cd` to the root checkout. Prefix each lane call with `Set-Location <worktree>;` and check that the lane log lands under `logs/<worktree>/`.
2. Every compiling `cargo` call goes through the lane, run from the worktree root: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly; never add `--target-dir`. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`. Run lane checks in the foreground, one at a time.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, `git rebase`, or delete a worktree. Commit once, locally, with the subject below and the attribution lines from your brief.
4. Edit only the files your packet names. Only DK-40b edits `dakara_e1_missions/mod.rs` and `labels.rs`. Never edit `db/database.sql`, the linter or another packet's file. Files stay under 700 lines (soft cap 500).
5. The seed files and everything under `docs/` are CRLF; keep them CRLF. Check with `[IO.File]::ReadAllText('<absolute path>')`: it must contain "`r`n" and no "`n" without a "`r" before it.
6. Seeds change in `db/resources/` only; never add a `db/scripts/` migration. Every chain has a comment above it with its evidence label and what it serves.
7. No new dialog row, no new sequence row, no client patch, no rewards. **The only stargate address grant is `grant_stargate_address 27` in chains 8313, 8432 and 8433. Never grant from a `player_loaded` chain** (finding F44: a live-DB guard rejects it).
8. Never launch the game client, the launcher or any lab tool.
9. No IPs, credentials, tokens or account names in any file.
10. Report to `<worktree>\.claude\reports\dakara\<packet>-packet-coder.md`: files changed, each check's final `status=` line, the commit hash, every deviation and why. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. A context stop past about 90k tokens: commit only passing work (subject suffix `(part N)`), write `docs/analysis/dakara-e1-rebuild/worknotes/<packet>.md` and return `blocked` with the reason `context`.

**Checks, in order, for every wave-6c packet** (from the worktree root; each must end `status=ok` or with every test passing):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine interact_tag_linter`
4. `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1_missions`

A failing check gets at most five fix rounds; then commit nothing, report, and return `failed`. If check 4 fails in a test of another packet's file, do not edit that file: report it and return `blocked`.

**Docs rows owed.** None in these packets. The UAT guide rows are DK-99e's; `docs/content/mission-chains.md` is DK-99b's. Say so in the report.

## Contract excerpt for wave 6c

**Notation** (adds to [wave 6](wave-6.md#contract-excerpt-for-wave-6) and work-packets C4):

| Short form | Rows |
|---|---|
| `livewire -> V` | `(id, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [V], "difficulty": 1}', 0, n)` |
| `grant 27` | `(id, 'grant_stargate_address', 27, NULL, '{}', 0, n)` |
| `crossed SGC_W1` | trigger `(id, 'stargate_crossed', 'SGC_W1', 'player', false, 0)` |
| victory chain | one `content_chains` row and its `content_actions` rows only: **no `content_triggers` row and no `content_conditions` rows** |

**Facts the packets rely on** (verified 2026-10-19 on `origin/main`):

- **Livewire (F42).** A launcher chain runs `start_minigame`; when the player wins, the minigame callback fires each `on_victory_chains` id directly, with no trigger and **no condition check**, so every gate belongs on the launcher. A loss or a closed window fires nothing. Precedent: `db/resources/Content/Seed/castle_706_708_chains.sql`, launcher 1356 and victory chain 1357; Castle 1292/1293 shows a `display_dialog` in a victory chain (the dialog uses the terminal's interaction pin). Test precedent: `chain_replay_tests/castle_702_704_executor.rs` lines 277-288 and `mission_708/stargate.rs` lines 208-230.
- **`Action::StartMinigame { minigame_type: String, difficulty: u32, on_victory_chains: Vec<i64> }`**, `crates/content-engine/src/actions.rs:220`. **`Action::GrantStargateAddress { stargate_id: i32 }`**, line 459.
- **`stargate_crossed` (F41).** Fires on the origin cell as the player steps through the open gate, before the cell lets go of them. The trigger's `event_key` is the destination world name and is matched against the event param `destination_world`; SGC_W1 is world 58. Mission state and archetype are populated, so `step_status` and `objective_status` conditions work. Precedent: Castle chain 1360 (`stargate_crossed 'Harset'`). Dispatcher: `crates/cell-content/src/cell/content/event_dispatch/stargate.rs`.
- **A DHD click that matches any `interact_tag` chain suppresses the dial window** (F25). So no chain may match `Dakara_E1_DHD` at a step where the player must dial. The 1651 and 1654 crossings complete on `crossed SGC_W1` instead.
- **Every matching chain fires, and conditions are read before any action runs.** Chain 8313 fires beside 8292 on the same Bra'tac click, even though 8292 advances the step.
- **Chain order in a result** is by chain id between equal priorities.

## DK-40b: replay labels and helpers for wave 6c

**Goal.** Give the step-2 packets two label arms and two helpers, so no two of them edit the shared test files. Test code only; no seed change.

**Files (only these two):**

- `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/mod.rs`
- `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/labels.rs`

**Do.**

1. In `mod.rs`, `labels_in`, add two arms before the `other =>` arm:
   - `Action::StartMinigame { minigame_type, difficulty, on_victory_chains } => format!("start_minigame({minigame_type},victory={on_victory_chains:?},d={difficulty})")` (if the variant has more fields, add `..`).
   - `Action::GrantStargateAddress { stargate_id } => format!("grant_stargate_address({stargate_id})")`.
   - So a launcher prints `8223:start_minigame(Livewire,victory=[8231],d=1)` and a grant prints `8313:grant_stargate_address(27)`.
2. In `mod.rs`, add `fire_stargate_crossed`, shaped exactly like `fire_enter_region`, marked `#[allow(dead_code)] // used by the wave-6c submodules (work-packets.md C7)`:

   ```rust
   /// Fire `stargate_crossed` to `destination`, as the cell does when the
   /// player steps through an open gate (`event_dispatch/stargate.rs`).
   fn fire_stargate_crossed(engine: &ChainEngine, ctx: &ExecutionContext, destination: &str) -> ResolvedActions
   ```

   Body: a new `ExecutionContext`, copy `world_id` and `params` from `ctx`, `set_param("destination_world", json!(destination))`, a `TriggerEvent` with `trigger_type: TriggerType::StargateCrossed`, `source_entity: None`, `target_entity: None`, `params: c.params.clone()`, then `engine.resolve_event(&event, &c)`.
3. In `mod.rs`, add `fire_victory`, with the same `#[allow(dead_code)]` line:

   ```rust
   /// The actions a Livewire win runs: victory chain `id`, fired by id with no
   /// condition check, as the minigame callback does (Castle 1357). Panics if
   /// the chain is missing or carries a condition (a condition there is never read).
   async fn fire_victory(pool: &sqlx::PgPool, id: i32) -> ResolvedActions
   ```

   Body: `crate::cell::content::engine_loader::load_single_chain_for_test(pool, id).await` (expect the query to succeed; `unwrap_or_else(|| panic!("victory chain {id} is not seeded"))`); `assert!(chain.conditions.is_empty(), "victory chain {id} has conditions, which are never evaluated: {:?}", chain.conditions)`; then a fresh `ChainEngine::new()`, `register_chain(chain)`, and `get_chain_actions(i64::from(id))` unzipped into `actions: Vec<(i64, Action)>` (each paired with `i64::from(id)`) and `action_delays: Vec<i32>`; return `ResolvedActions { actions, action_delays, ..Default::default() }`. Copy the shape of `castle_702_704_executor.rs` lines 277-288.
4. In `labels.rs`, add one `#[test]` `labels_cover_the_wave_6c_verbs`: a hand-built `ResolvedActions` with, at chain 8231, `Action::StartMinigame { minigame_type: "Livewire".into(), difficulty: 1, on_victory_chains: vec![8231] }` and `Action::GrantStargateAddress { stargate_id: 27 }`, plus one action at 9999 that must not show; assert `labels_in(&r, M1649_TERMINAL_CHAINS)` equals `["8231:start_minigame(Livewire,victory=[8231],d=1)", "8231:grant_stargate_address(27)"]`.

**Proof.** The new `labels.rs` test fails if either arm is removed (it would print `OTHER(..)`). The two helpers are proved by their first users in step 2.

**Commit subject:** `test(content): Dakara_E1 replay labels for Livewire and address grants, crossed and victory helpers (DK-40b)`

## DK-14c: Livewire on the 1649 Command Terminal

**Goal.** The first click on the Command Terminal at 1649 step 4922 (objective 5638 open) starts the Livewire hack instead of handing over the orders. Winning it shows 5834, gives item 6782 and completes 5638, exactly as the click did before. Every other terminal state is unchanged: the replay of 5834 (chain 8225) and the decode (8224) stay plain clicks (D-DK42).

**Files (only these two):**

- `db/resources/Content/Seed/dakara_e1_m1649_terminal_chains.sql`: edit chain 8223; append chain 8231 after the last chain in the file.
- `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/m1649_terminal.rs`

**Seed.**

| Chain | Trigger | Conditions | Actions |
|---|---|---|---|
| 8223 (edit) | unchanged: click `Dakara_E1_CommandTerminal` | unchanged: world 61; s1649/4922 active; o1649/5638 neq completed | `livewire -> 8231` (sort 0). The three old actions move to 8231 |
| 8231 (new, victory chain) | **none** | **none** | `display_dialog 5834` (0); `add_item 6782` `'{"container": 0, "qty": 1}'` (1); `complete_objective 1649 '5638'` (2) |

- 8223's description becomes `'1649 - 4922: terminal hack, start Livewire (victory 8231)'`; its comment says the click now starts Livewire (D-DK31, owner 2026-10-19) and the orders come from 8231.
- 8231: `content_chains` row `(8231, '1649 - 4922: Livewire victory on the terminal, download the orders', 'mission', 1649, true, 0)`. Comment: `PROJECT_FINAL_RECONSTRUCTION`; "No trigger row and no conditions: the Livewire callback fires it by id with no condition check (Castle 1357), so the gate is on launcher 8223."

**Tests** (`m1649_terminal.rs`):

1. `live_db_terminal_states_are_exclusive`: the `hack` vector becomes `["8223:start_minigame(Livewire,victory=[8231],d=1)"]`. Give each case a third column, the expected `display_dialog_count`: 0 for the two hack cases, 1 for the 8225 and decode cases. Assert it per case in place of the fixed `1`.
2. Replace the test that expects 8223's three labels (search the file for `8223:display_dialog(5834)` outside the cases table) with `live_db_hack_starts_livewire_and_the_win_gives_the_orders`:
   - At `terminal_ctx(&[])`, a terminal click gives exactly `["8223:start_minigame(Livewire,victory=[8231],d=1)"]`, `display_dialog_count == 0`, and `assert_no_deferred_actions(&r, 8223)`.
   - `let v = fire_victory(&pool, 8231).await;` then `labels_in(&v, M1649_TERMINAL_CHAINS)` equals `["8231:display_dialog(5834)", "8231:add_item(6782x1)", "8231:complete_objective(1649/5638)"]` and `delays_of(&v, 8231)` equals `[0, 0, 0]`.
3. Every other test in the file passes unchanged. 8231 never resolves from an event, so the negative tests need no edit; if one fails, report it rather than weakening it.

**Proof.** Reverting the seed (8223 back to three actions, no 8231) fails tests 1 and 2. A condition row added to 8231 fails `fire_victory`.

**Commit subject:** `feat(content): Dakara_E1 1649 terminal hack through Livewire, victory chain 8231 (DK-14c)`

## DK-15b: Livewire on Moh'katan's terminal

**Goal.** At 1650 step 4926 in the tent (world 62), clicking Moh'katan's terminal starts Livewire. Winning it runs the five actions the click ran before, in the same order: file 6787, advance to 4925, unbind the carrier 6760 from slot 454, despawn the hologram, bind Bra'tac's turn-in 6771 on slot 59.

**Files (only these two):**

- `db/resources/Content/Seed/dakara_e1_m1650_chains.sql`: edit chain 8258; append chain 8261 after the last chain in the file (8250-8260 are used).
- `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/m1650.rs`

**Seed.**

| Chain | Trigger | Conditions | Actions |
|---|---|---|---|
| 8258 (edit) | unchanged: click `Dakara_E1_StoryRm_MohkatanTerminal` | unchanged: world 62; s1650/4926 active | `livewire -> 8261` (sort 0) |
| 8261 (new, victory chain) | **none** | **none** | the five old 8258 actions, same rows and order: `add_item 6787` `'{"container": 0, "qty": 1}'` (0); `advance_step 1650 '4925'` (1); `remove_dialog_set 6760` `'{"slot": 454}'` (2); `despawn_entity` key `'Dakara_E1_StoryRm_BaalHologram'` (3); `add_dialog_set 6771` `'{"slot": 59}'` (4) |

- 8258's description becomes `'1650 - Terminal: start Livewire (victory 8261)'`; its comment says why (D-DK31, D-DK42, owner 2026-10-19).
- 8261: `content_chains` row `(8261, '1650 - Livewire victory on the terminal: give file 6787, advance to 4925, unbind 6760 on 454, despawn hologram, bind Bra''tac dsm 6771 on 59', 'mission', 1650, true, 0)`. Comment as 8231's: no trigger, no conditions, gate on 8258; the unbind and despawn still run before the new bind.

**Tests** (`m1650.rs`): rename `live_db_terminal_gives_the_file_and_removes_the_hologram` to `live_db_terminal_starts_livewire_and_the_win_gives_the_file`.

- The click at `at_step(Some(STORY_RM), 4926)` gives exactly `["8258:start_minigame(Livewire,victory=[8261],d=1)"]`, `display_dialog_count == 0`, `assert_no_deferred_actions(&r, 8258)`. Replace the comment "The terminal shows no dialog..." with "The Livewire window is the click's feedback (D-DK31)."
- `let v = fire_victory(&pool, 8261).await;` `labels_in(&v, M1650_CHAINS)` equals the five old labels, now prefixed `8261:`; `delays_of(&v, 8261)` equals `[0, 0, 0, 0, 0]`; the positional check (despawn before the 6771 bind) runs on `labels_in(&v, ..)` with the `8261:` labels.
- The three negative clicks (4924, 4925, world 61) stay as they are.

**Proof.** Reverting the seed fails the renamed test.

**Commit subject:** `feat(content): Dakara_E1 1650 terminal through Livewire, victory chain 8261 (DK-15b)`

## DK-30b-r: mission 1651 crossing by real gate travel

**Goal.** Owner ruling D-DK24: the player dials the SGC themselves. Bra'tac's status report at step 4932 also grants gate 27 (the SGC). At step 4930 a click on the Dakara DHD opens the dial window as usual (no chain matches it); dialling SGC W1 and stepping through advances 1651 to 4928. The SGC's US-30 receives the player at 4928, as before. There is no teleport and no recovery chain: a player who has not crossed simply dials again.

**Worktree.** `dk-30b`, after `ops` has rebased it onto `main` with DK-40b. Check first: `git -C <worktree> log --oneline -4` shows `8b1ac4673`'s subject on top of a `main` that contains DK-40b. If not, return `blocked` with the reason `rebase`.

**Files (only these two):**

- `db/resources/Content/Seed/dakara_e1_m1651_gate_chains.sql`
- `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/m1651_gate.rs`

**Seed.** Update the file header comment (it describes step 4930 and crossing C2; now say "by real gate travel, and the SGC address grant at Bra'tac's 4932 report").

| Chain | Trigger | Conditions | Actions |
|---|---|---|---|
| 8310 (edit) | `crossed SGC_W1` (was click `Dakara_E1_DHD`) | s1651/4930 active, **only** (drop the world condition: dialling from Omega Site's DHD also counts) | `advance_step 1651 '4928'` (0). Drop the teleport row |
| 8311 | delete (chain, trigger, conditions, actions) | | |
| 8312 | delete | | |
| 8313 (new) | click `Dakara_E1_StoryRm_Bratac` | world 62; s1651/4932 active | `grant 27` (0) |

- 8310's description becomes `'1651 - Step 4930: crossed to SGC_W1 by gate (C2), advance to 4928'`; its comment cites D-DK24 (owner 2026-10-19), F41 and Castle 1360, and says no chain may match the DHD at 4930 because a match would hide the dial window (F25).
- 8313's description is `'1651 - Status report to Bra''tac at 4932: grant the SGC address (gate 27)'`; comment: `PROJECT_FINAL_RECONSTRUCTION`; it co-fires with chain 8292 (dialog 5843, advance to 4930), which stays unchanged; a click grant, never `player_loaded` (F44); the grant is idempotent.

**Tests** (`m1651_gate.rs`). Delete the four old tests and the `SGC_H3` constant (and `TERMINAL_TAG` if nothing uses it). Add a constant `BRATAC_TAG: &str = "Dakara_E1_StoryRm_Bratac"` and a helper `report_ctx(archetype: i64, world: Option<i32>, step: i32)` shaped like `at_step` but with the archetype as a parameter. Then:

1. `live_db_crossing_to_the_sgc_at_4930_advances_to_4928`: for `world` in `[Some(DAKARA), Some(18)]` (18 is Omega Site), `fire_stargate_crossed(&engine, &at_step(world, 4930), "SGC_W1")` gives exactly `["8310:advance_step(1651->4928)"]` in `M1651_GATE_CHAINS`, `display_dialog_count == 0`, and `assert_no_deferred_actions(&r, 8310)`.
2. `live_db_crossing_stays_quiet_elsewhere`: no `M1651_GATE_CHAINS` label for: crossing to `DAKARA_NAME` or `"Omega_Site"` at 4930; crossing to `"SGC_W1"` at each of 4932, 4928, 4931, 4929 and 5171; crossing to `"SGC_W1"` with no 1651 state (`ctx_for(TAURI, Some(DAKARA))`).
3. `live_db_the_dhd_click_resolves_no_chain_so_the_dial_window_opens`: for each of 4932, 4930, 4928, 4931, 4929, 5171 and for no 1651 state, `fire_interact_tag(&engine, &ctx, DHD_TAG)` resolves **no action from any chain**: `assert!(r.actions.is_empty(), "step {step}: {:?}", r.actions)`. This is the guard for F25. If a chain outside the Dakara block makes it fail, do not weaken it: report the chain id and return `blocked`.
4. `live_db_bratac_report_grants_the_sgc_address`: for `archetype` in `[TAURI, FREE_JAFFA]`, a `BRATAC_TAG` click at `report_ctx(archetype, Some(STORY_RM), 4932)` gives exactly `["8313:grant_stargate_address(27)"]` in `M1651_GATE_CHAINS`; `labels_in(&r, M1651_CHAINS)` contains `"8292:advance_step(1651->4930)"`; `display_dialog_count(&r) == 1` (8292's 5843 only); `assert_no_deferred_actions(&r, 8313)`.
5. `live_db_no_grant_outside_the_report`: no `M1651_GATE_CHAINS` label for a `BRATAC_TAG` click at 4930 and at 5171 in `STORY_RM`, or at 4932 with world `Some(DAKARA)`; and `fire_player_loaded` for both `DAKARA_NAME` and `STORY_RM_NAME` at 4932 resolves no `M1651_GATE_CHAINS` label.

Import `fire_player_loaded` from `mission_1324` as the other submodules do.

**Proof.** Reverting 8310 to the DHD click fails tests 1 and 3; removing 8313 fails test 4; a world condition on 8310 fails the Omega Site case of test 1.

**Commit subject:** `feat(content): Dakara_E1 mission 1651 crossing C2 by real gate travel, SGC address at Bra'tac (DK-30b-r)`

## DK-33-r: mission 1654 crossing by real gate travel

**Goal.** The same ruling for crossing C3. Taking 1654 from Bra'tac grants gate 27 (it is already known after 1651; the grant is idempotent and covers a GM-skipped 1651). At step 4939 the DHD opens the dial window; stepping through to SGC W1 completes objective 5961 only (never `advance_step`: 5962 is the SGC's, US-31). No teleport, no recovery chain.

**Worktree.** `dk-33`, after `ops` has rebased it onto `main` with DK-40b. Check first: `git -C <worktree> log --oneline -4` shows `ffec26d33`'s subject on top of a `main` that contains DK-40b. If not, return `blocked` with the reason `rebase`.

**Files (only these two):**

- `db/resources/Content/Seed/dakara_e1_m1654_chains.sql`
- `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/m1654.rs`

**Seed.** Update the file header comment ("the DHD departure" becomes "real gate travel to the SGC").

| Chain | Change |
|---|---|
| 8430, 8431 | unchanged |
| 8432, 8433 | append `grant 27` at sort 3 to each; add one comment line: the SGC address for the C3 crossing (D-DK24, D-DK43) |
| 8434 | trigger becomes `crossed SGC_W1`. Conditions: s1654/4939 active (0); o1654/5961 neq completed (1); drop the world condition. Actions: `complete_objective 1654 '5961'` (0) only; drop the teleport. Description `'1654 - Step 4939 (crossing C3): crossed to SGC_W1 by gate, complete objective 5961 only'`. Keep the "never advance_step" sentence in the comment; add D-DK24, F41 and F25 (no DHD chain, so the window opens) |
| 8435 | delete (chain, trigger, conditions, actions) |

**Tests** (`m1654.rs`):

1. The two offer tests: append `"8432:grant_stargate_address(27)"` and `"8433:grant_stargate_address(27)"` to their expected vectors. `display_dialog_count` stays 1.
2. Replace `live_db_dhd_completes_5961_then_teleports_to_sgc` with `live_db_crossing_to_the_sgc_completes_5961_only`: `fire_stargate_crossed(&engine, &at_dhd(TAURI, "active"), "SGC_W1")` gives exactly `["8434:complete_objective(1654/5961)"]` in `M1654_CHAINS`; no label in it contains `advance_step`; `display_dialog_count == 0`; `assert_no_deferred_actions(&r, 8434)`.
3. Delete `live_db_dhd_recovery_teleports_once_5961_is_done`.
4. Replace `live_db_dhd_needs_dakara_and_step_4939` with `live_db_crossing_needs_step_4939_and_5961_open`: no `M1654_CHAINS` label for: crossing to `"SGC_W1"` with 5961 `"completed"`; crossing to `DAKARA_NAME` with 5961 `"active"`; crossing to `"SGC_W1"` with 1654 `"completed"`; crossing to `"SGC_W1"` with 1651 at 4930 and 1654 `"not_active"` (keep that case's comment).
5. Add `live_db_the_dhd_click_resolves_no_chain_during_1654`: at `at_dhd(TAURI, "active")` and `at_dhd(TAURI, "completed")`, `fire_interact_tag(&engine, &ctx, "Dakara_E1_DHD")` gives `r.actions.is_empty()`. Same `blocked` rule as DK-30b-r test 3.

Import `fire_stargate_crossed` from `super::*` (it comes with DK-40b); drop imports that become unused.

**Proof.** Reverting 8434 to the DHD click fails tests 2 and 5; removing a grant row fails test 1.

**Commit subject:** `feat(content): Dakara_E1 mission 1654 crossing C3 by real gate travel, SGC address on the offer (DK-33-r)`

## For the wave-6c reviewer

- Victory chains 8231 and 8261: no trigger row, no condition row, actions identical in rows and order to the old 8223 and 8258.
- No chain anywhere matches `Dakara_E1_DHD` after DK-30b-r and DK-33-r (`git grep -n "Dakara_E1_DHD" -- db/resources` shows only spawn and template rows).
- `grant_stargate_address` appears only in 8313, 8432 and 8433 among the Dakara chains, never under a `player_loaded` trigger.
- The negative tests are not weakened; the engine-wide DHD tests use `r.actions.is_empty()`, not a range filter.
- The merge holds stay: DK-30b-r and DK-33-r open no PR (the orchestrator opens them when US-30, US-31, D-DK40 and D-DK44 allow).
