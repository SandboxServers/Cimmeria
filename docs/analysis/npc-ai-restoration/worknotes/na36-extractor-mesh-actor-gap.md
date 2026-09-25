# NA36: navmesh/occluder extractor geometry gaps

Measured 2026-09-25 against the cooked `SGWGame/CookedPC` client tree and the
NA26/NA28 extraction pipeline (`crates/navmesh-extractor`).

## The bug

`staticmesh::collect_static_mesh_instances` filtered exports on
`class == "StaticMeshActor"` exactly. `AInterpActor` (Matinee-driven
movers), `AKActor` (rigid-body physics props) and
`AFracturedStaticMeshActor` (destructible meshes) all derive from
`AStaticMeshActor` in UE3 and carry the identical placement +
`StaticMeshComponent` shape — the same `Location` / `Rotation` /
`DrawScale` / `DrawScale3D` tagged properties, the same
`StaticMeshComponent` object reference, the same optional
`bCollideActors` gate — but the class-name filter dropped every export of
these three classes before the walker's `for` loop even visited them. Not
into a `SkipReason`: invisible. `coverage::COLLISION_BEARING_CLASSES` had
already named all three (plus `StaticMeshCollectionActor`) as a documented
gap; this packet closes it for the three that turned out to matter.

## Evidence chain

1. `data/spaces/README.md`'s existing "Replaced" note for `harset.nav`
   already recorded that NA26's rebuild lost 6 of 43 real accepted
   telemetry positions ("raised platforms at about y -59/-61... an actor
   class the extractor does not decode is the likely cause").
2. `docs/analysis/harset-rebuild/placements/data/harset_lastvalid_probes.txt`
   (41 rows, weighted by occurrence count) has 5 rows in that y band:
   `lv06_n1585 (9.5,-58.8,59.4)`, `lv07_n1256 (34.6,-61.3,-70.7)`,
   `lv14_n71 (47.2,-59.1,-90.6)`, `lv18_n25 (38.1,-61.3,-86.7)`,
   `lv20_n22 (25.6,-59.1,-82.0)`.
3. `docs/analysis/harset-rebuild/placements/B-world57-population-and-regions.md`
   separately flags seeded spawn 308 (`Harset_ShieldTower1` console) as
   "probably standing on one of the raised platforms the extractor does
   not decode" (NA29), 3.44 m above the rebuilt mesh.
4. `extract_map`'s class census (`--classes`) on the shipped Harset
   chunks: 31 `InterpActor` exports, all `NotDecoded`. No `KActor` or
   `FracturedStaticMeshActor` in the map at all.

Hypothesis at the start of this packet: item 4 explains items 1-3. It does
not, for either — see "What this did not fix" below. It is still a real,
independently-justified fix: 31 actors in Harset alone (965 across 15 of
23 maps) were completely invisible to the extractor and are not any more.

## The fix

`staticmesh::MESH_ACTOR_CLASSES` (`crates/navmesh-extractor/src/staticmesh/mod.rs`):

```rust
pub const MESH_ACTOR_CLASSES: &[&str] = &[
    "StaticMeshActor",
    "InterpActor",
    "KActor",
    "FracturedStaticMeshActor",
];
```

`collect_static_mesh_instances`'s class filter changed from an exact
string compare to `MESH_ACTOR_CLASSES.contains(&class)`. Nothing else in
the resolution chain changes: the archetype `collides()` gate, the
`CollideActors` component-level check, and mesh-ref resolution are all
class-agnostic already — they read tagged properties, not the export's
class name. A mover explicitly marked non-colliding is still (correctly)
skipped, unchanged from before.

`coverage::DECODE_STATUS` gained three rows marking `InterpActor`,
`KActor` and `FracturedStaticMeshActor` `Decoded`, which flips their
`collision_risk` column to `no` in the class-census TSV and removes them
from `extract_map`'s "undecoded classes present" summary — that summary
loop was also fixed to filter on `decode_status(class) == NotDecoded`
rather than printing every `COLLISION_BEARING_CLASSES` entry with a
nonzero count regardless of status (a latent inaccuracy the Decoded rows
exposed: it would otherwise have kept reporting `Terrain`/`Model`, both
long since decoded, as "undecoded" too).

`StaticMeshCollectionActor` — UE3's cooked batching actor, holding an
*array* of `StaticMeshComponent`s rather than one — is deliberately not
in `MESH_ACTOR_CLASSES`. It needs its own walk. The 23-map census below
found zero exports of it anywhere, so there is nothing to decode yet.

## 23-map class census (item 2)

Re-ran `extract_map --classes` over every map under `CookedPC/Maps`
(23 directories, matching `entities/spaces.xml` plus `Login_Map`) with the
fixed binary:

| Class | Maps carrying it (exports) | Total | Decoded now |
|---|---|---:|---|
| `InterpActor` | Agnos (2), Beta_Site_Evo_1 (94), Castle (14), Castle_CellBlock (53), Dakara_E1 (10), Harset (31), Harset_CmdCenter (1), Login_Map (16), Lucia (359), Menfa_Dark (125), Menfa_Light (50), Omega_Site (4), SGC_W1 (22), Sewer_Falls (2), Tollana (182) | 965 | yes |
| `KActor` | none | 0 | n/a |
| `FracturedStaticMeshActor` | none | 0 | n/a |
| `StaticMeshCollectionActor` | none | 0 | still `NotDecoded` — nothing to decode |

`InterpActor` is the only one of the three widened classes with any
shipped content — `KActor` and `FracturedStaticMeshActor` cost nothing
today but are correct to support (they are true `AStaticMeshActor`
siblings, and either could gain content in a future re-cook or a map this
tree hasn't sampled). The fix benefits all 15 `InterpActor`-carrying maps
uniformly the next time each is rebuilt; only Harset's family is rebuilt
in this packet (packet ownership is `data/spaces/harset*.nav/.occ` only —
see [work-packets.md NA36](../work-packets.md)).

Concrete validation from Castle (not rebuilt by this packet, but confirms
the fix does something real): 14 `InterpActor`s decode there, 11 security
camera heads, 1 antenna, 1 shelf box, and
`GLB-Global:GLB-RingTransporter00` at BigWorld (466.45, 70.06, 991.55) —
the map's ring-transport platform, which previously had **zero** collision
geometry despite being something players stand on.

## What this did not fix

Both original evidence items (the telemetry cluster and spawn 308) turned
out to be different problems, confirmed by direct investigation against
the *fixed* extraction — not assumed.

### The five clustered telemetry points

Inverting the CA05 axis mapping (`bw = (ue.Y, ue.Z, ue.X) / 100`,
`floor_probe::axis::AxisMapping::CA05`) gives each point's raw UE3
`Location`. A scratch tool (`examples/dump_actors_near.rs`, deleted
before this branch's final commit — not shipped) walked every export of
every class in the map, searching a wide horizontal radius at any height:

- `lv06` (9.542, -58.823, 59.404) → UE3 (5940.4, 954.2, -5882.3). Nothing
  of any class within 30 m horizontally at the target height, in either
  of the two chunks the search touched. `obj_slab --at
  9.542,-58.823,59.404,3,40` (a ±40 m vertical half-range) finds triangles
  only at `y[-70.00,-68.50]` — the ground-level plaza floor, ~10 m below.
- The chunk's own coverage row (`Harset-00000000`) balances exactly:
  `skip_collision_disabled=27`, zero of any other skip reason, 248
  StaticMeshActor-family actors all accounted for. Nothing failed to
  resolve here; nothing that could resolve was ever placed at this
  height.

This rules out "an export exists here and some `SkipReason` drops it" —
there is no candidate export at all. The leading unconfirmed hypothesis:
a genuinely *animated* `InterpActor` — a rising platform — whose cooked
`Location` sits at its resting (ground) pose, not its raised position at
the moment a real player stood on it. This is exactly the risk this
crate's own README already flagged before this packet ("a mover's cooked
`Location` is its editor-time pose, not necessarily where it rests at
runtime"). No `InterpActor` export was found within a useful radius of
this specific cluster to confirm or deny it — the nearest (a group of 5
in chunk `Harset-00000000`) sit 47 m away at true ground height. Left
open. A follow-up needs either a live-client `.location` reading at one
of the telemetry coordinates, or tracing whichever Matinee sequence (if
any) targets an actor in this area.

Two further `nav_inspect` failures against the same probe file (`lv19`,
dy +3.30 m; `lv24`, dy +3.42 m, different XZ, different height) were
**not** traced further. They are smaller gaps at different heights and
may not be the same problem; flagging rather than folding them into the
five-point hypothesis without evidence.

### Spawn 308 (`Harset_ShieldTower1`)

Its UE3 location (3772.0, -22300.0, -4136.0) has real, decoded geometry
within 3 m horizontally at almost exactly the target height:

```text
PrefabInstance  loc=(3768.0,-22530.0,-4258.0)  dxy=2.30 m  dz=-1.22 m
StaticMeshActor loc=(3772.0,-22604.0,-4136.0)  dxy=3.04 m  dz=0.00 m
PrefabInstance  loc=(3772.0,-22604.0,-4136.0)  dxy=3.04 m  dz=0.00 m
```

The `GA-TowTall01` tower's prefab and its `StaticMeshActor` **are**
present and already decoded (`StaticMeshActor` was never the gap). The
problem NA29 already identified stands confirmed: this is a tall,
hillside-mounted compound mesh whose cooked origin is not its walkable
console height. `obj_slab` finds no dominant flat level in the column
(20+ thin ramp slices between y -53 and -31 — a hillside, not a floor).
`nav_inspect` against the rebuilt `harset.nav` reports `dy=+3.45 m`,
unchanged by this fix. This needs an in-client `.location` reading at the
tower's actual console platform, exactly as NA29 recommended; there is no
static-analysis path to the right number.

## Seeded-spawn Y audit (item 3, Harset only)

Cross-referencing every still-open off-mesh Harset `spawnlist` row
([B-world57-population-and-regions.md](../../harset-rebuild/placements/B-world57-population-and-regions.md))
against the fixed extraction:

| Spawn | What | Classification | Action |
|---|---|---|---|
| 303, 304, 306, 307, 313 | Jaffa camp pair, bug baskets, Petbe's search object | Already resolved (NA29 made them mobile against the rebuilt mesh) | none |
| 308 `Harset_ShieldTower1` | Shield tower 1 console | (c) not a decode gap — Y-calibration on a hillside compound mesh | needs a live `.location` reading; no seed change (confidence too low) |
| 309 `Harset_ShieldTower2` | Shield tower 2 console | Already resolved (NA28/NA29 repinned to an adjacent terrace) | none |
| 310 `Harset_ShieldTower3` | Shield tower 3 console | Real terrain exists almost exactly at the seeded Y (two overlapping terrain sheets, `y[-31.0,-30.5]` and `y[-29.5,-29.0]`); the 24.61 m gap is a **navmesh-connectivity** problem — the nearest polygon is on a distant, disconnected component | out of scope (candidate for a tiled-rebuild follow-up, NA28-style) |
| 311 `Harset_ShieldControls` | Shield controls prop | Already LOW confidence / INFERRED; the ledger itself proposes deletion if unconfirmed | no change — an existing owner decision, not re-litigated |

No row crossed the "seed Y wrong, high confidence" bar this packet
requires before touching `db/resources`. No seed changes shipped.

## Rebuild and regression results

Params from the original NA26 build log
(`/c/Users/Steve/AppData/Local/Temp/cimmeria-castle/navmesh/na26/build_summary.txt`,
the `L1` rows): `partition=watershed agentHeight=1.8 agentClimb=0.6
agentRadius=0.6 ch=0.2 cs=0.3 minRegionSize=24 maxSimplificationError=1.3`,
`bounds=-420,-520,420,420` for Harset and `bounds=-120,-120,220,120` for
Harset_CmdCenter, via the promoted `bin64/NavBuilder.exe`.

| File | Before | After | Verdict |
|---|---|---|---|
| `harset.nav` | 29,768 / 15,287 / 42,379 (v/p/e) | 29,772 / 15,289 / 42,385 | rebuilt |
| `harset.occ` | 3,764,364 bytes | 3,764,999 bytes | rebuilt, self-check passed |
| `harset_cmdcenter.nav` | 1,130 / 570 / 1,580 | byte-identical | not touched |
| `sandbox.nav` (copy of the above) | — | — | not touched |
| `harset_market.nav` | — | 0 instances of any of the 3 classes | not touched |
| `harset_storagerm.nav` | — | 0 instances of any of the 3 classes | not touched |

`nav_inspect --probes` against both `probes/Harset.txt` (the 67-row
seeded NA26 set) and `harset_lastvalid_probes.txt` (41 rows) gives the
**identical** pass/fail set before and after (56/67, 34/41), same failing
rows both times, only poly-index renumbering elsewhere on the mesh. This
satisfies "must accept ≥ old" as an equality, not an improvement — none
of the new `InterpActor` triangles happened to land under a currently
failing probe.

## Tests

`crates/navmesh-extractor/src/staticmesh/mesh_actor_class_tests.rs` (new,
7 tests, package-backed synthetic fixtures, no cooked client tree):
`InterpActor`/`KActor`/`FracturedStaticMeshActor` each resolve a mesh
reference identically to `StaticMeshActor`; a class outside the family
(`Pawn`) is still fully ignored; `MESH_ACTOR_CLASSES`'s exact contents are
pinned; `StaticMeshCollectionActor` is pinned as NOT yet in the family.
`interp_actor_is_not_silently_invisible_to_the_walker` is the direct
regression guard — it fails if the class filter reverts to an exact
`"StaticMeshActor"` string compare.

`coverage::tests::decode_status_tracks_the_phases_that_have_landed` and
`collision_risk_is_the_intersection_not_the_whole_list` updated for the
new `Decoded` rows (both pinned tests, not new).

Full suites run against the rebuilt `harset.nav`/`harset.occ`:
`cimmeria-navmesh-extractor` 420/420, `cimmeria-entity` (folded into the
same nextest run, 738/738 combined), `cimmeria-services` 3,274/3,275 (1
live-DB self-skip, no `DATABASE_URL`), `cimmeria-server` 30/30. Targeted
Harset-family tests checked explicitly:
`cell::harset_placement_tests::*`, `cell::spawner::tests::harset::*`
(including `world57_placement::world57_mobile_placements_can_walk` and
`world57_placements_match_their_recorded_navmesh_verdict`),
`cell::service::tests::npc_ai::off_mesh_sentry`,
`cell::space_manager::tests::movement_validation::*`,
`navigation::line_of_sight_tests::*` — all pass unchanged.

## Cross-references

- [navmesh-build-pipeline.md §11](../../../engine/navmesh-build-pipeline.md#11-mesh-actor-class-gap-interpactor--kactor--fracturedstaticmeshactor-na36-2026-09-25) — the canonical writeup (method + evidence)
- [work-packets.md NA36](../work-packets.md) — packet summary and acceptance
- [data/spaces/README.md](../../../../data/spaces/README.md) — provenance and the corrected "Replaced" note
- [crates/navmesh-extractor/README.md](../../../../crates/navmesh-extractor/README.md) — "Known unknowns" bullet, updated
- [B-world57-population-and-regions.md](../../harset-rebuild/placements/B-world57-population-and-regions.md) — the seeded-spawn evidence this audit cross-referenced
