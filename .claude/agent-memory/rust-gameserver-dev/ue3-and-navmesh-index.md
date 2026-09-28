---
name: ue3-and-navmesh-index
description: Sub-index of the UE3 package, BSP/StaticMesh extraction, NavBuilder and navmesh memories (moved out of MEMORY.md to keep it small)
metadata:
  type: reference
---

# UE3 packages and navmesh

- [ue3-absent-property-defaults.md](ue3-absent-property-defaults.md) — an absent tagged property is the SGW class default (`Terrain.DrawScale3D` = 100).
- [ue3-staticmesh-extraction.md](ue3-staticmesh-extraction.md) — `bCollideActors=false` still carries kDOP; prefab archetype chains; match dotted Outer paths.
- [ue3-bsp-model-decode.md](ue3-bsp-model-decode.md) — empty Model = 108 bytes; BSP winding is opposite StaticMesh; Castle brush Models decode to stubs.
- [ue3-prefab-rig-anatomy.md](ue3-prefab-rig-anatomy.md) — component props start at byte 8; prefab meshes on imported archetypes; `.umap` is LZO.
- [navbuilder-obj-interop.md](navbuilder-obj-interop.md) — NavBuilder exits 0 on axis order, CRLF, winding and stray `*.obj` failures. Read before the OBJ writer.
- [navbuilder-obj-traps.md](navbuilder-obj-traps.md) — UE3->BW is `v x z y`; CRLF mandatory; stray non-`<hex8>o.obj` reads garbage bounds.
- [castle-staticmesh-coverage.md](castle-staticmesh-coverage.md) — Castle interior floors are BSP, not StaticMesh.
- [navmesh-recast-and-castle-topology.md](navmesh-recast-and-castle-topology.md) — Recast's unchecked 24-bit span index is the real `cs` floor.
- [navmesh-probe-and-bsp-traps.md](navmesh-probe-and-bsp-traps.md) — `NavGraph::locate` false negatives on stacked meshes; validate BSP filters on a second map.
- [harset-nav-does-not-cover-upper-quarters.md](harset-nav-does-not-cover-upper-quarters.md) — harset.nav covers only the plaza.
- [navmesh-containment-modes.md](navmesh-containment-modes.md) — per-world `navmesh_mode`; `TEST_SPACES_XML` pins space ids; `get_nearest_point` echoes on a miss.
- [npc-ground-clamp-and-detour-traps.md](npc-ground-clamp-and-detour-traps.md) — `moveAlongSurface` output is unprojected; build.rs never rebuilt `detour_wrapper.cpp`.
- [obj-slab-and-nav-inspect-probe-traps.md](obj-slab-and-nav-inspect-probe-traps.md) — obj_slab chunk pre-filter; the top up-facing surface may be the roof.
- [navmesh-onmesh-assertions-are-weak.md](navmesh-onmesh-assertions-are-weak.md) — `is_point_valid` and `find_path` pass on the wrong component.
- [map-data-placement-toolkit.md](map-data-placement-toolkit.md) — deriving spawn coordinates from a cooked map; heading = atan2(dx, dz).
- [telemetry-last-valid-is-mostly-synthetic.md](telemetry-last-valid-is-mostly-synthetic.md) — 77% of Harset `last_valid_*` rejects are (0,0,0).
- [occluder-sizing-and-los-truth.md](occluder-sizing-and-los-truth.md) — NA27 occluder paging; build-determinism and grazing-ray traps.
