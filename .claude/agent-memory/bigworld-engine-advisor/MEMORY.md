# BigWorld Engine Advisor Memory

## Phase −0.5 triage status (2026-05-13)

All five topic files have been triaged per #264 step 4. Bucket tags below each link.

## Verified ground truth (2026-07-25 docs/engine audit)

- [entity-def-and-pak-ground-truth.md](entity-def-and-pak-ground-truth.md) — **SGW uses only 3 property flags, zero client-replication flags**; data/cache is a merged build; watcher code IS in SGW.exe; Rust AoI has no hysteresis.
- [bw-reference-tree-absent.md](bw-reference-tree-absent.md) — `external/engines/BigWorld-Engine-2.0.1/` is NOT in this checkout and nothing fetches it; all `BW lib/...` citations are unfalsifiable here.

## Topic files

- [protocol-comparison.md](protocol-comparison.md) — **[PROMOTE → spec.protocol.mercury-wire-format]** — stock-vs-SGW wire divergences; mostly V5-confirmed, two items flagged for verification before promotion (rotation order, instanceID-vs-spaceID wording).
- [aoi-entity-introduction.md](aoi-entity-introduction.md) — **[PROMOTE → spec.world.world-entry]** — createOnClient property cascade for NPC AoI entry; V5-confirmed.
- [cache-stamp-system.md](cache-stamp-system.md) — **[PROMOTE → spec.engine.cooked-data-pipeline + spec.world.world-entry]** — two-system breakdown (entity cache stamps + cooked-data versioning); V5-confirmed.
- [sgwplayer-method-index-table.md](sgwplayer-method-index-table.md) — **[PROMOTE → spec.engine.entity-description-parse-chain]** — 157-method index table; highest-value PROMOTE in this agent's memory.
- [viewport-system.md](viewport-system.md) — **[PROMOTE → spec.world.world-entry §"viewport association"]** — client-side viewport memory layout + svidFollow algorithm; not in published V5 findings.

## Inline facts in this file

The lists below were pre-V5 working notes. Most are now V5-confirmed and overlap with the linked topic files; treat this section as a quick-reference index. Section-by-section status:

### Wire Format Divergences (SGW vs Stock BigWorld 2.0.1) — **[PROMOTE]**

V5-confirmed against `findings/mercury-protocol-internals.md`. ENABLE_ENTITIES 8-byte (uint64 dummy) confirmed — note the W-misc-gaps 1-byte claim was wrong; see `world-entry-pipeline.md` reconciliation section.

- **Packet flags**: SGW = 1 byte; stock BW = 2 bytes (uint16)
- **Footer byte order**: SGW = little-endian; stock BW = big-endian (network order)
- **Encryption**: SGW = AES-256-CBC + HMAC-MD5; stock BW = Blowfish ECB + XOR chaining + 0xdeadbeef magic
- **EntityTypeID in createBasePlayer**: SGW = uint8 (1 byte); stock BW = uint16 (2 bytes)
- **forcedPosition**: SGW = 49 bytes (adds velocity Vec3 + flags u8); stock BW = 36 bytes
- **ENABLE_ENTITIES payload**: SGW = 8 bytes (uint64 dummy); stock BW = 1 byte (uint8 dummy)

### Confirmed Correct in Rust Rewrite — **[PROMOTE]**

- EntityID type: i32 (matches stock BW `int32`)
- NULL_ENTITY = 0
- WSTRING encoding: u32 char count + UTF-16LE data
- createBasePlayer: [entityID:u32][classID:u8][propCount:u8] (6 bytes)
- createCellPlayer: [spaceID:u32][vehicleID:u32][pos:3xf32][rot:3xf32] (32 bytes)
- Entity method dispatch: 0xC0+index = base methods, 0x80+index = client methods
- resetEntities/enableEntities two-phase: server sends RESET, client auto-responds ENABLE

### Known Bugs in Rust Rewrite — **[RE-VERIFY]**

- **RESOURCE_FRAGMENT length prefix**: the path reference `mercury_ext.rs line 495` is stale — the file was refactored into `crates/services/src/mercury/protocol/resources.rs`. The u16 length-prefix fact itself is V5-confirmed and the fix has shipped (test `resource_fragment_uses_u16_length_prefix` guards it).

### Rotation Order Inconsistency in C++ Reference — **[RE-VERIFY]**

The three observed `client_handler.cpp` line variants need re-verification against the V5 evidence. The `protocol-comparison.md` topic file flags rotation order as "needs pcap confirmation" — resolve before promoting to `spec.protocol.mercury-wire-format` §4. Current Rust uses swapped order for createCellPlayer + forcedPosition (the path through world-entry).

### Space Data & Terrain Loading (CRITICAL) — **[PROMOTE]**

V5-confirmed against `findings/world-entry-pipeline.md` §6 "onClientMapLoad is the terrain-loading mechanism". Five-field signature confirmed: WSTRING areaName, WSTRING mapPath, INT32 worldId, VECTOR3 Location, VECTOR3 Direction. Method index 117 on SGWPlayer (per the SGWPlayer 157-method table).

### avatarUpdateExplicit (C->S 0x03) First Field is spaceID — **[PROMOTE]**

V5-confirmed against `findings/world-entry-pipeline.md` and the address-map. Worth a section in `spec.protocol.message-catalog` and `spec.world.world-entry`. Note the entry says "Rust server incorrectly reads payload[0..4] as entity_id" — verify whether this bug is still open in current `crates/services/src/cell/` before authoring section 5 of the chapter.

### Entity Method Call Wire Format (S->C) — **[PROMOTE]**

V5-confirmed. The `[msg_id][length][entity_id][args]` layout is the universal RPC dispatcher protocol — cross-link to `spec.engine.universal-rpc-dispatcher` (`0x00c6fc40`).

### CookedData ServerSource Event Chain — **[PROMOTE → spec.engine.cooked-data-pipeline]**

V5-confirmed against `findings/cooked-data-pipeline.md` (21 categories not 22; 5-event LibCategory subscription pattern). The "Category 7 char_creation NOT in categoryMaps → client loads from local PAK" finding is a key gameplay-affecting fact worth a section in the chapter.

### NPC Entity AoI Introduction — **[PROMOTE]**

V5-confirmed. Same content as the `aoi-entity-introduction.md` topic file.

### Cache Stamp vs Cooked Data Versioning — **[PROMOTE]**

Same content as the `cache-stamp-system.md` topic file.

### SGWPlayer Client Method Index Table — **[PROMOTE]**

Same content as the `sgwplayer-method-index-table.md` topic file.

### Open Questions — **[RE-VERIFY]**

Sub-slot encoding details: now confirmed in `findings/entity-property-sync.md` (threshold 62, sub-slot mechanism C++ code at `entity_method_descriptions.cpp:checkExposedForSubSlots()`). Move out of open-questions during chapter authoring.

## Key Source Locations

### BigWorld Engine 2.0.1

(Reference-only; these paths are immutable in `external/BigWorld-2.0.1/`. Path-rewrite from the prior `external/engines/BigWorld-Engine-2.0.1/` was done in the mechanical pass.)

- Entity ID types: `external/BigWorld-2.0.1/src/lib/network/basictypes.hpp`
- ID allocation: `external/BigWorld-2.0.1/src/lib/server/id_client.cpp`
- Client interface: `external/BigWorld-2.0.1/src/lib/connection/client_interface.hpp`
- BaseApp ext interface: `external/BigWorld-2.0.1/src/lib/connection/baseapp_ext_interface.hpp`
- Server connection (resetEntities, createPlayer): `external/BigWorld-2.0.1/src/lib/connection/server_connection.cpp`
- Encryption filter (Blowfish): `external/BigWorld-2.0.1/src/lib/network/encryption_filter.cpp`
- Entity method descriptions (exposed ordering): `external/BigWorld-2.0.1/src/lib/entitydef/entity_method_descriptions.cpp`
- Packet flags/format: `external/BigWorld-2.0.1/src/lib/network/packet.hpp`
- Sequence/channel types: `external/BigWorld-2.0.1/src/lib/network/misc.hpp`
- Direction3D serialization: `external/BigWorld-2.0.1/src/lib/network/basictypes.cpp` (roll, pitch, yaw order)
- Space data types: `external/BigWorld-2.0.1/src/common/space_data_types.hpp`
- Client spaceData handler: `external/BigWorld-2.0.1/src/client/entity_manager.cpp:1382`

### Cimmeria deprecated C++ (SGW customizations)

- Message IDs + format table: `deprecated/cpp/src/baseapp/mercury/sgw/messages.hpp` and `messages.cpp`
- Client handler: `deprecated/cpp/src/baseapp/mercury/sgw/client_handler.cpp`

### Rust Rewrite

- Entity types: `crates/common/src/types.rs`
- Mercury packet builder: `crates/mercury/src/packet.rs`
- Encrypted message builders: `crates/services/src/mercury/protocol/` (the prior `mercury_ext.rs` was split)
- BaseApp handler: `crates/services/src/base.rs`
- Cooked data handler: `crates/services/src/base/cooked_data.rs`
- Version info builder: `crates/services/src/mercury/protocol/` (resources / version-info submodule)

### Python Game Logic

- Account versionInfoRequest: `deprecated/python/base/Account.py:322`
- DefMgr.ResourceCategories: `deprecated/python/common/defs/Def.py:42` (cat 7 = "character_creation")
- Category 7 NOT in Account.py categoryMaps (lines 327-336) = no server-push for char_creation

(The original `src/` and `python/base/` paths above were rewritten to `deprecated/cpp/src/` and `deprecated/python/base/` in the mechanical pass.)
