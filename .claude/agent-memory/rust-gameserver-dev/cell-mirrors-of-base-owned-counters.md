---
name: cell-mirrors-of-base-owned-counters
description: The base owns level/training_points/tree spend; the cell holds mirrors that go stale unless EVERY base write path sends a BaseToCellMsg. Player CellEntity::level was 1 for everyone until AT-03.
metadata:
  type: project
---

`sgw_player.level`, `training_points`, and `tree_points_spent` are owned by the base (`ConnectedClientState` + the DB row). The cell's trainer gates (`ability_tree::evaluate_train`) read cell-side mirrors: `CellEntity::level` and `CellEntity::tree_progress`.

Until AT-03 (2026-09-26), nothing set a player's `CellEntity::level`. It stayed at the struct default of 1, so the trainer level gate and the AoI introduce level (`request_entity_update.rs`) both read 1.

The mirrors now update in three places:
- `InitPlayerState`: `level` and `tree_progress`, from `client_ready/player_init_row.rs`.
- `AbilityGranted`: training points and spend, returned by the purchase UPDATE.
- `ProgressionChanged`: sent by `handle_grant_xp` after a persisted level-up.

**Why:** a stale mirror under-counts, so the cell fails closed: nodes show locked and purchases are rejected. The base's `training_points >= cost` guard catches an over-count. The exception is `required_branch_points`, which the base never re-checks.

**How to apply:** any new base-side write to these columns must also send the cell mirror. That includes the AT-08 respec (points up, spend reset), which is the dangerous direction, and any GM `.givetp`. Otherwise the trainer UI and purchase gate drift until relog. Related: [[stat-with-no-consumer-trap]].
