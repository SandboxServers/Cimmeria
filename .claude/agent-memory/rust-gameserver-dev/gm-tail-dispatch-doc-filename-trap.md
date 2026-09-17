## Two similarly-named protocol docs — don't confuse them

`docs/protocol/` has both `client-method-dispatch-table.md` (SGWPlayer
interface CellMethods, indices 0-66-ish, the *interface* table) and
`cell-method-dispatch-table.md` (the full SGWPlayer + SGWGmPlayer tail,
including the 109+ GM-gated block). A task brief that says "index 221 is
documented at docs/protocol/client-method-dispatch-table.md:692" is
citing the wrong file by name — the GM tail (indices 109-225ish,
`gmMissionAssign`...`changeCoverStanceWeight`) lives in
`cell-method-dispatch-table.md`. Grep the actual index/method name across
both files before trusting a filename in a handoff packet.

## GM tail index → def-line offset counting convention

`crates/services/src/cell/cell_methods/gm/mod.rs` constants are commented
as `def line N. Offset K` where `index = 109 + K`, K counting *every*
`<Exposed/>` method in `SGWGmPlayer.def` document order starting at 0 for
`gmMissionAssign` (line 65) — including ones with no Rust handler yet.
To compute a new offset: find the nearest already-documented constant
above/below your target in the .def file, then count `<Exposed/>` blocks
between them inclusive. Example: `testLOS`=216 (offset 107, def line 619)
to `onPhysics`=221 (offset 112, def line 645) — five methods in between
(`toggleCombatLOS`, `trackMob`, `onXRayEyes`, `onInvisible`, then
`onPhysics` itself) exactly matches offset 107→112. The
`gm_indices_match_def_document_order` test in `gm/tests/mod.rs` re-asserts
every constant against this scheme — always add a new `assert_eq!` line
there when adding a GM index, not just the constant.

## Movement-validator bypass pattern (per-entity flag, checked pre-Layer-1)

`space_manager/entities.rs::apply_client_position_update_at` runs 4
layers (bounds/navmesh/speed/teleport) gated on a `CellEntity` bool
checked right after the block that resolves `bounds`/`last_valid` but
AFTER `touch_clock` runs (keep the clock ticking even on bypass, or a
future re-validation sees a huge stale dt). The bypass must still call
`update_entity_position` (not just return Accepted) — otherwise
`last_valid`/the spatial grid freezes at the bypass-start position, and
re-enabling validation later sees a huge "teleport" from the frozen point
and rubber-bands the entity back.

Reference implementation: `CellEntity::movement_unrestricted` (issue:
onPhysics/#gmsetfly/#gmsetghost, PR native-onphysics-fly-ghost),
`crates/services/src/cell/cell_methods/gm/physics.rs`.

## Any validator bypass that skips Layer 1 must keep the is_finite() gate unconditional

Security review caught this on first pass: skipping bounds/navmesh/speed/
teleport for a flagged entity is fine, but if `is_finite()` is bundled
inside "Layer 1" and skipped too, a NaN/Infinity position writes straight
through (`update_entity_position` does zero sanitization). This doesn't
corrupt the spatial grid (float->int cast saturates) — the real damage is
that it **poisons that entity's own kinematics state permanently**: any
`distance_to` computed against a NaN last-position is NaN, and EVERY
IEEE754 comparison against NaN (including `distance > TELEPORT_JUMP_UNITS`)
is `false`. So once physics/validation is restored, the hard teleport
reject silently stops firing for that one entity until it disconnects
(clears its `move_clock`/`CellEntity`) — a much worse bug than "the bypass
had a gap," because it outlives the bypass itself.

Fix pattern: pull `is_finite()` out as its own unconditional check, run
before ANY per-entity bypass branch, independent of the bypass flag. Don't
rely on it being folded inside whatever "Layer 1" does, because Layer 1
itself is what the bypass is trying to skip.
