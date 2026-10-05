---
name: debug-area-arena-shelf-daf2
description: DA-F2 (2026-10-05) - Debug Area arena moved to the east shelf (pit is water), ruin walls limit layouts, no-witness WARN rule (player_involved), leash-loop detector ignores target_dead/target_gone
metadata:
  type: project
---

DA-F2 (branch fix/daf2-arena-noise, 2026-10-05), from DA-06's live-client run and the colo
SigNoz on v2026-10-05.1.

- **The pit is water, players sink.** `.gotolocation` into the pit put the player at y -52.03
  while NPCs stood on the navmesh at -33 (the nav lies on the WaterCollisionPrefab plane). Fluid
  volumes on Ihpet_Crater_Light (9, from the handoff bundle's 14_TRIGGER_ACCESS.csv,
  `SGW_fluids.WaterCollisionPrefab_*`; BW = (ue.y, ue.z, ue.x)/100): the pit (247.7,-39.8,-721.4)
  scale 2.55, a big square (256, 5.9, -563) scale 4.15x2.24 (it causes the terrace gap
  x 185..300), and small circles at z -437..-500. Guard: `arena.rs::every_arena_row_stands_on_terrain_a_player_can_reach`
  (occluder Terrain layer within 0.5 u).
- **East shelf.** Flat terrain y -11.12, x 318-395, z -680..-749. It is a ruin floor: eye-level
  walls everywhere (long E-W wall z -736/-737 x 339-388; east wall x 393-400 z -680..-735), and
  three -18.9 holes. A grid search with +/-1 u parallel rays found ONE 24 u 3v3 layout: lines x 354
  and 378, z -738/-742/-746. NIDs on x 354 because DA-08's faction-yard pad is (394,-738) in the
  east gap. Fight 2 at x 343/364, z -696/-700 (39 u from the NIDs, best possible). Praxis
  aggro_radius 30 -> 28: `debug_area::reach` uses the scan's 2x radius and the yard's pinned
  faction-10 rows were 58.8 u away.
- **Survey method that worked.** A temporary ignored test on the real .nav/.occ: per cell, the
  highest occluder top the navmesh agrees with (Terrain vs Geometry), a wall map (spans in
  ground+0.6..ground+2.0), then `occ.sight` at eye 1.5 over candidate pairs. Navmesh agreement
  alone does NOT find walls or water.
- **No-witness WARN rule.** `SpaceManager::player_involved(entity, counterpart)`
  (cell-world `space_manager/player_involvement.rs`): entity, counterpart or any threat-list key
  is player side (player, PetState/pet registry, deployable, LabDummy). Gates
  `wire_npc_no_witnesses` (messaging.rs witness_audience) and `abilities.sequence
  outcome=no_witnesses`. NPC-only -> nothing at any level (owner decision).
- **Leash "loop" was kills.** All arena `event=loop` rows had `target_id 0`, enter rows
  `trigger=target_dead`, `npc_to_spawn` 0: post-kill resets. `detectors::leash::counts_toward_loop`
  now skips target_dead/target_gone. Castle/CellBlock also had 93 target_dead leashes in 7 days.

Related: [[debug-area-combat-zones-da04]], [[leash-reset-na12]], [[npc-vs-npc-1009]].
