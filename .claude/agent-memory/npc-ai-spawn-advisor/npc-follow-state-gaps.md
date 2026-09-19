---
name: npc-follow-state-gaps
description: AiState::Follow after PR #646 (GC1b-0) - use_player, entity_templates.move_speed and the leash spawn-snap skip all landed; the remaining gap is that Follow never auto-resumes after combat
metadata:
  type: project
---

`AiState::Follow` is wired end-to-end: handler
`crates/services/src/cell/service/npc_ai/follow.rs`, dispatched unconditionally
from `npc_ai/dispatch.rs:55,89`; executor arm
`cell/content/executor/world/mod.rs:149`.

**GC1b-0 (PR #646) — all three landed, verified on e824e8a2:**

1. `use_player: true` on `set_follow_target` — parsed at
   `content-engine/src/loader/action.rs:184`, resolved at
   `executor/world/mod.rs:162-178` with an `is_player` guard (a non-player
   trigger entity warns and leaves the target unresolved). Only way to follow a
   player; players carry no `tag`.
2. `entity_templates.move_speed` is a real nullable column
   (`db/resources/Entities/Tables/entity_templates.sql`), read at
   `cell/spawner/npcs.rs:148` (`COALESCE(t.move_speed, 0.6)`) and landed on the
   entity at `cell/space_manager/spawn.rs:169`. Template 10 "Col Marsh (pet)"
   is the only seeded row with a non-NULL value (0.9).
3. `npc_ai_leash` skips the spawn-snap while `follow_target_id.is_some()` —
   `npc_ai/leash.rs:48`.

**Still broken: Follow never resumes after combat.** Threat preemption pushes
Follow → Fighting (`combat/threat/aggro.rs:66-75` lists `Follow` as
preemptable) and `npc_ai_leash:57` ends at `AiState::Idle`, not back to Follow.
`follow_target_id` survives but is inert, and `dispatch.rs:59` only admits Idle
when `aggression > 0 || has_patrol || has_wander` — so a follower that ever
took damage stops following permanently until a content chain re-fires
`set_follow_target`. Only two production sites assign `AiState::Follow`:
`executor/world/mod.rs:189` and the GM console `cell/console/net.rs:428`.

**Clear shape.** `set_follow_target <tag> {}` is the canonical clear: no
`target_tag` and no `use_player` → `resolved_target = None` →
`follow_target_id = None`, `ai_state = Idle`, `nav_path.clear()`
(`executor/world/mod.rs:183-194`). It emits no wire packet and leaves
`last_movement_type = Some(Follow)` stale, which is harmless only because
`broadcast_movement_type(_, None, ..)` deliberately sends nothing
(`cell/abilities/messaging.rs:235-239`).

**Escort speed math.** AI tick is every 20th AoI tick = 2s
(`cell/service/message_loop.rs:126`); movement tick is 100ms. `follow.rs:80`
only re-paths when `nav_path` is empty, so the destination is recomputed once
per completed leg against a stale player position. Default `move_speed 0.6`
= 6.0 u/s vs a player's 8.125 u/s never converges — an escort template needs
`move_speed >= ~0.9`.

Related: [[castle-cellblock-navmesh-components]], [[npc-death-credit-and-respawn-gaps]]
