---
name: castle-world8-layout-and-authoring
description: Castle (world 8) floor heights, navmesh components, route and authoring traps found placing the 2026-09-28 population (throne floor is y 38.4, Alpha is entered from the NE ramp, Patrol point sets, empty Ogilvie moniker, nav_inspect quirks)
metadata:
  type: project
---

Learned placing the Castle population (branch content/castle-population, 2026-09-28;
ledger docs/analysis/castle-population/README.md).

**Layout facts not in the CA05 notes**
- Throne Room main floor is **y ~38.4** (x 344-382, z 632-674), not 41.2: 41.2 is the
  west dais (respawner, Access Panel) and 48.2 the east step / south balcony.
- Checkpoint Alpha room (y 55.2) is entered from the **north-east ramp** (Bravo road
  climbs via (836, 48.8, 561) -> (803, 55.2, 520)); its south opening is a 56-60 m
  ramp, not floor. Front Courtyard is flat y 28.18 at x 455-530, z 630-679.
- The Interrogation Block is reached down a stair at (275.6, 70.2 -> 67.2, 1015-1029)
  from a north-south corridor x 268-296; the "Infirmary" point set 2051 is the south
  end of the Copplemann corridor (x 347-362, z 881-908).
- Components: 250 = Armory..courtyard..field; 116 = Bravo, bunker, Alpha; 468 =
  Symbiote pocket; 501 = sealed mirror wing (never place there); 134/409/413 are tiny
  raised pockets a probe can land on by accident.

**Authoring traps**
- Patrol headers use `type 'Patrol', shape 'Path'` (what `.path_add` writes); only
  `type = 'AreaSet'` becomes a client region. Waypoints loop in point_id order; a
  2-point set is back-and-forth. Parallel sets 3 u apart keep a guard pair side by side.
- Heading 0 faces +Z, pi/2 faces +X.
- `DN_npc_int_Ogilvie_Castle` (8895) is an EMPTY string; 7342 is the usable 'Ogilvie'.
  Other Castle monikers: 6964 'Castle Medic', 7703 'Exterior NID Guard', 7552
  'Op-CORE Soldier', 7036 'Sgt. Stanton'. Check `texts.text` is non-empty, not just the id.
- `nav_inspect --probes FILE` wants whitespace `NAME X Y Z` lines; the component it
  prints for an OUT OF TOLERANCE probe is a far-away poly, not the probe's.
- Free id gaps below Harset (2026-09-28): templates 187-199, spawns 213-221 and
  283-299, point sets 2094-2099, points 2431-2499.

**Loot (D-CP09, 2026-09-28):** Castle hostiles roll tables 4 (guard), 5 (veteran/named)
and 6 (PRU); rows roll independently and an all-miss leaves no loot cursor. Drop only
items that work on click. Since #1021 an event-5 consumable works natively when its
ability is not the 597 filler and every effect script is HealHealth/HealFocus/StatBuff
(`consumable_use::classify`); chain 4001 is retired. Unwired boosts (Stealth 6206) are
ALSO blueprint inputs, so a 'blueprint component' rule alone lets them through. Mission
grants on 169/170/171 are add_item in death chains, not loot, so loot on them is safe.

**How to apply:** start any new Castle placement from the ledger's per-zone table and
the live-DB clearance guards in `live_db_castle_population.rs`; they encode the
respawner / ring-pad / mission-actor radii. Related: [[level-is-hp-and-xp]],
[[assist-aggro-na14]], [[faction-derived-aggro-na13]].
