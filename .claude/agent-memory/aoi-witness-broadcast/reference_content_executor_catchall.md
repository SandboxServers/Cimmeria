---
name: reference-content-executor-catchall
description: content/executor/mod.rs's Action dispatch match ends in a silent catch-all — unwired Action variants no-op at debug level instead of failing to compile
metadata:
  type: reference
---

File: `crates/services/src/cell/content/executor/mod.rs`, the big `match
action { ... }` inside `execute_actions` (~line 138-570).

The match is **not exhaustive by variant coverage** — it ends with:

```rust
other => {
    tracing::debug!(entity_id, chain_id, action = ?other, "Content: unhandled action");
}
```

So adding a new `Action::*` variant to `crates/content-engine/src/actions.rs`
compiles cleanly with zero executor wiring; it silently becomes a no-op logged
at debug. Confirmed live for two variants as of the Harset H03 packet:

- `Action::SpawnEntity { .. }` (actions.rs ~line 81) — no arm exists anywhere
  in `content/executor/`. Its doc comment says "Counterpart to
  `Action::DespawnEntity`" but nothing spawns anything yet.
- `Action::DespawnEntity { entity_tag }` (actions.rs ~line 97-101) — doc
  comment already promises `despawn_npc`-shaped behavior ("fan `LeftAoI` to
  every current witness, scrub the witness sets, then destroy... routes
  identically" to `DestroyTaggedEntity`), but no arm exists; it silently no-ops
  via the catch-all today. `Action::DestroyTaggedEntity` DOES have an arm
  (`world::destroy_tagged_entity`), but that arm calls bare
  `SpaceManager::destroy_entity` — NOT the witness-fanout `despawn_npc` its
  sibling's doc comment describes. This is audit defect H-B6 / overlap row U5.

**Implication for anyone adding a new `Action` variant**: the enum change
alone tells you nothing about whether it's implemented. Always grep
`content/executor/` for the variant name, don't trust the enum compiling.
