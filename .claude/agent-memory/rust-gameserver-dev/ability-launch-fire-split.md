---
name: ability-launch-fire-split
description: Since AT-10 handle_use_ability is launch-only; post-launch work (ammo, Ability_End, damage) lives in use_ability/fire.rs and may run a tick later; traps for tests and callers
metadata:
  type: project
---

Since AT-10 (2026-09-26) `handle_use_ability` returning `true` means "committed" (cooldown charged), NOT "damage resolved". A positive-warmup ability parks a `PendingCast` and `use_ability/warmup/tick.rs` fires it via `fire::fire_cast` later. Anything added to "after the hit" must go in `fire_cast` (or around it in the tick), never after `handle_use_ability` returns.

**Why:** the pre-AT-10 code resolved in one pass, so charged abilities hit at charge start; callers (kill credit, ground AoE) assumed synchronous damage.

**How to apply:**
- Tests: step time with `resolve_warmups(now, ...)` (cfg(test) re-export), not sleeps.
- Ground AoE collection (`all_npc_entity_ids`) only sees `class_id == 0x04`; plain `create_entity` NPCs are invisible to it — set `class_id` or use `spawn_npc`.
- `handle_use_ability` redirects 592 to the weapon's RANGED ability, so never match a parked cast by the client's ability id (a review found that bug in AT-10's first cut).
- Heredocs with backticks+apostrophes can break the Bash tool's parser; write python edit scripts with the Write tool instead.
- Related: [[cell-entity-direction-semantics]], [[witness-entity-method-dual-fn]].
