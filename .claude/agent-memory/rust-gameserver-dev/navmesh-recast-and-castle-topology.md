---
name: navmesh-recast-and-castle-topology
description: Recast's UNCHECKED 24-bit span index (the real cs floor, not the 16-bit ones), the bin-target `mod tests;` trap, and why Castle's probes will never be one component
metadata:
  type: project
---

# Navmesh: the limit that actually bites, and Castle's real topology

**Why:** the `castle.nav` spike burned a lot of time on hypotheses that a
single measurement would have killed. These are the measurements.

**How to apply:** read before touching `crates/navmesh-extractor`,
`deprecated/cpp/src/nav_builder/`, or any "why is the navmesh split" question.

## Recast has a FOURTH index limit and it is the one that bites

`rcCompactCell` packs a column's first-span index into **24 bits**
(`Recast.h:336`, `unsigned int index : 24`) and `rcBuildCompactHeightfield`
assigns it from a plain `int` with **no check**. Past 16,777,215 spans the
build silently produces nothing: log reads `Regions: 1`, `Contours: 0`,
`nverts=0 npolys=0`, and the old NavBuilder exited **0** having written a
60-byte `.nav` that loads fine.

It grows as `1/cs²`, so it is the limit you hit first when refining `cs` on a
map-sized area — long before the documented 16-bit vertex/edge caps.
Measured on Castle: whole map at `cs=0.3` is 13,936,045 spans (83 % of cap);
the interior crop at `cs=0.15` is 18,716,138 (over). `Castle_CellBlock` at
`cs=0.15` with its default ±400 chunk-padded bounds is 30,594,915 — cropping
to the real geometry (`bounds=-360,-250,110,110`) brings it to 9,524,285 and
it builds. **A finer build is always possible; it just has to be cropped.**

NavBuilder now logs the count every build and exits 3 over the cap, and an
empty poly mesh is exit 3 instead of a post-write WARNING at exit 0.

Full reference: `docs/engine/navbuilder-recast-limits.md`.

## A bin target's `mod tests;` needs `#[path]`

`src/bin/foo.rs` **is** a crate root, so `mod tests;` there looks for
`src/bin/tests.rs` — which would collide across every bin in the directory.
Use:

```rust
#[cfg(test)]
#[path = "foo/tests.rs"]
mod tests;
```

Bin unit tests do count for coverage; a bin with all its logic in `main()` is
0 % covered because `main` is never called by `cargo test`. Split into
`parse_args_from(&[String])` and `run(&mut impl Write, …) -> io::Result<u8>`
with stderr lines collected into a `Vec<String>` rather than printed.

## Castle (World 8): the extraction currently produces two probe groups

**Updated 2026-09-19.** An earlier revision of this note said "three
components and always will be". That was wrong on both counts: the third
group was an extractor bug, not geometry, and nothing about the count is
permanent — it is a property of what the extractor emits, which changes.

Current state: 40,068 verts / 19,815 polys / 55,101 edges, 549 components,
the eleven probes in **two** groups (exterior, and everything indoors).

- **Interior ↔ throne-room storey: SOLVED, and it was ours.**
  `CA-Interior:CA-large_hallway_ramp_a_00` at BW (355.04, 55.04, 863.48) is
  authored `DrawScale3D = (-1,1,1)`. Mirroring reverses triangle winding,
  NavBuilder decides walkability from winding alone, and the walker emitted
  the transformed vertices in their original order — so the ramp's tread
  reached Recast as a **ceiling**. `ActorTransform::apply_triangle` now
  swaps two vertices when the scale determinant is negative. 198 of
  Castle's 6,436 actors are mirrored (93 of Cellblock's 2,098).
  `throne_room` then merges into the interior component
  (17,006 → 53,556 m²). The "12.00 m step with nothing between" reading
  was the symptom, not the cause; no parameter set would ever have fixed it.
- **The 108-byte `Brush`-owned `Model`s are correct cooked data**, not a
  decoder gap: export-table `serial_size = 108`, bytes are bounds + zero
  counts + a live `Polys` ref, and 36 of the 38 brushes in
  `Castle-00080003` are `CSG_Subtract` room volumes already baked into the
  level `Model`. Brush 11 is the stairwell shaft, x[339.7,370.4]
  y[47.0,65.0] z[845.6,881.3].
- **Still split: exterior ↔ interior.** A three-hop chain of terrain
  shelves at (725.6, 30.4, 462.9) → (675.6, 18.6, 488.3) →
  (622.4, 24.0, 508.2), 2.4–3.4 m steps, banks at 45–58°. Outdoor ground,
  not a building; `slope=70` shortens the chain but never joins it.
- **InterpActors are a dead end**: all 14 in Castle are 11 security-camera
  heads, an antenna, a shelf box and `GLB-RingTransporter00`. No lift, no
  door. (The `EM-Elevator00` are StaticMeshActors, already extracted.)
- The `armory` probe is a cross-world ring drop zone
  (`ring_transport_regions` region 34, world 8, fed from region 33 in
  world 12) — not a walk-in room.

Full write-up: `docs/engine/castle-navmesh-connectivity.md`.

**Transferable lesson**: when a navmesh splits at a place the geometry
clearly covers, check the *winding* of the actors there before reaching
for Recast parameters. A negative-determinant transform is invisible in
every count the extractor reports.

## Two tool traps

- `nav_inspect` reporting a probe `ok` at `h` close to `--h-tol` means it is
  **not** on a polygon. `armory` read `h=1.49 m ok` because `minRegionSize=24`
  deleted its 11 m² pad and the probe snapped to the floor beside it. Read
  the `h=` column, not the word.
- `bCollideActors = false` actors still carry kDOP collision. 1,570 of them
  in Castle, including hidden `PrecipPlanes` snow cards the artist put **in
  doorways**. Suppressing them took the whole-map mesh from 997 to 553
  components and removed 182k m² of "walkable" icicle and sign tops.
