# Engine Documentation

> **Last updated**: 2026-07-25

BigWorld Technology internals, Cheyenne Mountain Entertainment (CME) framework, and engine subsystems. 13 documents.

## Documents

| Document | Description | Status |
|----------|-------------|--------|
| [entity-type-catalog.md](entity-type-catalog.md) | All 18 entities + 18 interfaces with full property/method tables | HUB - complete |
| [bigworld-architecture.md](bigworld-architecture.md) | BigWorld 1.9.1 cell/base/client architecture: entity manager, space manager, connection layer | Complete |
| [cme-framework.md](cme-framework.md) | CME PropertyNode, EventSignal (750 types), Atrea scripts, SpaceViewport | Complete |
| [cooked-data-pipeline.md](cooked-data-pipeline.md) | .pak format, XSD schemas, CookedElementBase, gSOAP deserialization, Mercury resource delivery | Complete |
| [cooked-data-pak-format.md](cooked-data-pak-format.md) | Cooked-data PAK file format: on-disk layout, entry table, compression, client read path | Complete |
| [ue3-package-format.md](ue3-package-format.md) | SGW UE3 package binary format (ver 486 licensee fork): section ordering + the `total_header_size` trap, LZO chunking, variable-length export trailers, actor/component serial prefixes, ULevel `Actors` layout, property tag stream, HUD↔world coordinate swizzle | Complete |
| [entity-def-guide.md](entity-def-guide.md) | Entity definition (`.def`) file format: property/method declarations, interfaces, type aliases | Complete |
| [character-visual-components.md](character-visual-components.md) | Character visual components: how avatar appearance (model, skin, equipment) is composited | Complete |
| [client-visual-system.md](client-visual-system.md) | Client visual system: rendering, scene graph, how entities are drawn | Complete |
| [watcher-system.md](watcher-system.md) | BigWorld watcher system: classes, protocol, network, Python API | Deep-dive (Phase 5) |
| [space-management.md](space-management.md) | Cell spaces, WorldGrid, BSP tree, ghost entities, load balancing | Deep-dive (Phase 5) |
| [entity-lod-system.md](entity-lod-system.md) | BigWorld entity property LOD (not used by SGW) | Deep-dive (Phase 5) |
| [distributed-checkpointing.md](distributed-checkpointing.md) | Distributed backup, crash recovery, reviver system | Deep-dive (Phase 5) |

## Key References

- **BigWorld reference source**: `external/engines/BigWorld-Engine-2.0.1/` — **not vendored and not fetched by `setup.ps1`**. Nothing in the repo creates `external/engines/`; a fresh checkout will not have it. Docs citing paths under it are quoting an externally-obtained copy. Both [1.9.1](https://github.com/v2v3v4/BigWorld-Engine-1.9.1) and [2.0.1](https://github.com/v2v3v4/BigWorld-Engine-2.0.1) are on GitHub
- **Cimmeria engine code** (active): the Rust workspace under `crates/` — see `crates/mercury/` (Mercury transport) and `crates/services/` (base/cell services)
- **Legacy C++ engine code** (historical, not extended): `deprecated/cpp/src/` (UnifiedKernel — `common/`, `mercury/`, `entity/`)
- **Entity definitions**: `entities/defs/`, `entities/entities.xml`
- **Space definitions**: `entities/cell_spaces.xml`
- **Cooked data**: `data/cache/*.pak`, client XSD schemas

## Entity Statistics

Counted from `entities/defs/` by direct children of the `<Properties>`, `<ClientMethods>`, `<BaseMethods>`, and `<CellMethods>` blocks.

| Category | Count |
|----------|-------|
| Entity types | 18 |
| Interfaces | 18 |
| Properties declared in the 18 entity defs | 270 |
| Properties declared in the 18 interface defs | 166 |
| Methods declared in the 18 entity defs | 479 |
| Methods declared in the 18 interface defs | 337 |
| SGWPlayer properties — declared in `SGWPlayer.def` alone | 68 |
| SGWPlayer methods — declared in `SGWPlayer.def` alone | 175 (59 client / 20 base / 96 cell) |
| SGWPlayer properties — flattened over parents + 11 interfaces | 221 |
| SGWPlayer methods — flattened over parents + 11 interfaces | 514 (149 client / 79 base / 286 cell) |

The own-file and flattened figures differ by roughly 3x, so always say which one you mean. Earlier revisions of this table labelled the 68/175 own-file counts as "with interfaces", which they are not.
