---
name: health-mutation-paths
description: Every code path that mutates HEALTH.cur in the Rust cell, which ones have a ChainEngine, and the three ordering traps (script-after-death, submit threat leak, DoT no-death)
metadata:
  type: project
---

# HEALTH mutation paths + the hooks that can see them

Written for H04 (`entity_health_below` content trigger). Verify line numbers before
acting — they drift.

## Paths that damage/heal a combatant

| Path | File | Has `&ChainEngine`? |
|---|---|---|
| Primary single-target | `cell/abilities/use_ability/handle.rs:618` → `damage_apply::apply_damage_to_target` | no — caller wrapper `use_ability/kill_credit.rs` has it |
| Ground-AoE secondaries | `cell/abilities/dispatch.rs:241` | no — returns dead-ids Vec to caller |
| Cone-AoE secondaries | `cell/abilities/cone_aoe/fan_out.rs:133` | no — stashes `attacker.last_aoe_deaths` |
| DoT/HoT pulses | `cell/effects/pulsing/tick.rs` (`calculate_damage`, raw fallback ~:269) | **no, and none reachable** |
| Content-driven effect | `cell/content/effect_apply.rs:170` `dispatch_by_name` | yes (content executor) |
| Player regen | `cell/service/ticks/regen.rs` | **players only** (`all_player_entity_ids`) — NPCs never regen |
| NPC respawn reset | `cell/service/ticks/npc_respawn/mod.rs:167` | n/a |

`HEALTH.max` is written **only** at spawn (`space_manager/spawn.rs:187/191`). No effect
script touches `max`, so a pct_before/pct_after comparison across one hit always shares a
denominator.

## The five `handle_use_ability_with_kill_credit` call sites (all player-driven combat)

`cell_methods/player/combat/mod.rs:54`, `cell_methods/player/world/auto_cycle.rs:114`,
`cell_methods/player/interaction/interact.rs:135`, `service/ticks/pending_holster.rs:72`,
`service/ticks/auto_cycle.rs:185`. Bare `handle_use_ability` in production is only
`kill_credit.rs:64`, the three inside `dispatch.rs` (ground AoE), and `npc_ai/fight.rs:494`
(NPC attacker — deliberately excluded from content events).

## Three ordering traps

1. **Effect scripts run after the death transition.** In `damage_apply/mod.rs`,
   `target_died` is latched at :179 from the NVP damage only, death transition at :313, and
   `dispatch_by_name` at :508 is *not* inside the `if !target_died` guard (:475). Two
   consequences: (a) an ability whose damage comes only from a `MeleeDamage` script kills
   without ever running `apply_death_transition` (no loot, no threat fanout, no dead-state
   broadcast); (b) a `HealHealth` script on a killing blow can raise `cur` above 0 on an
   entity that already has `BSF_DEAD`. Anything deciding "did this hit kill?" should read
   `combat::is_dead_state(state_field)`, not `HEALTH.cur <= 0`.
2. **`npc_ai_submit` leaks player combat state.** `service/npc_ai/lifecycle.rs` clears
   `npc.threat_list` directly instead of going through
   `combat::threat::clear_dead_npc_from_all_player_threat`. Every aggroed player keeps the
   NPC in `threatened_mobs` → `BSF_InCombat` stuck on, weapon stays drawn, and `regen_tick`
   (which requires `threatened_mobs.is_empty()`) skips the player forever. Submit also
   leaves the player's auto-cycle armed and the NPC on `HOSTILE_FACTION`, so the player
   keeps shooting a surrendered NPC until it dies.
3. **Pulsing DoT never kills properly.** `pulsing/tick.rs` applies damage but calls no
   `mark_npc_dead` / `apply_death_transition`; a DoT that takes an NPC to 0 leaves a
   walking 0-HP entity. Also means a DoT can silently cross a health threshold that a
   per-hit crossing hook will never observe.
