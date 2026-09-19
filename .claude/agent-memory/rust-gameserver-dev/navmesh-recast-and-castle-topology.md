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

## Castle (World 8) is three components and always will be

The 11 named probes sit in three groups (exterior / interior upper /
throne-room storey) and **no Recast parameter set changes that** — verified
across slope=60/70, agentClimb=1.2/1.5, agentRadius=0.2/0.3/0.15,
agentHeight=1.2, minRegionSize 2..32, maxSimplificationError 1.3..3.5, and
`cs=0.15 ch=0.1` crops.

- Interior ↔ throne-room storey: rims overlap in XZ at `h=0.00 dy=+12.00 m`
  exactly, around `(276…292, ·, 865…886)`. Two storeys, 4 m slab between.
  Prop evidence (concrete cover at y 43.20 and 55.40) confirms both are real.
- Exterior ↔ throne-room storey: terrain banks measured at **45–58°** plus
  3–8 m cliffs. `slope=70` shortens the chain but never joins them.
- **InterpActors are a dead end**: all 14 in Castle are 11 security-camera
  heads, an antenna, a shelf box and `GLB-RingTransporter00`. No lift, no
  door. (The three `EM-Elevator00` are StaticMeshActors, already extracted.)
- The `armory` probe is a cross-world ring drop zone
  (`ring_transport_regions` region 34, world 8, fed from region 33 in
  world 12) — not a walk-in room.

What is left: `Polys` / `ModelComponent` BSP in the interior chunks and
`StaticMeshCollectionActor`. A stairwell would be there if anywhere.

## Two tool traps

- `nav_inspect` reporting a probe `ok` at `h` close to `--h-tol` means it is
  **not** on a polygon. `armory` read `h=1.49 m ok` because `minRegionSize=24`
  deleted its 11 m² pad and the probe snapped to the floor beside it. Read
  the `h=` column, not the word.
- `bCollideActors = false` actors still carry kDOP collision. 1,570 of them
  in Castle, including hidden `PrecipPlanes` snow cards the artist put **in
  doorways**. Suppressing them took the whole-map mesh from 997 to 553
  components and removed 182k m² of "walkable" icicle and sign tops.
