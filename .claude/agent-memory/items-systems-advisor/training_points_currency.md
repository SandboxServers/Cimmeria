---
name: training-points-currency
description: Trainer uses training_points (integer point system), NOT Naquadah; TrainerResult::NotEnoughMoney in crates/game is a misnomer
metadata:
  type: project
---

The ability trainer uses **training points** (`sgw_player.training_points`, integer), not Naquadah. Each ability costs 1 training point regardless of `abilities.training_cost`.

- `abilities.training_cost` column exists in the DB schema but is **not used** by the Python trainer reference for the `trainAbility` flow.
- `CostToRespec` in `onTrainerOpen` IS Naquadah (wire: `INT32`) — used for the respec flow, not per-ability training.
- `TrainerResult::NotEnoughMoney` in `crates/game/src/interactions/trainer.rs` is a misnomer. The check is `training_points < 1`, not a currency balance check.

The base-side atomic guard:
```sql
UPDATE sgw_player
SET abilities = abilities || $1::integer,
    training_points = training_points - 1
WHERE player_id = $2
  AND training_points > 0           -- currency gate
  AND NOT (abilities @> ARRAY[$1::integer])  -- double-debit guard
RETURNING training_points
```

Anyone implementing respec must not conflate training_points with Naquadah.
