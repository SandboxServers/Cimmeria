---
name: cell-startup-caches-vs-base-roundtrip
description: The cell has a DB pool at startup and ~20 SpaceManager caches, including entity_templates since Harset H03 — don't reach for a cell→base round-trip inside a content-executor action, it breaks chain action ordering.
metadata:
  type: project
---

# Cell startup caches beat cell→base round-trips inside the content executor

`crates/services/src/cell/service/startup.rs` loads roughly twenty DB caches
straight onto `SpaceManager` (`dialog_set_maps`, `mission_defs`, `stargates`,
`ability_defs`, `effect_defs`, `item_defs`, `loot_tables`, `respawners`,
`ring_regions`, `archetype_ability_trees`, and since Harset H03
`spawn_templates` — every `resources.entity_templates` row as a prototype
`SpawnRecord`). The cell **does** have a DB pool at startup.

**Why this matters:** `crates/services/src/cell/messages/base_to_cell.rs` used
to assert "the cell has no template cache", which pushes you toward copying the
GM `.spawn` pattern (`CellToBaseMsg::GmSpawnNpc` → base query →
`BaseToCellMsg::GmSpawnNpcReady`). Inside a **content-executor action** that is
not merely slower, it is wrong:

- The executor runs a chain's actions as an **ordered list** against one
  `&mut SpaceManager` borrow. The action after a spawn is routinely
  `set_aggression` / `generate_threat` / `add_dialog_set` /
  `set_interaction_type`, all of which resolve via `find_entity_by_tag`.
  Defer the spawn and every one of them misses — the chain half-executes with
  nothing but "entity tag not found" debug lines. Not testable-around.
- A reply message carries a `space_id` captured at request time, and instanced
  (per-player) spaces can be torn down in flight.

**How to apply:** for anything a content action needs synchronously, add a
startup cache next to the existing ones. Keep the round-trip only for
*authoring* commands, where a live query reflecting DB edits without a restart
is the point. Accept that a cache is a startup snapshot.

Related: [[stat-with-no-consumer-trap]] — same family of "the plumbing exists
but nothing reads it" checks before building on a claim in a doc comment.
