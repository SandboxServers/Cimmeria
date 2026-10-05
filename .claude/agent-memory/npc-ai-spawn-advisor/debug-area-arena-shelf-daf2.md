---
name: debug-area-arena-shelf-daf2
description: DA-F2 (2026-10-05) - Debug Area arena moved to the east shelf (pit is water), ruin walls limit layouts, pull map guards it, no-witness WARN rule (player_present), leash-loop detector ignores target_dead/target_gone
metadata:
  type: project
---

DA-F2 (PR #1244, branch fix/daf2-arena-noise, 2026-10-05), from DA-06's live-client run and the
colo SigNoz on v2026-10-05.1, plus review round 1.

- **The pit is water, players sink.** `.gotolocation` into the pit put the player at y -52.03
  while NPCs stood on the navmesh at -33 (the nav lies on the WaterCollisionPrefab plane). Fluid
  volumes on Ihpet_Crater_Light (9, from the handoff bundle's 14_TRIGGER_ACCESS.csv,
  `SGW_fluids.WaterCollisionPrefab_*`; BW = (ue.y, ue.z, ue.x)/100): the pit (247.7,-39.8,-721.4)
  scale 2.55, a big square (256, 5.9, -563) scale 4.15x2.24 (it causes the terrace gap
  x 185..300), and small circles at z -437..-500. Guard: `arena.rs::every_arena_row_stands_on_terrain_a_player_can_reach`.
- **East shelf.** Flat terrain y -11.12, x 318-395, z -680..-749, a ruin floor: eye-level walls
  everywhere (long E-W wall z -736/-737 x 339-388 with a N-S stub at x 339 to z -739; east wall
  x 393-400), rubble on the north half. Fight 1 lines x 354 (NID) / 378 (Praxis) at
  z -738/-741/-744. With the third pair at z -746, NID3 saw past the stub's south end and pulled
  the fight-2 room's SW corner (x 328-336, z -730..-734, 23-29 u). Fight 2 x 343/364,
  z -696/-700. Praxis aggro_radius 28 (reach guard's 2x margin; DEBUG-only reach).
- **Pull map is the guard, not fixed spots.** `debug_area_combat/arena_pull_map.rs`: Praxis
  removed (they mask player pulls), lone player stepped over a 2 u navmesh grid at 4 height
  hints, NID pulls allowed only in the strip. Runs ~30 s in debug. A layout search reusing ONE
  scene (move entities + `aggro.radius_override`) tried 168 configs in ~100 s.
- **Ring pad 39 for daf1:** no 7 u clear disc on the shelf meets 12 u from fight 2 + 32 u from
  NIDs; best is (331, -693), clear r 6.5, console (334, -691.7).
- **No-witness WARN rule (SUPERSEDED 2026-10-05 by #1265: the AoI clause now tests the shooter only; a player in range of only the target made false WARNs at the arena).** `SpaceManager::player_present(entity, counterpart)`
  (cell-world `space_manager/player_presence.rs`): a player has entity or counterpart within
  their own `aoi_radius` (stale-witness fault), OR entity/counterpart/threat-list key is player
  side (player, pet, deployable, LabDummy). Review round 1 added the in-range clause: the first
  version (involvement only) silenced the broken-witness-list case. Test trap: `scene()` fixtures
  with NPCs created after the AoI pass are "in range" now; put NPC-only cases >100 u away.
- **Leash "loop" was kills.** All arena `event=loop` rows had `target_id 0`, enter rows
  `trigger=target_dead`, `npc_to_spawn` 0. `detectors::leash::counts_toward_loop` skips
  target_dead/target_gone (table test pins every label); `Dropped::merge` keeps
  `target_out_of_aoi` when a corpse is dropped later in the same pass.
- **Docs EOL trap:** `sed -i` on a CRLF-blob doc (npc-ai.md) rewrote it LF (whole-file diff).
  Edit docs with a Python script that preserves `\r\n`.

Related: [[debug-area-combat-zones-da04]], [[leash-reset-na12]], [[npc-vs-npc-1009]].
