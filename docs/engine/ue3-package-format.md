---
title: "SGW UE3 Package Binary Format"
type: reference
audience: engineers
last_updated: 2026-09-19
---

# SGW UE3 Package Binary Format

> **Type**: reference
> **Audience**: engineers working on [`crates/upk-objects/`](../../crates/upk-objects/), asset extraction, or cover-node recovery
> **Companions**: [cooked-data-pak-format.md](cooked-data-pak-format.md) (the *other* binary format — BigWorld's `.pak`, unrelated), [cover-system.md](../reverse-engineering/findings/cover-system.md) (why cover nodes matter), [crash-dumps.md](../client/crash-dumps.md) (diagnosing a package the client rejects)

Empirical findings about the binary layout of Stargate Worlds' cooked UE3
packages (`.um`, `.umap`, `.upk`). SGW ships a **licensee fork** of UE3 that
deviates from stock in several places, and every deviation below cost real time
to find by staring at bytes. Read this before pointing a stock-UE3 parser at an
SGW package.

**Build identifier**: `file_ver = 486`, `licensee_ver = 6` (this document's
source sample). **Note**: a separate QA `.umap` sample (`Castle-000a0002.umap`,
used by the `castle.nav` spike, issue #46) reads `licensee_ver = 8` — both
values are apparently in circulation across SGW's cooked content. The wire
*format* itself is identical either way; only a handful of `Ar.Ver()`
(Epic-version, not licensee-version) gate constants in native `Serialize`
overrides depend on version, and both samples share `file_ver = 486`. See
[`../reverse-engineering/findings/bsp-model-polys-serialize.md`](../reverse-engineering/findings/bsp-model-polys-serialize.md)
for the `Model`/`Polys` cross-check that surfaced this.

> **Provenance.** These findings come from a 2026 effort to splice actor +
> component clusters from the shipped 8293 beta build into the QA build the
> Cimmeria client targets, recovering cover nodes the QA maps are missing
> (~30+ per Castle_CellBlock tile, ~1,332 across all Atrea maps). A working
> single-node splicer was built and produced structurally valid, LZO-recompressed
> output — but the twelve `tools/ue3_*.py` scripts that implemented it are **no
> longer in the repository** and client-load validation was never completed. The
> format findings survived; the toolchain did not. `tools/ue3_extract_cover_nodes.py`
> and `tools/upk_parser.py` are what remain. If splicing is ever revived,
> everything below is the spec you would rebuild against.

## Section ordering

For a typical cooked QA package (`Castle_CellBlock-fffefffd.umap`):

```text
[summary]              0..101
[name table]         101..25686
[import table]     25686..45146
[export table]     45146..219790
[depends table]   219790..229286   <- 4 bytes per export, mostly zero
[serial blobs]    229286..end-of-file
```

### The `total_header_size` trap

**`total_header_size` from the package summary is the end of the *depends
table*, not the end of the export table.** It marks the start of the serial
blobs.

The depends table sits between the export table and the serial blobs. Treating
`total_header_size` as the export-table end makes the last export entry appear
to be about 9 KB long. This is the single most common failure when porting a
stock-UE3 parser to SGW.

## Compression

QA `.umap` files are LZO-compressed (`compression_flags = 2`); shipped 8293 beta
`.um` files mostly are too.

Once decompressed, the in-memory buffer has bytes laid out at their **original
uncompressed offsets** — so every summary offset (`name_offset`,
`import_offset`, `export_offset`, `total_header_size`) is a valid direct index
into the decompressed buffer. No offset translation needed.

Writing a package back out in stock-cooker-compatible form means 1 MB top-level
chunks, each LZO1X-compressed into 128 KB sub-blocks, with `compression_flags = 2`
and chunk descriptors in the summary. A Castle_CellBlock round-trip produced
2.49 MB on disk against 2.79 MB for the original QA tile, and decompressing the
output matched the in-memory uncompressed body byte-for-byte.

## Export table — variable-length trailers

Each export-table entry has a **40-byte fixed preamble** (`class_idx`,
`super_idx`, `outer_idx`, name FName, archetype, flags `u64`, `serial_size`,
`serial_offset`) followed by a **variable-length trailer that the standard UDK
schema does not account for**. SGW's licensee build adds per-entry data —
apparently a `ComponentMap<FName, INT>` whose size varies per actor.

| Class | Entry size | Trailer size |
|---|---|---|
| `SGWSpecCoverNode` (Actor) | 80 bytes | 40 bytes |
| `SGWCoverNodeComponent` | varies | varies |
| Average across all entries | 93 bytes | ~53 bytes |
| Max observed | — | needs a `max_trailer` bound of at least 2000 for ComponentMap-heavy actors |

**A sequential walker cannot assume a fixed stride.** It has to detect each
entry's end by probing forward for the next valid preamble.

### Cover-node trailer layout (40 bytes)

Identical across every `SGWSpecCoverNode` export entry inspected in the 8293
source:

```text
+0..3   ExportFlags        = 0x00000001
+4..7   constant           = 0x000000ce (= 206; unknown semantics)
+8..11  zero
+12..15 NetIndex           = incrementing per-entry (e.g. 0x3ba, 0x3bb, 0x3bc...)
+16..31 PackageGuid        = 16 zero bytes for cover nodes
+32..39 PackageFlags + pad = 8 zero bytes
```

The only non-zero, non-constant fields are `ExportFlags` and `NetIndex`.

## Actor serial blob — 32-byte binary prefix

`SGWSpecCoverNode` exports carry a **32-byte non-property prefix** ahead of the
property tag stream, decoded by comparing cover nodes across the two builds:

```text
+0..3   class_idx (duplicate)  = -(class_import_idx + 1)
+4..7   class_idx (duplicate)  = same as +0
+8..11  -1
+12..15 -1
+16..19 -1
+20..23 per-instance hash    (e.g. 0x03af88e0 — appears position/spawn-derived)
+24..27 zero
+28..31 per-instance NetIndex (small positive integer)
```

The two duplicate `class_idx` slots at `+0..7` are **real import references** —
any tool that relocates an actor between packages must remap them, not copy them
through. The remaining fields are per-instance state.

## Component serial blob — 226-byte prefix + 594-byte suffix

`SGWCoverNodeComponent` exports wrap their property stream in substantial
opaque binary data:

```text
+0..225      class-specific Serialize data    (probably cover-slot geometry refs)
+226..413    property tag stream (15 FName refs, terminated by 'None')
+414..1007   class-specific Serialize data    (cover-slot positions / fire links / force-field state)
```

The property stream carries `CullDistance`, `CachedCullDistance`,
`HiddenEditor` (bool), and a nested `LightingChannels` struct with
`bInitialized` and `Static`. It contains **no `ObjectProperty` refs**, and an
empirical scan found no `class_idx`-shaped values in the 226-byte prefix — so
copying both binary regions verbatim is safe when relocating a component.

## ULevel binary layout

The `PersistentLevel` export (class `Level`) serializes:

```text
+0..15           16-byte binary header (4 × i32: 313, 684, 0, [self_export_idx])
+16..19          i32 Actors_count
+20..23          i32 WorldInfo standalone ref (WorldInfo_0's export index — NOT Actors[0])
+24..            Actors[count] — count × i32 export indices
+24+count*4..    remaining ULevel data (URL, Model, ModelComponents, GameSequences,
                 cached physics data, NavLists, CoverLists, …)
```

Note the trap at `+20`: that slot is a standalone `WorldInfo` reference, **not**
the first element of the `Actors` array. The array starts at `+24`.

Appending an actor means: read the count at `+16`, insert an `i32` export index
at `+24 + count*4`, increment the count, grow `Level`'s `serial_size` by 4 in its
export-table entry, and shift every downstream export's `serial_offset` by +4.

Sanity check before trusting a parse: the count at `+16` should be in
`[1, 10000]`. SGW levels run roughly 200–1500 actors.

## Property tag stream layout (UE3 ver 486)

```text
loop until 'None':
    FName name                     (i32 name_idx + i32 name_number)
    FName type
    i32  size
    i32  array_index
    type-specific extras:
        StructProperty:  FName struct_name (8 bytes)
        ArrayProperty:   FName inner_type  (8 bytes, since ver 332)
        BoolProperty:    u32 value (4 bytes; size == 0, no value bytes follow)
        (others):        no extras
    value bytes of length `size`
```

Three version-specific traps:

- **`BoolProperty` has `size == 0`** and carries its value as 4 tag-embedded
  bytes where the value bytes would normally be. Miss this and every subsequent
  property in the stream is misaligned.
- **`ByteProperty` has no enum-name FName in ver 486.** That is a UDK ver 633+
  addition. A parser written against modern UDK will over-read here.
- **`ArrayProperty` carries no inner-type FName in ver 486 either.** The
  "since ver 332" row in the table above describes stock UE3; SGW's stream does
  not have it. A flat byte-skip that jumps by the tag's declared `size` — which
  already covers the array's entire nested content, inner `None` terminators
  included — lands byte-exactly on the next tag. Consuming an extra 8 bytes
  desynchronises the stream. Verified against 1744 `Terrain` exports whose
  native trailer then decodes to the exact declared export size.

## Coordinate system

The SGW in-game HUD reports coordinates in a swizzled, scaled form:

```text
world (X, Y, Z) in unreal units  =  HUD (Z, X, Y) × 100
```

HUD X maps to world Y, HUD Y to world Z, HUD Z to world X. Validated against
`SGWSpecCoverNode_105`, which sat ~800 uu from a player standing position where
the HUD read `X=-295.407, Y=68.511, Z=-169.726`.

Apply this whenever you correlate a HUD reading against package data or
server-side entity positions.

## Terrain actor serial blob — property stream + native trailer

`Terrain` is an `AActor` subclass, so its export opens with the 32-byte actor
prefix above, then a normal property tag stream, then a native trailer written
by `ATerrain::Serialize` (`SGW.exe` @ `0x007517C0`):

```text
+0x000  i32   Heights.Num              = NumVerticesX * NumVerticesY
+0x004  u16   Heights[N]               0x8000 == no displacement
+????   i32   InfoData.Num             = same N
+????   u8    InfoData[N]              bit 0 = TID_Visibility_Off
+????   i32   AlphaXSize               binary copy of the tagged property
+????   i32   AlphaYSize               binary copy of the tagged property
+????   i32   WeightedTextureMaps.Num  1..3 in shipped content
+????   [i32 len + len bytes] * that count
+????   i32   WeightMapTextures.Num    0 in shipped content
+????         lighting GUIDs + foliage proxy data — 92..3304 bytes, undecoded
```

Parsed by [`crates/upk-objects/src/terrain/`](../../crates/upk-objects/src/terrain/);
triangulated into navmesh collision geometry by
[`crates/navmesh-extractor/src/terrain.rs`](../../crates/navmesh-extractor/src/terrain.rs).

Four things a new parser gets wrong:

- **Height scale is `(h - 32768) / 128` in actor-local units**, then scaled by
  `DrawScale * DrawScale3D.Z`. Only that bias maps the flat `0x8000` sheet in
  Castle_CellBlock onto the shipped navmesh's BW y ≈ 0 ground plane.
- **`DrawScale3D` defaults to `(100, 100, 100)`, not `(1, 1, 1)`,** when the
  property is absent. All 144 `Castle` terrains and 400 of 1600
  `Castle_CellBlock` terrains omit it; at `(1, 1, 1)` their patches would be
  1 cm wide instead of the 100 cm the world grid demands.
- **`InfoData` visibility is read per-quad, keyed by the quad's lower-left
  corner vertex.** The final heightmap row and column therefore never gate a
  quad.
- **`NumSectionsX * NumSectionsY` is a render partition of one heightmap**, not
  a terrain count. A `Castle` chunk has one `Terrain` export and 25
  `TerrainComponent` exports; a `Castle_CellBlock` chunk has 25 `Terrain`
  exports and 25 `TerrainComponent` exports. Walk every `Terrain`-class export
  and treat each independently.

## Open format questions

Unresolved when the splicing effort stopped. Each is a real gap in the format
understanding above, not merely a tooling limitation.

| Question | Why it matters |
|---|---|
| Are `NetIndex` collisions tolerated? | `NetIndex` appears in both the export trailer (`+12`) and the actor prefix (`+28`). If a relocated actor carries a `NetIndex` the destination package already uses, replication behaviour is unknown. Synthesising a fresh index (max existing + 1) is the obvious mitigation, untested. |
| What is the per-instance hash at actor prefix `+20..23`? | Differs per actor. Candidates: position-derived (`HashLocation`), spawn-time-derived, or replication-related. Comparing hashes across actors with known positions would settle it. |
| What is in the component's 594-byte suffix? | Likely cover-slot geometry — slot positions and fire-link references. If it references other exports by index, any relocation that copies it verbatim silently corrupts those references. |
| Does `Level` hold post-`Actors` references needing patching? | After `Actors[]` come URL, Model, ModelComponents, GameSequences, `NavListStart`, `CoverListStart`, `CoverListEnd`. SGW's cover-linkage data specifically is undecoded. |

**Resolved** (2026-09-19, issue #46): the `Model` (BSP) export format —
`UModel::Serialize`'s full field order and sizes, `FBspNode`/`FBspSurf`
layouts, and whether cooked packages strip `Polys` (they don't) — is now
documented in
[`../reverse-engineering/findings/bsp-model-polys-serialize.md`](../reverse-engineering/findings/bsp-model-polys-serialize.md)
**and implemented** in
[`crates/upk-objects/src/model/`](../../crates/upk-objects/src/model/).
That was previously an open question for this document's scope too (a
`Model` export's serial data is exactly the kind of "variable-length
trailer" this document otherwise catalogs) but is substantial enough to
warrant its own finding doc rather than a section here.

### `Model` / `Polys` deserializers

[`crates/upk-objects/src/model/`](../../crates/upk-objects/src/model/) is
the live decoder, structured like its `static_mesh` sibling:

| File | Holds |
|---|---|
| [`model/mod.rs`](../../crates/upk-objects/src/model/mod.rs) | The full `UModel` / `UPolys` wire-layout table in module docs, plus re-exports |
| [`model/types/mod.rs`](../../crates/upk-objects/src/model/types/mod.rs) | `Model`, `BspNode`, `BspSurf`, `BspVert`, `Poly`, `Polys`; the `EPolyFlags` / `EBspNodeFlags` filter table; `Model::triangulate` (node → convex fan) and `Model::surf_normal` |
| [`model/parse/mod.rs`](../../crates/upk-objects/src/model/parse/mod.rs) | `deserialize_model` / `deserialize_polys` and the field-offset constants |
| [`model/types/tests.rs`](../../crates/upk-objects/src/model/types/tests.rs) | Triangulation, winding, flag-filter and out-of-range unit tests |
| [`model/parse/tests.rs`](../../crates/upk-objects/src/model/parse/tests.rs) | Byte-exact wire-format fixtures, including the "an empty `Model` is exactly 108 bytes" arithmetic self-check |

Two properties of this decoder are worth knowing before you use it:

- **Exact consumption is enforced.** Both entry points error if the
  declared fields stop short of, or overrun, the export's serial data.
  Everything after `Verts` is skipped by declared size, so an upstream
  off-by-one surfaces *only* as a non-zero remainder; accepting it
  silently would let a mis-parsed `Nodes` array reach downstream code
  looking plausible.
- **The `EPolyFlags` bit meanings are assumed, not re-derived.** The
  filter is a named table in `model/types/mod.rs`, and
  `Model::triangulate` reports a per-flag triangle exclusion count for
  *every* entry regardless of whether the active filter uses that bit —
  so a wrong assumption shows up as an implausible drop count rather
  than as a silent hole. On `Castle-000a0002.umap` the only observed
  `PolyFlags` values are `0xE00` and `0x200`, and the filter excludes
  nothing.

The consumer is
[`crates/navmesh-extractor/src/bsp.rs`](../../crates/navmesh-extractor/src/bsp.rs),
which classifies each `Model` by its owning export's class (`Level` →
world space; `Brush`/`BlockingVolume` → actor transform;
`TriggerVolume`/`DynamicTriggerVolume` → excluded).

## Related documents

- [`crates/upk-objects/`](../../crates/upk-objects/) — the live Rust
  deserializers for UE3 objects in these packages (`StaticMesh`, `Terrain`,
  `Model`/`Polys`, `Texture2D`, bulk data, cross-package export index).
- [`../reverse-engineering/findings/bsp-model-polys-serialize.md`](../reverse-engineering/findings/bsp-model-polys-serialize.md) —
  `UModel`/`UPolys`/`FBspNode`/`FBspSurf`/`FPoly` binary layout, byte-exact
  validated against real package data.
- [cooked-data-pak-format.md](cooked-data-pak-format.md) — BigWorld's `.pak`
  resource format. Different format, different pipeline; do not confuse them.
- [cover-system.md](../reverse-engineering/findings/cover-system.md) — what the
  cover nodes in these packages do at the gameplay layer, and why the QA build's
  missing nodes matter.
- [crash-dumps.md](../client/crash-dumps.md) — the minidump pipeline to reach
  for when the client rejects a package you have written.
