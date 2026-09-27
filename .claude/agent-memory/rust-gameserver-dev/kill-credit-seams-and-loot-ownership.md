---
name: kill-credit-seams-and-loot-ownership
description: Where kill XP and mission kill credit are decided (grant_kill_xp, credited_player), the four credit call paths, and that corpses have no loot owner
metadata:
  type: project
---

Kill XP and mission kill credit are two separate paths; changing "who gets credit" needs both.

- **XP:** decided once, in `death/side_effects.rs::grant_kill_xp` (via `kill_xp_payout` -> `SpaceManager::credit_recipient`). Every kill reaches it through `resolve_death`, so do not gate `grant_xp` at the `damage_apply` call sites; the DoT pulse and GM `.kill` would miss it.
- **Mission credit (`EntityDeath`):** NOT in `resolve_death` (it has no `ContentEvents`). It is raised by four callers, all through `use_ability::kill_credit::credited_player`: `credit_single_target`, `credit_ground_deaths`, the warmup tick, and `effects/pulsing/tick.rs::dot_kill_credit`. The NPC AI fight tick calls bare `handle_use_ability` except for pets (PT-06).
- `fire_entity_death` uses `killer_entity_id` as the chain source; `IncrementCounter` writes that entity's `counters`. Credit the owner entity, not the pet.
- **No loot ownership exists.** Corpse loot has no owner; `handle_interact` / `handle_loot_item` gate only on range and `player_id`. Any "loot belongs to X" request is a new tagging feature.

**Why:** PT-06 (2026-09-27) found the pet-kill bug spread over these paths; the XP half had one seam, the mission half had four.
**How to apply:** when changing kill attribution (pets, shared tags, group credit), edit `credit_recipient`/`credited_player`, not the call sites. See [[effect-scripts-run-after-the-death-check]].
