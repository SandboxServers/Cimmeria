---
name: witness-entity-method-dual-fn
description: WitnessEntityMethod has TWO witness_entity_method fns (logging wrapper + emitter); idbase selected by entity_is_player
metadata:
  type: project
---

`CellToBaseMsg::WitnessEntityMethod` dispatch has **two** functions named
`witness_entity_method` — a signature change must touch both:

- `crates/services/src/base/world_entry/cell_dispatch/aoi_dispatch.rs` — the
  wire-logging wrapper that `route` actually calls. Logs via
  `wire_log::log_outbound_entity_method` then delegates.
- `crates/services/src/base/world_entry/cell_dispatch/aoi.rs` — the real
  emitter that builds the packet via `build_entity_method_packet`.

**Why:** A grep for `witness_entity_method(` construction sites won't flag the
wrapper as a callsite of the emitter — clippy catches the arity mismatch only
after the emitter is changed.

**How to apply:** When changing the `WitnessEntityMethod` enum or the emitter
signature, update the enum variant in
`crates/services/src/cell/messages/cell_to_base.rs`, the extraction in
`aoi_dispatch.rs::route`, the wrapper `aoi_dispatch.rs::witness_entity_method`,
AND the emitter `aoi.rs::witness_entity_method`.

**idbase selection (post-#278):** the enum carries `entity_is_player: bool`.
Emitter picks `IDBASE_SGW_PLAYER` (61) for player ghosts, `IDBASE_NPC_DEFAULT`
(62) for NPCs. Matters only for method indices >=61 (they encode differently
per idbase). Stamped at construction time from
`space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player)` — compute ONCE
before any witness loop (same for every witness). `WitnessEntityMethod` is
never deferred (no `DeferredAoiMsg` witness variant), so no deferred-path
plumbing needed.

`send_entity_method_to_self_and_witnesses` (in `cell/abilities/messaging.rs`)
is the helper for player state that must reach observers — self + witness
fanout, collapses to witness-only for NPCs (no client). Used by the combat +
death broadcast paths.
