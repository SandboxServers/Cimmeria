---
name: castle-staticmesh-coverage
description: Castle (World 8) interior floors are BSP, not StaticMesh — the Phase 1.2 extractor recovers 85% of actors but ~0% of the Interrogation Block floor plane, so the BSP decoder blocks castle.nav
metadata:
  type: project
---

Measured 2026-09-19 over all 144 `CookedPC/Maps/Castle` chunks with
`crates/navmesh-extractor`'s `extract_map` CLI (branch
`navmesh/castle-extract-cli`).

**Why:** the castle.nav spike (issue #46 / CA14) had to decide whether a
StaticMesh-only navmesh was worth building now or whether the BSP decoder
(expensive, needs Ghidra RE) is on the critical path.

**How to apply:** don't plan a Castle navmesh on the StaticMesh path
alone, and don't re-derive these numbers — re-run the CLI instead.

- 6,430 `StaticMeshActor` exports, **85.1% resolved**, 1,292,291
  triangles, 2.9 s wall clock. Every one of the 961 skips is a prefab
  archetype stub; zero index misses, decode failures or collision-free
  meshes.
- **Interior floors are absent.** Grid-probing the floor plane of the
  Interrogation Block (`Castle-000a0002`, y = 66.79) finds a StaticMesh
  floor under 2 of 1,365 points. The Level-5 comms room is 19.8%. Every
  column has geometry over it — that geometry is roof and ceiling.
- All 16 chunks with `ModelComponent` exports (built BSP surfaces, as
  opposed to the empty default `Model`/`Polys` pair every chunk ships)
  are interior tiles. That is the floor.
- **PrefabInstance does not own actors via `Outer`** — map-wide
  `prefab_outer_actors` is 0; every actor is outered to
  `PersistentLevel`. The prefab's actors *are* separately exported, but
  their cooked `StaticMeshComponent` is a ~76-byte archetype stub. In
  `Castle-000a0002` it is exactly 147 PrefabInstance / 147 archetype
  actors / 147 skips / 0 resolved. Closing it means resolving
  `ExportEntry::archetype` through the `PackageIndex` — worth ~15% more
  actors.
- The `NAVMESH-WORKER-RULES.md` claim that `Castle-000a0002` holds
  "~2.5k StaticMeshActor, ~220 Brush, ~300 PrefabInstance, 3 Terrain" is
  wrong on all four. Measured (and confirmed independently with
  `tools/upk_parser.py`): 844 StaticMeshActor, 47 Brush, 147
  PrefabInstance, 1 Terrain, out of 3,679 total exports.

Walkability here means what Recast means — see
[[navbuilder-obj-interop]] item 3, which is what moved the
Interrogation Block number from a misleading 4.3% to 0.1%.
