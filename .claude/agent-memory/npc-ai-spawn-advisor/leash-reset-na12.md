---
name: leash-reset-na12
description: NA12 leash/reset policy facts — NPC->spawn horizontal metric with 5 u band + 20 u vertical cap, walk home with evade, 5 s re-aggro window, and the test-clock trap that hides aggro/leash loops
metadata:
  type: project
---

NA12 (branch `npcai/na12-leash-reset`, 2026-09-25) replaced the snap leash. Policy lives in
`npc_ai/leash/policy.rs`; entry in `leash/begin.rs`; tick + arrival in `leash/mod.rs`; target pruning in
`npc_ai/fight_target.rs`.

- **Metric:** horizontal NPC->spawn. Leash if > radius+5 (`beyond_band`), |dy| > 20 (`vertical_cap`), or inside the
  band when the NPC wants to chase and the target is further from spawn than the NPC (`chase_outward`). A
  stationary NPC never leashes by distance; it disengages only via lost target.
- **Radius:** `entity_templates.leash_distance` (nullable, CHECK > 0, not COALESCEd) -> `CellEntity.leash.distance_override`; NULL = 50.
- **Lost target:** dead / entity gone / beyond `aoi_radius` for 5 s. Every threat drop calls `exit_player_combat`;
  every leash entry/arrival calls `drain_npc_from_player_combat` (threat list UNION players whose set names the NPC).
- **Evade:** `generate_threat` returns None for a Leashing NPC (no threat, no player combat) — content threat too.
- **Arrival** needs route finished AND <=1.5 u horizontal; snap fallback on no route or 20 s. Followers reset in place.
- **Trap for tests:** leash clocks use `Instant::now()`, so a compressed-time loop test sees the 5 s re-aggro window
  stay open forever and cannot detect a loop. Age `leash.*` instants by the simulated step (see
  `tests/npc_ai/leash_reset.rs::age_leash_clocks`).
- **Open for NA13:** with an aggressive NPC and an unreachable-but-in-AoI player, the NPC stands Fighting forever
  (no_path); proximity aggro radius/LoS is NA13's fix, not the leash's.

Related: [[leash-and-fight-exit-traps]], [[ai-telemetry-and-aggro-dead-ends]]
