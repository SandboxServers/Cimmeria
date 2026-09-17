---
name: facing-preservation-primitive
description: Server-authoritative position writes must use SpaceManager::update_position_preserving_facing — update_entity_position zeroes facing when passed [0,0,0]
metadata:
  type: reference
---

**Block on sight:** a teleport that calls
`SpaceManager::update_entity_position(id, pos, [0, 0, 0], vel)`. That method
writes `direction` unconditionally from its `[i8; 3]` parameter, so the
`[0, 0, 0]` every "I have no new facing to supply" caller passes snaps the
moved entity's facing to north. Witnesses render it; the moved player sees it
on their own avatar.

`SpaceManager::update_position_preserving_facing(entity_id, position, velocity)`
(`crates/services/src/cell/space_manager/entities.rs`, landed 2026-09-17) is the
position-only writer — it never touches `direction`, so the facing survives by
construction rather than by a capture/restore the caller can forget. Both
writers share the private `write_position` tail. `update_entity_position` is
still correct for a caller that genuinely *is* setting direction (the inbound
client-position accept path, NPC movement tick).

History worth knowing: the capture-then-restore workaround was independently
copy-pasted at four call sites (`console/placement.rs::location`,
`console/travel::snap_in_current_space`, `client_move.rs::reject_outcome`'s
recovery relocation) — and was **missing entirely** from the native `gm*`
handlers (`cell_methods/gm/travel.rs`: `gmGotoXYZ`, `gmGoto`, `gmSummon`) for
as long as they existed. A workaround that has to be remembered at each site
will be forgotten at some site; prefer the primitive that cannot be misused.

Related but a different mechanism: the **cross-space** leg carries facing as
`TransferDestination::rotation`, which defaults to `[0.0; 3]` and flows through
`GateTravel` into the destination entity's `direction` unconditionally. That one
still needs an explicit read of the subject's current facing before teardown —
`update_position_preserving_facing` does not help there.

Regression guard: `native_gm_travel_preserves_facing` in
`crates/services/src/cell/cell_methods/gm/tests/travel.rs` (verified to fail on
revert, one arm per handler). Use a non-zero, non-symmetric facing in any new
guard — `[0, 0, 0]` still matches after the bug is reintroduced.

See [[authorized-teleport-paths]] for every path that writes position, and
[[snap-back-termination]] for the recovery relocation this shares.
