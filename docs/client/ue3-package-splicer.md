# UE3 Package Splicer

> **⚠ The tools this document describes are not in the repository.**
> As of 2026-07-25, none of the twelve `tools/ue3_*.py` scripts referenced
> below exist under [`tools/`](../../tools/) — several survive only as
> stale `.pyc` files in `tools/__pycache__/`, and four
> (`ue3_cover_near.py`, `ue3_probe_layout.py`, `ue3_dump_export_entry.py`,
> `ue3_dump_first_exports.py`) leave no trace at all. The only UE3-related
> tools actually present are [`tools/ue3_extract_cover_nodes.py`](../../tools/ue3_extract_cover_nodes.py)
> and [`tools/upk_parser.py`](../../tools/upk_parser.py), neither of which
> is mentioned below.
>
> Treat this document as a **reference record of the UE3 binary format
> findings and the splice algorithm** — that content is still accurate and
> was expensive to recover. Do **not** treat it as a runnable how-to; every
> command below will fail with "No such file or directory". Reconstructing
> the toolchain from this description is the prerequisite for any further
> splicing work.

Tooling and findings for binary editing of Stargate Worlds UE3 packages — specifically, splicing actor + component clusters from one cooked `.um`/`.umap` into another. Built to recover cover nodes that exist in the shipped 8293 beta but are missing from the QA build the Cimmeria server targets.

This document is **part reference** (UE3 format details discovered empirically) and **part how-to** (which tool does what, in what order). Diagnosing a failed splice depends on being able to read the dumps the client writes when it rejects a malformed package — that machinery is documented separately at [crash-dumps.md](crash-dumps.md).

See also: [crash-dumps.md](crash-dumps.md) for the in-process minidump pipeline you'll need when a splice triggers a load crash, [client-tools.md](../client-tools.md) for the AteraLoader / AtreaRL stack the client runs under, [gameplay/npc-ai.md](../gameplay/npc-ai.md) (§"Cover System") for why the missing cover matters at the gameplay layer, and [reverse-engineering/](../reverse-engineering/) for the broader RE catalog.

## Status snapshot

| Task | State |
|---|---|
| Splicer v0 for one cover node | Done — produces structurally valid output |
| Source bytes remapped (FName / import / export refs) | Done — verified by re-cataloging output |
| `PersistentLevel.Actors` array patched | Done — 866 → 867 entries in the test case |
| Client-load validation | Pending — manual test outstanding (use [crash-dumps.md](crash-dumps.md) pipeline to diagnose failures) |
| LZO recompression of output | Done — 1 MB chunks, 128 KB sub-blocks, matches stock cooker output |
| Scale to 30+ cover nodes per tile | Blocked on v0 validation |

## The problem

The QA build (`SGWGame/CookedPC/Maps/*.umap`) is the level data Cimmeria's client loads, but it's missing many of the cover nodes present in the shipped 8293 beta build (`Data1/*.um`). Without those cover nodes, NPCs have nothing to hide behind — combat AI is incomplete (see [gameplay/npc-ai.md](../gameplay/npc-ai.md), §"Cover System"). The motivating numbers: ~30+ cover nodes per Castle_CellBlock tile, ~1,332 unimplemented Atrea cover nodes in total.

No existing UE3 modding tool will rewrite cooked binary packages — UDK, UE Explorer, and `umodel` are read-only; SGW Map Editor Pro explicitly disclaims UPK rewrites. So we built one.

**Build identifier**: `file_ver = 486`, `licensee_ver = 6`.

## Coordinate system

The SGW in-game HUD displays coordinates in a swizzled-meters form. Empirically:

```
world (X, Y, Z) in unreal-units  =  HUD (Z, X, Y) × 100
```

HUD X maps to world Y, HUD Y maps to world Z, HUD Z maps to world X. Validated against `SGWSpecCoverNode_105` (~800 uu from player standing position when HUD reported `X=-295.407, Y=68.511, Z=-169.726`). Apply this conversion whenever picking candidate cover nodes by player location — `ue3_cover_near.py` does it for you via `--hud`.

## SGW UE3 binary format — empirical findings

What follows is what we learned by inspecting actual bytes. SGW's UE3 fork (ver 486) deviates from stock UE3 in several places that matter for splicing.

### Section ordering

For a typical cooked QA package (`Castle_CellBlock-fffefffd.umap`):

```
[summary]              0..101
[name table]         101..25686
[import table]     25686..45146
[export table]     45146..219790
[depends table]   219790..229286   <- 4 bytes per export, mostly zero
[serial blobs]    229286..end-of-file
```

`total_header_size` from the package summary is the **end of the depends table** (i.e. the start of serial blobs), not the end of the export table. The depends table sits between the export table and the serial blobs; treating `total_header_size` as the export-table end is wrong and causes the last export entry to look like it's 9 KB long. This is the single most common trap when porting a stock-UE3 parser to SGW.

### Compression

QA `.umap` files are LZO-compressed (`compression_flags = 2`); shipped 8293 beta `.um` files mostly are too. Decompression is implemented in `tools/ue3_lzo.py` using `lzokay`. The decompressed in-memory buffer has bytes laid out at their original uncompressed offsets — so all summary offsets (`name_offset`, `import_offset`, `export_offset`, `total_header_size`) are valid as direct indices into the decompressed buffer.

The splicer **recompresses on output** via `compress_package` in `tools/ue3_lzo.py` — 1 MB top-level chunks, 128 KB LZO1X sub-blocks, matching the stock SGW cooker output. The Castle_CellBlock test case compresses to **2.49 MB on disk vs. 2.79 MB for the original QA tile** — round-trip-validated (decompress matches the in-memory uncompressed body byte-for-byte).

### Export table — variable-length trailers

Each export-table entry has a **40-byte fixed preamble** (`class_idx`, `super_idx`, `outer_idx`, name FName, archetype, flags `u64`, `serial_size`, `serial_offset`) followed by a **variable-length trailer** that the standard UDK schema does not account for. SGW's licensee build adds per-entry data — likely a `ComponentMap<FName, INT>` whose size varies per actor.

| Class | Entry size | Trailer size |
|---|---|---|
| `SGWSpecCoverNode` (Actor) | 80 bytes | 40 bytes |
| `SGWCoverNodeComponent` | varies | varies |
| Average across all entries | 93 bytes | ~53 bytes |
| Max observed | — | needs `max_trailer >= 2000` for ComponentMap-heavy actors |

A sequential walker cannot assume a fixed stride; it has to detect each entry's end by probing forward for the next valid preamble. See `tools/ue3_export_table.py` for the adaptive walker.

### Trailer layout (cover-node entries, 40 bytes)

Identical across every `SGWSpecCoverNode` export entry inspected in the 8293 source:

```
+0..3   ExportFlags        = 0x00000001
+4..7   constant           = 0x000000ce (= 206; unknown semantics)
+8..11  zero
+12..15 NetIndex           = incrementing per-entry (e.g. 0x3ba, 0x3bb, 0x3bc...)
+16..31 PackageGuid        = 16 zero bytes for cover nodes
+32..39 PackageFlags + pad = 8 zero bytes
```

The two non-zero non-constant fields are `ExportFlags` and `NetIndex`. The splicer copies the trailer verbatim from src; the NetIndex collision question (whether dst already has an actor with the same NetIndex) is an open risk for v0 — see [open questions](#open-questions).

### Actor serial blob — 32-byte binary prefix

`SGWSpecCoverNode` exports have a **32-byte non-property prefix** before the property tag stream. Structure decoded by comparing src and dst cover nodes:

```
+0..3   class_idx (duplicate)  = -(class_import_idx + 1)   <- MUST REMAP
+4..7   class_idx (duplicate)  = same as +0                <- MUST REMAP
+8..11  -1
+12..15 -1
+16..19 -1
+20..23 per-instance hash    (e.g. 0x03af88e0 — appears to be position/spawn-derived)
+24..27 zero
+28..31 per-instance NetIndex (small positive integer)
```

The two duplicate `class_idx` slots at `+0..7` are real import references — they must be remapped to the destination's class import index, not copied verbatim. The other fields are per-instance state inherited from the src actor; copying them verbatim is the v0 strategy, with the same NetIndex caveat as the trailer.

### Component serial blob — 226-byte prefix + 594-byte suffix

`SGWCoverNodeComponent` exports have substantial pre- and post-property binary data:

```
+0..225      class-specific Serialize data    (probably cover-slot geometry refs)
+226..413    property tag stream (15 FName refs, terminated by 'None')
+414..1007   class-specific Serialize data    (cover-slot positions / fire links / force-field state)
```

The property stream contains `CullDistance`, `CachedCullDistance`, `HiddenEditor` (bool), and a nested `LightingChannels` struct with `bInitialized` and `Static`. No `ObjectProperty` refs. The 226 + 594 bytes of binary data outside the property stream are opaque to the cataloger — we copy them verbatim during splice. An empirical scan showed no embedded `class_idx`-shaped values in the component prefix, so verbatim is safe at v0.

### ULevel binary layout

The `PersistentLevel` (export of class `Level`) serializes:

```
+0..15           16-byte binary header (4 i32: 313, 684, 0, [self_export_idx])
+16..19          i32 Actors_count
+20..23          i32 WorldInfo standalone ref (WorldInfo_0's export index — NOT Actors[0])
+24..            Actors[count] — count × i32 export indices
+24+count*4..    remaining ULevel binary data (URL, Model, ModelComponents, GameSequences,
                 cached physics data, NavLists, CoverLists, etc. — opaque to v0)
```

To append a new actor:

1. Read count at `+16`.
2. Insert `i32 new_actor_idx` at offset `+24 + count*4`.
3. Increment count at `+16`.
4. Grow Level's `serial_size` by 4 in its export-table entry.
5. Shift every downstream export's `serial_offset` by +4.

Sanity check before applying: count at `+16` must be in `[1, 10000]`. SGW Levels we've seen run ~200–1500 actors.

### Property tag stream layout (UE3 ver 486)

```
loop until 'None':
    FName name                     (i32 name_idx + i32 name_number)
    FName type
    i32  size
    i32  array_index
    type-specific extras:
        StructProperty:  FName struct_name (8 bytes)
        ArrayProperty:   FName inner_type  (8 bytes since ver 332)
        BoolProperty:    u32 value (4 bytes; size == 0, no value bytes follow)
        (others):        no extras
    value bytes of length `size`
```

`BoolProperty` is the trap: it has `size == 0` and a 4-byte tag-embedded value where the value bytes would normally live. Forgetting this skews the rest of the stream. `ByteProperty` does NOT have an enum-name FName in ver 486 (that's a UDK ver 633+ addition).

## Tool inventory

All tools live under `tools/` and follow the same conventions: stdlib + `lzokay` only (no other deps), single-file Python, importable as modules for composability. Outputs are CSV-like or hex-dump where data, structured prose where decisions.

### Read-only utilities

| Tool | Purpose |
|---|---|
| `ue3_lzo.py` | LZO-decompress a UE3 package into a byte buffer. Used as a fallback path inside the other tools. |
| `ue3_dump_cover.py` | Walk a package's export table by class-index byte-pattern matching, extract `SGWSpecCoverNode` / `CoverLink` / etc. actors, decode `Location` and `Rotation` property values, emit CSV. |
| `ue3_cover_near.py` | Diff cover-node sets between src and dst, filter to those within a radius of a given world position. Accepts HUD coords via `--hud` (with the axis swizzle baked in). |
| `ue3_export_table.py` | Adaptive sequential walker for the export table. Probes forward from each known-valid entry for the next valid preamble. Handles SGW's variable trailers. Default `max_trailer = 2000` (cover ComponentMap-heavy actors). |
| `ue3_dump_cluster.py` | Given a package and an actor name, BFS by `Outer` index to collect the actor's full export cluster (actor + sub-components). |
| `ue3_catalog_refs.py` | For each export in a cluster, parse the property stream and record every FName index reference, every ObjectProperty value reference, plus prefix size. Outputs a remap manifest. Includes a robust property-stream anchor finder that scans byte-by-byte for the start. |
| `ue3_verify_dst.py` | Take a cluster manifest + a dst package, check (a) which name strings are missing from dst's name table, (b) which imports need to be added (matched by full outer chain), (c) which cross-cluster export refs can be resolved (e.g. `PersistentLevel`). Verdict: `READY` / `BLOCKED`. |

### Diagnostic helpers (kept for re-use, not part of the splicer path)

| Tool | Purpose |
|---|---|
| `ue3_probe_layout.py` | Report a package's section ordering and probe per-entry trailer sizes for the export table. Used to discover SGW's average 93-byte entry size. |
| `ue3_dump_export_entry.py` | Dump raw bytes of cover-node export-table entries to inspect the trailer structure. Used to find the 40-byte cover-node trailer layout. |
| `ue3_dump_first_exports.py` | Hex-dump the first N bytes of the export block at varying strides. Used during the alignment-and-stride debugging. |
| `ue3_dump_names.py` | Dump a package's name table. Companion to `ue3_dump_cover.py` when you need to confirm a name index by sight. |

### Splicer

| Tool | Purpose |
|---|---|
| `ue3_splicer.py` | The splice engine. Reads src + dst, computes all remaps, patches FName / import / export references in the cluster's serial bytes, extends dst's name/export tables, patches `PersistentLevel.Actors`, lays out the output package and LZO-recompresses it to match the stock cooker format. |

## How to splice an actor cluster

Single-cover-node splice flow, implemented in `tools/ue3_splicer.py`:

1. **Parse both packages** (LZO-decompress transparently). Walk export tables with the adaptive walker.
2. **Identify the source cluster** — BFS by `Outer` from the named actor. For `SGWSpecCoverNode_105` the cluster is exactly 2 exports (1 actor + 1 component, ~1296 bytes total serial data).
3. **Catalog references inside the cluster**:
   - Every FName index used (property names, type names, struct names, `NameProperty` values).
   - Every `ObjectProperty` value (export and import refs).
   - Every `class_idx` / `super_idx` / `archetype_idx` / `outer_idx` in the export-table entries themselves.
4. **Verify against destination**:
   - For each src name string, check dst name table; collect names-to-add.
   - For each src import index, resolve the full outer chain (e.g. `Engine.SGWCoverNodeComponent.CoverNode`), find the dst import with the matching chain; collect imports-to-add.
   - For each cross-cluster export ref, find the dst export with same class + name (e.g. `PersistentLevel`).
5. **Build remap tables**: `src_name_idx → dst_name_idx`, `src_import_idx → dst_import_idx`, `src_export_idx → dst_export_idx`. In-cluster exports get appended-at-end indices (e.g. `dst_export_count + 1`, `+2`).
6. **Patch cluster serial bytes** in-memory:
   - At every FName offset (from the catalog), rewrite `name_idx` via the remap.
   - At every `ObjectProperty` value offset, rewrite the `i32` via export/import remap.
   - In the actor's 32-byte binary prefix, scan for `class_idx`-shaped `i32`s (cluster-used import refs) and remap them — empirically catches the duplicate `class_idx` at `+0..7`.
7. **Build new export-table entries** for the cluster — start from the src entry's bytes, patch `class_idx` / `super_idx` / `outer_idx` / `name_idx` / `archetype_idx` via the remaps. Keep the variable trailer verbatim (including `NetIndex`).
8. **Patch `PersistentLevel.Actors`**:
   - Read count at `Level.serial_offset + 16`.
   - Insert new actor's dst index at `Level.serial_offset + 24 + count*4`.
   - Increment count at `+16`.
   - Update Level's `serial_size += 4` in its export-table entry.
9. **Lay out the output file** as a fresh uncompressed package:
   - Strip the LZO-compressed-chunk descriptors from the summary.
   - Place name table at offset 101 (right after the new shorter summary).
   - Import table, export table, depends table (existing dst + 4 bytes per new export) sequentially after.
   - Recompute every existing dst export's `serial_offset`: if `< Level.serial_offset` add the section-shift delta; if `> Level.serial_offset` add delta + 4 (Level grew by 4).
   - Place existing dst serial blobs after the new `total_header_size` (with Level's blob replaced by the patched version).
   - Append cluster blobs at the very end.
10. **LZO-recompress** via `compress_package`: split the post-summary region into 1 MB top-level chunks, LZO1X-compress each into 128 KB sub-blocks, build the on-disk summary with `compression_flags = 2` + chunk descriptors.
11. **Write to `<dst>.spliced`** — never overwrite the original.

### Castle_CellBlock test case (`SGWSpecCoverNode_105`)

Concrete numbers from the single-node validation run:

| Property | Source (8293) | Spliced (QA) |
|---|---|---|
| Source path | `castle_cellblock_fffefffd.um` | — |
| Destination path | — | `Castle_CellBlock-fffefffd.umap.spliced` |
| Cluster exports | 2 (`#957` component, `#972` actor) | appended as `#2375`, `#2376` |
| Cluster serial bytes | 1296 (1008 + 288) | 1296 (unchanged size, contents remapped) |
| Names needed | 24 | 23 already in dst, 1 added (`HiddenEditor`) |
| Imports needed | 3 | all 3 already in dst (matched by outer chain) |
| Cross-cluster refs | 1 (`PersistentLevel`) | resolved to dst `#373` |
| `PersistentLevel.Actors` | 866 | 867 (new actor appended at end) |
| Output size | — | 2,492,043 bytes on disk (LZO-compressed, 6,245,399 bytes uncompressed) |
| Walker re-parse | — | All 2376 entries walk cleanly |
| Catalog re-check | — | FName indices match dst positions; ObjectProperty refs point to cluster's new indices |

The cluster has zero `ObjectProperty` refs from the component itself (it's self-contained) and exactly one from the actor — its component — which gets remapped to the new in-cluster position.

### Diagnosing a load failure

If the spliced `.umap` triggers an `SGW.exe` crash on load — the most likely failure mode for v0 — pull the resulting minidump through the [crash-dump pipeline](crash-dumps.md). The faulting RVA usually points straight at the UE3 loader function that rejected the package, which tells you which structural assumption the splicer violated. Trigger one of the deliberate-crash console commands first to confirm the pipeline works end-to-end before relying on it for splice debugging.

## Open questions

Tracked here so they're not lost between sessions. Each one is a v0 risk we accepted in order to ship the single-node splice; resolving them is part of the scale-up to all 1,332 missing nodes.

| Question | Why it matters | What we'd need to learn |
|---|---|---|
| Are NetIndex collisions tolerated? | The spliced actor's NetIndex (in trailer `+12` and prefix `+28`) is copied verbatim from src. If dst already has an actor with that NetIndex, replication may break. | Load and observe; if NPCs near the spliced cover behave strangely, synthesize a fresh NetIndex (max existing + 1). |
| What's the per-instance hash at actor prefix `+20..23`? | Differs per-actor; copied verbatim by the splicer. Might be position-derived (HashLocation), spawn-time-derived, or replication-related. | Compare hashes across actors in dst with known positions to see if there's a deterministic relationship to (x, y, z). |
| What's in the component's 594-byte binary suffix? | Likely cover-slot geometry (slot positions, fire-link references). Copied verbatim. If it references other exports by index, the remap misses them. | Compare suffix bytes across multiple cover nodes; look for export-index-shaped values that change between src and dst. |
| Does `Level` have post-`Actors` references we should patch? | After `Actors[]` comes URL, Model, ModelComponents, GameSequences, NavListStart, CoverListStart, CoverListEnd, etc. SGW's specific cover linkage data isn't yet decoded. | Crash analysis after the first load attempt should hint at which subsystem reads what. |
| Will the splicer scale to 30+ nodes per tile, then to all 1,332 across all Atrea maps? | The single-node splice took five recon tasks. Scaling needs the per-node work amortized. | Once v0 validates, run the splicer in a loop over `ue3_cover_near.py`'s candidate list. |

## Asset locations

| Asset | Path |
|---|---|
| 8293 source build | `Downloads/Stargate Worlds (0.8293.0.43485) (beta)/Data1/` (user-local) |
| QA destination build | `source/projects/SGW/Stargate Worlds-QA/Working/SGWGame/CookedPC/Maps/` (user-local) |
| Ghidra MCP for `SGW.exe` | TCP `:8100`, project `SGW`, with `SGW.exe` pre-opened |
| Per-system Ghidra findings | [docs/reverse-engineering/](../reverse-engineering/) |
