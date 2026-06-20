# Engine Documentation

> **Last updated**: 2026-06-20

BigWorld Technology internals, Cheyenne Mountain Entertainment (CME) framework, and engine subsystems. 12 documents.

## Documents

| Document | Description | Status |
|----------|-------------|--------|
| [entity-type-catalog.md](entity-type-catalog.md) | All 18 entities + 18 interfaces with full property/method tables | HUB - complete |
| [bigworld-architecture.md](bigworld-architecture.md) | BigWorld 1.9.1 cell/base/client architecture: entity manager, space manager, connection layer | Complete |
| [cme-framework.md](cme-framework.md) | CME PropertyNode, EventSignal (750 types), Atrea scripts, SpaceViewport | Complete |
| [cooked-data-pipeline.md](cooked-data-pipeline.md) | .pak format, XSD schemas, CookedElementBase, gSOAP deserialization, Mercury resource delivery | Complete |
| [cooked-data-pak-format.md](cooked-data-pak-format.md) | Cooked-data PAK file format: on-disk layout, entry table, compression, client read path | Complete |
| [entity-def-guide.md](entity-def-guide.md) | Entity definition (`.def`) file format: property/method declarations, interfaces, type aliases | Complete |
| [character-visual-components.md](character-visual-components.md) | Character visual components: how avatar appearance (model, skin, equipment) is composited | Complete |
| [client-visual-system.md](client-visual-system.md) | Client visual system: rendering, scene graph, how entities are drawn | Complete |
| [watcher-system.md](watcher-system.md) | BigWorld watcher system: classes, protocol, network, Python API | Deep-dive (Phase 5) |
| [space-management.md](space-management.md) | Cell spaces, WorldGrid, BSP tree, ghost entities, load balancing | Deep-dive (Phase 5) |
| [entity-lod-system.md](entity-lod-system.md) | BigWorld entity property LOD (not used by SGW) | Deep-dive (Phase 5) |
| [distributed-checkpointing.md](distributed-checkpointing.md) | Distributed backup, crash recovery, reviver system | Deep-dive (Phase 5) |

## Key References

- **BigWorld reference source**: `external/engines/BigWorld-Engine-2.0.1/` (1.9.1 also available)
- **Cimmeria engine code** (active): the Rust workspace under `crates/` — see `crates/mercury/` (Mercury transport) and `crates/services/` (base/cell services)
- **Legacy C++ engine code** (historical, not extended): `deprecated/cpp/src/` (UnifiedKernel — `common/`, `mercury/`, `entity/`)
- **Entity definitions**: `entities/defs/`, `entities/entities.xml`
- **Space definitions**: `entities/cell_spaces.xml`
- **Cooked data**: `data/cache/*.pak`, client XSD schemas

## Entity Statistics

| Category | Count |
|----------|-------|
| Entity types | 18 |
| Interfaces | 18 |
| Total properties (all entities) | ~300+ |
| Total methods (all entities) | ~500+ |
| SGWPlayer properties (with interfaces) | 68 |
| SGWPlayer methods (with interfaces) | 175+ |
