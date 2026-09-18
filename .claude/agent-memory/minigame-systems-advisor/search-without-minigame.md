---
name: search-without-minigame
description: Two working no-minigame routes for "search a container / collect an item" mission steps — the chain-1032 interact pattern and NpcInteractionType::Loot
metadata:
  type: project
---

# "Search X" steps do NOT need a minigame

When a mission step says *Search the containers*, *Search someone's quarters*,
or *Collect the footage*, there are two fully working server paths today. Reach
for these before proposing a minigame — they are seed-only, zero Rust.

## Route A — interact → grant → destroy (the chain 1032 pattern)

Canonical worked example, `db/resources/Content/Seed/castle_cellblock_chains.sql`
chain 1032 (Ambernol vial):

- trigger `interact_tag` on the object's spawn tag
- condition `step_status <mission> <step> eq active`
- actions, in the Python-canonical order: `add_item` → `destroy_entity` →
  (optional `set_aggression` / `generate_threat` for an ambush) →
  `display_dialog` → `play_sequence` → `advance_step`

The object needs an interaction bit to be right-clickable at all. The vial
inherits `INT_NormalLoot` from its entity template — see the linter baseline
note at `crates/content-engine/tests/interact_tag_linter.rs:120-133`. For a
new object, set `INT_MissionWorldObject` (1073741824) explicitly, and mirror it
with a login-restore chain (pattern of chains 1045/1046/1065).

## Route B — a real lootable container window

`NpcInteractionType::Loot` is a first-class static interaction type:
`crates/services/src/cell/interactions/dispatch/interact.rs:170-182` routes it to
`send_loot_display` → `onLootDisplay` (flat method 114), and
`interactions/loot.rs:87` handles `lootItem(index)` with take-all race handling
and auto-clears `INT_NormalLoot` when the list empties.

Caveat worth stating out loud: today the only code path that *populates*
`entity.loot` and sets `interaction_type = Loot` is NPC death
(`cell/abilities/loot_drop.rs:103-107`). A world-placed container that is
lootable on spawn has no wiring — that is a small Rust item (populate `loot`
from a `loot_table_id` at spawn time), not a minigame item. Loot tables
themselves already load from `resources.loot`
(`cell/spawner/loot.rs:25-57`).

## What the original did

Abilities 1661 / 2837 "Minigame Win Container" — *"Does 100% damage to
container"* — show the original design ran an Activate-family minigame and
destroyed the container on win. `activateMG.fev` has explicit **search**
variants (`docs/client/audio-voice-inventory.md:515`). So the minigame route is
period-authentic; Route A is the cheap, working equivalent.
