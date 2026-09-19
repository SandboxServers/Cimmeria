---
name: navbuilder-obj-interop
description: NavBuilder's loadOBJ has four silent-failure traps (axis order, CRLF, reversed winding, stray *.obj in the chunk dir) and exits 0 on all of them — read before touching crates/navmesh-extractor/src/obj.rs
metadata:
  type: project
---

`deprecated/cpp/src/nav_builder/mesh.cpp` `Mesh::loadOBJ` fails **silently**
on all four of these — NavBuilder exits 0 and writes either nothing or an
empty 72-byte `.nav` with `npolys = 0`.

**Why:** measured end to end against `bin64/NavBuilder_d.exe` during the
castle.nav spike (issue #46 / CA14, 2026-09-19), by workers `nav-axis`
(NavBuilder side) and `nav-extract` (extractor side).

**How to apply:** before changing the OBJ writer, the probe's walkability
test, or where the combined OBJ lands.

1. **Axis order.** World mapping is `bw = (ue.Y, ue.Z, ue.X) / 100`.
   `loadOBJ` does `v.x = obj_z/100, v.y = obj_y/100, v.z = obj_x/100`
   (mesh.cpp:104-107), so the OBJ must be written `v <ue.X> <ue.Z> <ue.Y>`.
   Raw `(X, Y, Z)` puts UE3's horizontal Y on BigWorld's up axis and every
   floor rasterises as a wall.
2. **CRLF, not LF.** The face parser loops `while (pos < length - 1)`
   (mesh.cpp:115), so an LF-terminated `f` line loses its last index
   whenever that token is one digit. 14 polys vs 6 on the same fixture.
3. **Winding verbatim.** `loadOBJ` pushes each face as
   `(faces[i], faces[i-1], faces[0])` (mesh.cpp:123-128) — reversed. The
   axis swizzle is an even permutation (det +1), so Recast's normal is the
   negation of the emitted order's: `N_recast.y = -n_ue3.z`. A "walkable"
   test that omits the negation reports **ceilings as floors**, which in a
   multi-storey interior looks entirely plausible. It moved one measured
   coverage number from 0.1% to 4.3%.
4. **Chunk directory purity.** `chunked` mode globs `*.obj` and derives
   chunk bounds from a `<hex8>o` stem. One non-matching file leaves the
   bounds uninitialised → "Failed to create heightfield", no output, exit
   0. Also never name a group `o Terrain_*` — those groups are skipped
   wholesale (mesh.cpp:88-96). Chunk ids do **not** offset vertices; actor
   `Location` is already world-absolute.

See [[castle-staticmesh-coverage]] for what the extractor actually
recovers under these rules.
