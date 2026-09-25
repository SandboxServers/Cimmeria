# Harset placements: method, evidence classes and the guessed-coordinate ledger

> Type: how-to plus reference. Audience: coordinator and worker agents placing Harset content without an in-client walk. Written 2026-09-19 on the owner's instruction: stop waiting for the M0 walk, place from map data and navmeshes, label every guess, keep one list the owner can correct after a playtest.

Every coordinate placed under this method is a **guess with a stated evidence class and confidence**, not a pin. The owner corrects them in one pass after a playtest. Nothing here overrides D-H15 for anything the owner has actually walked; it replaces "wait" with "estimate, label, and make correction cheap".

## The single ledger

Every worker records each placed or changed coordinate in its own file under this directory (`placements/<cluster>.md`, one table row per coordinate) and the coordinator assembles them into [README.md](README.md). Row columns, in this order:

| Column | Meaning |
|---|---|
| ID | `PL-<cluster>-<nn>` |
| What | the spawn, respawner, arrival pin, region or prop, with its seed id (spawn_id, respawner_id, point_set id, chain id) |
| World | 57, 68, 69 or 70 |
| X, Y, Z, heading | the seeded values (BigWorld metres, Y up, heading radians) |
| Evidence class | one or more of the classes below |
| Confidence | HIGH, MEDIUM, LOW, or NO-IDEA |
| Checks run | on-mesh (component id), floor Y from `obj_slab`, same component as the arrival hub, clear of sealed geometry, heading derivation |
| How to verify in-client | one sentence: where to stand and what should be true |
| How to correct | the seed file and row to edit |

**NO-IDEA rule.** If there is no usable evidence, do NOT invent a coordinate to fill a row. Leave the thing unseeded, and list it in a `## No idea` section of the cluster file with the specific reason (which evidence was searched and what was missing). The coordinator surfaces every NO-IDEA item to the owner at the end.

## Evidence classes

| Class | Meaning | Typical confidence |
|---|---|---|
| AUTHORED | an existing seed row, script constant or DB row from the original data | HIGH |
| TELEMETRY | a `last_valid_*` position from SigNoz `movement.validation_reject` logs: the server accepted a real player there, so it is walkable and on-mesh | HIGH for "walkable", says nothing about purpose |
| MAP-LANDMARK | a named prefab mesh from `archetype_census` (for example `GA-Bank00`, `JF-Tent00`, `EM-Quartermaster01`) whose name says what the place is | MEDIUM to HIGH for "the place is here" |
| MAP-GEOMETRY | floor height, doorway, approach direction and sealed-vs-open from `obj_slab`, terrain or BSP | HIGH for Y and reachability |
| MAP-MARKER | a map actor with a position: TriggerVolume, InterpActor door, `SGWSpecCoverNode`, CameraActor | MEDIUM (names are generic; purpose is inferred) |
| SPEC-DESCRIPTIVE | words in the spec, audit, worknotes or original scripts ("left of the gate", "Tau'ri vendors on the lower level") | LOW to MEDIUM |
| INFERRED | reasoned from neighbouring evidence | LOW |

A row needs at least one MAP-GEOMETRY check (floor and reachability) to be above LOW, however good its landmark evidence is.

## Tools and where things are

All outputs are under `C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\navmesh\harset-placement\` (call it `$O`); already generated, do not regenerate unless you need a different map.

| Need | Tool / file |
|---|---|
| Per-chunk OBJ (StaticMesh + Terrain + BSP) for each map | `$O\Harset`, `$O\Harset_CmdCenter`, `$O\Harset_Market`, `$O\Harset_StorageRm` (from `extract_map`) |
| What surfaces exist at a spot and at which heights (true floor Y, sealed geometry) | `obj_slab <chunk-dir> --at NAME=X,Y,Z[,HALF_XZ[,HALF_Y]] --levels 0.5` (also `--column X,Z`, `--line`, `--box`) |
| Is a point on the navmesh, how far vertically, and which connected component | `nav_inspect <nav> --probes <file>` (file lines: `NAME X Y Z`); add `--gaps` for holes |
| Named prefab meshes with world positions (landmarks) | `archetype_census` output: `data/*_arch_meshes.tsv` (one row per mesh) and `data/*_arch_positions.tsv` (every instance) in this directory |
| Actors with positions (TriggerVolume, InterpActor, cover nodes) | `extract_actors <map dir> --json` (actor names are generic) |
| Kismet logic | `extract_kismet <map dir> [--graph]` (mostly ambient sound and door animation in Harset) |
| Real-player known-walkable points | `$O\harset_lastvalid_probes.txt` (33 server-accepted points from 3 days of SigNoz, plus the existing gate, DHD and ring rows). Regenerate with SigNoz `signoz_aggregate_logs`, service `cimmeria-server`, filter `body CONTAINS 'rejected by the navmesh layer' AND space_id = 65544`, group by `last_valid_x, last_valid_y, last_valid_z` (65544 = Harset; `client_x/y/z` are where the player tried to go, so they mark holes) |

Binaries (release): `extract_map`, `nav_inspect`, `obj_slab`, `archetype_census` in `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\agent-a0f7c73a36ed8cf83\target\release\` (built from the merged navmesh pipeline, PR #683); `extract_actors` and `extract_kismet` in `.claude\worktrees\harset-placement\target\release\`. If you rebuild: `lane.sh cargo build --release -p cimmeria-navmesh-extractor --bins`. `archetype_census` needs `CIMMERIA_PACKAGE_INDEX=C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\navmesh\nav-extract\package_index.bin`. Cooked maps: `C:\Users\Steve\source\projects\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC\Maps\<map>`.

Navmeshes: `data/spaces/harset.nav` (world 57) and `data/spaces/harset_storagerm.nav` (world 70). **Worlds 68 (Command Center) and 69 (Market) have no navmesh**, so on-mesh checks are impossible there; use floor, walls and doorways from `obj_slab` and say so in the row.

## Coordinate facts (verified 2026-09-19)

- BigWorld metres, Y up. UE3 to BigWorld: `BW = (ue.y/100, ue.z/100, ue.x/100)`; actor `Location` in chunk files is world-absolute.
- Round-trip checks done: the Command Center respawner (0, 0.355, -20) sits on a flat floor at Y about 0.0 to 0.5; the Harset gate row (-0.076, -67.274, 38.011) is a prefab origin about 2 m ABOVE the plaza floor (about -69.2 at that spot).
- `is_point_valid` gate (`crates/entity/src/navigation/mod.rs`): horizontal offset under 2 x radius (radius 0.6), dy in [-2 x radius, +4.0]. Put spawn Y on the floor, not between storeys.
- `harset.nav`: 39,652 verts, 19,345 polys, **1,939 components**. The plaza and most of the hub is component 187 (24,770 m2). Many real-player stuck points sit in small separate components (for example 956, 1028, 1042), which is the P0 defect H53 works around. Prefer placements in the hub component; a placement in another component is unreachable for NPC pathing and must say so.
- World 57 runs the navmesh in `advisory` mode (H53), so an off-mesh point does not rubber-band players; it still hurts NPC pathing and `is_point_valid` tests. Prefer on-mesh.

## Lessons carried over from Castle (do not repeat)

- Romney was placed from raw umap data into a SEALED, unfinished wing (untextured walls, hole in the floor). Check the candidate's tile has a connected walkable component that reaches it.
- Reconstructed rows all had heading 0 and faced walls. Derive heading from the doorway or approach direction (face the way a visitor arrives), never leave 0 by default.
- 24 to 41 percent of StaticMeshActors are `bCollideActors=false` set dressing and mirrored instances flip winding; both are handled by the extractor, so trust the OBJ, not an eyeballed actor list.

## Landmarks already read from the world-57 census (MAP-LANDMARK, positions BigWorld x, y, z)

| Place | Evidence | Approx position |
|---|---|---|
| Jaffa Zone (west, y about -41) | `JF-Tent00/01`, `JF-MilitaryTent00`, `JF-LargeBuilding_FrontEnt00`, many `JF-Brazier00`, `GA-Barracks01` | x -200 to -135, z 36 to 135 (barracks at -198, 84; large building front at -135, 62) |
| OP-CORE zone (east, y -41 to -23) | `EM-Tent_*`, `EM-Quartermaster01`, `EM-Infirmary00-Generator`, `EM-Bunker00`, `EM-WaterTower00`, `EM-ViewScreen02` | x 100 to 250, z -20 to 160 (quartermaster 189, 157; infirmary generator 150, 106; bunker 202, -9; water tower 249, 46) |
| Bazaar / marketplace (lower level, y -60) | `GA-MerchantTent01/02`, `JF-Tent03`, `EM-Tent_large*` | x 100 to 126, z 51 to 94 |
| Bank | `GA-Bank00` x5, `CA-Courtyard_Str00` | about (-188, -41, 162) |
| Shield towers x3 | `GA-TowTall01`, `GA-TowMed00`, `GA-TowShort01` | (-226, -41, 38), (-166, -31, 235), (0, -31, 289) |
| Plaza guard posts | `GA-GuardPost00` x6 | about (13, -69, 4) |
| Petbe / palace quarter (candidate) | `HP-Brazier00` x2 next to the existing `HarsetRingLeftTop` spawn | about (-160, -28, 232) |
| Doorways (candidate Market/Storage doors) | `GA-large_doorway_open_a_00`, `GA-large_doorway_close_a_00`, `GA-small_doorway_close_a_00` | (233, -69, 77), (258, -69, 100), (-273, -52, 58) |

These are census facts. Turning a landmark into a placement (where exactly to stand an NPC, which way to face) still needs the MAP-GEOMETRY check.

## Seed and test rules for placement work

Follow the repo rules (CLAUDE.md, TESTING.md): no `db/scripts` migrations, edit seeds in `db/resources/` directly; every behaviour change needs a test that fails when the row is removed; keep `docs/**/*.md` CRLF; id ranges from the ledger (`spawnlist` 300-399, `point_sets` 2100-2149 with `point_set_points` 2500-2799, `entity_templates` 200-299, respawners 20, 22, 23). Build only through `/c/Users/Steve/AppData/Local/Temp/cimmeria-castle/lane.sh`, one cargo at a time, `-p` targets, own DB via `reload-db.sh` (never the shared `sgw_harset` or `sgw`). Do not stage `Cargo.lock`. Commit after every finished item; do not push.

Design rules that still stand: shared-hub NPCs (worlds 57 and 68) are never hostile and never targets of mission verbs (D-H03); hostile NPCs in 69 and 70 are mission-scoped `spawn_entity`, never spawnlist rows; do not reuse template 204 for ambient Jaffa (H22 hand-off); every spawn row sets `respawn_secs` (D-H17); the seed `setval` for `entity_templates_template_id_seq` must stay 248 or higher.
