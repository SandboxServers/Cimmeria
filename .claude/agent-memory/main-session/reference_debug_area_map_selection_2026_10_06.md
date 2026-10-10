---
name: reference-debug-area-map-selection-2026-10-06
description: "2026-10-06 QA cooked-map/nav study of Castle, Tollana, Dakara_E1 and Agnos for a future permanent Debug Area; Castle remains the practical first choice pending client memory and clearance tests."
metadata:
  type: reference
---

The detailed, sourced survey and exact navmesh footprint diagrams are in
`docs/analysis/debug-area/map-selection-study.md`. It is research only; world
1300 still uses Ihpet_Crater_Light.

- Current QA chunks and committed navmeshes were directly extracted. Tollana
  has 15 animated ring pieces at three sites, Dakara_E1 has 10 at two sites;
  the `InterpActor` extraction found no animated ring pieces on Agnos. A rig
  does not imply a working server transport or a safe arrival pin.
- Tollana's four southern district centers are near prefab nav islands. Four
  leads 30 m east probe to main component 89 and flat extracted floors. These
  are candidate pads, pending client collision and animation clearance tests.
- Castle's main indoor stations probe to component 250; exterior checkpoint
  and bunker to component 116. Rings connect developers, not NPC navigation.
- Authored cover nodes exist on all four maps; only Castle of these four is
  currently seeded for its world. Cover counts are not validated usable cover.
- The current Debug Area's 161 distinct looks failed the 32-bit client at a
  3,227 MB working set; group switching reached 3,936 MB virtual. These are
  Ihpet measurements, not a comparative Castle/Tollana/Dakara memory result.
  Current AoI is 150 m with a 25 m exit margin and no wall/room filtering.
- The QA client was installed but no client/server session ran in this study.
  Empty-map RAM, ring travel, multi-developer behavior and repeat tours remain
  the selection gate. The reported `someworlds.zip` was not locally available.
