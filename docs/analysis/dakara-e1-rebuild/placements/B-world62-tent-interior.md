# Placements B: world 62 (Dakara_E1_StoryRm), the tent interior

> Type: reference (placement ledger). Cluster **B** of the Dakara placement pass (packet DK-02). Written 2026-10-06 against `main` @ `ddd549797`. Method and evidence classes: the [Harset METHOD](../../harset-rebuild/placements/METHOD.md). Index: [README.md](README.md). World 61 rows: [A-world61-story-placements.md](A-world61-story-placements.md).

Same rules as cluster A: every row is a guess with an evidence class, nothing was walked in a client, heading is `atan2(dx, dz)`, and things with no evidence are in the README's `## No idea`.

## The room

World 62 is **one** furnished tent and nothing else (the map has two tiles, 75 resolved static meshes, 7 skeletal meshes, 25 terrain sheets, one blocking volume; no `PlayerStart`, no `Trigger`, no `InterpActor`). The props are in [data/Dakara_E1_StoryRm_props.tsv](data/Dakara_E1_StoryRm_props.tsv).

| Fact | Value | Evidence |
|---|---|---|
| Floor | y 0.0, flat, from x 58 to 83.8 and z 17.2 to 42.4 | `obj_slab` on a 32 m box around the tent: 1,273 m2 of near-horizontal surface at y[0.0, 0.5] (floor plus terrain), and the shipped mesh's interior component below |
| Standable interior | `dakara_e1_storyrm.nav` **component 3**: 28 polys, 380.9 m2, x[58.0, 83.8] y[0.1, 0.7] z[17.2, 42.4] | `nav_inspect` |
| The rest of the mesh | component 0 is the open ground outside (8,798 m2, y 0.1 to 0.3, the whole 100 m tile); components 2, 4, 5, 1 are the tent roof and awnings at y 7 to 11 (not walkable in practice) | `nav_inspect` |
| Interior and outside are **separate** nav components | the tent wall separates them; the point (72.1, 0.0, 46.0) beyond the north curtain resolves to component 0 | `nav_inspect` probe |
| Tent | two `JF-MilitaryTent00` static meshes at the same spot (70.6, 0.0, 30.2), rotated 67.5 and -95.6 degrees | `extract` of the map |
| Door markers | two `GA-Awning_Door01`: (70.74, 0.0, 40.08) yaw -90 and (70.52, 0.0, 33.92) yaw 90 | the map |
| Dividers and curtains | `GA-Divider00` at z 31.7 to 32.3 (x 61, 66, 71) and 43.1; `Ga-Curtain01` at (73.5, 4.8, 31.6) and (60.1, 4.8, 31.4); `Ga-Curtain00` at (72.1, 0.0, 44.8) | the map |

Reading of the layout (INFERRED from the dividers and the props): a front room at z 17 to 31.7 holding a table (`JF-Table01` at (71.4, 0.0, 24.0)) with three chairs, two propaganda posters and five skeletal meshes lying on the table top at y 1.1 (maps or scrolls), lit by two point lights and two emitters at the table; and a back room at z 32.3 to 42.4 holding cushions (`Ga-Pillow*` at (73 to 76, 0.1 to 0.3, 34 to 36)), jewelry boxes, scarab pots, cabinets, a sleep rack, a `GA-Hologram_Platform00` at (66.2, -0.12, 34.3) and a `GA-Monitor00` on a `GA-Deco_01` stand at (62.5, 1.85, 40.5). The two awnings mark two doors: the inner one in the divider wall (z 33.9) and the outer one at the end of the back room (z 40.1).

CS-02's authored respawner 25 is at (71, 0.05, 30), the tent centre, in the front room just north of the divider.

## Rows

All seven rows are in component 3, so every one is reachable from the respawner. They cannot be reached from component 0 (the outside) and need not be: world 62 is instanced and nobody walks in from outside.

| ID | What | World | X | Y | Z | Heading | Evidence class | Confidence | Checks run | How to verify in-client | How to correct |
|---|---|---|---|---|---|---|---|---|---|---|---|
| PL-DK-B-01 | Respawner 25 (existing seed row) | 62 | 71.00 | 0.05 | 30.00 | n/a | AUTHORED | HIGH for walkable | on-mesh, component **3**, h 0.00 m, dy -0.13 m; inside the tent footprint at its centre | Die inside the tent; you stand at the tent centre | `db/resources/Worlds/Seed/respawners.sql` row 25; no change needed |
| PL-DK-B-02 | Exit flap 1 (`_FromCommand`, "Return to Dakara"), outer door | 62 | 70.74 | 0.00 | 39.40 | 3.1416 | MAP-MARKER (`GA-Awning_Door01` at (70.74, 0.0, 40.08), the door at the end of the back room) + MAP-GEOMETRY | LOW-MEDIUM | on-mesh, component **3**, h 0.00 m, dy -0.10 m; 0.7 m inside the awning; heading faces into the room | The door at the far end of the room from the table | DK-04 flap spawn |
| PL-DK-B-03 | Exit flap 2 (`_FromMohkatan`), inner door | 62 | 70.52 | 0.05 | 34.50 | 3.1416 | MAP-MARKER (`GA-Awning_Door01` at (70.52, 0.0, 33.92), in the divider wall) + MAP-GEOMETRY | LOW | on-mesh, component **3**, h 0.00 m, dy -0.26 m; 0.6 m inside the awning; 5 m from B-02 | The opening in the divider between the two rooms | DK-04 flap spawn |
| PL-DK-B-04 | Bra'tac | 62 | 69.50 | 0.00 | 27.60 | 0.5586 | MAP-LANDMARK (the table and its three chairs at (68.7 to 73.9, 0.0, 23.8 to 25.8)) + MAP-GEOMETRY | LOW | on-mesh, component **3**, h 0.00 m, dy -0.10 m; 3.5 m from the table edge, clear of the divider at z 31.7; heading faces the respawner | Spawn, turn toward the table; two Jaffa stand in front of it | DK-05 spawn |
| PL-DK-B-05 | Moh'katan | 62 | 73.60 | 0.00 | 27.60 | 5.4578 | same | LOW | on-mesh, component **3**, h 0.00 m, dy -0.15 m; 4.1 m from Bra'tac; heading faces the respawner | Beside Bra'tac | DK-05 spawn |
| PL-DK-B-06 | Moh'katan's Terminal (`DN_ob_DakaraE1StoryRm_int_MohkatanTerminal`) | 62 | 62.50 | 0.02 | 40.50 | 1.8158 | MAP-LANDMARK (`GA-Monitor00` on `GA-Deco_01`, the only monitor in the room) | LOW-MEDIUM | the stand is **off-mesh** (the prop sits 1.85 m up on its pedestal); the player stands at (64.5, 0.12, 40.0), on-mesh component **3**, dy -0.13 m; heading faces that stand point | The corner of the back room by the north wall | DK-03 prop, DK-05 spawn |
| PL-DK-B-07 | Ba'al hologram (mission 1650 scene, `DN_npc_int_Baal_DakaraE1_Hologram`) | 62 | 66.20 | -0.12 | 34.30 | 1.5708 | MAP-LANDMARK (`GA-Hologram_Platform00`, the map's only one, at (66.2, -0.12, 34.3) with scale z 1.2) | MEDIUM | on-mesh, component **3**, h 0.00 m, dy -0.61 m (the polygon is the platform top, about y 0.5, above the row's y); heading faces the room | The raised disc in the back room | DK-03 template, mission 1650 packet |

The four player-facing NPC and terminal points (B-04 to B-07) leave the table, the divider line and the beds clear. The exit awnings at z 40 and z 34 and the respawner at z 30 are all inside 12 m of each other, so a player who has just entered stands in front of Bra'tac without walking.

## What this answers for DK-04

- **Two flaps in, two flaps out, one interior.** The strings name `_ToCommand`, `_ToMohkatan`, `_FromCommand` and `_FromMohkatan`. The room has exactly two door markers, so the map agrees with two exits. Which exit belongs to which tent is arbitrary (B-02 and B-03).
- **The tents on world 61 are adjacent.** Camp A's three tents are within 26 m of each other ([cluster A](A-world61-story-placements.md#camp-a-the-command-healing-and-mohkatan-tents-a-01-to-a-05)). DK-04's rule "use one shared exit point if DK-02 finds the two tents adjacent" therefore applies: PL-DK-A-04 at (141.0, -20.8, 288.0) is the shared return point. The exit chains do not need to split on mission state.
- **Respawner 25 is correct** (B-01).
