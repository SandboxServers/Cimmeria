---
name: Cache Stamp System Deep Dive
description: Complete analysis of BigWorld createCacheStamp / versionInfoRequest / ClientCache mechanism as implemented in SGW's C++ server
type: reference
---

> [!NOTE] PROMOTION TARGET: spec.engine.cooked-data-pipeline + spec.world.world-entry §"property cache deltas"
>
> Triaged 2026-05-13 (Phase −0.5 step 4). Content describes two separate systems (entity cache stamps + cooked-data versioning) that should land in two different bible chapters. Cross-references V5 `cooked-data-pipeline.md` for the 21-category subscription pattern; the entity-cache-stamp half here adds value the V5 docs don't cover. Keep until both chapters are scaffolded.

## Entity-Level Cache Stamps (createCacheStamp) -- SEPARATE from Cooked Data Versions

Two completely different systems share similar naming:

### 1. Entity Property Cache Stamps (CellApp -> BaseApp -> Client)

- `createCacheStamp(propertySetId, callback, invalidate)` on CellEntity
- CellApp calls `beginCacheStamp()` -> runs Python callback (which calls entity methods) -> `endCacheStamp()`
- Messages generated during callback are intercepted by `sendClientMessage()` and stored in a cache buffer instead of sent directly
- Buffer sent to BaseApp as `CELL_BASE_UPDATE_CACHE_STAMP (0x11)`: [entityId:u32][propSetId:u32][invalidate:u8][messages...]
- Each message in buffer: [messageId:u8][flags:u8][length:u16][args:bytes]
- BaseApp stores these in `CachedEntity::PropertySet` with version tracking
- When a witness enters AoI, BaseApp sends delta (only messages newer than witness's known version)
- `MaxPropertySets = 2` (only 2 cache groups per entity)
- LEAVE_AOI wire format includes `cacheStamp=0` (always 0 in SGW C++)
- `cacheStampsReset()` called when entity first registers with BaseApp (SGWSpawnableEntity.py:117)
- This is the NPC AoI introduction mechanism: `createOnClient()` calls cached inside stamp

### 2. Cooked Data Versioning (versionInfoRequest / onVersionInfo)

- Client sends `versionInfoRequest(categoryId, version)` (wire 0xC0) to server
- Server compares version against its PAK MetaData version
- Responds with `onVersionInfo(categoryId, serverVersion, requiredUpdates, invalidateAll, invalidated[])`
- If invalidateAll=true, client purges its local cache for that category
- Client loads from local .pak files, and may request individual elements via `elementDataRequest` (0xC1)

### Key Insight: These Are Completely Separate Systems
- Entity cache stamps are about NPC property replay for AoI
- Cooked data versions are about items/abilities/missions definitions
- They share no wire protocol interaction
