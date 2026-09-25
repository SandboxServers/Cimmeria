---
name: obj-slab-and-nav-inspect-probe-traps
description: obj_slab and nav_inspect both return answers that depend on how you asked - the box you pick changes which chunks load (so a column can contradict itself), levels are triangle extents not box extents, the topmost up-facing surface in a roofed room is the roof, and nav_inspect's h is Y-biased so probing at the wrong Y manufactures a false off-mesh
metadata:
  type: reference
---

# Placing a coordinate from map data: how the two tools lie to you

Used for the Harset placement pass (`docs/analysis/harset-rebuild/placements/`).
Binaries: `obj_slab`, `nav_inspect`, `archetype_census`, `extract_map`,
`extract_actors`, `extract_kismet` — built from `cimmeria-navmesh-extractor`.

**Why:** four separate readings during one placement session were wrong in a way
that *looked* authoritative, and one of them (a column reporting no floor where
there is one) would have kept a working door disabled. **How to apply:** read
this before deriving any coordinate, floor height or on-mesh verdict from these
tools.

## obj_slab

1. **The box changes the answer, including for `--column`.** The chunk
   pre-filter loads chunks near the `--at`/`--box` argument (`chunks N read, M
   skipped by the grid pre-filter (margin 60 m)`), and `--column` only sees what
   got loaded. A column 10 m outside the box can report **only down-facing
   surfaces** where a box centred on it reports a real up-facing floor. Seen at
   Harset (0, -231): a box at (-0.25, -240.9, half 4) said "no floor", boxes
   centred on the point at half 2 / 6 / 20 all said `-67.64` up-facing.
   **Always centre the box on the point you are asking about, and re-run at two
   box sizes.** Watch the `chunks N read` count change.
2. **`levels` bounds are TRIANGLE extents, not box extents.** A level line
   reading `x[-10.2, 10.2] z[30.7, 46.1]` inside a 2 m box is telling you about
   triangles that merely *intersect* the box. Use the area/triangle count to
   judge "is there a surface here"; never read the x/z range as a footprint.
3. **The topmost up-facing surface is the roof.** In a roofed interior the
   highest `faces_up=true` in a column is the ceiling's top face, not the floor.
   Harset_Market's floor is 3.61 but a naive max-of-up-facing over a grid
   returns 17.26-19.24 wherever the roof is above the sample. Take the lowest
   plausible up-facing surface above the ground plane, or cross-check against a
   floor-standing prop's origin.
4. **Prop origins are the cheapest floor evidence there is.** A floor-standing
   prop (`EM-StandingLight05`, `GA-Torch00`, `JF-Brazier00`, `GA-Fence*`) has its
   actor origin at its base. `archetype_census`'s `*_arch_positions.tsv` gives
   them in BigWorld coordinates already. Two symmetric props also hand you the
   centreline and the approach axis for free.

## nav_inspect

1. **`h` (horizontal distance to the nearest poly) depends on the probe's Y.**
   The nearest-poly search is Y-biased, so probing a point at a Y 0.3 m off the
   real floor can report `h = 3.26` where probing the floor Y reports `h = 0.00`
   on the same XZ. A grid swept at one convenient Y will invent mesh holes.
   **Probe at the measured floor Y, from `obj_slab`.**
2. **`h = 0.00` means "inside a polygon"; anything else means "off-mesh, by
   `h`".** That is the useful distinction, not `ok` / `OUT OF TOLERANCE` — the
   default tolerances (`--h-tol 2`, `--v-tol 3`) are much looser than
   `is_point_valid`'s (XZ `< 2 * radius` = 1.2, dy in `[-1.2, +4.0]`).
3. **`dy` is `probe_y - poly_y`.** So `dy = -28.58` means the mesh is 28.58 m
   **above** the probe. Reading the sign backwards turns "the mesh is on the
   storey above" into "the point is in the basement".
4. **Robustness is a ring test, not a point test.** Probe the centre plus rings
   at `r = 0.6` (agent radius) and `r = 1.2`; count how many samples are
   interior to the *same* component. A point that is interior but 0.5 m from a
   sliver's edge is a re-pin waiting to happen.
5. **Component membership is the reachability answer, and it is not optional.**
   `harset.nav` has 1,939 components. A point can be `h = 0.00` and still be
   unreachable for NPC pathing because it is in a 3-poly fragment. Name the
   component id in every placement row.

## The pattern that worked

`archetype_census` for a landmark → `obj_slab --at <landmark> --column` for the
floor Y → `nav_inspect --probes` on a grid *at that Y* → ring test the finalist
→ assert it in a live-DB test that loads the real `.nav` and reads the
coordinate back out of the seed. See
`crates/services/src/cell/harset_placement_tests.rs`. A test that hardcodes the
coordinate cannot fail when the seed row is reverted; a test that only reads the
DB cannot fail when the coordinate is unstandable. You need both halves.
