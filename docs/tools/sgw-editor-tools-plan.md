# SGW Editor Tools Plan

> **Approach**: Option B — Use UE3 source as reference documentation to build focused standalone tools
> **Reference UE3 source**: `github.com/gameboys84/unrealengine3` (early 2004 build, same era as SGW)
> **Created**: 2026-03-11

---

## Rationale

SGW.exe contains a full UE3 UnrealEd activated via AtreaLoader binary patches, but it's unstable and can't be extended. Rebuilding the full editor from UE3 source would require months of porting work and still face content serialization mismatches with SGW's CME-modified engine.

Instead, we build **targeted tools** that solve specific emulator needs, using the UE3 source as a format reference. Each tool is a standalone Rust crate in the Cimmeria workspace.

---

## Tool 1: `cimmeria-upk` — UE3 Package Parser Library

### Purpose
Core library that reads UE3 `.upk` and `.umap` package files. Foundation for all other tools.

### Binary Format (from UE3 `Core/Inc/UnLinker.h`)

```
┌─────────────────────────────────────────┐
│ FPackageFileSummary (Header)            │
├─────────────────────────────────────────┤
│  Tag: u32             (0x9E2A83C1)     │
│  FileVersion: i32     (epic | licensee)│
│  PackageFlags: u32                     │
│  NameCount: i32       NameOffset: i32  │
│  ExportCount: i32     ExportOffset: i32│
│  ImportCount: i32     ImportOffset: i32│
│  Guid: [u32; 4]                        │
│  Generations: Vec<GenerationInfo>      │
├─────────────────────────────────────────┤
│ Name Table (at NameOffset)             │
│  NameCount × FNameEntry                │
│    string (length-prefixed) + flags    │
├─────────────────────────────────────────┤
│ Import Table (at ImportOffset)         │
│  ImportCount × FObjectImport           │
│    ClassPackage: FName                 │
│    ClassName: FName                    │
│    PackageIndex: i32                   │
│    ObjectName: FName                   │
├─────────────────────────────────────────┤
│ Export Table (at ExportOffset)         │
│  ExportCount × FObjectExport           │
│    ClassIndex: i32                     │
│    SuperIndex: i32                     │
│    PackageIndex: i32                   │
│    ObjectName: FName                   │
│    ObjectFlags: u32                    │
│    SerialSize: i32                     │
│    SerialOffset: i32                   │
│    ComponentMap: Map<FName, i32>       │
├─────────────────────────────────────────┤
│ Object Data (at each SerialOffset)     │
│  Per-class serialization               │
└─────────────────────────────────────────┘
```

### Key Reference Files
- `Core/Inc/UnLinker.h` — struct definitions
- `Core/Src/UnLinker.cpp` — `FLinkerLoad` (load), `FLinkerSave` (save)
- `Core/Inc/UnObjVer.h` — version constants (`PACKAGE_FILE_TAG = 0x9E2A83C1`)
- `Core/Src/UnObj.cpp` — object serialization

### Implementation
```
crates/upk/
├── Cargo.toml
└── src/
    ├── lib.rs          # Public API
    ├── header.rs       # FPackageFileSummary parsing
    ├── names.rs        # Name table (FNameEntry)
    ├── imports.rs      # Import table (FObjectImport)
    ├── exports.rs      # Export table (FObjectExport)
    ├── reader.rs       # BinaryReader with UE3 type support
    ├── package.rs      # Package struct tying it together
    ├── objects/        # Per-class deserializers
    │   ├── mod.rs
    │   ├── actor.rs    # AActor position/rotation/scale
    │   ├── kismet.rs   # Sequence/SeqAct/SeqEvent nodes
    │   ├── static_mesh.rs
    │   └── brush.rs    # BSP geometry
    └── error.rs
```

### Phase 1 Deliverables
- [x] Read package header, validate tag + version
- [x] Parse name table
- [x] Parse import/export tables
- [x] Resolve object class names via import/name cross-reference
- [x] List all objects in a package with class, name, size
- [x] CLI tool: `upk-info <file.upk>` — dump package summary
- [x] Handle LZO-compressed packages (3,892 of 5,039 are compressed)

**Proof-of-concept:** `tools/upk_parser.py` — Python parser, 5,039/5,039 files (100%)
**Rust crate:** `crates/upk/` — Production library with `upk-info` CLI, validates 100% of packages

### Phase 2 Deliverables
- [x] Read object serial data for AActor subclasses (position, rotation, scale)
- [x] Read Kismet sequence data (nodes, connections, variables)
- [ ] Read BSP geometry (vertices, surfaces)
- [x] Read static mesh references
- [x] Handle SGW's licensee version offset

**Actor extraction tool:** `tools/extract_actors.py` — Python batch zone extractor (summary/SQL/JSON/CSV)
**Results across entire SGW client (24 zones, 4,116 tiles):**
- 670,881 actors extracted total
- 174,540 cover nodes, 27,809 triggers, 949 interp actors, 14,548 sound emitters
- Parse errors: 505/4,116 tiles (0.01% error rate)

**Kismet extraction tool:** `tools/kismet_extractor.py` — Python sequence graph extractor (survey/graph/SQL/JSON)
**Results across entire SGW client (24 zones, 4,621 tiles):**
- 487,022 Kismet nodes parsed, 49,716 sequences
- 88,968 event triggers, 116,613 actions
- 88,113 content chains extracted
- Parse errors: 195/4,621 tiles (0.004% error rate)

**Rust crate CLIs:**
- `extract_actors` — Rust actor extractor with class distribution summary + JSON output
- `extract_kismet` — Rust Kismet extractor with graph display + chain counting
**Rust results across entire SGW client (24 zones, 4,621 tiles):**
- 812,608 actors (27 class types), 487,217 Kismet nodes, 88,878 content chains
- Zero parse errors, ~50 seconds per full extraction (release build)

### Validation
- [x] Parse all 5,039 .upk/.umap files — **5,039/5,039 (100%)** — 2.8M exports, 1.25M names, 530K imports
- [x] Rust crate validates **5,021/5,021 CookedPC packages (100%)** — zero failures
- [x] Cross-reference against Ghidra findings for SGW.exe serialization addresses
- [x] Extract actors from all 24 zones — **812,608 actors** (Rust), **670,881** (Python) with Location, Rotation, DrawScale
- [x] Extract Kismet from all 24 zones — **88,878 content chains** (Rust) from 487K nodes, zero errors

---

## Tool 2: `sgw-world-viewer` — Map Viewer & Entity Placer

### Purpose
Visualize SGW maps and place/edit entities. Output entity placement data to the server database (not back to .umap files).

### Architecture
```
┌──────────────────────────────────────┐
│         Tauri Desktop App            │
│  ┌────────────────────────────────┐  │
│  │    2D/3D Map Viewport          │  │
│  │  (wgpu or three.js WebGL)      │  │
│  ├────────────────────────────────┤  │
│  │    Entity Palette / Inspector  │  │
│  │  (React + entity defs from DB) │  │
│  ├────────────────────────────────┤  │
│  │    Properties Panel            │  │
│  │  (position, rotation, tags)    │  │
│  └────────────────────────────────┘  │
│              ▲                       │
│              │ IPC                   │
│  ┌───────────┴────────────────────┐  │
│  │  Rust Backend                  │  │
│  │  - cimmeria-upk (map parsing)  │  │
│  │  - sqlx (DB read/write)        │  │
│  │  - navmesh renderer            │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
```

### Data Flow
1. **Read** .umap tiles → extract geometry + original actor placement
2. **Read** .nav files → overlay navigation mesh
3. **Read** server DB → show current entity placements (NPCs, spawns, interactables)
4. **Write** entity changes → update server DB tables (NOT .umap files)

### Key Features
- 2D overhead map view with zoom/pan (priority — simpler than 3D)
- Actor markers with class icons (spawn regions, NPCs, stargates, cover sets)
- Click to select, drag to move, inspector panel for properties
- Filter by entity type (SGWMob, SGWSpawnRegion, SGWCoverSet, etc.)
- Import actor positions from .umap as starting data
- Export placement data as SQL INSERT statements

### Implementation Priority
1. 2D map view with BSP/terrain outline from .umap
2. Actor position overlay from .umap exports
3. Entity palette from entities.xml definitions
4. Drag-and-drop entity placement
5. Database integration for persistence
6. 3D view (stretch goal)

---

## Tool 3: `sgw-kismet-extractor` — Kismet Script Converter

### Purpose
Parse Kismet visual scripts from .umap files and convert them to the Cimmeria content engine's trigger/condition/action chain format.

### Why This Matters
SGW maps contain embedded Kismet sequences that drive:
- Mission triggers (enter region → start mission)
- NPC behavior (spawn events, dialog triggers)
- World events (stargate activation, environmental effects)
- Cinematic sequences (Matinee triggers)

Currently these are locked inside binary .umap files. The content engine uses database-driven chains instead.

### Data Model Mapping

| Kismet Concept | Content Engine Equivalent |
|---|---|
| `SeqEvent_*` (trigger) | `ContentTrigger` |
| `SeqCond_*` (condition) | `ContentCondition` |
| `SeqAct_*` (action) | `ContentAction` |
| `Sequence` (container) | `ContentChain` |
| `SeqVar_*` (variable) | Context parameters |
| Wire connections | Chain ordering + condition branching |

### UE3 Kismet Object Layout (from `UnrealEd/Src/Kismet.cpp`)
```
USequence
├── SequenceObjects: TArray<USequenceObject*>
│   ├── USequenceEvent     (triggers: LevelLoaded, Touch, etc.)
│   ├── USequenceCondition (comparisons, switches)
│   ├── USequenceAction    (gameplay effects)
│   └── USequenceVariable  (data storage)
└── InputLinks / OutputLinks (wiring between nodes)
```

### Deliverables
- [x] Parse Kismet sequence exports from .umap
- [x] Build in-memory node graph from serialized data
- [x] Generate content_chains SQL from node graph
- [x] Report unmappable nodes (manual review needed)
- [x] CLI: `tools/kismet_extractor.py <zone_dir> --sql|--graph|--json|--survey`

**Proof-of-concept:** `tools/kismet_extractor.py` — Python graph extractor
**Validated across all 24 zones:** 487,022 nodes → 88,113 content chains

---

## Tool 4: `sgw-asset-browser` — Package Content Browser

### Purpose
Browse and inspect UE3 package contents without the full editor. Useful for understanding what's in each .upk/.umap.

### Features
- Tree view: Package → Groups → Objects
- For each object: class, name, size, flags, dependencies
- Import/export cross-reference graph
- Search across all packages by class or name
- Texture preview (if we add DXT decompression)
- Sound preview (if we add FMOD integration)

### Implementation
- Tauri app reusing the admin panel's React framework
- Backend: `cimmeria-upk` crate for parsing
- Package index cached in SQLite for fast cross-package search

---

## Implementation Order

### Sprint 1: Foundation (cimmeria-upk)
1. Package header + name table parser
2. Import/export table parser
3. Object listing CLI tool
4. Validate against SGW .upk/.umap files
5. Store version findings (SGW's FileVersion + LicenseeVersion)

### Sprint 2: Actor Extraction
1. AActor property deserialization (position, rotation, scale, class)
2. Extract all actors from a .umap tile
3. Build zone-wide actor database from all tiles in a zone
4. Generate SQL for initial entity placement data

### Sprint 3: Kismet Extraction — COMPLETE
1. ~~USequence/USequenceObject deserialization~~ — tagged property parser handles all Kismet classes
2. ~~Node graph reconstruction (events → conditions → actions)~~ — full wiring decoded (InputLinks/OutputLinks/VariableLinks)
3. ~~Content chain SQL generation~~ — 88,113 chains from 24 zones
4. Validate against known mission triggers in Castle Cellblock

### Sprint 4: World Viewer
1. 2D map renderer (BSP outlines + actor markers)
2. Entity palette from entity definitions
3. Interactive placement with DB persistence
4. NavMesh overlay from .nav files

### Sprint 5: Asset Browser
1. Package tree view UI
2. Cross-package dependency graph
3. Search index
4. Preview support for common types

---

## UE3 Source Files to Study First

These files in the reference UE3 source contain the format knowledge we need:

| Priority | File | What It Tells Us |
|----------|------|-------------------|
| **P0** | `Core/Inc/UnLinker.h` | Package header, export, import struct definitions |
| **P0** | `Core/Src/UnLinker.cpp` | FLinkerLoad — how packages are read byte-by-byte |
| **P0** | `Core/Inc/UnObjVer.h` | Package version constants and compatibility |
| **P1** | `Core/Src/UnObj.cpp` | UObject::Serialize — base object serialization |
| **P1** | `Engine/Src/UnActorSerialization.cpp` | AActor property serialization (position, etc.) |
| **P1** | `Editor/Src/UnEditor.cpp` | How the editor reads/writes map data |
| **P2** | `UnrealEd/Src/Kismet.cpp` | Kismet node serialization and graph structure |
| **P2** | `Engine/Inc/UnSequence.h` | USequence/USequenceObject class hierarchy |
| **P3** | `Core/Inc/UnName.h` | FName serialization format |
| **P3** | `Core/Inc/UnType.h` | UProperty system (how object properties serialize) |

---

## Existing Community Tools (Reference)

These existing UE3 tools can inform our implementation:

| Tool | Language | Notes |
|------|----------|-------|
| UE Viewer (umodel) | C++ | Gildor's tool — reads UE1-4 packages, extracts meshes/textures |
| UPKUtils | Java | Package editing for XCOM modding |
| Unreal Package Lib | C# | .NET library for UE3 package parsing |
| nightly.link UAssetAPI | C# | Modern UE4/5 asset parser (different format but similar concepts) |

We should study **umodel**'s UE3 package reader in particular — it handles version variations across dozens of UE3 licensee games, which is exactly the problem we face with SGW's CME modifications.

---

## ~~Risk: SGW Version Identification~~ — RESOLVED

SGW uses **Epic Version 486, Licensee Version 6-8, Engine Version 3004**. All packages are consistent.
- Header has all conditional fields (TotalHeaderSize, FolderName, DependsOffset, EngineVersion, CookerVersion, Compression)
- FNameEntry uses FString + u64 flags
- FObjectExport is variable-length due to ComponentMap (TMap<FName,i32>) and GenNetObjCount (TArray<i32>)
- 77% of packages use LZO compression (flag 0x02), 23% are uncompressed
- Full binary format documented in `memory/upk-format.md`

---

## Content We Can Extract (Value Map)

| Content | Source | Emulator Value | Difficulty |
|---------|--------|----------------|------------|
| Actor positions (NPCs, objects) | .umap exports | **HIGH** — populate spawn data | Medium |
| Kismet sequences | .umap exports | **HIGH** — mission triggers | Hard |
| Zone geometry (BSP) | .umap exports | **MEDIUM** — world viewer | Medium |
| Static mesh references | .umap imports | **LOW** — asset inventory | Easy |
| Material references | .upk exports | **LOW** — visual reference | Easy |
| Texture data | .upk exports | **LOW** — preview only | Medium |
| Sound cue graphs | .upk exports | **LOW** — audio debugging | Hard |

---

## Related Documents

- [CME Framework](../engine/cme-framework.md) — SGW's UE3 modifications
- [Cooked Data Pipeline](../engine/cooked-data-pipeline.md) — .pak file format
- [Space Management](../engine/space-management.md) — Zone/space architecture
- [AtreaLoader Config](../technical/atrealoader-config.md) — Editor activation patches
