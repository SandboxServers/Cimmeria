# Stat-with-no-consumer trap (Cimmeria stat porting)

A stat existing in `crates/entity/src/stats/` does **not** mean anything reads
it. Several `StatList` entries are wired end-to-end on the *wire* (default in
`StatList::new()`, membership in `PUBLIC_STATS`, an entry in
`mercury/aoi/create.rs`'s initial NPC stat block, a row in a `.speedstats`-style
console dump) while having **zero** server-side read site that applies them to
behavior. The client applies them; the server never did.

Confirmed instance (2026-09-17, P47 `.speed`): `MOVEMENT_SPEED_MOD` (6) and
`ROTATION_SPEED_MOD` (81). `cell/service/ticks/npc_movement.rs` stepped NPCs by
the `CellEntity.move_speed` **field**, never by the stat. P47 added
`effective_move_speed(base, stats) = base * (movementSpeedMod.cur.max(0) / 100)`
to close the movement half; `ROTATION_SPEED_MOD` is still client-applied only
(the NPC tick snaps yaw whole in one tick, no turn-rate integration exists).

**Before porting any stat setter**, grep the constant across `crates/` AND
`entities/` and classify each hit as definition / default / wire-payload /
readout / **actual consumer**. If there's no consumer, a faithful port sets a
stat nothing reads — say so before writing code rather than shipping a test
that only asserts `stat.cur`.

`entities/defs/alias.xml` is the canonical semantics source for what a stat is
*supposed* to do — e.g. `movementSpeedMod`: "multiplies movement speed by
curr/100". Cite that, not a pre-bible protocol doc.

## Related conventions confirmed the same pass

- `Stat::set_current` **clamps** into `[min, max]` silently. Legacy Python
  relies on that; the native `gm/stats.rs` `gmSetHealth` handler instead
  **rejects** out-of-range with feedback. Reject-don't-clamp is the house
  precedent for GM setters. Validate every stat a multi-stat setter touches
  *before* writing any of them, so the write is atomic.
- Publication pattern for a stat change (copy from `cell_methods/gm/stats.rs`,
  not reinvented): mutate → `serialize_dirty()` if `is_player` else
  `serialize_dirty_public()` → `clear_dirty()` →
  `crate::cell::abilities::send_entity_method(target, ON_STAT_UPDATE, payload,
  tx, space_mgr)`. That helper routes direct-to-client for a player and fans
  out `WitnessEntityMethod` for an NPC. `serialize_dirty*` always emits a
  4-byte u32 count prefix, so `!payload.is_empty()` is not a "has entries"
  check.
- `StatList` has **no** empty constructor and no `remove` — an absent-stat
  branch cannot be exercised through a real fixture from outside
  `cimmeria-entity`. Test the formatting/fallback function directly and say
  why (existing precedent: `console/stats.rs::format_stat_line`).
