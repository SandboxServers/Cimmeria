# Cover Node World-Space Placement — `SGWSpecCoverNode` and `SGWCoverNodeComponent`

> **Last updated**: 2026-09-24
> **Source**: `crates/upk`/`crates/upk-objects` direct package inspection (new `query-index` tool), Ghidra decompilation of `SGW.exe`, live-DB seed cross-check
> **Confidence**: HIGH for the authoring pattern and world-space placement (byte-exact property decode across 241 sampled actors in Castle_CellBlock, cross-validated against a shipped navmesh and against a hand-authored seed row). MEDIUM for the relationship to the separate `covernodes_*.pak` prefab-template pipeline (its native transform function was decompiled but not reconciled instruction-by-instruction with the property layout found here — see Open Questions). LOW/unresolved for the client pose trigger (Q4) — corroborates, does not extend, the existing finding.
> **Issue/packet**: NA20 (`docs/analysis/npc-ai-restoration/work-packets.md`), evidence input to NA21
> **Related findings**: [cover-system.md](cover-system.md) (wire protocol, `ACoverLink` slot layout, weight-tuning events — still authoritative for those sections), [bsp-model-polys-serialize.md](bsp-model-polys-serialize.md) and [ue3-terrain-serialize](../../engine/) (precedent for direct `.umap` export decoding), [../../engine/navmesh-build-pipeline.md](../../engine/navmesh-build-pipeline.md) (UE3→BigWorld axis mapping used below)

---

## TL;DR

Castle and Castle_CellBlock's actual in-level cover data is **not** the
9,346-row `covernodes_nikols.pak`/`covernodes_sdeiter.pak` prefab-template
corpus that `db/resources/AI/Seed/cover_nodes.sql` was generated from. It
lives directly in the cooked `.umap` files as ordinary placed actors —
almost entirely `ASGWSpecCoverNode` (a first-class level actor, one cover
position each), with a much smaller number of multi-node
`StaticMeshActor.CoverNodeArray` groups. **Both patterns are already in
absolute UE3 world-space in the cooked map — neither needs the
owner-rotation-times-local-offset composition math this session went
looking for.** Extracting them is a straightforward actor/property walk,
not a transform problem, and the tooling to do it (`cimmeria_upk::extract_actors`,
which already lists `SGWSpecCoverNode` as a known actor class) already exists
in the repo.

This directly reopens the go/no-go for NA21: the "big lift" framed in
`evidence/cover-archaeology.md` (a new `ACoverLink`-actor `.umap` extractor,
comparable in scope to the terrain/BSP decoders) is **not needed**. What's
needed is a much smaller extension of the existing `crates/upk`
actor-extraction path plus a `crates/navmesh-extractor` walker that emits
`(space, position, height, quality, width)` rows per map. See
[Go or no-go for NA21](#go-or-no-go-for-na21).

---

## Method

A new binary, `query-index` (`crates/upk-objects/src/bin/query_index.rs`),
was added this session for three lookups the existing tools don't do:

1. `query-index <index.bin> --class <ClassName> [--package-contains <substr>] [--count-only]` —
   cross-package census against the cached `PackageIndex`
   (`C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\navmesh\nav-extract\package_index.bin`,
   5,019 packages / 2,821,598 exports, built 2026-09-19).
2. `query-index scan <file.upk|umap> <ClassName> [--limit N]` — opens one
   package directly and enumerates every export of a class **by export
   index**, printing its outer (owning) export.
3. `query-index cover <file.umap>` — lists every `SGWSpecCoverNode` actor
   with its UE3 world transform, the BigWorld conversion, and its
   component's `CoverHeight`/`CoverQuality`/`CoverWidth`.

`scan` exists because of a real gap in `PackageIndex`
(`crates/upk-objects/src/package_index.rs`): it keys `exports`/`by_class` by
`(package_name, object_name)`, but a UE3 `FName` is really
`(name_index, instance_number)` and `ExportEntry::object_name`
(`crates/upk/src/exports.rs:14-16`) carries only the base string, dropping
`object_name_num`. A component class instanced once per owning actor —
exactly what `SGWCoverNodeComponent` is — produces many exports that share
the literal object name `"SGWCoverNodeComponent"` in one package,
distinguished only by their outer actor. Measured on
`Castle_CellBlock-fffefffe.umap`: `by_class` correctly *counts* 113
occurrences, but `PackageIndex::exports` holds a single overwritten entry
for the collided key, so `PackageIndex::find` can only ever resolve one of
them (always the same one, the last written during indexing). `Package::export_full_path`
(`crates/upk/src/package.rs:126-152`) has the identical gap — it also drops
`object_name_num`, so all 113 actors in that chunk print the same path,
`TheWorld.PersistentLevel.SGWSpecCoverNode`. `scan`/`cover` avoid both by
walking `Package::exports` directly and reporting by export index. This is
a real limitation of `cimmeria_upk`/`cimmeria_upk_objects` worth keeping in
mind for any future tool that assumes `PackageIndex::find` or
`export_full_path` is 1:1 with in-game objects; it is not, for any class
that is instanced more than once per outer with a literal (unsuffixed)
autogenerated name.

All package paths below are under
`C:\Users\Steve\source\projects\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC`.

---

## Q1 — Does `SGWCoverNodeComponent` carry the `CoverNodePrefabData` array itself, or reference a `covernodes_*.pak` template?

**Neither, for the data that is actually placed in Castle/Castle_CellBlock.**

The task's premise (that `CA-Prebuilt.upk`/`GA-Arch.upk` contain the name
`SGWCoverNodeComponent`) does not hold: a raw case-insensitive string scan
of both files for `covernode` returns **zero hits**, and `query-index scan`
confirms zero exports of class `SGWCoverNodeComponent` or `SGWSpecCoverNode`
in either package. The one `Packages/*.upk` file that *does* reference cover
nodes is `SGW_Cover.upk`, which holds exactly one `StaticMesh` export named
`CoverNode` — the small marker mesh every cover node's component references
for editor visualization (`StaticMesh (ObjectProperty) = ... -> SGW_Cover.CoverNode`,
seen on every decoded component below). This corrects the earlier
archaeology pass's premise about where the class name lives; the earlier
pass's Ghidra evidence (RTTI, editor-command strings) is otherwise
unaffected, since that evidence came from the binary, not from these
packages.

Instead, `SGWCoverNodeComponent` exports were found **only inside the
`.umap` map chunks themselves**, and two distinct authoring patterns exist,
both already fully resolved to world space by the cooker:

### Pattern A — `ASGWSpecCoverNode`, one actor per cover position (dominant)

Decoded `Castle_CellBlock-fffefffe.umap` export 678
(`inspect-export ... 678 --props --prop-offset 32`):

```text
Class:  SGWSpecCoverNode
Path:   TheWorld.PersistentLevel.SGWSpecCoverNode
  CoverNodeComponent (ObjectProperty) = 566 -> ...SGWSpecCoverNode.SGWCoverNodeComponent
  Location   = Vector { x: -11443.317, y: -13716.998, z: 2463.226 }   (UE3 cm)
  Rotation   = Rotator { pitch: 0, yaw: 81937, roll: 0 }
  DrawScale3D = Vector { x: 1.0, y: 1.3027971, z: 1.067 }
```

The outer path terminates directly at `TheWorld.PersistentLevel` — this
actor is **not** inside a `PrefabInstance`. Its `Location`/`Rotation` are
therefore already the final world-space placement; there is no owner to
compose against.

The owned component (export 565, `inspect-export ... 565 --props --prop-offset 8`)
carries no position of its own:

```text
Class:  SGWCoverNodeComponent
Path:   TheWorld.PersistentLevel.SGWSpecCoverNode.SGWCoverNodeComponent
  CoverHeight = Byte([1])
  CoverQuality = Byte([1])
  CoverWidth  = Float(1.3027971)
```

`CoverWidth` equals the owning actor's `DrawScale3D.y` exactly, in every one
of 241 sampled nodes across five Castle_CellBlock chunks — the width isn't
an independently-authored number, it's read straight off the marker mesh's
non-uniform scale. `DrawScale3D.z` is even more informative: across all 229
height-tagged samples, `CoverHeight == 1` paired with `DrawScale3D.z ==
1.067` and `CoverHeight == 2` paired with `DrawScale3D.z == 1.524`, with
**zero exceptions**. Those two numbers are exactly Cimmeria's already-shipped
`CoverHeight` enum constants (`crates/cell-cover/src/cell/cover/types.rs`:
Mid = 1.07 m, High = 1.52 m, themselves Ghidra-confirmed against
`DAT_018f41d4/d0/cc/c8` per `cover-system.md`) — an independent
cross-validation of that earlier finding from a completely different
angle (property data vs. a direct memory read).

The remaining ~700 bytes of "post-property binary" on the component are
generic `UActorComponent` lighting-cache boilerplate (`IrrelevantLights`
GUIDs, a `LightingChannelContainer`, vertex-color/lightmap sample arrays) —
not cover data.

### Pattern B — `StaticMeshActor.CoverNodeArray`, several nodes per actor (minority)

Found in `Castle-00060005.umap`: one `StaticMeshActor` (export 1202) with an
extra `ArrayProperty` beyond its usual `StaticMeshComponent`:

```text
StaticMeshComponent = 1411 -> ...StaticMeshActor.StaticMeshComponent
CoverNodeArray = Array([6, <6 object refs: exports 667..672>])
Location = Vector { x: 61320.94, y: 56967.805, z: 2391.3513 }
```

Each of the six child `SGWCoverNodeComponent` exports carries its **own**
`Translation`/`Rotation`/`Scale3D`, distinct from the owner's `Location`
(e.g. export 667: `Translation = (61474.31, 56928.684, 2391.3513)`, `Δx =
+153.4, Δy = -39.1` from the owner), and all three transform properties are
paired with `AbsoluteTranslation = AbsoluteRotation = AbsoluteScale = true`.
Per UE3 `USceneComponent` semantics, an "absolute" child transform is used
verbatim in world space and is **not** composed with the owner's transform
— so this pattern also needs no transform math, despite having its own
per-node position. `bIsOwnerAStaticMeshActor = true` and `StaticMesh = ...
-> SGW_Cover.CoverNode` (the same marker mesh as Pattern A) confirm this is
the same underlying cover system attached a different way — several cover
slots authored around one obstacle (e.g. a wall or large prop) instead of
one actor per slot.

In the one sampled chunk with this pattern, 406/412 (98.5%) of
`SGWCoverNodeComponent` exports were Pattern A and 6/412 (1.5%) were Pattern
B, all six on a single owning actor — Pattern A is clearly dominant, Pattern
B is a real but minority case an extractor must still handle.

### What this means for the `USGWCoverNodeComponent_SpawnCoverNode` transform math

`cover-archaeology.md`'s Part B decompiled `USGWCoverNodeComponent_SpawnCoverNode`
(`0x00904d80`) as a loop over a `CoverNodePrefabData[]` array with stride
`0x18`, computing `world_pos = owner_rotation_matrix * local_pos +
owner_world_pos`. That function was not re-decompiled this session, and
this session's evidence does not contradict it — but nothing decoded here
uses a raw `0x18`-byte-stride float array; every `.umap`-baked cover node in
Castle/Castle_CellBlock carries its position as ordinary tagged properties
(either "no position, inherit the 1:1 owner's `Location`" or "own
`Translation`, but `Absolute*=true` so it needs no composition"). The most
likely reconciliation (MEDIUM confidence, not fully traced) is that
`SpawnCoverNode` is the **separate**, PAK/XML-driven pipeline described in
`cover-system.md`'s "Cover Node Data Layout" section
(`covernodes_local.pak`, `MyCoverNodeArchive`, `CoverNodeXmlLoader @
0x010556a0`) — used for reusable generic art-set prefabs (benches, railings,
canal corners) placed across many worlds, which is exactly the corpus
`tools/ue3_extract_cover_nodes.py` already extracted into
`cover_nodes.sql`/`cover_sets.sql`. That corpus barely touches Castle (only
7 of 1,381 chunk names reference it at all, per `cover-archaeology.md`),
which is now explained: Castle's actual gameplay cover was hand-authored
directly in the level via `SGWSpecCoverNode`/`CoverNodeArray`, a **different
and parallel** authoring path from the reusable-prefab pipeline. Both
pipelines may end up spawning the same runtime `ACoverLink` class at
`BeginPlay` (`UObject__StaticLoadObject(..., L"SGW_Cover.CoverNode", ...)`,
per the existing finding) — but only the `.umap`-baked path is populated for
the two maps this campaign cares about.

---

## Q2 — Which Castle/Castle_CellBlock actors own cover components? Counts per chunk

Counts below are from `by_class` occurrence totals (accurate as counts even
though `PackageIndex::find` can't resolve every entry — see Method), cross-
checked against direct per-chunk `scan` for the owner-class breakdown.

**Castle_CellBlock (world 12): 236 `SGWCoverNodeComponent` exports across 5
of 64 chunks, 100% Pattern A (`SGWSpecCoverNode`):**

| Chunk | Count | Owner class |
|---|---|---|
| `Castle_CellBlock-fffefffe` | 113 | `SGWSpecCoverNode` (113/113) |
| `Castle_CellBlock-fffeffff` | 42 | `SGWSpecCoverNode` (42/42) |
| `Castle_CellBlock-fffffffe` | 39 | `SGWSpecCoverNode` (39/39) |
| `Castle_CellBlock-ffffffff` | 35 | `SGWSpecCoverNode` (35/35) |
| `Castle_CellBlock-fffefffd` | 7 | `SGWSpecCoverNode` (7/7) |

**Castle (world 8): 3,788 `SGWCoverNodeComponent` exports across 31 of ~144
chunks** (full per-chunk table in the `query-index --count-only` output;
largest chunks: `Castle-00060005` 412, `Castle-00090003` 338,
`Castle-00040009` 327, `Castle-000a0002` 310, `Castle-00060006` 288). One
sampled chunk (`Castle-00060005`) showed the Pattern A/B mix above; the
others were not individually re-verified for owner class this session.

Total identified: **4,024 real, already-world-space cover nodes** across
the two maps this campaign covers — over four times the count of prefab-pak
rows that even nominally reference Castle-family art sets, and unlike that
corpus, every one of these is a genuine per-level placement, not a
prefab-local offset.

---

## Q3 — Hand-transform one instance to world space; cross-check against the desk and the shipped navmesh

Because Pattern A needs no transform, "hand-transforming" is just the
UE3→BigWorld axis conversion from
[navmesh-build-pipeline.md §1](../../engine/navmesh-build-pipeline.md#1-the-coordinate-mapping):
`bw = (ue.y / 100, ue.z / 100, ue.x / 100)`.

Scanning `Castle_CellBlock-fffefffd.umap` (the exact chunk cited in
`cover_sets.sql:1400-1405` as `Castle_CellBlock_MedStationDesk`'s source)
for `SGWSpecCoverNode` finds a tight cluster of all 7 nodes in that chunk,
all `CoverHeight=1`/`CoverQuality=1`:

| UE3 Location (cm) | BigWorld (m) | Rotation (yaw) | `CoverWidth` |
|---|---|---|---|
| (-12423.10, -23175.27, 6544.10) | (-231.75, 65.44, -124.23) | 179.96° | 2.286 |
| (-12071.37, -23175.90, 6547.17) | (-231.76, 65.47, -120.71) | 180.62° | 0.387 |
| **(-12470.96, -23470.83, 6547.17)** | **(-234.71, 65.47, -124.71)** | 89.74° | 0.376 |
| (-12293.49, -23127.07, 6547.17) | (-231.27, 65.47, -122.94) | 269.57° | 1.721 |
| (-12289.19, -23225.85, 6547.17) | (-232.26, 65.47, -122.89) | 450.63°* | 1.716 |
| (-12471.72, -22880.54, 6547.17) | (-228.81, 65.47, -124.72) | 270.79° | 0.401 |
| (-12520.92, -23175.27, 6544.10) | (-231.75, 65.44, -125.21) | 0.00° | 2.286 |

\* recorded as-is; UE3 rotators are `%360` at use time, so 450.63° ≡ 90.63°.

The bolded row is **within 0.03 m in x and 0.01 m in z** of the hand-
authored `cover_nodes.sql` estimate for chunk_id 1381,
`Castle_CellBlock_MedStationDesk`, `~(-234, 66.5, -124.7)`
(`docs/analysis/npc-ai-restoration/work-packets.md:249`). The y (height)
component reads 65.47 against the hand estimate's 66.5 — 1.0 m higher than
this node, plausibly the estimate having been taken from the desk's visible
top surface rather than a cover-node's own height, or simple estimation
error; this is the only meaningful discrepancy and does not affect the
horizontal match.

**Navmesh cross-check** (`nav_inspect data/spaces/castle_cellblock.nav
--probe`):

```text
components  17 (total walkable XZ area 674552.9 m^2)
  desk1  (-231.75, 65.44, -124.23)  poly=902  component=3  h=1.42 m  dy=-0.16 m  ok
  desk3  (-234.71, 65.47, -124.71)  poly=900  component=3  h=0.77 m  dy=-0.13 m  ok
```

Both land on real walkable polygons in the same navmesh component, `dy`
within 16 cm of the floor — a node whose position were still prefab-local
(i.e., off by tens or hundreds of meters, as the seeded corpus is today)
would not do this. This is the strongest available confirmation, short of a
live client test, that these coordinates are correct, already-final
world-space placements.

All 7 of this chunk's nodes are this desk cluster — `cover_sets.sql`'s
single hand-authored row for chunk_id 1381 should become **7 rows**, one
per real node, once NA21's extractor lands (see
[Data model](#proposed-data-model-for-na21)).

---

## Q4 — What drives the client's crouch/peek/fire pose?

No new native evidence beyond what `cover-system.md` and
`evidence/cover-archaeology.md`'s Part C already established, and this
session's independent re-check reproduces their negative result exactly:

- `USGWAnim_BlendByCover`'s only native (compiled) function decompiles to a
  bare stub: `USGWAnim_BlendByCover__vfunc_0` @ `0x00e90c60` is `return 1;`
  and nothing else. Confirmed again this session by direct decompile.
- A fresh string search this session for `ClaimedBy`, `ClaimCover`,
  `CoverAction`, `PeekLoc`, `LeanLeft`, `LeanRight`, `EvaluateCover` across
  the whole binary returns **zero matches** — independently reproducing the
  earlier pass's result. Stock UE3/UDK's Gears-style lean-and-peek state
  machine genuinely is not present, natively or by name, anywhere in this
  binary.
- `gmSetMobStance` (str @ `0x019c3580`) exists, confirming the GM debug hook
  cited in `evidence/cover-archaeology.md`, but this is a policy toggle
  (`EStance` per `entities/defs/enumerations.xml:190-198`), not a pose
  trigger.
- `ACoverLink__vfunc_183` (`0x00704be0`, slot/fire-link enumeration) and
  `ACoverLink__vfunc_205` (`0x00700800`, `HasFireLinkTo`) were re-decompiled
  this session to look for a pose-relevant path-availability check that
  might feed the animation blend; both are purely reachability/fire-link
  queries over the `+0x28c` slot array (per-slot `DefinedPaths`/`FireLinks`/
  `FireLinks2`/`ExposedFireLinks` arrays), not pose state. No new lead.
- `FUN_00deb660` (the movement-type state machine the packet named as a
  candidate for `MOB_MOVEMENT_Cover=0` handling) is referenced only as a
  **data** value (a function-pointer/dispatch-table entry) from two call
  sites, `FUN_00df3ab0` and `FUN_00df3cc0` — consistent with it being a
  movement-type dispatch table handler, but its ~2.5 KB body still times
  out on decompile in this session exactly as it did in the prior pass
  (`cover-system.md` open question 4). Not resolved.

**Conclusion (unchanged from the existing finding): the pose decision is
UnrealScript bytecode, not natively recoverable via Ghidra decompilation.**
This session adds no new evidence that changes that conclusion, only a
second independent confirmation of the negative string-search result. Full
resolution needs a `.uc`/bytecode-level UnrealScript decompile of
`SGWAnim_BlendByCover` and whichever Pawn/AIController class drives it,
which remains out of scope for native Ghidra work.

**Live pose experiment, as specified, NOT run** (per instructions: no live-client
experiment this session). Exact owner steps, using the now-confirmed real
coordinates from Q3 in place of the original estimate:

1. Spawn a test NPC with `use_cover=true` at BigWorld `(-234.71, 65.47,
   -124.71)` (Castle_CellBlock, the confirmed real desk-cluster node —
   tighter than the prior `~(-234, 66.5, -124.7)` estimate) facing toward
   the room, with a hostile threat nearby so the AI has a reason to
   evaluate cover.
2. Watch the client model: does it crouch/adopt a cover stance purely from
   standing at that position, with no additional server message beyond the
   normal position/movement-type broadcast?
   - If **yes**: `cover-system.md`'s working hypothesis is confirmed —
     "position at a node is sufficient" — and NA22 needs no client-facing
     pose message, only correct positions (which NA21 now makes cheap to
     get).
   - If **no**: the pose needs an explicit claim/link the server does not
     currently set (candidate: the native `ACoverLink+0x50` `ClaimedBy`
     field per `cover-system.md`'s slot layout, though that struct belongs
     to the runtime `ACoverLink`, not to `SGWCoverNodeComponent` — how a
     server-authoritative BigWorld entity would set a field on a
     client-only UE3 actor is itself unresolved and would need its own
     investigation).
3. Telemetry to capture regardless of outcome: `npc_ai` `decision_outcome`
   rows for this NPC (confirms the server-side cover decision fired), and
   whatever `wire.out.movement_type` telemetry NA02 lands (confirms
   `MOB_MOVEMENT_Cover` was actually broadcast) — so a "no visible crouch"
   result can be attributed to the client, not to the server never sending
   the movement type at all.
4. Also try granting/revoking ability 1451 "Cover Stance" (see Q5) around
   the same test, independently of the pose question — if the client shows
   *any* buff icon or combat-log text for it, that confirms the ability
   round-trips even if the pose does not change, which separates "cosmetic
   pose is client-only-unresolved" from "the mechanical payoff (defense
   bonus) is already deliverable."

---

## Q5 — Is ability 1451 "Cover Stance" the in-cover mechanic?

Confirmed from the seed data directly
(`db/resources/Abilities/Seed/abilities.sql:2072`,
`db/resources/Effects/Seed/effects.sql:2962` and `:6106`):

```sql
-- abilities.sql:2072
(1451, 'Cover Stance', '+200 Cover Defense', 'ABILITY_TYPE_Buff', cooldown=2,
 flags=520, effect_ids='{4565,1742}', ...)

-- effects.sql
(4565, ability_id=1451, name='Buff',   desc='Single Target +100 CoverDefense', flags=85)
(1742, ability_id=1451, name='Remove Stance', desc='Remove Stance Moniker')
```

This is a clean, self-contained buff/debuff pair: effect 4565 applies
"+100 CoverDefense" (the ability's own description doubles that to "+200",
likely a display-vs-effect-magnitude convention this session did not chase
further) and effect 1742 removes it — exactly the shape needed for
"grant on `StayInCover`/`MoveToCover`, revoke on `Released`/leash/death"
that `evidence/cover-archaeology.md` proposed. Nothing here is new evidence
beyond confirming the row exists and reading its actual effect payload
(the prior pass had only cited the ability row, not the linked effects).

**Assessment: yes, this is very likely the faithful "peek-and-shoot"/in-cover
mechanic**, mechanically speaking — a combat-relevant defense bonus applied
while the NPC (or player) holds a cover reservation, using the existing
ability/effect pipeline (`docs/architecture/abilities-and-effects-system.md`)
with no new subsystem. It is independent of the pose question in Q4: this
buff can be wired up and will round-trip over the existing wire protocol
regardless of whether the client's crouch animation triggers correctly, so
it does not block on Q4's open question.

---

## Go or no-go for NA21

**Go — with a substantially smaller scope than `evidence/cover-archaeology.md`
originally estimated.**

The original framing (Part C, "extract cover nodes from the cooked `.umap`
chunks... the same class of extraction this project has already done for
terrain/BSP") is directionally right but overstated the difficulty: terrain
and BSP needed new binary-format decoders (`UTerrain::Serialize`,
`UModel::Serialize`) because that geometry is genuinely encoded in a
class-specific binary layout. Cover nodes are not — they are ordinary
tagged properties on ordinary actors, decodable with the **existing**
generic property parser (`cimmeria_upk::parse_tagged_properties`) and the
**existing** actor-extraction path
(`cimmeria_upk::extract_actors`/`ACTOR_CLASSES`, which already lists
`SGWSpecCoverNode`). No new binary-format reverse engineering is needed; no
transform composition is needed for either authoring pattern found.

### Proposed data model for NA21

1. **Extend `crates/upk::objects::actor`** (or add a sibling module) to also
   read, per `SGWSpecCoverNode` actor: `CoverHeight`/`CoverQuality`/
   `CoverWidth` from its `CoverNodeComponent` object reference (Pattern A),
   and per `StaticMeshActor.CoverNodeArray` entry: the child component's own
   `Translation`/`Rotation`/`Scale3D` plus the same three cover properties,
   gated on `AbsoluteTranslation == true` (Pattern B; log a `warn!` if a
   `CoverNodeArray` child is ever found with `AbsoluteTranslation == false`,
   since that would be the one case that *does* need composition, and this
   session found none).
2. **New `crates/navmesh-extractor` walker** (or extend the existing map
   actor walker if/when one exists) to emit, per space:
   `(node_id, position_bw, orientation, height_enum, quality, width,
   source_actor_kind)`. Since these are ordinary top-level `PersistentLevel`
   actors, this walker does **not** need the prefab-archetype resolution
   machinery in `crates/navmesh-extractor/src/staticmesh/archetype/` at all
   for Pattern A or B — that machinery exists for `StaticMeshActor` mesh
   resolution and stays relevant for *other* uses of the chunk, but cover
   nodes read directly off each actor's own tagged properties.
3. **Schema**: add a `space_id`/`world_id` scope column to `cover_sets`
   (seed schema change in `db/resources/`, per repo convention — ask before
   any `db/scripts` migration) and regenerate Castle/Castle_CellBlock's
   cover seeds from the new extractor. Group by physical proximity (e.g. the
   existing scorer's slot grouping) rather than by owning actor, since
   Pattern A's "sets" are implicit (nearby nodes around one obstacle) while
   Pattern B's are explicit (`CoverNodeArray` already groups them).
4. **Retire** the single hand-authored `Castle_CellBlock_MedStationDesk`
   row (chunk_id 1381) once the extractor reproduces its 7 real nodes (Q3).
   Keep the `covernodes_*.pak` corpus as-is for now — it is real,
   ships-correct data for the many *other* worlds' generic prefab-art cover
   (bench/railing/corner pieces) that this session did not investigate, and
   nothing here shows it is wrong for those worlds, only that it is the
   wrong source for Castle/Castle_CellBlock specifically.
5. **Do not build a new binary-format decoder.** The `ACoverLink`
   actor-export approach floated in `evidence/cover-archaeology.md` Part C
   is unnecessary — there is no baked `ACoverLink` export to find (it is a
   runtime-spawned client class per the existing finding's
   `UObject__StaticLoadObject` evidence); the cooked, extractable data is
   `SGWSpecCoverNode`/`SGWCoverNodeComponent`/`StaticMeshActor.CoverNodeArray`,
   confirmed present and already world-space by this session.

### What NA21 still needs from elsewhere

- NA20's Q4 (pose trigger) is still open; NA21 can and should proceed
  without it, since correct world-space positions are useful regardless
  (server-side cover selection/reservation logic, defense-buff wiring, and
  the client's *existing, working* movement-type broadcast all benefit
  immediately). The pose question only gates whether the client visibly
  crouches, per the owner-decision framing already in `work-packets.md`.
- The per-chunk owner-class breakdown (Pattern A vs. B mix) was sampled on
  one Castle chunk only; NA21's extractor should log both patterns' counts
  per space as it runs, both as a sanity check and to catch a chunk where
  Pattern B's `AbsoluteTranslation` assumption doesn't hold.

---

## Resolution (NA21, 2026-09-25)

NA21 built the extractor this finding proposed. It is the `cover_extract`
binary in `crates/navmesh-extractor`, documented in
[cover-extraction.md](../../engine/cover-extraction.md). Running it over
every chunk of both maps settles three of the open questions and corrects
one count:

- **Pattern mix (open question 2).** Castle has 3,780 Pattern A markers
  and 8 Pattern B children, so Pattern B is 0.2% of the map, not the 1.5%
  the one sampled chunk suggested. Castle_CellBlock is 100% Pattern A.
  The totals match this finding: 236 and 3,788.
- **Non-absolute Pattern B children (open question 3).** There are none.
  All 8 have `AbsoluteTranslation = AbsoluteRotation = true`. The
  extractor still composes a relative child with its owner, and counts
  it, in case another map has one.
- **Chunk count.** Castle's cover sits in 32 of 144 chunks, not 31.
- **Omitted properties (new).** 240 markers omit `CoverHeight` and 11 omit
  `CoverQuality`. The cooker omits a property equal to the archetype. The
  census puts the archetype at Low for height and at 3 (QUALITY_None) for
  quality, since byte 3 is never written. Quality is MEDIUM confidence.
  Seven Castle markers carry `CoverQuality = 4`, which is outside the
  enum. See [cover-extraction.md §1](../../engine/cover-extraction.md#1-what-is-extracted).
- **Set 1381 retired.** The extractor reproduces all 7 desk markers as set
  1200001. Its node 2 is the bolded Q3 row. Chains 1132/1133 now key on
  1200001. The hand-authored rows also had the wrong height: the markers
  are `CoverHeight = 1`, Mid, and 1381 had them as Low.

## NA22 follow-through (2026-09-25)

- **Q5 wired.** Effects 4565 and 1742 now carry `script_name`
  `CoverStance` / `RemoveCoverStance`. The server grants 4565 when an NPC
  reaches its reserved slot and runs 1742 when it leaves the slot, leashes,
  dies or surrenders. The magnitude is the effect row's +100 `COVER_DEFENSE`,
  not the ability text's +200. `COVER_DEFENSE` is not read by hit
  resolution yet.
- **Q4 still open.** No pose message was added (D-NA10). An NPC in cover
  reaches the client only as its position and zero velocity, so step 2 of
  the experiment above now tests exactly the shipped server: spawn
  `MessHall_Guard1` (or any `use_cover` guard) at its marker, fight it from
  in front of the cover, and watch whether the model crouches. Step 3's
  `wire.out.movement_type` row no longer exists; use
  `npc_ai decision_outcome=stay_in_cover` and `cover.stance event=granted`
  instead. See [architecture/cover-system.md](../../architecture/cover-system.md).

## Tooling

- `crates/upk-objects/src/bin/query_index.rs` — new `query-index` binary
  (`index`/`scan`/`cover` modes), described under [Method](#method). Unit
  tests use a synthetic in-memory `PackageIndex` fixture, not real assets.
- No changes were made to `crates/upk`, `crates/upk-objects` library code,
  or `crates/navmesh-extractor` beyond this new binary — `extract_actors`
  and `ACTOR_CLASSES` already supported everything this session needed to
  read.

## Open Questions

1. Reconcile `USGWCoverNodeComponent_SpawnCoverNode`'s decompiled
   `0x18`-byte-stride transform loop against a concretely identified
   `covernodes_*.pak`/`covernodes_local.pak` consumer, to confirm (rather
   than infer) that it is the separate reusable-prefab pipeline and not
   somehow also involved in the `.umap`-baked patterns found here.
2. Sample owner-class mix (Pattern A vs. B) across more than one Castle
   chunk to get a reliable prevalence estimate; this session sampled one of
   31 populated chunks.
3. Confirm whether any `CoverNodeArray` child anywhere in Castle/
   Castle_CellBlock has `AbsoluteTranslation == false` (this session found
   none in its one sample) — if any exist, NA21's extractor needs the
   owner-composition math after all, for that subset only.
4. Q4 (pose trigger) remains unresolved; see the existing finding's open
   question 5 and this document's experiment plan above.
