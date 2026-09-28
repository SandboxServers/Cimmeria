---
name: same-space-target-gate
description: #906 same-space cast gate lives in fire_los (FireLos::OtherSpace); instanced test fixtures silently split caster and target into different spaces
metadata:
  type: project
---

`SpaceManager::get_entity` searches EVERY space, so any handler that takes a
client target id must compare `get_entity_space_id` itself. For player
`useAbility` the gate is `FireLos::OtherSpace` in
`crates/cell-combat/src/cell/abilities/use_ability/fire_los.rs`, evaluated by
`refuse_without_line_of_sight` (launch in `handle.rs`; warmup fire already
interrupts on space in `warmup/tick.rs`). Refusal = `onErrorCode(0, id, 0)`
InvalidEntity + `abilities` DEBUG `cast_refused reason=target_other_space`.
The auto-cycle tick (`crates/cell/.../ticks/auto_cycle.rs`) filters the live
target by space and stops the loop.

**Test-fixture trap (found 2026-09-28, #906):** a `SpaceManager` fixture that
declares a world `Instanced="true"` with no startup space makes EVERY
`create_entity` / `spawn_npc` open a brand-new space. The use_ability
`make_mgr` and cell-methods `make_mgr_with_player` did this, so ~7 combat tests
had caster and target in different instances and only passed because the
cross-space hole existed. Both fixtures are now `Instanced="false"` with a
startup space. For a deliberate second instance use `allocate_space_id` +
`create_space_instance` + `create_entity_in_space`.

**Why:** a new cross-space gate will "break" tests whose fixture was wrong all
along. **How to apply:** when a same-space check fails existing tests, check
the fixture's `Instanced` flag before touching the gate.
