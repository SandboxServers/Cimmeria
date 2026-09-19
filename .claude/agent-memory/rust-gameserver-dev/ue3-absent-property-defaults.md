---
name: ue3-absent-property-defaults
description: An absent UE3 tagged property means the CLASS default, not the generic engine default — how to recover the real default from world layout (SGW Terrain DrawScale3D case)
metadata:
  type: project
---

**An absent property in a cooked UE3 export means "equal to the class
default", and the class default is licensee-modifiable. Never assume the
stock UE3 value.**

**Why:** SGW's `ATerrain` omits `DrawScale3D` on all 144 `Castle`
terrains and 400 of 1600 `Castle_CellBlock` terrains. Defaulting to the
generic actor value `(1,1,1)` makes every terrain patch 1 cm wide and
the decoded map ~1/100 scale — which *still parses cleanly* and produces
plausible-looking triangles. It is a silent wrong answer, not a crash.
The real default is `(100, 100, 100)`.

**How to apply:** Two cheap recovery techniques, both used to settle the
terrain case (2026-09-19, `navmesh/terrain-decoder`):

1. **World-layout arithmetic.** Actor `Location`s are absolute. If N
   actors tile an exact grid, the per-unit scale is forced:
   Castle_CellBlock's 1600 terrains sit on an exact 2000 cm grid with 20
   patches each ⇒ 100 cm/patch; Castle's 144 terrains on a 10000 cm grid
   with 100 patches each ⇒ 100 cm/patch. Two different authoring
   conventions agreeing on the same number is strong evidence.
2. **A known world coordinate.** For the axis that layout can't pin (Z
   here), sample the decoded value under a seed/telemetry point. The
   gate-room/DHD seed at BW y 55.10 decodes to 55.14 at Z-scale 100 and
   110.28 at 200 — a clean 2x discriminator.

Corollary that resolved the apparent contradiction: 1200 of the 1600
Castle_CellBlock terrains *do* write `DrawScale3D` explicitly, as
`(100,100,200)`. That is consistent, not contradictory — UE3 serialises
a property only when it differs from the default, and those actors are
all flat (`0x8000` heights) so the doubled Z is unobservable. **If some
instances write a value and others omit it, the written value is by
definition NOT the default** — that fact alone rules out the naive
"whatever the majority writes" guess.

Related: `.claude/agent-memory/game-archaeology-specialist/ue3-terrain-serialize.md`,
`docs/engine/ue3-package-format.md` §"Terrain actor serial blob".
