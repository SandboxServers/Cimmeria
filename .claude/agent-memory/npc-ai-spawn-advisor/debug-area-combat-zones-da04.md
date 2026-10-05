---
name: debug-area-combat-zones-da04
description: DA-04 (2026-10-04) Debug Area world 1300 combat zones - pit/arena geometry and spectator spots, west-wing cover rooms, Z9 terrain, lethal-squad damage math, seed-file and shared-pin traps
metadata:
  type: project
---

Measured on `ihpet_crater_light.nav/.occ` + `cover_extract` for world 1300 while authoring
DA-04 (templates 1370-1377, spawns 13600-13641, own files `*_debug_area_combat.sql`).

- **Pit (Z6):** occluder geometry slab top -33.2 for r <= 40-55 m around (250, -725);
  nav floor -32.33. Crater floor 25-40 m above on N/E/S, but the **SW side is a gentle
  ramp** (a210: -32.6 at r60, -26 at r75, -17 at r120): a player there is inside the
  4 u band, so the NID squad sits on the EAST side (x 262), 40+ u from the ramp.
  No cover markers in the pit. **The pit floor is a water collision plane** (y -33.28,
  WaterCollisionPrefab_Square, terrain 10-25 m below; DA-08 finding): NPCs there stand on
  rendered water. DA-08 put ring pads near Z6/Z9: keep hostile aggro radii off them. Players start the fights only within 150 u AoI
  (plaza is ~200 u away).
- **West wing (Z8):** open-air ruin; a wall on x 151 (z -960..-914) splits the hall from
  a west room with a doorway at z -961..-963. The doorway point (204,-926) sits inside a
  wall column, and (185,-928) / (190,-926) are inside props: occluder LoS from them reads
  Blocked. Use (186,-935) for the hall and (156,-962) for the west room's doorway.
  Marker 130000021/8 (High/Best, facing east) holds a rifleman authored 0.8 u behind it.
- **Z9 terrain:** nav and occluder terrain disagree by up to ~2 u around respawner 131
  (`nav_inspect` dy = probe.y - surface.y, so a NEGATIVE dy means the mesh is ABOVE the
  probe). Mobile spawns are grounded onto nav; author near the mesh height. A rise
  between x 438 and 466 blocks the respawner's view of the lethal squad. The pit's
  mesh also undulates ~2 u over its flat water plane (dy -1.9..+0.8 on the west half).
- **Fixes from review (2026-10-04):** fight 2 moved to the west half (44+ u from every
  NID, beside DA-08's pad at (210,-725)); rifleman 3 moved to marker 130000029/1, 37+ u
  from riflemen 1/2 (beyond `MAX_COVER_DISTANCE` 30, so a seeking rifleman never steals
  it). Seeking with a shot walks <= 10 u (`IN_RANGE_MAX_MOVE`), without one <= 30 u.
- **Lethal math:** 559 is `RangedPhysicalDamage`: Focus first, Health only from overflow
  (`overflow*100/F*F/300 + H`). A fresh Commando (760 H / 1570 F) takes about 17 hits;
  4 SMG NPCs (one shot per 2 s AI tick) kill in about 5-10 ticks. NPC damage does not
  scale with level; level is HP only.
- **Seed traps:** `db/database.sql` lists seed files explicitly; a new file needs an `\ir`
  line. DA-01 (#1223) raised the base setval footers to 1399/13799 and made
  `live_db_aggression::seed_overrides_only_the_chain_armed_spawns` skip world DebugArea;
  each DA packet pins its own overrides/radii in its own live-DB test.
- **Lane trap (2026-10-04):** quiet-mode lane jobs in a fresh worktree died in 0.3 s
  ("...log: No such file"); another lane's prune deleted the empty log dir. Workaround
  `LANE_VERBOSE=1 ... > file`. A hand-made worktree also needs the `external/` junction.

Related: [[debug-area-map-survey]], [[npc-vs-npc-1009]], [[cover-peek-los-na23]].
