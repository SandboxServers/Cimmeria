---
name: npc-class-filter-and-dead-target-traps
description: all_npc_entity_ids is SGWMob-only (class 0x04) so being-class NPCs like Col Marsh never tick; spawn_npc fixtures force 0x04 and hide it; HEALTH alone is not "dead" for a player target
metadata:
  type: project
---

Found in NA24 (UAT-1, 2026-09-25).

- `SpaceManager::all_npc_entity_ids()` admits only `class_id == 0x04`. `entity_templates.class = 'being'` (0x01) covers
  26 templates: mostly props (crates, consoles, corpses, elevator buttons) but also **Col Marsh (template 10)**. A being put
  in `Follow` by content was never AI-ticked or movement-ticked. NA24 added `ai_driven_npc_entity_ids()` (mobs + beings in a
  non-combat behaviour state) for the AI tick, movement tick and movement detector. Never admit a being in `Fighting`:
  `generate_threat` moves any shot entity to Fighting and a prop would fire the default ability.
- **Fixture trap:** `SpaceManager::spawn_npc` hardcodes class 0x04, and the GC1 escort chain-replay calls the follow
  handler directly, so every escort test passed while the real seed row could not move. Use `spawn_npc_from_record` with
  `class: "being"` to test the seed shape.
- A player corpse can be healed (a chain on `useItem` during the Defeat Window), so `HEALTH <= 0` is not a dead test for
  targets; check `combat::is_dead_state(state_field)` too. Player death now purges the player from all threat lists in
  `resolve_death` (`npc_ai::purge_dead_player_from_threat`).
- NPCs have no `witnesses` set (only players do); "who sees this NPC" is `space_mgr.get_witnesses_of(id)`.

**How to apply:** any new per-NPC sweep, pick the right id query on purpose; any "is the target dead" check, use the state bit.
