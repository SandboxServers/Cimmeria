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

**Adding one is four edits, always the same four** (DU-03 added
`dialog_screen_text`): a `load_*` fn in the matching `cell/spawner/<topic>.rs`,
a `pub use` in `cell/spawner/mod.rs`, a field + `HashMap::new()` in
`cell/space_manager/mod.rs`, and an `if let Some(ref pool) = self.db_pool`
block in `cell/service/startup.rs`. Pick the failure level deliberately:
`monologue_dialog_ids` is `error!` because an empty cache silently strips ~42%
of dialog screens; a cache whose consumer fails closed with its own warn is
`warn!`.

**Dialog screen text specifically is reachable.** `resources.dialog_screens` is
already queried at startup (`load_monologue_dialog_ids`), and
`spawner::load_dialog_screen_text` now caches `screen_id → text` for the
`npc_bark` action. `screen_id` is **globally unique across all 13,467 seeded
rows** (verified by counting the seed), so a flat `HashMap<i32, String>` is
correct — no `(dialog_id, screen_id)` key needed. `dialog_screens.text` is
`NOT NULL`. Cost is roughly 1.3 MB per cell. So a content verb that needs
original 2009 line text never needs a `"text"` param; name the `screen_id` and
resolve server-side.

Related: [[stat-with-no-consumer-trap]] — same family of "the plumbing exists
but nothing reads it" checks before building on a claim in a doc comment.
