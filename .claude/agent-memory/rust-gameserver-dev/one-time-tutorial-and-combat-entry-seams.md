---
name: one-time-tutorial-and-combat-entry-seams
description: CS-03 one-time tutorials (sgw_player_tutorials, show_tutorial, tutorial_shown) and the player_entered_combat trigger - where each piece lives, the DB-decides-first round trip, and the tooling traps hit while building it
metadata:
  type: project
---

Learned implementing Class Start v6 CS-03 (2026-10-05).

- **Round trip:** cell `executor/tutorial.rs` marks `CellEntity::shown_tutorials`
  then sends `RecordTutorialShown`; base `world_entry/shown_tutorials.rs`
  does one CTE (owner check + `INSERT ... ON CONFLICT DO NOTHING RETURNING`)
  and answers `TutorialRecorded{First|AlreadyShown|Refused}`; only `First`
  calls `send_dialog_display` (player as speaker). `Refused` un-marks.
- **Hydration:** `player_init_row` ARRAY subquery -> `InitPlayerState.shown_tutorials`
  (7 struct literals in `crates/cell/.../base_messages/tests`), merged with
  `extend`, not replaced.
- **Condition context:** `populate_world_context` also fills
  `ctx.shown_tutorials`, so every dispatcher in `world_context_contract_tests`
  ALL gets it for free; it fails closed when `None`.
- **Combat entry:** `enter_player_combat` (cell-combat, sync) pushes onto
  `SpaceManager::pending_combat_entries`; `content::fire_pending_combat_entries`
  drains it on the 100 ms tick in `message_loop.rs`. Duel `enter` does not push.
- **Loader:** `convert_condition` parses the operator with `?` first, so an
  unknown operator DROPS the row (ungated chain). `tutorial_shown` is handled
  before that parse. Non-tutorial ids are refused in
  `refuse_chains_with_unknown_tutorials`, called from `engine_loader` only.
- **Tooling traps:** a Python `open(...,'w')` without `encoding='utf-8'`
  writes an em dash as cp1252 0x97 and breaks rustc; `cat > /dev/null;` before
  a heredoc hangs the Bash tool on stdin; `\` line continuations inside a
  Python `"""` string in a heredoc vanish. Write edit scripts with the Write
  tool into the scratchpad and run them with `py`.

Related: [[ability-grant-provenance-seams]], [[python-write-mangles-utf8-and-crlf]].
