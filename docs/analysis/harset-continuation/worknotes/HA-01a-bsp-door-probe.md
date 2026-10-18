# HA-01a — BSP/door probe for Market (69) and Storage (70)

Date: 2026-10-18. Offline, read-only time-boxed probe. No lab, no client, no Ghidra, no cargo.
Nothing in this note has been tested in game.

## Result: PARTIAL

- **Recovered, high confidence:** the placed doorway meshes in both interiors, with exact positions and yaws.
  They are read straight from the cooked `.umap` exports.
- **Inferred, medium confidence:** which doorway is the exit to Harset.
- **Estimated, low to medium confidence:** the interior landing points. They are derived from door position plus the 7–8 m clearance rule, and their floor heights are not checked against geometry.
- **Not recoverable from interior data:** the world-57 (Harset exterior) door positions. The interiors carry no link back to world 57.

The four HA-D07 hand readings are not replaced yet. The landing estimates below could replace the two interior-arrival readings after one geometry check (see Next step). The two world-57 door readings still need a hand reading or a separate exterior probe.

## Premise correction

Research note §1 says the arrival gap exists because of "BSP the extractor does not read". That is stale.
The navmesh-extractor Phase 1.4 BSP decoder has shipped (`cimmeria_upk_objects` model + bsp, finding `docs/reverse-engineering/findings/bsp-model-polys-serialize.md`).
NA26 already built `harset_market.nav` and `harset_storagerm.nav` from StaticMesh + Terrain + BSP.
BSP was also not needed for this probe: both interiors mark their doorways with placed `StaticMeshActor`s, which carry an explicit Location.

## Method

1. Checked for existing tools first: `tools/README.md`, `crates/*/src/bin`, `crates/navmesh-extractor/README.md`, `data/spaces/README.md`, agent memory.
   - `tools/extract_actors.py` lists actors but does not resolve the StaticMesh name behind each StaticMeshActor, so a door mesh looks like any other mesh.
   - `extract_map`, `obj_slab` and `nav_inspect` (Rust) would check geometry but need a lane build. No binaries were built in the worktree and `data/navmesh_inputs` is absent.
2. Wrote a throwaway script in the session scratchpad (not in the repo) on top of `tools/upk_parser.py`.
   - For each tile `.umap` it maps each StaticMeshComponent to its StaticMesh by the component's outer actor.
   - It then prints each actor's class, name, BigWorld position, yaw, prefab, archetype and mesh.
   - Conversion used: `bw = (ue.Y, ue.Z, ue.X) / 100`, yaw = `Rotation.Yaw * 360 / 65536`.
3. Grepped the dumps for door, arch, stair and volume names, then read the Kismet dump for the volumes' purpose.

Client source, read-only: `..\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC\Maps\...\Harset_Market\*.umap` and `Harset_StorageRm\*.umap`.

## Evidence

### Market (world 69)

| Export | Tile | Actor | Mesh | BW position | Yaw |
|---|---|---|---|---|---|
| 1006 | Harset_Market-00000000 | StaticMeshActor_116 | `GA-DoorArch00` | (97.28, 3.52, 60.32) | -180 |
| 1551 | Harset_Market-00000000 | StaticMeshActor_7 | `GA-DoorArch00` | (19.92, 4.80, 1.28) | -90 |
| 1087 / 1092 / 1120 / 1674 | Harset_Market-00000000 | StaticMeshActor_157/158/171/80 | `GA-Stairs_Short00` | x 16.6–23.2, y -0.03/0.73, z 28.0–32.8 | -90 |

Supporting context:

- Respawner 22 is at (48, 3.61, 78). Set 2121 spans X[30,100] Y[3.6,13.6] Z[30,100]. Market floor reads are y ≈ 3.5–3.7: Brazier at (90.10, 3.64, 39.30), CritterZones at y 3.68.
- Arch 1006 sits on the X=100 edge of set 2121, at floor height.
- Arch 1551 sits at the far end of a torch-lit corridor. Wall torches at y 7.28 are paired at x 15.1/24.6, z 9.24 and z 20.36. That suggests a passage about 9.5 m wide, x ≈ 15–25, running z ≈ 1–21, with its floor near y 4.8.
- The short stairs at z 28–33 link that corridor to the vestibule candidate E07 (20.0, 3.84, 32.9).
- The single Market TriggerVolume (export 2503, (68.34, -2.07, 58.87)) is the trigger half of `WaterCollisionPrefab_Square_1`. It pairs with the StaticMeshActor at (68.32, 1.76, 58.88). It is water collision, not a door.
- Python parse caveat: in the main tile, `Brush` exports fail to parse (truncated property read at offset ~267–273 of a 269-byte buffer). These are per-export failures. All StaticMeshActor, PrefabInstance and volume rows parsed, so no door rows are known to be lost. Brush geometry was not read.

### Storage (world 70)

| Export | Tile | Actor | Mesh | BW position | Yaw |
|---|---|---|---|---|---|
| 28 | Harset_StorageRm-ffff0000 | StaticMeshActor_1037 | `GA-DoorArch00` | (61.36, 5.12, -2.60) | -90 |
| 2133 | Harset_StorageRm-00000000 | StaticMeshActor_370 | `GA-small_doorway_open_a_00` | (51.20, 3.84, 30.08) | -90 |
| 2387 | Harset_StorageRm-00000000 | StaticMeshActor_7 | `GA-small_doorway_open_a_00` | (51.20, 3.84, 21.12) | 90 |

Supporting context:

- Respawner 23 is at (50, 0, 44). Set 2122 spans X[19,84] Y[0.3,4] Z[38,97]. The upper-wing candidates E13–E15 are at y ≈ 5.2, z 5.8–40.7. The pocket at y 4.80 is PL-C-13, and the ceiling of 4.00 is PL-C-14.
- `GA-DoorArch00` is the same arch mesh used for both Market arches. It sits just outside the room's z ≈ 0 edge, level with the upper wing (y 5.12 against E13/E14 at y ≈ 5.2, z 5.8/9.5). It is the only exterior-style arch in the Storage map.
- The two `GA-small_doorway_open_a_00` actors face each other across x = 51.2, at z 21.1 and z 30.1. They have a Kismet prefab sequence (`GA-small_doorway_open_a_00_Pf0_Seq`, with remote events and sounds). They are internal doorways between the upper wing and the pen area, not the zone exit.
- **The three "unexplained" volumes are resolved: they are reverb-zone touch triggers.**
  - TriggerVolume_26, _5 and _6 (exports 3944/3945/3946) are at (53.92, 3.84, 5.76), (27.68, 3.84, 40.96) and (55.36, 3.84, 15.36).
  - Each feeds a `SeqEvent_Touch` in Main_Sequence that drives Sequences 821/822/823. Those are the SFXReverb sequences LivingRoom, Room and StoneRoom. The exact index-to-name mapping is likely but unverified.
  - 3945 also fires `SeqAct_PlaySound` 686.
  - None of them is a zone-line.
- The 19 fence volumes are confirmed as `GA-Fence00/03_Pf0` prefab archetypes.

## Coordinates (BigWorld, interior space)

| Item | Value | Confidence |
|---|---|---|
| Market exterior door (most likely) | arch 1006, (97.28, 3.52, 60.32), yaw -180 | position HIGH; "this is the exit" MEDIUM |
| Market second arch | arch 1551, (19.92, 4.80, 1.28), yaw -90 | position HIGH; role UNKNOWN (second exit or decorative end of corridor) |
| Market landing, door 1006 | ≈ (89.5, 3.6, 60.3), 7.8 m inward along -X | LOW–MEDIUM; floor y taken from nearby props |
| Market landing, door 1551 | ≈ (19.9, 4.8, 9.0), 7.7 m inward along +Z | LOW–MEDIUM; corridor floor inferred from torch heights |
| Storage exterior door | arch 28, (61.36, 5.12, -2.60), yaw -90 | position HIGH; "this is the exit" MEDIUM |
| Storage landing | ≈ (61.4, 5.2, 5.2), 7.8 m inward along +Z | LOW–MEDIUM; matches the upper-wing height of E13–E15 |

Inward direction is taken from which side of the arch the room lies, not from yaw. The arch mesh's forward axis is unknown, so yaw alone does not say which way is in. Both Market landings and the Storage landing lie outside their spawn sets (2121/2122). That is expected: spawn sets are not walkable bounds, and the 1.5 m pad rule applies to spawns, not arrivals.

## Limits

- Landing heights are not checked against geometry, so a landing could fall inside a step, a rail or a prop. The Market corridor floor (y 4.8) and the Storage upper wing (y 5.1–5.2) rest on prop heights only.
- Which arch is the Harset exit is inferred from how each arch sits in the room. Nothing in the interior data names the exit. No zone-line, teleporter, PlayerStart or Note was found in either map.
- Market's two arches cannot be told apart offline. Either could be the street door.
- The world-57 door positions are out of scope for interior data. The Harset exterior `.umap` tiles could be searched the same way for `GA-DoorArch00`/doorway meshes, but that only yields candidates. Matching an exterior arch to Market or Storage still needs a judgement call or a hand reading.
- Brush and BSP geometry were not decoded in this probe. It was not needed for door positions.

## What it would take to ship

1. **Geometry check of the three landings (no client needed).**
   - Lane build of `cimmeria-navmesh-extractor --release --bin extract_map --bin obj_slab`.
   - `extract_map` on the two interiors into a scratch directory.
   - Run `obj_slab` column and level queries at each landing to read the true floor y and confirm 1.5 m clearance.
   - Optionally probe the existing `harset_market.nav` / `harset_storagerm.nav` with `nav_inspect` at the same points. Their probe results are 3/5 and 5/5 (advisory).
   - Expected effort: one foreground lane build plus a few queries.
2. **Choose Market's exit arch.** Arch 1006 is the default: it sits on the main hall at the respawner's floor height. Arch 1551 may be a second exit. A single owner or lab glance would settle it, and the lab needs the user's go-ahead.
3. **World-57 doors:** keep the HA-D07 hand readings, or run the same actor dump on the Harset exterior tiles to list arch candidates for a reviewer to pair.
4. A shippable extractor would add a `--mesh` (component → StaticMesh) column to `tools/extract_actors.py` or to the Rust `extract_actors`. That needs a separate packet, because this probe adds no tool to the repo.

## Next step

Run ship step 1 (geometry check of the three landings). If the floors confirm, propose replacing the two HA-D07 interior-arrival hand readings with the checked landings, and keep the two world-57 readings as they are.
