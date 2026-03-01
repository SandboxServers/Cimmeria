# Engine Documentation

> **Last updated**: 2026-03-01

BigWorld Technology internals, Cheyenne Mountain Entertainment (CME) framework, and engine subsystems.

## Documents

| Document | Description | Status |
|----------|-------------|--------|
| [entity-type-catalog.md](entity-type-catalog.md) | All 18 entities + 18 interfaces with full property/method tables | HUB - complete |
| [bigworld-architecture.md](bigworld-architecture.md) | BigWorld cell/base/client architecture overview | Stub |
| [cme-framework.md](cme-framework.md) | CME PropertyNode, EventSignal, SoapLibrary | Stub |
| [cooked-data-pipeline.md](cooked-data-pipeline.md) | .pak format, XSD schemas, CookedElementBase | Stub |
| [watcher-system.md](watcher-system.md) | BigWorld reactive property monitoring | Stub |
| [space-management.md](space-management.md) | Cell spaces, grid chunks, load balancing | Stub |

## Key References

- **BigWorld reference source**: `external/engines/BigWorld-Engine-2.0.1/` (1.9.1 also available)
- **Cimmeria engine code**: `src/lib/` (UnifiedKernel)
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
