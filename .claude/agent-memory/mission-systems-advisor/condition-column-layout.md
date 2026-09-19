---
name: condition-column-layout
description: Which content_conditions column each supported condition type reads; malformed rows can be dropped while leaving the chain ungated.
metadata:
  type: project
---

# `content_conditions` column layout per condition type

Source of truth: `crates/content-engine/src/loader/condition.rs::convert_condition`.
Row shape: `(chain_id, condition_type, target_id, target_key, operator, value, sort_order)`.

| condition_type | target_id | target_key | value |
|---|---|---|---|
| `mission_status` | mission_id | — | `'not_active'` \| `'active'` \| `'completed'` |
| `step_status` | mission_id | step id **as TEXT** | `'not_active'` \| `'active'` \| `'completed'` |
| `objective_status` | mission_id | objective id **as TEXT** | free-form string, compared verbatim |
| `archetype` | — | — | archetype id **as TEXT** |
| `counter` | — | counter name | integer as text |
| `stat_below_max` | stat id | — | — (operator/value ignored) |

An asymmetry that catches people:

- `step_status` / `objective_status` ids are **strings** in `target_key` while
  the mission id is an **integer** in `target_id`.

## Why a malformed row is worse than a broken chain

`convert_condition` returns `Option`. A `None` makes `build_chains_from_rows`
**drop that one condition row and keep the chain** — so a typo'd column does not
disable the chain, it publishes it **ungated**.

Operators: `eq`, `neq`, `gte`, `lte`, `gt`, `lt`.

Missing-context defaults in the evaluator (`conditions.rs`):
- `step_status` / `objective_status` / `mission_status` → `"not_active"` (**fails open** for `neq` gates)
- `archetype` → `-1` (so `neq 8` is TRUE when archetype was never populated)
- `stat_below_max` → fails closed

`fire_dialog_open` and `fire_dialog_choice` do **not** populate `archetype` —
only `fire_interact_tag`, `fire_interact_template`, `fire_player_loaded` do. So
an archetype gate on a `dialog_choice` chain is not just useless, it is
silently always-true.
