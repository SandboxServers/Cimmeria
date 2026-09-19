# `ATerrain::Serialize` recipe — real-data validation

> **Last updated**: 2026-09-19
> **Source**: `Castle-000a0002.umap` decode, `castle.nav` spike (issue #46), worker `nav-bsp-re`
> **Issue**: [#46](https://github.com/SandboxServers/Cimmeria/issues/46) — navmesh extraction
> **Split from**: [bsp-model-polys-serialize.md](bsp-model-polys-serialize.md), which had grown past the 500-line cap with two independent subjects appended to it.

---

## Summary

This session also exercised the existing `ATerrain::Serialize` recipe
(`.claude/agent-memory/game-archaeology-specialist/ue3-terrain-serialize.md`,
previously 92% confidence, validated only against the `Castle_CellBlock`
map) against `Castle-000a0002.umap`'s own `Terrain` export — a different map
(`Maps/Castle/`, not `Maps/Castle_CellBlock/`) and a much larger sample
(522123 bytes vs. ~9372). Full details are in the agent-memory file's
"Real-data validation, Castle-000a0002.umap" section; summary:

- **One bug found and fixed**: a flat byte-skip property-tag parser must
  *not* apply the `ArrayProperty` extra-8-byte-inner-type-FName rule from
  `docs/engine/ue3-package-format.md`'s general property-stream section —
  that rule is for a *recursive* parser that walks into the array's
  contents; a parser that just jumps forward by the tag's declared `size`
  (treating the array as an opaque blob) does not encounter it and adding
  it misaligns everything downstream. With that fixed, the whole 1620-byte
  property stream parsed cleanly to one `None` terminator (no ambiguity —
  the earlier "first `None` is inside `Layers`, use the last one" GOTCHA
  was specific to a recursive-parser design, not the byte-skip approach).
- **Trailer layout confirmed byte-exact** on real data:
  `Heights.Num=10201` (=101×101=`NumVerticesX*NumVerticesY` exactly),
  `InfoData.Num=10201`, `AlphaXSize`/`AlphaYSize` binary copies both matched
  the tagged-property values, `WeightedTextureMaps.Num=3` (this tile has 3
  texture layers, not the "usually 1" from the smaller sample),
  each `WTM[i].Num=163216` (=404×404=`AlphaXSize*AlphaYSize` exactly, all
  three), `WeightMapTextures.Num=0`. Consumed 521959 of 522123 bytes — the
  164-byte remainder is the un-decoded lighting/foliage trailer, same as
  the smaller sample's 152 bytes (expected variance, not an error). Heights
  are genuinely non-flat (5149 distinct `u16` values across 10201 samples,
  range 44226–54199) — real shape data, plausible.
- **Resolves the "3 Terrain actors × many TerrainComponents" framing**:
  it's not one architecture. `Castle-000a0002.umap` has exactly **one**
  `Terrain`-class export with `NumSectionsX=NumSectionsY=5`, and
  `NumSectionsX * NumSectionsY = 25` matches this tile's `TerrainComponent`
  export count exactly — components are spatial/LOD partitions of one
  `Terrain`'s data. `Castle_CellBlock` instead has 25 *separate* small
  `Terrain` actors (per the original agent-memory worked example) with no
  `TerrainComponent` subdivision. Both are legitimate UE3 patterns; a
  decoder must walk every `Terrain`-class export in a chunk independently
  rather than assuming a fixed count.
- **Contradicts an established campaign fact**: the worker-rules brief for
  this spike states `Castle-000a0002.umap` has "3 Terrain" — the actual
  count, confirmed by a full class-histogram scan, is **1**. (`Castle-000a0003.umap`,
  named alongside it, was not checked in this session — the "3 Terrain"
  figure may describe that tile, a different tile, or an aggregate across
  several tiles; flagging the specific discrepancy rather than guessing
  which.)
- **Could not close**: cross-checking decoded terrain height against a
  known world-8 outdoor seed coordinate. No world-8 respawner in
  `db/resources/Worlds/Seed/respawners.sql` (4 rows total) falls inside this
  chunk's ~X∈[200,300)/Z∈[1000,1100) footprint. `Castle-000a0002.umap` is
  one of the campaign's two named *interior* tiles, so its `Terrain` data
  may be a basement/ground-cap plane beneath the BSP interior rather than a
  walkable outdoor surface — genuinely open, not just unattempted. Closing
  this needs an outdoor Castle tile with a matching seed coordinate.

Net: Phase 1.3 (`Terrain` decoder) confidence raised from 92% to ~96% — the
layout is now validated on two structurally different real samples across
two different maps, with one parser bug found and fixed. The coordinate
cross-check remains open for a future session with access to an outdoor
tile + matching seed data.
