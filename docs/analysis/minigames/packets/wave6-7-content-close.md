# Waves 6-7: content, UAT and close-out

> Type: how-to. Part of the [minigames work packets](../work-packets.md); the contract (C1-C7) and the dispatch rules live there.
> Content packets read [content-map.md](../content-map.md) (MG-R5) and [spec-launch-flow.md](../spec-launch-flow.md) (MG-R7). Every input below was confirmed against them at G1 (D-MG22); a coder still re-reads the named seed line before editing it.

## Content rules (inline for every wave-6 packet)

- PowerShell only, with `Set-Location <wt>;` on every command.
- Cargo runs only through `pwsh tools/build-lane/lane.ps1` (except `cargo fmt`). Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, in the foreground.
- **Seeds are the source of truth.** Edit `db/resources/` only; never add `db/scripts/*.sql` migrations.
- A new seed file gets its `\ir resources/Content/Seed/<file>.sql` line in `db/database.sql`, after the last `Content/Seed` line.
- Chain ids come from the packet's C7 block only. Grep the range in `db/resources/` first; if any id is taken, stop and return `blocked`.
- The chain row shape follows the existing pairs: the `start_minigame` rows in `db/resources/Content/Seed/castle_cellblock_chains.sql` (line 198: `(1024, 'start_minigame', NULL, 'Livewire', '{"on_victory_chains": [1025]}', 0, 0)`) and the `interact_tag` and step-condition rows in `db/resources/Content/Seed/beta_e1_m1147_chains.sql` (chains 23391 and 23393). Read both before writing.
- **Tests:** type 6 chain-replay, in `crates/cell-content/src/cell/content/chain_replay_tests/`, following `livewire_pairs.rs`. That means four assertions: the launcher resolves exactly one `StartMinigame`, it resolves nothing at the wrong step, `execute_actions` emits one `CellToBaseMsg::StartMinigame` with the authored fields, and the victory chain's action list matches the seed. Register the module in `chain_replay_tests/mod.rs`.
- Files: soft cap 500 lines, hard cap 700. Docs are CRLF. No `git stash`, prune or push; make one local commit with the brief's attribution. No lab.
- **Lanes:**

  ```powershell
  Set-Location <wt>; cargo fmt -p cimmeria-cell-content
  Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings
  Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 <the packet's test module name>
  ```

- **Docs.** Add a row to [mission-chains.md](../../../content/mission-chains.md) for each mission touched, and update [zone-audit.md](../../../content/zone-audit.md) if a zone row changes. Doc-update-map row "Seeds / content chains".

Wave 6 ships like every other wave: `ops` merges each packet branch into `minigames/integration` with `merge-noff.ps1`, and the campaign ships one PR at the end. Merge order: MG-62, MG-61, MG-64, MG-74a, MG-74b. MG-62 goes after MG-63b (wave 2) because both append to the same hand-authored `effect_nvps` block. MG-74a goes after MG-61; its `db/database.sql` lines sit beside the Debug Area lines, away from MG-61's.

## MG-61: mission 582 step 2307, Converse with Marsh

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: MG-CH3 merged into `minigames/integration` · Status BlockedDependency
- Worktree `mg-61` · Branch `minigames/mg-61-m582-marsh` · Commit `MG-61: mission 582 step 2307 plays the Marsh conversation minigame`
- **Ids:** chains 24000-24001 (C7 reserves 24000-24009; the rest stay unused).
- **Inputs (content-map.md §6 and §10, confirmed at G1):** Marsh is spawn 344 in world 68, interact tag `CmdCenter_Marsh`. Dialog 2433 is the step's prompt. The game is `ConverseBasicHumanoid` at difficulty 1. Step 2307's objective is 2675 and the next step is 3274. Marsh's existing chains 6501 and 6527 fire on the same tag while missions 1360 (step 4038) and 1361 (step 4694) are active; the launcher must not steal those clicks.
- **Files:**
  1. `db/resources/Missions/Seed/mission_steps.sql:5335`: row 2307 `step_enabled` goes from `false` to `true`. Change nothing else on the row.
  2. New `db/resources/Content/Seed/minigames_m582_chains.sql` (header comment naming D-MG22 and MG-61):
     - launcher **24000**: trigger `interact_tag 'CmdCenter_Marsh'`; conditions `world 68` (Grep `'world',` under `db/resources/Content/Seed/` for the row shape), `mission_status 582 eq active`, `step_status 582 '2307' eq active`, `step_status 1360 '4038' neq active`, `step_status 1361 '4694' neq active`; actions in order `display_dialog 2433`, then `start_minigame` with target key `'ConverseBasicHumanoid'` and params `{"difficulty": 1, "on_victory_chains": [24001]}`.
     - victory chain **24001** (no trigger row): `complete_objective 582 '2675'`, then `advance_step 582 '3274'`. Grep both action names in `db/resources/Content/Seed/` and copy the column use of an existing row; if either action name differs, use the seeded spelling and say so in the report.
     - No failure chain: a loss leaves step 2307 active, and the player clicks Marsh again. The session's own `failure` result is the feedback.
  3. The `\ir resources/Content/Seed/minigames_m582_chains.sql` line in `db/database.sql`.
- **Not in scope.** Mission 582 cannot be offered today: its giver Blackstock (NPC 948) has no spawn and set 530 is unbound (M10, content-map §10). Do not add an offer path; the test sets the mission and step active directly. Dialog 2431 (`accepts_mission_id = 582`, `dialogs.sql:4345`) stays as is.
- **Test:** new `chain_replay_tests/minigames_m582.rs` with the four assertions, plus: the launcher resolves nothing while step 4038 of mission 1360 is active (the guard), and the seeded step 2307 row has `step_enabled = true`. On revert of the seed edit the step-enabled assertion fails; on revert of the chain file the launcher resolves nothing. Lane filter `minigames_m582`.
- **Docs:** [mission-chains.md](../../../content/mission-chains.md), a mission 582 row naming the offer gap (M10).

## MG-62: Converse abilities 1327 and 2089

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: MG-24, MG-63b and MG-CH3 merged into `minigames/integration` · Status BlockedDependency
- Worktree `mg-62` · Branch `minigames/mg-62-converse-abilities` · Commit `MG-62: Converse abilities launch the conversation minigame`
- **Ids:** NVPs 25501-25504 in the hand-authored block MG-63b opened (25500-25509). No chains: spec-launch-flow names no victory or failure outcome for either ability, so the session result is the only outcome.
- **Inputs (spec-launch-flow.md §6, D-MG20):** effect 1643 (ability 1327) and effect 2813 (ability 2089). The `StartMinigame` script (MG-24) reads `minigame_type` and `difficulty` from the effect's NVPs; the caster plays.
- **Files:**
  1. `db/resources/Effects/Seed/effects.sql`: `script_name` (the last column, NULL today) becomes `'StartMinigame'` on effect 1643 (line 2915) and effect 2813 (line 4144). Re-read each line first; change only that column.
  2. `db/resources/Effects/Seed/effect_nvps.sql`, in the `-- minigames hand-authored (25500-25509)` block, after MG-63b's 25500 row:

     ```sql
     INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (25501, 1643, 'minigame_type', 'ConverseBasicHumanoid');
     INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (25502, 1643, 'difficulty', '1');
     INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (25503, 2813, 'minigame_type', 'ConverseBasicHumanoid');
     INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (25504, 2813, 'difficulty', '1');
     ```

     Grep `25501` first; if taken, stop and return `blocked`. If the block header is missing, MG-63b has not merged: return `blocked`.
- **Test (type 3 live-DB):** new `crates/cell-effect-scripts/src/cell/effects/start_minigame_seed_live_db_tests.rs`, copying the shape of MG-63b's `movement_hold_seed_live_db_tests.rs`: effects 1643 and 2813 load with `script_name = "StartMinigame"`, the registry resolves it, and each carries `minigame_type = ConverseBasicHumanoid` and `difficulty = 1`. Register it beside MG-63b's module. On revert the script name is NULL and the first assertion fails.
- **Lanes:** `cargo fmt -p cimmeria-cell-effect-scripts`; `lane.ps1 cargo clippy -p cimmeria-cell-effect-scripts --all-targets -- -D warnings`; `live-db-test.ps1 start_minigame_seed`.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md) § ability launch: the two seeded abilities.

## MG-63: Prison Boot 1597 (replaced)

Replaced at G1 by **MG-63a** (movement lock while a minigame or hold is active) and **MG-63b** (the boot's `MovementHold`/`MovementRelease` scripts and seeds), both in wave 2: [wave2-launch.md](wave2-launch.md). Chains 24020-24029 stay unused; chains 1022-1025 already carry the boot.

## MG-64: shell placeholder timer and unknown-name rejection

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: MG-04 (integrated) · Status Ready
- Worktree `mg-64` · Branch `minigames/mg-64-shell-timer` · Commit `MG-64: shell minigames show a countdown and bad game names fail at load`
- **Decision (D-MG3 = A, D-MG22, spec-shells.md §8 and §10).** The shells stay client-declared; no content is wired (content-map finds no seeded objective that names a shell). Two server fixes only: the shells get a timer they can show, and an unknown or wrong-case minigame name is caught at content load, not at launch.
- **Files:**
  1. `crates/minigame/src/minigame/games/placeholder.rs`:
     - `started()` sends `fullgamestate {timerState: 0, timeRemaining: 60}` (replacing `{gameStarted: 'false'}`), then `timerupdate {timerState: 1, timeRemaining: 60}`. Numbers go as `SfsValue::Number`.
     - `needs_tick()` returns true; count ticks (250 ms each). At 240 ticks (60 s), send `failure` and return `GameOutput::Failure`, once.
     - Keep the D-MG13 5 s floor on a client `victory` unchanged.
  2. `crates/content-engine/src/loader/action.rs`, the `"start_minigame"` arm (lines 124-163): a `const MINIGAME_TYPE_NAMES: &[&str]` holding the exact names `cimmeria_minigame` registers (copy them from `crates/minigame/src/minigame/games/kind.rs`; do not add a crate dependency). A target key not in the list (case-sensitive) logs WARN `reason = "unknown_minigame_type"` with the chain id and drops the row, like the existing difficulty check at line 151.
- **Tests:**
  - type 1 in `crates/minigame/src/minigame/games/placeholder_tests.rs`: `started()` emits `timerState` and `timeRemaining` (spec-shells §10 item 4); 239 ticks send nothing and the 240th sends `failure` and returns `Failure`; a `victory` at 4 s is still refused.
  - type 1 in `crates/content-engine/src/loader/tests/action_conversion.rs`, beside the difficulty tests (line 476): `'hack'` and `'NoSuchGame'` are dropped; `'Hack'` and `'Livewire'` load.

  Revert `started()` and the first test fails; revert the name check and `'hack'` loads.
- **Lanes:** `fmt`, `clippy`, then `lane.ps1 cargo nextest run -p <crate>` for `cimmeria-minigame` and `cimmeria-content-engine`, each in turn. No live DB.
- **Docs:** [minigame-system.md](../../../gameplay/minigame-system.md), the shell rows (60 s server timeout, failure at expiry) and the loader's name check.

## MG-65: objective 4203 upgrade (not applicable)

D-MG6 = A: objective 4203 keeps Harset's D-H13 interact route. Nothing to build; chains 24040-24049 stay unused.

## MG-74a: Debug Area minigame terminals (seeds and chain-replay test)

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: MG-61 merged into `minigames/integration` (the launcher pattern and the `minigames_m582` test module this one sits beside); MG-64 (integrated: the loader accepts `Hack`) · Status BlockedDependency
- Worktree `mg-74a` · Branch `minigames/mg-74a-debug-area-terminals` · Commit `MG-74a: Debug Area terminals launch every ported minigame`
- **Decision (D-MG23).** Six reusable terminals in the Debug Area (world 1300), one per ported game plus one shell, so a GM can launch each game by hand and the MG-73 lab recipes have a fixed target. They belong to no mission and have no step gate. Server-authoritative seeds only; no client patch. The plaza's Livewire terminal (spawn 13004, chains 13004-13005) stays as it is.
- **Ids (C7, MG-74 block).** Chains 24050-24057 (block 24050-24069), templates 1600-1605 (block 1600-1609), spawns 14100-14105 (block 14100-14109), dialog 60120, screens 200020-200021 (block 200020-200029). Before writing, Grep `db/resources/` for `\b240[56][0-9]\b` (chains), `VALUES \(160[0-9],` (templates), `VALUES \(1410[0-9],` (spawns), `VALUES \(60120,` (dialog) and `\b20002[0-9]\b` (screens). If any id is taken, stop and return `blocked`.
- **Rules (inline):**
  - PowerShell only, with `Set-Location <wt>;` on every command. Never run cargo directly; every compiling call goes through `pwsh tools/build-lane/lane.ps1` (except `cargo fmt`). Run lanes in the foreground.
  - Seeds only: edit `db/resources/` and `db/database.sql`; never add `db/scripts/*.sql`.
  - One SQL row per `INSERT` statement, and the exact two-line shape of `debug_area_plaza_chains.sql` lines 101-117 for chains. Template and spawn rows are **one line each**, with the explicit column list: the Debug Area NPC tests in `crates/cell/src/cell/service/tests/npc_ai/debug_area/` parse every `entity_templates*.sql` and `spawnlist*.sql` line by line, and a wrapped row breaks them.
  - Files: soft cap 500 lines, hard cap 700. Docs are CRLF (this packet edits none).
  - No `git stash`, `git worktree prune` or push; one local commit with the brief's attribution. No lab.
- **Files:**
  1. New `db/resources/Entities/Seed/entity_templates_debug_area_minigames.sql`. Header comment: "NEW CONTENT (minigames campaign, MG-74, D-MG23): Debug Area minigame terminals, templates 1600-1609". Then `SET search_path = resources, pg_catalog;`. Six rows that copy template 303 (`db/resources/Entities/Seed/entity_templates.sql`, the `(303, 'Em-Props.EM-ViewScreen02', ...` row; re-read it first) value for value, changing only `template_id`, `template_name`, and adding `display_name`:

     ```sql
     INSERT INTO entity_templates (template_id, static_mesh, body_set, components, flags, interaction_type, event_set_id, level, alignment, faction, name_id, name, patrol_path_id, patrol_point_delay, template_name, class, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, ability_set_id, ammo_type, loot_table_id, primary_color_id, secondary_color_id, skin_tint, weapon_item_id, static_interaction_sets, trainer_ability_list_id, speaker_id, has_dynamic_properties, interaction_set_id, move_speed, use_cover, display_name) VALUES (1600, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_WallTerminal', NULL, 4, 256, NULL, 1, 0, 1, 7550, NULL, NULL, NULL, 'Debug Area - Minigame Terminal CrystalGame', 'being', NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0, 0, 0, NULL, '{}', NULL, NULL, true, NULL, NULL, NULL, 'Minigame: CrystalGame');
     ```

     | template | game (`target_key`, case-sensitive) | `template_name` suffix | `display_name` |
     |---|---|---|---|
     | 1600 | `CrystalGame` | CrystalGame | `Minigame: CrystalGame` |
     | 1601 | `Alignment` | Alignment | `Minigame: Alignment` |
     | 1602 | `GoauldCrystals` | GoauldCrystals | `Minigame: GoauldCrystals` |
     | 1603 | `ConverseBasicHumanoid` | ConverseBasicHumanoid | `Minigame: ConverseBasicHumanoid` |
     | 1604 | `Livewire` | Livewire | `Minigame: Livewire` |
     | 1605 | `Hack` | Hack (shell) | `Minigame: Hack (shell)` |

     Every row keeps `interaction_type` 256 (INT_MinigameLivewire, the only minigame cursor bit with a Confirmed click path; content-map.md §9, Q-C2). Footer, with a comment that these rows load after `entity_templates.sql`'s own footer:
     `SELECT pg_catalog.setval('entity_templates_template_id_seq', GREATEST((SELECT MAX(template_id) FROM entity_templates), (SELECT last_value FROM entity_templates_template_id_seq), 1609), true);`
  2. New `db/resources/Worlds/Seed/spawnlist_debug_area_minigames.sql`, same header style, `SET search_path = resources, pg_catalog;`, then one row per terminal in the shape of `spawnlist_debug_area_plaza.sql` line 41. A row on the plaza's north edge, all on navmesh component 17 (nav_inspect, probe floor heights below), facing south (heading 3.1416); x 252 stays free as the walkway to the dummies:

     | spawn | x | y | z | template | tag |
     |---|---|---|---|---|---|
     | 14100 | 234.0 | 6.66 | -903.0 | 1600 | `DebugArea_Minigame_CrystalGame` |
     | 14101 | 239.0 | 6.73 | -903.0 | 1601 | `DebugArea_Minigame_Alignment` |
     | 14102 | 244.0 | 6.78 | -903.0 | 1602 | `DebugArea_Minigame_GoauldCrystals` |
     | 14103 | 257.0 | 6.86 | -903.0 | 1603 | `DebugArea_Minigame_ConverseBasicHumanoid` |
     | 14104 | 262.0 | 6.90 | -903.0 | 1604 | `DebugArea_Minigame_Livewire` |
     | 14105 | 267.0 | 6.82 | -903.0 | 1605 | `DebugArea_Minigame_Hack` |

     Every row: `heading` 3.1416, `world_id` 1300, `set_name` NULL, `is_stationary` true. Footer:
     `SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 14109), true);`
  3. New `db/resources/Content/Seed/debug_area_minigames_chains.sql`. Header: a table of tag → chain like `debug_area_plaza_chains.sql` lines 1-11, naming MG-74 and D-MG23, and the note "Reusable GM test fixtures; no mission, no step gate. Every cursor bit is a template default (1600-1605), so there is no `set_interaction_type`." Then:
     - Launchers **24050-24055**, one per row of the table above in the same order (24050 CrystalGame ... 24055 Hack). Each: `content_chains` row `(<id>, 'Debug Area - minigame terminal: start <Game>', 'space', 1300, true, 0)`; `content_triggers` row `(<id>, 'interact_tag', 'DebugArea_Minigame_<Game>', 'player', false, 0)`; `content_actions` row `(<id>, 'start_minigame', NULL, '<Game>', '{"difficulty": 1, "on_victory_chains": [24056], "on_failure_chains": [24057]}', 0, 0)`. No condition rows.
     - **24056** (no trigger row): `content_chains` `(24056, 'Debug Area - minigame terminal: win line', 'space', 1300, true, 0)`; action `(24056, 'npc_bark', NULL, NULL, '{"screen_id": 200020, "speaker": "Terminal", "channel": "say"}', 0, 0)`.
     - **24057** (no trigger row): the same with `'Debug Area - minigame terminal: loss line'` and `screen_id` 200021.
  4. `db/database.sql`, three `\ir` lines, each **directly after** an existing Debug Area line (not at the end of a section, so this packet never touches the lines MG-61 or MG-62 add):
     - `\ir resources/Entities/Seed/entity_templates_debug_area_minigames.sql` after `\ir resources/Entities/Seed/entity_templates_debug_area_lords.sql`;
     - `\ir resources/Worlds/Seed/spawnlist_debug_area_minigames.sql` after `\ir resources/Worlds/Seed/spawnlist_debug_area_lords.sql`;
     - `\ir resources/Content/Seed/debug_area_minigames_chains.sql` after `\ir resources/Content/Seed/debug_area_ring_chains.sql`.
     Check that the templates line still loads before the spawnlist line (foreign key) and both before `_foreign_keys.sql`.
  5. `db/resources/Dialogs/Seed/dialogs.sql`: after the dialog 60110 row (before the `-- TOC entry` footer), a comment "Minigames campaign, MG-74: the Debug Area minigame terminals' bark holder. Owns dialog id 60120." and
     `INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (60120, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, 'Debug Area - minigame bark holder (never displayed)');`
     The `dialogs_dialog_id_seq` footer (100011) stays.
  6. `db/resources/Dialogs/Seed/dialog_screens.sql`: after the 60110/200010 row, a comment "Minigames campaign, MG-74: the Debug Area minigame terminals' result lines, spoken by `npc_bark` (chains 24056-24057). Owns screen ids 200020-200029." and
     `INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (60120, 200020, 'Minigame round trip complete: the server recorded your win.', 0, 0);`
     `INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (60120, 200021, 'Minigame round trip complete: the server recorded your loss.', 0, 0);`
     Then the footer `SELECT pg_catalog.setval('dialog_screens_screen_id_seq', 200010, true);` becomes `200021`.
  7. `crates/content-engine/tests/it/interact_tag_linter/allowlist.rs`: after the `("debug_area_ring_chains.sql", 13810..=13818)` line, add
     `| ("debug_area_minigames_chains.sql", 24050..=24055) // DebugArea_Minigame_*: templates 1600-1605 default INT_MinigameLivewire (MG-74)`
     with a two-line comment above it in the style of the `debug_area_plaza_chains.sql` block (reusable terminals, no mission, the cursor bit is a template default).
  8. New `crates/cell-content/src/cell/content/chain_replay_tests/minigames_debug_area.rs`, registered in `chain_replay_tests/mod.rs` as `mod minigames_debug_area;` directly after `mod marsh_barks;` (alphabetical; before `mod minigames_m582;`).
- **Test (type 6 chain-replay with live-DB SQL, the C-rules four-assertion shape).** Reuse `super::livewire_pairs::{resolve_interact, execute_and_drain_starts}` (both `pub(super)`) and `super::super::engine_loader::load_single_chain_for_test`; `crate::test_support::require_db_or_skip` opens each test. Do not copy the harness. One table `const TERMINALS: [(i32, &str, &str); 6]` of (launcher chain, game, tag) drives:
  1. `live_db_each_debug_area_terminal_starts_its_game`: for each row, `resolve_interact(&pool, chain, tag, &[])` returns exactly one `Action::StartMinigame` with `minigame_type == game`, `difficulty == 1`, `on_victory_chains == [24056]`, `on_failure_chains == [24057]`.
  2. `live_db_debug_area_terminal_ignores_other_tags`: each launcher resolves nothing for the tag `DebugArea_LivewireTerminal` (the plaza terminal) and nothing for the next row's tag.
  3. `live_db_debug_area_terminal_reaches_base`: for each row, `execute_and_drain_starts(actions)` returns exactly one send equal to `(7301, game.to_string(), 1, vec![24056])` (7301 is `livewire_pairs`' player entity id; keep it as a local `const` with a comment naming it).
  4. `live_db_debug_area_result_chains_bark`: chain 24056's action list is exactly `[Action::NpcBark { screen_id: 200020, speaker: "Terminal".into(), channel: 0 }]`, and 24057's the same with 200021.
  5. `live_db_debug_area_terminals_are_seeded`: SQL on `resources.spawnlist` joined to `resources.entity_templates`: spawns 14100-14109 are exactly the six rows above (ids, tags, template ids), all `world_id = 1300` and `is_stationary`, each template has `interaction_type = 256` and the listed `display_name`; `resources.dialog_screens` rows 200020 and 200021 carry the two texts.

  On revert of the chain file, tests 1-4 panic on a missing chain; on revert of the spawn or template file, test 5 fails; on revert of the screens, test 5 fails.
- **Lanes (all must pass):**

  ```powershell
  Set-Location <wt>; cargo fmt -p cimmeria-cell-content
  Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings
  Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 minigames_debug_area
  Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine --test it interact_tag
  Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell debug_area
  Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 debug_area
  ```

  The last two are regressions: the Debug Area placement, reach and lineup guards read every world-1300 seed row, and the live-DB Debug Area tests count their own blocks. A failure there means a row is malformed or collides; report it rather than editing those tests.
- **Docs:** none in this packet; MG-74b writes them.

## MG-74b: Debug Area minigame terminals (docs)

- Agent: packet-coder · Host: windows · Wave 6 · Depends on: MG-74a merged into `minigames/integration` · Status BlockedDependency
- Worktree `mg-74b` · Branch `minigames/mg-74b-debug-area-docs` · Commit `MG-74b: document the Debug Area minigame terminals`
- **Rules (inline):** PowerShell only, `Set-Location <wt>;` on every command. Docs are **CRLF**: write with the Edit tool on the existing files and keep CRLF; check with `[IO.File]::ReadAllText` (absolute path) that no bare LF was added. Keep the target file's line wrapping (debug-area.md wraps prose near 78 columns; tables are one line per row). Never hand-edit a `<!-- gen:NAME -->` block. No `git stash`, prune or push; one local commit. Doc-update-map row "Game systems, content chains, or content-engine actions" (`docs/content/`).
- **Files:**
  1. `docs/content/debug-area.md`:
     - In the id-block list (the bullets after "Each packet's rows are in their own seed files:", ending with the DA-10 bullet), add a last bullet: "- MG-74 (the minigame terminal row, minigames campaign): `entity_templates_debug_area_minigames.sql` (templates 1600-1605), `spawnlist_debug_area_minigames.sql` (spawns 14100-14105) and `debug_area_minigames_chains.sql` (chains 24050-24057), with dialog 60120 and screens 200020-200021, inside the block templates 1600-1609, spawns 14100-14109, chains 24050-24069. See Minigame terminals." with "Minigame terminals" linking the anchor `#minigame-terminals`.
     - In "Which station tests what", the Minigames row (Station "Z2 Livewire terminal", Steps "DA-U10") gets Station "Z2 Livewire terminal; Z2 minigame terminal row (MG-74)" and Steps "DA-U10; the minigames lab recipes" with the last three words linking `../analysis/minigames/uat-lab.md` (MG-73 writes that file; if it does not exist yet, leave the words unlinked).
     - A new `### Minigame terminals` subsection directly before `### Services copied from the hub`: one paragraph ("Six wall terminals on the plaza's north edge (z -903, facing south) start each minigame the minigames campaign ported, plus the Hack shell. A click fires the terminal's `interact_tag` chain, which starts the game at difficulty 1. A win speaks 'Minigame round trip complete: the server recorded your win.' in chat (chain 24056), a loss or the shell's 60 s timeout the matching loss line (chain 24057). The terminals belong to no mission; the cursor bit is INT_MinigameLivewire (256) on every one, because that is the only minigame bit whose click path is confirmed."), then the six-row table from MG-74a (spawn, position, template, tag, game), then the note: "> **Placement is unchecked in the client.** The row was probed on `ihpet_crater_light.nav` (every terminal on the plaza's floor, x 252 left free as the walkway to the dummies), not walked in the client. The `display_name` labels are also unverified on a `being`-class object; the lab recipes check both."
  2. `docs/analysis/debug-area/README.md`, the "Id blocks" table: a last row `| Templates 1600-1609, spawns 14100-14109, chains 24050-24069, dialog 60120, screens 200020-200029 | MG-74 (minigames campaign) |`.
- **Test:** none (docs only). Run `pwsh tools/lint-md.ps1` on both files and fix what it reports in the lines you added.

## Wave 7: UAT, docs and close-out

Order: MG-71, MG-73, MG-72, the campaign review loop (skill § 4), then MG-99. Each packet merges into `minigames/integration`; the campaign's one PR opens after MG-99.

### MG-71: cross-game TCP session test and REPL UAT index

- Agent: packet-coder · Host: windows · Wave 7 · Depends on: waves 3-6
- Worktree `mg-71` · Branch `minigames/mg-71-session-uat` · Commit `MG-71: every server-validated game survives a real SFS session`
- **Files:** new `crates/minigame/src/minigame/server/ported_games_tests.rs`, registered from `server/mod.rs` under `#[cfg(test)]`. For each game whose `validation()` is `"server"`, the test drives a real loopback SFS session through the existing listener test harness (copy the setup from `server/listener_tests.rs`, the Hack flow): login with the ticket, `joinOK`, the first golden move, then a client `victory`, which must be refused with no result sent.
- **Docs:** new `docs/analysis/minigames/uat-repl.md` (CRLF). For each game: the golden scripts, the command that replays them (`pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-minigame --bin minigame_repl -- --script <file>`), and what a human checks in the transcript. This is the REPL-style logic UAT that [AGENTS.md](../../../../AGENTS.md) asks for.
- **Lanes:** C2 for `cimmeria-minigame`.

### MG-72: docs sweep

- Agent: documentation-writer · Host: windows · Wave 7 · Depends on: waves 1-6
- Worktree `mg-72` · Branch `minigames/mg-72-docs` · Commit `MG-72: minigame docs match the shipped games`
- **Scope:** walk [doc-update-map.md](../../../agents/doc-update-map.md) for the campaign's union of changes.
  - [minigame-system.md](../../../gameplay/minigame-system.md): the game table (validation, tiers, cheats), the launch flow, failure chains and the flags, and a "Testing by hand" line pointing at the Debug Area terminals ([debug-area.md § Minigame terminals](../../../content/debug-area.md#minigame-terminals), MG-74).
  - [minigame-wire-formats.md](../../../reverse-engineering/findings/minigame-wire-formats.md): the per-game message tables, linking the specs.
  - [CAT-K-minigame.md](../../../security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md): the status of each finding.
  - `crates/minigame/README.md`, if one exists, or the crate docs in [crates/README.md](../../../../crates/README.md).
  - The `minigame-systems-advisor` memory: `game-implementation-status.md`.
  - The spec fixes the G1 report lists as owed at close-out.
  - Then `python tools/docs-gen/regen.py --skip-crate-graph` and `--check`, and `pwsh tools/lint-md.ps1` on every touched file.
- **Not here:** `docs/gap-analysis*` and `docs/project-status.md` (MG-99 only).

### MG-73: lab UAT recipes (ask-user-first)

- Agent: documentation-writer · Host: windows · Wave 7 · Depends on: waves 3-6
- Worktree `mg-73` · Branch `minigames/mg-73-lab-uat` · Commit `MG-73: lab UAT recipes for the minigames`
- **Files:** new `docs/analysis/minigames/uat-lab.md` (CRLF). Add one recipe per playable path:
  - the Livewire cellblock pair;
  - each ported game (CrystalGame, Alignment, GoauldCrystals, ConverseBasicHumanoid, Livewire) through its Debug Area terminal (MG-74: `.gotolocation DebugArea`, then the plaza's north row; spawns 14100-14104, tags `DebugArea_Minigame_<Game>`), with the win line (screen 200020) and the loss line (screen 200021) as the expected chat;
  - mission 582/2307 (GM-set the mission active: it has no offer path, M10);
  - ability 1327 or 2089;
  - Prison Boot: on zone load while mission 689 is open the wearer cannot walk but can use item 3438; a Livewire win frees them; a minigame window holds movement (MG-63a);
  - the shell 5 s refusal and the 60 s server timeout (MG-64), launched from the Debug Area Hack terminal (spawn 14105); the timeout must speak the loss line;
  - D-MG11's dialog switch, on and off;
  - the MG-74 placement check: each of the six terminals stands on the plaza floor, shows its `Minigame: <Game>` nameplate (`display_name` on a `being` object is unverified), and shows the Livewire cursor on hover.

  Each recipe gives the steps, the expected client view, the server-log lines (`reason=` values), and the SigNoz query. It is written for a Haiku `lab-driver`, with the gotchas first: stand 4-6 m from targets, clear dialogs first, and verify the driver's claims against the server log.
- **Rule:** the first line of the file says **"Ask the user before any lab use."** This packet writes recipes only and runs nothing.
- **Docs:** [unified-uat.md](../../../guides/unified-uat.md) is MG-99's job; link to it from here.

### Campaign review

Run per the skill § 4 over `minigames/integration` against `main`, in an `ops` worktree named `minigames-review`. Its fixes merge into `minigames/integration` before MG-99.

### MG-99: close-out

- Agent: documentation-writer · Host: windows · Wave 7 · Depends on: the campaign review
- Worktree `mg-99` · Branch `minigames/mg-99-close-out` · Commit `MG-99: minigames campaign close-out`
- **Files:**
  - `docs/gap-analysis*` and `docs/project-status.md`, changed once: the minigame rows, saying what is server-validated, what stays client-declared, and what is owed;
  - [unified-uat.md](../../../guides/unified-uat.md): a "Minigames" section pointing at `uat-repl.md` and `uat-lab.md`, with the owner-run lab steps marked ask-first;
  - this campaign's README: every row's final status, the missing-content register with its final verdicts, and the open owner decisions;
  - `RESUME.md`: phase closed.
- **Then (ops):** one PR from `minigames/integration`; after it merges, retire the worktrees (`rm-worktree.ps1 --merged-into <PR> <name>...` for every packet worktree, then `mg-integration`); empty `.claude/state/active-campaign`; post the board handoff.
