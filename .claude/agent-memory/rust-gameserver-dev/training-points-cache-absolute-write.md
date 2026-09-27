---
name: training-points-cache-absolute-write
description: handle_grant_xp writes sgw_player.training_points ABSOLUTELY from ConnectedClientState.player_training_points, so every other training-point writer must refresh that cache or the next level-up erases its change
metadata:
  type: project
---

`handle_grant_xp` (base-methods `progression/mod.rs`) does
`UPDATE sgw_player SET exp=$1, level=$2, training_points=$3` where `$3` is the
session cache `player_training_points` plus levels gained. It never re-reads the
row. So any other path that changes `training_points` in SQL (trainer purchase
`persist_purchase`, respec, GM `gmGiveTrainingPoints`) must write the
`RETURNING` value back into `ConnectedClientState.player_training_points`
(filtered on `active_player_id == player_id`), or the first level-up after it
silently restores the old count.

**Why:** found while adding `gmGiveTrainingPoints` (2026-09-26); the guard test
`grant_training_points_tests::a_later_level_up_keeps_the_granted_points` fails
when the cache refresh is removed, and nothing else would have noticed.

**How to apply:** any new points/level/xp writer on the base side: add in SQL
(`col = col + $n ... RETURNING`), refresh the cache, and add a live-DB test that
runs `handle_grant_xp` afterwards. Cell mirror is `CellEntity::tree_progress`,
fed by `AbilityGranted` / `ProgressionChanged` / `TrainingPointsGranted`.

Related: [[stat-with-no-consumer-trap]], [[gm-feedback-cell-base]].
