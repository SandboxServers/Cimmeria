---
name: throttle-key-hides-transitions
description: A per-entity LogThrottle window swallows the row that says the entity's state CHANGED; key it by (entity_id, kind). Plus the two other movement-telemetry traps from the PR #700 review.
metadata:
  type: project
---

A `LogThrottle`-style "first now, then 1 per interval, report
`suppressed = N`" window keyed by **entity alone** hides transitions.

**Why:** in `movement_telemetry`, all three hard-reject outcomes
(`Rejected` / `Recovered` / `CorrectionSuppressed`) were routed through
one per-entity window. An entity on its way to `CorrectionSuppressed`
spends its whole `MAX_SNAP_BACK_CORRECTIONS` budget as ordinary rejects
first — at the 10 Hz client rate that is ~0.5 s, always inside the
window the first reject opened. So the ERROR row that says "the server
has stopped correcting this client" was swallowed and surfaced a second
late. It was caught by an *existing, unrelated* guard
(`identity_propagation::correction_suppressed_carries_account_and_player_id`,
which bursts the budget then looks for the ERROR row), not by review.

**How to apply:** whenever a throttled log has more than one row shape
for the same key, make the shape part of the key
(`HashMap<(u32, &'static str), _>`) and make `forget(id)` a `retain`.
Per-kind windows cost nothing in the steady state (a stuck entity
repeats one kind) and bound the alternating case at one row per kind per
interval. Single-shape callers pass one constant.

## Two other traps from the same area

- **`destroy_space` is a second teardown path.** When the last player
  leaves an instanced space, every remaining NPC is removed there and
  `destroy_entity` never runs for it. Anything released only in
  `destroy_entity` (`movement_telemetry.forget`,
  `movement_validator.forget`) leaks one slot per NPC per instance and
  hands a recycled entity id stale state.
- **A per-entity `SpaceManager` lookup needs the entity to exist.**
  `diagnose_point` / `navmesh_short_hash` / `world_name_for_space`
  resolve through `entity_space`, so a telemetry test that invents an
  entity id the fixture never created silently gets `None` for every
  navmesh field. In `movement_validation` tests, `navmesh_manager()`
  creates entity **100** and only that id is in the meshed space; to
  emit two throttled rows for it, call
  `mgr.movement_telemetry.forget(100)` between them.

Related: [[observability-test-and-throttle-traps]].
