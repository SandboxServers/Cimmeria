# CB-N04a worknotes: Cellblock ambient cast (cells)

Status: CB-U08 UATPending. The lab row has not run; the cast is unverified live.

## What was added

- Spawns 520, 521: template 176, Op-CORE prisoners, `is_stationary`, world 12,
  tags `Cellblock_Ambient_Prisoner1` / `_Prisoner2` (`spawnlist.sql`).
- Holder dialog 60160 (never displayed) in `dialogs.sql`; screen 200160 in
  `dialog_screens.sql` (sequence set to 200160).
- Chain 11010 in `castle_cellblock_chains.sql`: `enter_region('Castle_Cellblock.Region2')`,
  once = true, gate `step_status(639, 2117) = 'active'`, `npc_bark(200160)` in
  chat as `Op-CORE Soldier`.
- Docs: `#### Cellblock ambient cast (CB-N04a)` in `docs/content/mission-chains.md`.

## Tests (chain_replay_tests/cellblock_ambience.rs)

- `barks_fire_once_in_their_phase`: 11010 is once, resolves one NpcBark
  (screen 200160, speaker, CHAN_say). Fails on revert (`engine_for` panics).
- `barks_refuse_adjacent_states`: no resolve when the step is `not_active` or
  `completed`. Fails if the gate is wrong.
- `cast_rows_are_placed_safely`: exactly the two rows, world 12, template 176,
  stationary, 117 m from every NID guard and 22 m from protected anchors.
  Fails on missing rows (count 0) or a row moved off world 12.

## Mutation proof (live-db-test.ps1 cellblock_ambience)

- One edit: 11010 `'active'` to `'completed'` and spawn 520 world 12 to 13.
  Result: `status=failed exit=100`, 3 failed (fire, placement, refuse).
- Undone with the Edit tool. Result: `status=ok exit=0`, 3 passed.

## Telemetry (existing events only)

- Span `content.npc_bark`, INFO `Content: npc bark` (`chain_id`, `screen_id`).
- Executor WARN `reason` = `screen_not_cached` | `empty_text` | `actor_not_player`.
- Spawner per-entity log line: named by CB-N04s, not re-located here.

## Correction (N04m)

- CB-U08 step 4 expects only the 11010 bark line. No message 5040.

## Lane checks (worktree cellblock-cb-n04a2)

- fmt, clippy -D warnings (cell-content): status=ok exit=0 each
- nextest cellblock_ambience | marsh_barks: 17 passed
- live-db cellblock_ambience 3, marsh_barks 14, debug_hub_stationary 1,
  spawnlist_sequence 1: all passed
- `git diff --check` with `core.whitespace=cr-at-eol`: clean
