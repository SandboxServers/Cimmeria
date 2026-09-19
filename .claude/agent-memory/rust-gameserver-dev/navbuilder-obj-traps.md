---
name: navbuilder-obj-traps
description: NavBuilder_d.exe silently eats bad input — exits 0 with no file, needs CRLF OBJs, and the UE3->BigWorld axis mapping is a Y/Z column swap. Full detail in docs/engine/navmesh-build-pipeline.md.
metadata:
  type: reference
---

**Read `docs/engine/navmesh-build-pipeline.md` before touching anything in the
navmesh build chain.** Verified 2026-09-19 against `bin64/NavBuilder_d.exe`
(branch `navmesh/navbuilder-axis-check`). The four things that cost time:

1. **Axis mapping**: `bw = (ue_y/100, ue_z/100, ue_x/100)`. The OBJ must be
   written `v <ue_x> <ue_z> <ue_y>` (Z-up → Y-up), and NavBuilder's
   `loadOBJ` then does `bw = (obj_z/100, obj_y/100, obj_x/100)`. Net map is a
   *rotation*, so UE3's native triangle winding is emitted verbatim — no
   winding flip. Getting the columns wrong yields `npolys = 0`, not a
   mirrored mesh.
2. **CRLF is mandatory.** `mesh.cpp:115` loops `while (pos < line.length()-1)`,
   so an `f` line needs a trailing char after the last index. With LF, every
   face whose third index is one digit (`f 1 2 3`) is dropped silently.
3. **NavBuilder exits 0 on every failure** — `exportNavmesh` logs `FAULT` and
   returns `void`. Test for a non-empty output file, never `$?`.
4. **Any `*.obj` in a `chunked` input dir whose stem is not `<hex8>o` reads
   uninitialised chunk bounds** (`chunk.cpp`'s else branch sets only
   `chunkId_`) and kills the run with "Failed to create heightfield". The
   combined `<map>.obj` that `extract_map` writes triggers exactly this — keep
   chunk OBJs in their own directory. `tools/build-navmesh.{sh,ps1}` guard it.

Also: the chunk id pads `bmin`/`bmax` but never translates vertices, and
`o Terrain_*` groups are skipped outright by `loadOBJ` — do not name a terrain
group that way.

Connectivity tool: `nav_inspect` (`crates/navmesh-extractor/src/bin/`). Shipped
meshes are already fragmented (castle_cellblock 50 components, harset 1939), so
"one connected region" is never a valid global gate — probe-pair reachability is.
