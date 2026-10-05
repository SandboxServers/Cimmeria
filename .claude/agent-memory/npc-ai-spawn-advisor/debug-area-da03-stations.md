---
name: debug-area-da03-stations
description: DA-03 (2026-10-04) Debug Area world 1300 faction yard / AI slope / passive gallery - faction picks that never fight, the yard z shift, terrace trench, gallery counts, nameless hostiles, test seams
metadata:
  type: project
---

Placed by DA-03 (branch debugarea/da03-factions-gallery, 2026-10-04); reference page
docs/content/debug-area.md.

- **Never-fighting friendly/neutral factions.** Factions 1, 5, 6, 7, 9 have no HOSTILE cell
  in their reaction row (no `seeks_npc_targets`) and faction 10 is not hostile to them.
  Faction 3 (what players react as) IS mutually hostile with 10: a "friendly" faction-3 row
  near a hostile pen starts NPC-vs-NPC fights. Non-10 NPCs cannot be damaged at all, so the
  only damageable neutral is a faction-10 spawn with `aggression_override` 3.
- **Ihpet open-ground heights.** Around the plan's yard centre (416, -786) the navmesh drifts
  2-4 m off the terrain north of z -790; rows were moved to z -806..-794. The terrace has a
  trench at z -602 (nav ~15.4 under an occluder top of 23.06): only rows z -592 / -612 walk.
  The flat basin x 110-135, z -720..-670 (y -7.19) agrees within 0.2 m.
- **Gallery.** 101 faction-10 templates on main: 99 placed, 140/141 (children) excluded;
  24 have ability sets (2/3/4/5), 75 fall back to 592; 62 have a NULL or empty `name_id`
  (blank nameplate). The DA-02..04 station templates 1300-1399 are excluded by range.
- **Seed layout.** db/database.sql lists seed files explicitly (no glob), so packet files
  need an `\ir` line placed AFTER the base file whose fixed `setval` footer would otherwise
  lower the sequence again (point_sets.sql's footer is a hard 2122).
- **Test seam.** `castle_standoff.rs`'s `parse_insert`/`seed` are now `pub(super)`;
  `tests/npc_ai/debug_area/` builds a world-1300 scene from the seed files through
  `spawn_npc_from_record` on the real nav+occ and runs `npc_idle_aggro_scan_for_test`.
- **Review of #1222.** Template 24 (NID Guard) carries `assist_radius` 26: placed at the
  front of the pen it was 24.1 u from the shootable pinned Jaffa and only the 4 u vertical
  band (5 cm margin after the navmesh snap) kept it out. It now stands at the back (28 u).
  Reach guards must split damageable rows from hostile ones and read every world-1300 seed
  file (`reach.rs`), and a NEUTRAL pin narrows only what the pinned NPC seeks: it is still
  an NPC-vs-NPC target for factions hostile to 10.
- **Traps hit.** cell-world `live_db_aggression::seed_overrides_only_the_chain_armed_spawns`
  pins every `aggression_override` row and every template `aggro_radius`; a passive gallery
  trips it (DA-04 filters world DebugArea). A fresh worktree needs the `external/` junction
  before `cimmeria-entity` (Detour) builds.

Related: [[debug-area-map-survey]], [[faction-derived-aggro-na13]], [[assist-aggro-na14]].
