---
name: sgw-mission-objective-arrays
description: sgw_mission objective-array semantics, sentinel-id exhaustion in 0x7000_0xxx, and the live-DB mission test shape
metadata:
  type: project
---

# `sgw_mission` objective arrays — verified 2026-09-18 against repository DDL

DDL (`db/sgw/Missions/Tables/sgw_mission.sql`, matches live DB exactly):
all four `*_ids` columns are `integer[] NOT NULL` with **no** column default,
**no** CHECK constraint, **no** index beyond `missions_pkey (player_id, mission_id)`.
One FK: `missions_player_id_fkey → sgw_player(player_id) ON DELETE CASCADE`.
`repeats integer DEFAULT 0 NOT NULL` (the #118 column).

**The load-bearing semantic:** `active_objective_ids` is the *full objective
roster of the current step*, not the not-yet-done subset. Two places depend on it:

- `crates/entity/src/missions.rs::MissionInstance::complete_objective` flips
  status in-place and leaves the entry in `active_objectives`.
- `cell/service/base_messages/player_init/mod.rs` (~line 172) rebuilds
  `active_objectives` **only** from `active_objective_ids`, marking an entry
  COMPLETED iff it also appears in `completed_objective_ids`.

`active_objective_ids` is the current step's roster. `completed_objective_ids` may
also contain objectives from prior steps. Hydration recreates resendable objective
rows only from `active_objective_ids`; completed ids outside that roster remain
available to content conditions but are not resent to the client.

**Sentinel-id exhaustion:** the `0x7000_0Xxx` live-DB sentinel space is fully
allocated at the `_X00` granularity (0 through F). New live-DB test families
need a fresh prefix, not another `0x7000_0*` offset. The `ci-live-db` nextest
profile serialises tests because shared resource fixtures can collide.

**`array_length(col, 1)` returns NULL for `'{}'`** — use `cardinality()` in
test SQL assertions.
