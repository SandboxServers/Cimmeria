---
name: map-data-placement-toolkit
description: Placing spawns/regions from cooked UE3 map data without an in-client walk - obj_slab's silent chunk pre-filter, the BigWorld heading convention proved from seed rows, chunk name encoding, and the 4-corner BoundingBox point-set convention
metadata:
  type: project
---

**Read before deriving any spawn or region coordinate from a cooked map.** Learned 2026-09-19 placing Harset worlds 68/69/70 (packets H12/H14/H15, ledger `docs/analysis/harset-rebuild/placements/`).

**Why:** the Castle reconstruction shipped a packet of heading-0 rows facing walls and one NPC inside a sealed wing. These are the mechanical traps that let that happen.

**How to apply:**

- **`obj_slab --column X,Z` silently reports `no surface` for any column outside the chunks the `--at`/`--box` grid pre-filter loaded.** A tight `--at` loads 4 of 5 chunks and the whole far half of the map reads as void. Always pass a `--box` spanning the entire map alongside the columns. Check the `chunks N read, M skipped` line.
- A 5 m grid of `--column` probes, rendered as a text height map (top-most upward-facing face with tilt < 20° in a Y band), recovers the whole floor plan in one run. Refine to 2 m around each candidate. Values much higher than the floor are wall tops and furniture, not floors.
- **BigWorld heading = `atan2(dx, dz)`** — 0 = +Z, pi/2 = +X. Proved from `spawnlist`, not assumed: Harset lieutenants 235 (x -4.44, h pi/2) and 236 (x +4.33, h 3pi/2) face each other across a threshold, and plaza guards 225/234 (x -18.7, h ~pi/2) face inward while 228/231 (x +18.6, h ~3pi/2) mirror them.
- **Spawn Y = floor + 0.05.** Authored anchors sit 0.00-0.07 above their `obj_slab` floor (Harset: Anat +0.07, point set 2079 +0.00, respawner 21 +0.035). Erring high is safe (`is_point_valid` allows +4.0 up, only -1.2 down); erring low clips.
- **UE3 -> BigWorld: `BW = (ue.y/100, ue.z/100, ue.x/100)`**, and an actor `Location` in a chunk file is world-absolute. UE3 yaw in degrees maps straight onto BW heading in radians. Chunk directory names are `<zzzz><xxxx>` in signed hex, 100 m per chunk (`ffff0000` = z -1, x 0).
- **`archetype_census` covers prefab/archetype meshes only** (121 of 769 StaticMeshActors in Harset_CmdCenter), so absence from the TSV does NOT mean absence from the map. Use it for landmarks, `obj_slab` for geometry.
- **Server-spawned props are not overlays of map meshes.** The authored Harset DHD spawn has no DHD mesh anywhere near it. Co-locating a prop spawn with a map actor that draws the same mesh (e.g. template 245 and `GA-PuzzleStation00`) z-fights.
- **4-corner `BoundingBox` point sets**: three corners carry the floor Y and the **fourth carries the ceiling** — the asymmetry is `GenericRegion.workaround()`'s and `cell/spawner/regions.rs` warns not to "normalize" it. `load_regions_from_db` takes the AABB; `is_point_in_region` widens it 1.5 m on **every** axis including Y, while `region_contains_xz` is exact and Y-blind.
- **Stop a region short of the room its visitors arrive from**, or an `enter_region` trigger on it never fires for a player who loads in already inside (the `player_loaded` edge-trigger shape — see [player-loaded-edge-trigger-race.md](player-loaded-edge-trigger-race.md)).
- **`resources.spawnlist.tag` has a UNIQUE constraint**, so a "no duplicate tags" guard cannot be made to fail by seeding one; say so in the test doc rather than claiming it guards against duplicates.

See also [navmesh-onmesh-assertions-are-weak.md](navmesh-onmesh-assertions-are-weak.md), [ue3-absent-property-defaults.md](ue3-absent-property-defaults.md), [entity-template-seed-authoring.md](entity-template-seed-authoring.md).
