---
name: interact-range-and-logcapture-traps
description: interact_target_in_range had no same-space check until AT-04; SpaceManager::get_entity searches every space; LogCapture tests flake under multi-threaded cargo test but pass under nextest
metadata:
  type: project
---

- `SpaceManager::get_entity` looks up an id across ALL spaces, and positions are per-space coordinates. So any "is X near Y" check that compares only positions is fooled by a target in another space at nearby coordinates. `interact_target_in_range` had exactly this hole until AT-04 (2026-09-26) added a `get_entity_space_id` comparison. A new proximity gate needs the same check.
- LogCapture-based tests (for example `trainer::tests::outcome_split_player_missing_vs_no_archetype` and `dialog_choice_gate_tests::dialog_choice_for_unopened_dialog_is_rejected`) can fail under `cargo test` with default threading, because the capture is global. Under `cargo nextest` (process per test) they pass. Confirm with nextest before chasing them.

**Why:** both cost a debugging round in AT-04.
**How to apply:** when you write a range or proximity gate, compare spaces. When a LogCapture assertion fails only under `cargo test`, rerun with nextest before treating it as a regression.
