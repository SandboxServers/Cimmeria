# Cover Extraction (UE3 `.umap` → `resources.cover_*` seeds)

> **Last updated**: 2026-09-25
> **Status**: Castle_CellBlock (world 12) and Castle (world 8) extracted and seeded (NA21). Other worlds still have no cover rows.
> **Evidence**: [cover-world-placement.md](../reverse-engineering/findings/cover-world-placement.md) (where the data lives), [cover-system.md](../reverse-engineering/findings/cover-system.md) (enum meanings, runtime slot layout)

How the NPC cover system and the content engine's `player_entered_cover`
triggers get their world-space cover positions, and how to regenerate them.

```text
CookedPC/Maps/<Map>/<Map>-XXXXXXXX.umap
        │  cover_extract (crates/navmesh-extractor)
        ▼
db/resources/AI/Seed/cover_sets.sql     one row per cover set, world-scoped
db/resources/AI/Seed/cover_nodes.sql    one row per cover node, BW metres
        │  cell startup: cover::load_cover_sets / load_cover_nodes
        ▼
Cover { index: per-world CoverIndex }   cell/cover/spatial.rs
```

## 1. What is extracted

Castle's gameplay cover is authored directly in each map chunk, in two
shapes. Both are already absolute UE3 world space, so neither needs the
owner-transform composition the older prefab corpus needed.

| Pattern | Actor | Where the transform comes from | Castle_CellBlock | Castle |
|---|---|---|---|---|
| A | `SGWSpecCoverNode`, one per node | the actor's `Location` / `Rotation` / `DrawScale3D` | 236 | 3,780 |
| B | `StaticMeshActor` with a `CoverNodeArray` | each `SGWCoverNodeComponent`'s own `Translation` / `Rotation` / `Scale3D`, flagged `Absolute* = true` | 0 | 8 |

The walker visits every `SGWCoverNodeComponent` export once and lets its
outer export's class decide the pattern. It does not look components up by
name, because 113 components in one chunk share the literal name
`SGWCoverNodeComponent`. A Pattern B child that is not absolute is composed
with its owner's transform and counted. None exist in either Castle map.

Per node it emits:

| Column | Source | Notes |
|---|---|---|
| `pos_x, pos_y, pos_z` | Pattern A actor `Location`, Pattern B component `Translation` | BigWorld metres, `bw = (ue.y, ue.z, ue.x) / 100` ([navmesh-build-pipeline.md §1](navmesh-build-pipeline.md#1-the-coordinate-mapping)) |
| `orient` | the marker's local +X axis, pushed through the same scale-then-rotate chain as mesh vertices | radians in `[0, 2π)`, facing `(cos, sin)` in BW `(x, z)`. This is the convention `cell/cover/scoring.rs` uses, **not** entity yaw: `orient = π/2 − yaw`. A mirrored marker (negative `DrawScale3D.x`) faces the way its rendered arrow does. |
| `height` | `CoverHeight` byte | `ECoverHeight` ordinal: 0 Low, 1 Mid, 2 High, 3 LOS. Every Mid marker has `DrawScale3D.z = 1.067` and every High one 1.524, matching the heights read out of `SGW.exe`. |
| `quality` | `CoverQuality` byte | `ECoverQuality` ordinal: 0 Good, 1 Better, 2 Best, 3 None |
| `width` | `CoverWidth` | metres, equal to the marker's `DrawScale3D.y` |
| `tail` | none | four zero bytes; the column is a leftover of the retired `.pak` record format |

### Omitted properties

The cooker writes only properties that differ from the archetype, so a
missing cover property is the archetype's value, not zero. The values
below come from a census of all 4,024 Castle markers:

| Property | Archetype value | Evidence |
|---|---|---|
| `CoverHeight` | 0, Low | 240 markers omit it, and all have `DrawScale3D.z = 0.71` (the Low height) or an unconfigured 1.0. Only bytes 1 and 2 are ever written. |
| `CoverWidth` | 1.0 | Omitted exactly when `DrawScale3D.y = 1.0`. |
| `CoverQuality` | 3, None | Bytes 0, 1, 2 and 4 are written, and 3 never is. MEDIUM confidence, because no script package in the client tree carries `Default__SGWCoverNodeComponent` to read. The 11 markers affected also have an explicit 0.0 width, which makes them placeholders. |

Seven Castle markers carry `CoverQuality = 4`, which is outside the enum.
They are emitted as QUALITY_None and counted in the seed header.

## 2. Sets and ids

A cover set is what an NPC reserves against and what
`player_entered_cover` reports as `cover_set_id`.

- A Pattern B owner is one set.
- Pattern A markers are grouped by single linkage. Two markers share a set
  when they are within 3.5 m horizontally and 1.0 m vertically of each
  other, transitively. The thresholds keep the med-station desk's seven
  markers in one set: their nearest neighbours are 2.3 to 3.0 m apart.
- Set ids are `world_id * 100000 + n`, numbered in chunk-file then export
  order, and node ids run 0.. within a set. The same client build always
  produces the same ids. A different build may renumber them.

Result: Castle_CellBlock has 58 sets and Castle has 481. Most sets have 1
to 8 nodes. The largest is 105 nodes in a 17 × 14 m courtyard in Castle.
The desk is set **1200001**.

Content chains key on set ids. Chains 1132/1133 (mission 639, objective
2484) use 1200001. The live-DB test
`cover_chain_key_is_the_set_a_player_at_the_desk_is_in`
(`chain_replay_tests/mission_639_cover.rs`) fails if a re-extract moves
the desk to another id.

## 3. Schema

`resources.cover_sets.world_id` (`NOT NULL`, FK to `resources.worlds`)
scopes each set, and `resources.cover_nodes.width` carries the marker
width. The cell joins every node to its set's world and partitions the
spatial index on it. `CoverIndex::nearby` takes a world id and never
returns a node from another world. This matters in practice: Castle and
Castle_CellBlock overlap in BigWorld coordinates. Instances of one world
share that world's cover.

The 9,346 rows decoded from `covernodes_nikols.pak` /
`covernodes_sdeiter.pak` by `tools/ue3_extract_cover_nodes.py` were dropped
from the seed. That corpus is a library of per-prefab templates, so its
positions are prefab-local offsets. Loading them as world positions put
every node in the wrong place in every world. The script stays in the repo
as the only decoder for that format.

## 4. Regenerating the seeds

```bash
cargo run --release -p cimmeria-navmesh-extractor --bin cover_extract -- \
  --map "12=Castle_CellBlock=<CookedPC>/Maps/Castle_CellBlock" \
  --map "8=Castle=<CookedPC>/Maps/Castle" \
  --sets-out db/resources/AI/Seed/cover_sets.sql \
  --nodes-out db/resources/AI/Seed/cover_nodes.sql \
  --client-build "Stargate Worlds-QA/Working (SGW.exe QA client, SGWGame/CookedPC)"
```

`--map` takes `<world_id>=<world name>=<map directory>`, using `=` because
a Windows path contains `:`. List every world that should have cover. The
tool rewrites both files whole, and a world you leave out loses its rows.
It prints one summary line per map, with node, set and pattern counts,
defaults applied and nodes skipped, and writes the same lines into both
seed headers. The output is deterministic, so a re-run against the same
client is a no-op diff. It needs no package index, because cover nodes are
top-level actors with no cross-package references.

After regenerating, check that:

1. The summary shows `skipped 0` and `0 composed` for every map.
2. The set id of any cover set a content chain keys on has not moved. The
   live-DB test above catches the desk. `grep player_entered_cover
   db/resources/Content/Seed` finds the others.
3. `live-db-test.sh cover` and `live-db-test.sh mission_639` pass.

## 5. Tests

- `crates/navmesh-extractor/src/cover/tests.rs` covers both patterns,
  absolute and composed Pattern B children, mirrored scale, the UE→BW
  mapping and facing convention, archetype defaults, out-of-range bytes,
  grouping (the desk fixture uses NA20's real coordinates) and the SQL
  rows. It builds synthetic packages with `test_support`, so it runs in CI
  without client assets.
- `crates/cell-cover/src/cell/cover/tests.rs::nearby_never_returns_another_worlds_nodes`,
  `scoring.rs::pick_best_never_picks_another_worlds_slot` and
  `service/ticks/cover.rs::cover_detection_tick_ignores_another_worlds_nodes`
  pin the per-world index.
- `loader_live_db_tests.rs` loads the real seed. It checks that world 12's
  236 nodes are scoped to world 12, that the `.pak` rows stay retired, and
  that a node sits within 2 m of the retired set 1381's desk position.

## Cross-references

- [cover-world-placement.md](../reverse-engineering/findings/cover-world-placement.md): NA20 finding, the evidence for everything above
- [navmesh-build-pipeline.md](navmesh-build-pipeline.md): the axis mapping and the extractor crate this tool lives in
- `crates/cell-cover/src/cell/cover/`: loader, per-world spatial index, reservation, scoring, detection
- [NPC AI work packets](../analysis/npc-ai-restoration/work-packets.md): NA21 (this), NA22 (cover behaviour)
