---
name: debug-area-map-survey
description: 2026-10-04 Debug Area map survey - SGWSpecCoverNode counts per client map, Ihpet_Crater_Light crater-floor layout, navmesh-vs-occluder terrain height gaps, player-safe NPC-vs-NPC faction pairs
metadata:
  type: project
---

Survey for a dedicated Debug Area world (research only, 2026-10-04; recommended map
Ihpet_Crater_Light as a new world id with its own name).

**Cover nodes per client map** (`cover_extract` run into a scratch file; only Castle 8 and
Castle_CellBlock 12 are in the seed): Omega_Site 0, Harset 0, Omega_Site_CmdCenter 0,
Harset_CmdCenter 0, SGC 0, SGC_W1 391, Harset_Market 274, Agnos_Library 288, Castle 3,788,
Sewer_Falls 4,949, Ihpet_Crater_Light 6,324, Dakara_E1 9,847, Menfa_Light 15,492,
Beta_Site_Evo_1 24,758, Agnos 25,292, Lucia 35,854, Tollana 69,054. Cover is map-authored
only: a map with 0 nodes cannot host a cover course without hand-made nodes.

**Ihpet_Crater_Light crater floor** is ONE navmesh component (rank 3 by area, 212k m2): south
walled compound with rooms (gate prefab at its south apex, ~(251, 10.6, -990)), side
compounds, a flat sunken pit (y ~-32, 60 m wide, centre (250, -725)), a 420 m flat terrace
(y 23.1, z -575..-625, x 50..470, gap x 185..300) and a fortress (y 31-42). The big rank-1
component is the outer rim terrain and rank-2 a y 0.2 plane: ignore both.

**Trap: on open terrain the navmesh and the occluder terrain disagree by up to ~3.3 m**
(e.g. (420, -846): nav -0.1, occluder terrain 2.9-3.6; (450, -905): nav 11.3, terrain 14.4).
Interior floors, the terrace and the pit agree within ~1 m. Take terrain spawn heights from
`occluder_extract probe` columns or pick spots where both agree; an LoS probe from a
nav-height point can report Blocked at its own start.

**Sewer_Falls** is flat (99% of its 77.9k m2 main component at y 12-13) and unused, but
cluttered: the largest clear disc is r ~9.5 m.

**Player-safe NPC-vs-NPC pairs** (mutually HOSTILE, neither hostile to players in either
direction): 2-19, 2-27, 2-29, 23-29, 27-29, 27-36. Faction 10 is mutual-hostile with 2, 3 and
11. No seeded template uses 2, 19, 23, 27, 29 or 36, and a non-10 NPC cannot be damaged by
players ([[faction-10-gates-everything]]).

Related: [[npc-vs-npc-1009]], [[cover-behaviour-na22]], [[harset-zone-evidence]].
