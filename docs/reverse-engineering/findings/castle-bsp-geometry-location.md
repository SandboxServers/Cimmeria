# Where Castle's BSP world geometry actually lives (measured, whole-map)

> **Last updated**: 2026-09-19
> **Source**: whole-map sweep of `Maps/Castle` and `Maps/Castle_CellBlock`, workers `nav-bsp-decoder` / `nav-bsp-quality`
> **Issue**: [#46](https://github.com/SandboxServers/Cimmeria/issues/46) — navmesh extraction
> **Split from**: [bsp-model-polys-serialize.md](bsp-model-polys-serialize.md), which had grown past the 500-line cap with two independent subjects appended to it.

---

## Summary

Three questions came up during implementation about *which package*
holds the collidable BSP. All three were settled by measurement
against the cooked tree; the tests that produce these numbers are
`crates/navmesh-extractor/tests/it/bsp_castle_model_decode.rs` and
`bsp_castle_floor_evidence.rs`.

**Only 16 of the 144 `Maps/Castle` chunks carry BSP world geometry**,
and they are exactly the 16 that carry `ModelComponent` exports — the
interior tiles: `00040009 0004000a 00050007 00060003 00070002 00070003
00070004 00080002 00080003 00080004 00090002 00090003 00090004 000a0002
000a0003 000a0004`. The other 128 chunks hold only 108-byte stub
`Model`s. Map-wide the decoder emits **7,952 triangles** from 761
`Model` exports, with zero decode failures, zero out-of-range node
indices, and zero triangles removed by the PolyFlags filter.

**The persistent (master) `<MapName>.umap` holds no BSP world
geometry.** This was worth checking because `enumerate_chunks` skips
the master file (it has no `-<HEX8>` suffix), so a chunk-only walker
never opens it. Both `Castle_CellBlock/Castle_CellBlock.umap` and
`Castle/Castle.umap` hold exactly three `Model` exports — the root
builder brush, the level's own, and one `TriggerVolume`'s — and the
first two are 108-byte empty stubs. Feeding the master package to
`collect_bsp_triangles` yields zero triangles from either map.

**`ModelComponent` can be ignored for collision.** In UE3 a
`UModelComponent` lists the node indices of the level `Model` it
renders; its source anchor is `UnModelRender.cpp`. BSP collision is
served by `UModel` itself — the decompiled point-classify walker
(`UnModelCollision.cpp`, `iChild[3]` at `+0x28`) descends the Model's
own tree from the root and never consults a component. The one way
that shortcut could break is a component switching collision off for
its slice, so all 844 `ModelComponent`s across the 16 interior tiles
were property-scanned: they declare only `CachedCullDistance` (844),
`CullDistance` (844), `bForceDirectLightMap` (836),
`bAcceptsLights`/`bAcceptsDynamicLights` (618 each, all `False`) and
`LightingChannels` (2). No collision flag appears at all — cull
distance and lighting only. A regression test pins this.

### Floor evidence (`Maps/Castle`)

The spike's decisive question. Using the authored surface normal
(`Vectors[vNormal]`) rather than emitted winding, so the answer does
not depend on the winding question above, and the mapping
`BW = (ue.y/100, ue.z/100, ue.x/100)`:

| Probe point (BW) | Tile | Result |
|---|---|---|
| Zuritska cell `(268.0, 66.79, 1042.59)` | `000a0002` | **BSP floor present** — upward face 9 cm above the recorded Y, surf `n.z = 1.000`, `PolyFlags 0xE00` |
| Romney corridor end `(244.0, 66.79, 1036.0)` | `000a0002` | **BSP floor present** — upward face 25 cm above the recorded Y, surf `n.z = 1.000`, `PolyFlags 0xE00` |
| Level-5 comms room `(271.7, 55.2, 858.0)` | `00080002` | **BSP floor present** — upward face at 0.0 cm, surf `n.z = 1.000`, `PolyFlags 0xE00` |

A 1 m grid over each tile's full 100×100 BW footprint finds an
upward-facing BSP face within ±1.5 BW units of the floor plane at
**4,518 of 10,000** points for the Interrogation Block floor
(`000a0002`, BW y 66.79; 133 candidate faces, 4,472 m²) and **2,389 of
10,000** for the Level-5 comms floor (`00080002`, BW y 55.20; 59
faces, 2,398 m²). For contrast, a StaticMesh-only probe of the same
Interrogation Block found a floor at 2 of 1,365 grid points. **Castle
interior floors are BSP, not StaticMesh.**

### `Castle_CellBlock` flat sheets

The shipped `data/spaces/castle_cellblock.nav` contains large flat
walkable sheets that are neither terrain nor recovered StaticMesh.
They are BSP, and they live in the **chunks**, not the master
package: 10 of the 64 `Castle_CellBlock` chunks carry a non-stub
level `Model` (up to 287 KB), together 4,115 triangles. Near-
horizontal BSP area bucketed by BW y lines up with the nav sheets:

| Nav sheet | BSP near-horizontal area within ±1 m | BSP XZ bounds at that height |
|---|---|---|
| 30,499 m² at BW y ≈ 94.6, x[-384.4,-212.5] z[-239.8,-56.2] | 27,354 m² | x[-385.3,-211.8] z[-240.6,-55.4] |
| 22,838 m² at BW y ≈ 53.4, x[-169.6,-36.1] z[-193.9,-22.3] | 23,329 m² (at the y 53.5 bucket) | x[-170.2,-35.4] z[-194.6,-21.6] |
| 3,836 m² at BW y ≈ 24.8, x[-148.3,-37.0] z[-178.3,-34.9] | 6,233 m² (at the y 24.5 bucket) | x[-149.0,-36.3] z[-179.0,-34.2] |

The XZ bounds agree to under a metre in every case. (The y ≈ 24.8 row
over-reports because a ±1 m window also catches the 23,329 m² slab at
y 24.0, which is the *underside* of the y 53.5 volume — the two
buckets share XZ bounds exactly.)
