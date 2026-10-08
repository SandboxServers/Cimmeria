# Placements C: world 61 landmark census, fixed points, navmesh and cover

> Type: reference (evidence pack). Packet DK-02. Written 2026-10-06 against `main` @ `ddd549797`, client build QA4046 (the owner's installed copy). The commands that reproduce every number are in the [DK-02 worknote](../worknotes/DK-02.md). The rows built on this evidence are in [A-world61-story-placements.md](A-world61-story-placements.md) and [B-world62-tent-interior.md](B-world62-tent-interior.md).

Positions are BigWorld metres, Y up. UE3 to BigWorld: `bw = (ue.y, ue.z, ue.x) / 100`, applied to actor `Location` values, which are world-absolute in the cooked tiles. Yaw is the UE3 `Rotation.yaw` in degrees.

## What was extracted

| Source | Result |
|---|---|
| `Maps/Dakara_E1` | 401 tiles (400 chunk files plus `Dakara_E1.umap`), 56,763 actors, 0 parse errors |
| Actor classes | StaticMeshActor 23,011; Terrain 10,000; SGWSpecCoverNode 9,847; SpeedTreeActor 9,396; Brush 1,160; PrefabInstance 1,132; PointLight 731; Emitter 503; Trigger 271; AmbientSoundSGW 218; TriggerVolume 51; InterpActor 25; no `PlayerStart`, no `SGWStargate`, no spawn actor of any kind |
| Static meshes resolved to a name | 22,505 direct actors (350 distinct meshes) plus 406 prefab-stub components (14 distinct meshes) |
| PrefabInstances resolved to a template name | 1,132: 716 vegetation (`SPT-`/`Spt-`), 352 props (braziers, torches, wall torches, standing lights), 64 structures, tents, the gate, rings, Ha'taks and two Kismet logic modules |
| `Maps/Dakara_E1_StoryRm` | 2 tiles, 75 resolved static meshes, 7 skeletal meshes, 0 parse errors |
| Kismet | 616 sequences, 7,007 nodes; the gameplay ones are the gate (`SeqEvent_Stargate`), two Ha'taks ("Designer 0: Unhide Hatak", "Designer 1: Hatak Explosion") and two ring transporters ("Outgoing/Incoming Ring Transport"); the rest is weather, sound and tent cloth |

`extract_actors` reports every actor's `name` as its class (`PrefabInstance`, `StaticMeshActor`), so the **mesh and prefab names** needed for a landmark census do not come from that tool. They come from the tagged properties: a `PrefabInstance`'s `TemplatePrefab` import, and a `StaticMeshActor`'s `StaticMeshComponent.StaticMesh` import. The worknote has the recipe.

## Fixed points: the coordinate conversion

| Check | Seed or client value | Map value | Difference |
|---|---|---|---|
| Gate 25 row, [stargates.sql:85](../../../../db/resources/Worlds/Seed/stargates.sql) | (96.174, -15.164, 253.206) | `GLB-StargateBase_Goa01` mesh (96.515, -16.850, 250.855) | 0.34 m in x, 2.35 m in z; the row sits 1.69 m above the base (the row is a stand point above the base, not the origin) |
| Gate volume, point set 1005 | (96.147, -15.164, 253.706) | the same mesh | 0.37 m in x, 2.85 m in z |
| Gate prefab origin | | `GLB-Stargate_Prefab` (99.697, -16.850, 252.675) | 3.5 m in x from the row; the audit's (96.3, -12.5, 252.6) is the gate ring `InterpActor`/mesh pair at y -12.5, 0.1 m from the row in x |
| DHD, `spawnlist` 38 | (98.087, -16.769, 237.335) | `GLB-DHD_00` mesh (97.938, -16.820, 237.584) | 0.15 m, 0.05 m, 0.25 m: **0.29 m horizontal** |
| Ring 1 | `GLB-RingTransporter00_Pf0` (-139.333, -16.365, 48.953), five `InterpActor` at the same point | platform surface at y -16.17 on the navmesh | 0.19 m above the prefab origin |
| Ring 2 | `GLB-RingTransporter00_Pf0` (358.030, -16.210, 102.070), five `InterpActor` | platform surface at y -16.02 on the navmesh | 0.19 m above the prefab origin |

Two independent checks agree to under 0.4 m (the DHD) and to the known stand-point offset (the gate), and both rings sit on the navmesh at the same 0.19 m offset. The conversion is `(ue.y, ue.z, ue.x) / 100`, matching the [arrival-points survey](../../../../.claude/agent-memory/main-session/reference_map_arrival_points.md).

### Gate, DHD, ring and Ha'tak fixed points

| Thing | Position | Note |
|---|---|---|
| Stargate base mesh | (96.515, -16.850, 250.855) yaw 84.4 | `GLB-StargateBase_Goa01`; Kismet events 6100-6113 per the audit |
| DHD mesh | (97.938, -16.820, 237.584) yaw 84.4 | `GLB-DHD_00` |
| Ring transporter 1 | (-139.333, -16.365, 48.953) | Ha'tak 1 (`GA-Hatak00_Pf0`, sequence instance 872) at (-111.710, 353.916, 78.091): 40 m from the ring horizontally, 370 m above it |
| Ring transporter 2 | (358.030, -16.210, 102.070) | Ha'tak 2 (sequence instance 871) at (367.160, 359.597, 136.130): 35 m from the ring |
| Third ring mesh | (140.187, -2.402, -493.569) | `GLB-RingTransporterBase_TC00`, a lone base mesh with no prefab, far to the -z; not a mission device (no `InterpActor` set, no Kismet) |

The audit said each ring "stands under" a Ha'tak. It is 35 to 40 m to one side of it, not directly beneath. The positions in the audit are right.

## Prefab and mesh landmark census (task 1)

The full census is [data/Dakara_E1_landmarks.tsv](data/Dakara_E1_landmarks.tsv) (763 rows: source, mesh or prefab, position, yaw, tile). It holds every instance of the structure classes below; furniture and set dressing (benches, vases, fences, crates, books, vegetation, torches, braziers) are in the per-mesh census [data/Dakara_E1_arch_meshes.tsv](data/Dakara_E1_arch_meshes.tsv) and [data/Dakara_E1_arch_positions.tsv](data/Dakara_E1_arch_positions.tsv) (the `archetype_census` output for the 500 prefab-stub components) or not kept.

The map has **no** `JF-`, `HB-` or `GA-` mesh named for a repository, a med tent, a command post or the Superweapon. The names that exist:

| Class | Mesh or prefab | Count | Where | Used by |
|---|---|---|---|---|
| Gate and rings | `GLB-Stargate_Prefab`, `GLB-StargateBase_Goa01`, `GLB-DHD_00`, `GLB-RingTransporter00_Pf0`, `GLB-RingTransporterBase_TC00`, `GA-Hatak00_Pf0` | 1, 1, 1, 2, 1, 2 | see above | fixed points |
| Tents (Free Jaffa) | `JF-MilitaryTent00` (29 direct + 10 prefab), `JF-Tent00` (36 + 21), `JF-Tent02` (6 + 6), `JF-Tent03` (46 + 1), `JF-Tent01` (2) | 39 / 57 / 12 / 47 / 2 | 39 groups, [data/Dakara_E1_tent_groups.tsv](data/Dakara_E1_tent_groups.tsv) | A-01 to A-03 |
| Tents (merchant) | `GA-MerchantTent00/01/02` | 7 + 1 prefab, 9, 8 | the plaza-side merchant rows; one prefab at (306.7, -17.1, 129.6) | none |
| Tents (Tau'ri-flavoured) | `HB-Tent_Large00`, `HB-Tent_med00`, `HB-Tent_small00` | 2 + 1 prefab, 4, 2 prefab | x 400 to 530, z 450 to 560 | A-08, A-09 |
| Vehicle, portal | `HB-Humvee_02`, `HB-BPortal00` | 1, 1 | (483.9, -6.3, 456.6), (512.2, -4.6, 552.4) | A-07 |
| Containers | `GA-Container00` (34), `HB-Container00/01` (65) | | `GA-` in 15 clusters at the map edge and beside the artillery, `HB-` in 8 clusters in the HB camp | none |
| Weapons | `GA-HeavyArtillary00` (1), `Tol-IonCannon00` (2) | | (282.4, -17.1, 88.2); (46.5, -12.3, -74.7) and (212.2, -12.3, -58.4) | A-15, A-16 |
| Barracks | `GA-Barracks00` (1) | | (261.2, -21.8, 22.8) | none |
| Wall and gate arches | `JF-HighWallArch00` (14), `JF-GuardWallArch00` (6), `JF-SimpleWallArch00/01/02` (78), `JF-LowWallArch00` (26) | | the matched pair at x -212 and x 411; a single one at (72.1, -21.1, 441.7) | A-10, A-11 |
| Large buildings | `JF-LargeBuilding_MainHall00` (1), `FrontEnt00` (7), `SideEnt00` (6), `Column` (12), `JF-Arena00` (1), `JF-Temple00` (1), `JF-Temple02` (2) | | main hall (56.0, -9.3, 568.4); arena (506.8, -4.6, 544.6); temples at (-349.1, -5.0, 443.3), (-165.4, -21.6, 38.5), (484.4, -8.6, 457.7) | none |
| Houses | `JF-MedBuilding01..06` (206), `JF-DecBuilding00/01` (19), `JF-SmallBuilding00/01` (8) | | the walled city | none |
| Bridges | `JF-BridgeLarge_*` (16) | | the courtyard span at (96.9 to 162.7, -21.0, -62.2 to -68.7) and two others | A-14 |

### The four furnished camps

Four sites near the gate hold the same set of Earth-military furnishings inside one to three `JF-MilitaryTent00` meshes (`EM-MedicalBox00`, `EM-Microscope00`, `EM-ComputerTower00`, two `EM-ViewScreen02`, `EM-ShelfBox*`, `EM-Cafeteria_Crate*`, `EM-SupplyCrate*`, `JF-SleepRack01`), plus stacks of `GA-AmmoCrate00` and `GA-MerchantBasket05/07` that read as supply stores. They are direct static mesh actors, so they carry **no** `TriggerVolume` (which is why the audit's trigger-based tent groups missed them).

| Camp | Centre (x, z) | Distance to gate row | `JF-MilitaryTent00` positions (x, z) | `EM-MedicalBox00` | Computer and view screens |
|---|---|---|---|---|---|
| A | (135, 270) | 42 to 51 m | (132.7, 270.8), (129.5, 292.9), (152.6, 282.1) | 3 | yes |
| B | (35, 270) | 54 to 63 m | (49.5, 257.9), (56.7, 279.9), (30.3, 270.7) | 0 | yes |
| C | (214, 228) | 118 m | (213.8, 229.1), (220.5, 199.6) | 3 | yes |
| D | (-18, 205) | 125 m | (-7.9, 233.2), (-18.2, 205.9), (-22.4, 177.8) | 1 | yes |

A and B are symmetric about the gate's x (96.5): A at +42, B at -50 from it. Which is "east" is the [axis question](README.md#the-axis-question). A is the nearer and the only one of the two with medical boxes.

Camp A also has four `JF-MilitaryTent00` copies buried under it (y -40.9 and -55.7 at (153.5, 281.6)); they sit in navmesh components 478 to 480 and are an authoring leftover, not part of the walkable city.

### The tent groups (the audit's "Eastern tents")

[data/Dakara_E1_tent_groups.tsv](data/Dakara_E1_tent_groups.tsv) clusters every tent mesh (all kinds) into 39 groups (70 m linking distance) with centroid, distance to the gate, kinds and the navmesh component under the centroid. 17 groups are in component 279 (inside the wall), 19 in the outer component 324, and one each in 348 and 520. The groups nearest the gate:

| Group | Centroid (x, y, z) | Tents | Distance | Component |
|---|---|---|---|---|
| G01 | 138, -21.4, 282 | 3 military (camp A) | 51 m | 279 |
| G02 | 46, -21.5, 269 | 3 military (camp B) | 54 m | 279 |
| G03 | 80, -15.4, 358 | 2 `JF-Tent03` | 106 m | none within 1 m of the floor |
| G04 | 1, -21.6, 201 | camp D plus merchant tents | 109 m | 279 |
| G05 | 207, -21.5, 207 | camp C plus merchant tents | 120 m | 279 |

The audit's tent groups at (270..320, -18, 80..165) and so on are G09, G12 and G16 and others in this table; they are 200 to 280 m from the gate and are the trigger-volume tents. The mission strings' "Eastern tents" cannot be pinned to any of the 39 (no group is named, and no axis is named in the map).

## Navmesh evidence (`data/spaces/dakara_e1.nav`)

699 components, tiled (2,773 tiles of 38.4 m). Components the rows touch:

| Component | Area | Bounds (x / y / z) | What |
|---|---|---|---|
| **279** | 221,875 m2 | [-214, 424] / [-22.4, 16.8] / [-239, 441] | the walled city: gate plaza, DHD, CS-02 start, camps A to D, rings, courtyard ground |
| **324** | 377,549 m2 | [-496, 661] / [-16.8, -1.4] / [-94, 725] | the outer ground: every far camp, the HB camp |
| 336 | | terrace at y -14.0 above the courtyard | an island |
| 477, 504, 481 | | tent roofs and tables inside camp A | islands |
| 478 to 480 | | the buried tents under camp A | islands |
| 619, 637, 620 | | pockets in the HB camp | islands |

**279 and 324 are not connected.** `nav_inspect --gap-pair 279,324 --gap-h 6 --gap-v 3` finds six places where they come within 6 m: 1.50 m at (-214.2, -13.4, 132.0), 1.82 m at (414.0, -13.2, 199.5), 1.87 m at (412.8, -13.2, 206.7), 2.09 m at (-213.6, -13.4, 139.3), 2.64 m at (74.5, -16.8, 436.6) and 3.53 m at (67.5, -16.8, 435.7). All six are at the three wall arches. So the arches are the wall's gates, and the shipped mesh has no link through any of them.

The CS-02 start point (100, -17.4, 230) is on component 279 as the audit says, but the walkable surface under it is at y -16.8 (`obj_slab`), and the mesh polygon there is at about -17.45, 0.65 m **below** the render floor. The `is_point_valid` window (dy in [-1.2, +4.0]) accepts it, so this is a note, not a defect.

## Cover (task 4)

| Question | Answer | Evidence |
|---|---|---|
| Do `Cache/covernodes_*.pak` hold Dakara_E1 sets? | **No.** The packs hold prefab and mesh templates, not maps. `covernodes_nikols.pak` has 1,332 entries, `covernodes_sdeiter.pak` 1,186, `covernodes_pault.pak` is an empty zip (22 bytes). The entries are named `_<MeshOrPrefab>-<w>-<h>` (for example `_EM-CommandPost00-15-15`, `_AN-Cover_Low_Platform1-14-15`), 1,337 distinct template names in all, with no map or world key. | `zipfile` listing of both files |
| Do any of those template names appear in Dakara_E1? | 58 of the 1,337 names are meshes the map uses (`GA-AmmoCrate00`, `GA-MerchantBasket05/07`, `GA-MerchantCuttingTable00`, `JF-FenceStr00`, `GA-Cover_INT_High08`, `GLB-DHD_00`, `GLB-StargateBase_Goa01`, `Tol-IonCannon00`, `HB-Container00/01`, and others). They cover **3,130 of the 22,505** direct static mesh actors. | name join of the pak entries against [the mesh census](data/Dakara_E1_landmarks.tsv) |
| What is the true Dakara_E1 cover-node count? | **9,847** `SGWSpecCoverNode` actors, **0** `CoverNodeArray` components, grouped by `cover_extract` into **1,643 sets from 36 of the 400 chunks**. Defaults applied: height on 2,432 nodes, quality on 20, width on 281; quality out of range 0; skipped 0. | `extract_actors --summary` and `cover_extract` |
| Is that the audit's number? | The audit said "at least 8,022, 99 parse errors". The Rust reader parses all 401 tiles with 0 errors and finds 9,847. The audit's figure came from the Python reader's summary mode. | worknote |
| Where is it dense? | Cell centres with the most nodes (60 m cells): (273, 92) 378; (324, 89) 364; (274, 271) 329; (206, -86) 320; (-88, 36) 320; (27, -93) 316; (85, -90) 307; (211, -31) 281; (87, 389) 257. The courtyard has 599 nodes within 60 m of its centre, the gate plaza 401, the artillery site 930, camp A 327, the two wall arches 57 and 38, the HB camp **0**. | grid over the actor list |

This agrees with [cover-world-placement.md](../../../reverse-engineering/findings/cover-world-placement.md): the in-map actors are the cover data, the paks are the retired template corpus. `cover_sets` still has no row for world 61 (nobody has seeded them); a seed would be 1,643 sets and 9,847 nodes from `cover_extract --map 61=Dakara_E1=<map dir>`, which this packet did not write.
