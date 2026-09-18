---
name: content-engine-condition-gotchas
description: Adding a content-engine Condition — the fail-open loader trap, where world ids actually live, and the 17-site populator contract
metadata:
  type: project
---

# Content-engine `Condition` gotchas

Learned adding `Condition::World` (Harset H07, commit `03ad0fc1`).

## A rejected condition row UNGATES the chain (fail-open)

`build_chains_from_rows` (`crates/content-engine/src/loader/mod.rs`) builds conditions
with `filter_map`: when `convert_condition` returns `None` the **row** is dropped with a
`warn!("Unknown condition_type, skipping")` and the **chain is kept**. So a chain whose
only condition is malformed loads with `conditions: []` and fires unconditionally.

**Why this matters:** the instinct "reject the bad row loudly" is backwards for any
*gating* condition. Rejecting turns a typo into "fires everywhere"; accepting it and
answering `false` in the evaluator turns it into "never fires". For a door teleport or a
world gate, never-fires is strictly safer. Prefer: build the condition, `warn!` at load.

**How to apply:** whenever a new loader arm has a validation branch, ask what the chain
does with the row gone — not just whether the operator was sane.

## Fail-closed is the minority convention, and that's deliberate

`MissionStatus`/`StepStatus`/`ObjectiveStatus` all `.unwrap_or("not_active")`, so they can
fail **open** on an unpopulated context (`mission_X neq active` is true when nothing
populated it). Only `StatBelowMax` and `World` fail closed. Reviewers will ask — say which
side you picked and why in the variant's doc comment.

## Typed field vs `params` key

`ExecutionContext.params` is stringly-typed and read through `unwrap_or`, which cannot
distinguish "unpopulated" from "populated with the default". Use a typed `Option<T>` field
when that distinction is load-bearing. `space_id` is a runtime instance handle
(`(cell_id << 16) | local_index`), *not* a world id — several instances of one world share
a world id, so nothing downstream can re-derive the world from the context.

## World ids live only in the DB

`entities/spaces.xml` carries world **names** only — `WorldDef` had no numeric id before
H07. The id is in `resources.worlds`, column **`world`** (not `world_name`; every loader
aliases it `w.world AS world_name`). Five cell-side loaders JOIN that table and throw the
id away. H07 added `WorldDef.world_id`, stamped at startup by
`SpaceManager::stamp_world_ids` from `spawner::load_world_ids`, plus
`world_id_for_world()` / `get_entity_world_id()` in `space_manager/queries.rs`.

Known bug found in passing: `cell/console/net.rs` binds `world_id` to `e.space_id.0` and
puts that in the `onMapInfo` payload. Use `get_entity_world_id` instead.

## A condition that reads new context state needs all ~17 dispatchers

Production `ExecutionContext`s are built in `cell/content/event_dispatch/*.rs`
(cover ×4, dialog ×2, interaction ×2, inventory ×2, lifecycle ×2, mission ×2, region ×3).
There is **no chokepoint** — each calls `engine.resolve_event` itself, and giving
`resolve_event` a `SpaceManager` would invert the crate dependency. `docs/content/
extending-the-engine.md` says "called from every `fire_*` site" and means it.

`fire_player_loaded` is the special one: it takes `world_name` from its **caller** and can
fire before the entity is in a space, where any space-derived resolution degrades to
`"Unknown"`/`None`. Populate *above* the explicit `set_param` so the caller's value wins,
and fall back to a name-based lookup for the numeric form.

**Better than a 17-way test:** a services-side constructor taking `(entity_id,
&SpaceManager)` that builds the ctx *and* populates, so omission is unrepresentable. Not
done in H07 only because packet H04 was editing the same lines.
