---
name: leash-and-fight-exit-traps
description: Why Cellblock guards "freeze" off-spawn — fight->Idle and leash never clear nav_path or player threatened_mobs; Idle aggression-0 NPCs are never ticked; find_path start extents (0.5) far tighter than is_point_valid; DT_PARTIAL_RESULT silently accepted
metadata:
  type: project
---

Verified 2026-09-24 against main b0b594e9 (Cellblock aggro/stuck audit).

- **Fight exits park the NPC in place.** `npc_ai/fight.rs:126-176` (threat empty / target dead / gone) -> Idle with no
  home return and no `nav_path` clear; leash (`fight.rs:179-216` + `leash.rs:48`) raw-writes spawn but also keeps
  `nav_path`, so the movement tick (which runs for ANY state with a path) walks it back out. Then Idle + aggression 0
  = excluded by `dispatch.rs:277` = frozen until damaged. This, not navmesh, is the first suspect for "stuck off spawn".
- **Player-side threat is never drained** on leash or fight->Idle — only death/lifecycle call
  `clear_dead_npc_from_all_player_threat`/`exit_player_combat`. Player keeps BSF_InCombat, no regen (`regen.rs:62`).
- **Leashing is not preemptable** in `generate_threat` (`threat/aggro.rs:106-112`).
- **Nav extents mismatch:** `NavMesh::find_path` start lookup uses `START_EXTENTS 0.5` (`entity/src/navigation/mod.rs:51`)
  but `is_point_valid` / spawner `on_navmesh` tolerate 3.0 h / 4.0 above. `on_navmesh=true` does NOT mean pathable;
  check `y - ground_y` in `spawner.npc_behaviour`. Spawn Y is never snapped to the mesh.
- **DT_PARTIAL_RESULT ignored** (`detour_ffi.rs` checks only DT_FAILURE): cross-component chase walks to the island
  edge and logs plain `chase`, no path_fail.
- **Python aggression = EMobAggressionLevel** (1 HOSTILE..5 DEFAULT); default from
  `FACTION_REACTION_TABLE[player=3][mob]` ([3][10]=HOSTILE). Rust `aggression > 0` is the wrong test for 2-5. Python
  `setAggression` also broadcasts GENERICPROPERTY_MobAggression; Rust sends nothing.
- Cellblock: only ArmYourself_NIDGuard (chain 1008) and PRU (chain 1032) ever get aggression; the 12 template-24
  topside guards never proximity-aggro — python parity, python only had death subscriptions for them.

- **Stale velocity = "running in place".** Only final-waypoint arrival/death/respawn/submit zero `velocity`; every
  `nav_path.clear()` stop (attack-in-place `fight.rs:550`, preempt, cover release, leash) keeps the chase velocity,
  which `EntityMoved` re-sends every 100 ms -> client AvatarFilter extrapolates then snaps back.
- `broadcast_movement_type(None)` emits NO wire byte, so the client keeps CombatAdvance/Leash after the fight ends.
- No GM check anywhere in aggro/threat/targeting; GMs are exempt from navmesh snap-back (`client_move.rs:283`), so a
  GM target can sit off-mesh (find_path end-poly fail, LoS Unknown = clear).

Related: [[ai-telemetry-and-aggro-dead-ends]], [[hostility-and-stationary-gates]], [[castle-cellblock-navmesh-components]]
