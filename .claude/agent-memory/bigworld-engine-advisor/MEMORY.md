# BigWorld Engine Advisor Memory

## Key Protocol Differences: Stock BigWorld vs SGW

See [protocol-comparison.md](protocol-comparison.md) for detailed findings.

### Wire Format Divergences (SGW vs Stock BigWorld 2.0.1)
- **Packet flags**: SGW = 1 byte; stock BW = 2 bytes (uint16)
- **Footer byte order**: SGW = little-endian; stock BW = big-endian (network order)
- **Encryption**: SGW = AES-256-CBC + HMAC-MD5; stock BW = Blowfish ECB + XOR chaining + 0xdeadbeef magic
- **EntityTypeID in createBasePlayer**: SGW = uint8 (1 byte); stock BW = uint16 (2 bytes)
- **forcedPosition**: SGW = 49 bytes (adds velocity Vec3 + flags u8); stock BW = 36 bytes
- **ENABLE_ENTITIES payload**: SGW = 8 bytes (uint64 dummy); stock BW = 1 byte (uint8 dummy)

### Confirmed Correct in Rust Rewrite
- EntityID type: i32 (matches stock BW `int32`)
- NULL_ENTITY = 0
- WSTRING encoding: u32 char count + UTF-16LE data
- createBasePlayer: [entityID:u32][classID:u8][propCount:u8] (6 bytes)
- createCellPlayer: [spaceID:u32][vehicleID:u32][pos:3xf32][rot:3xf32] (32 bytes)
- Entity method dispatch: 0xC0+index = base methods, 0x80+index = client methods
- resetEntities/enableEntities two-phase: server sends RESET, client auto-responds ENABLE

### Known Bugs in Rust Rewrite
- **RESOURCE_FRAGMENT length prefix**: `mercury_ext.rs` line 495 uses u32 (DWORD_LENGTH) but SGW message table says WORD_LENGTH (u16). Must be u16.

### Rotation Order Inconsistency in C++ Reference
- **createCellPlayer** (client_handler.cpp:400): rotX, rotZ, rotY (Y/Z swapped)
- **forcedPosition in createCellPlayer context** (client_handler.cpp:411): rotX, rotZ, rotY (Y/Z swapped)
- **Standalone forcedPosition** (client_handler.cpp:570): rotation.x, rotation.y, rotation.z (NOT swapped)
- Rust world-entry path correctly uses the swapped order for both; if standalone forcedPosition is added later, it needs the unswapped order

### Open Questions
- Exposed method sub-slot mechanism (for > 62 methods per entity) -- unlikely needed for SGW entities

## Key Source Locations

### BigWorld Engine 2.0.1
- Entity ID types: `external/engines/BigWorld-Engine-2.0.1/src/lib/network/basictypes.hpp`
- ID allocation: `external/engines/BigWorld-Engine-2.0.1/src/lib/server/id_client.cpp`
- Client interface: `external/engines/BigWorld-Engine-2.0.1/src/lib/connection/client_interface.hpp`
- BaseApp ext interface: `external/engines/BigWorld-Engine-2.0.1/src/lib/connection/baseapp_ext_interface.hpp`
- Server connection (resetEntities, createPlayer): `external/engines/BigWorld-Engine-2.0.1/src/lib/connection/server_connection.cpp`
- Encryption filter (Blowfish): `external/engines/BigWorld-Engine-2.0.1/src/lib/network/encryption_filter.cpp`
- Entity method descriptions (exposed ordering): `external/engines/BigWorld-Engine-2.0.1/src/lib/entitydef/entity_method_descriptions.cpp`
- Packet flags/format: `external/engines/BigWorld-Engine-2.0.1/src/lib/network/packet.hpp`
- Sequence/channel types: `external/engines/BigWorld-Engine-2.0.1/src/lib/network/misc.hpp`
- Direction3D serialization: `external/engines/BigWorld-Engine-2.0.1/src/lib/network/basictypes.cpp` (roll, pitch, yaw order)

### Cimmeria C++ (SGW customizations)
- Message IDs + format table: `src/baseapp/mercury/sgw/messages.hpp` and `messages.cpp`
- Client handler: `src/baseapp/mercury/sgw/client_handler.cpp`

### Rust Rewrite
- Entity types: `crates/common/src/types.rs`
- Mercury packet builder: `crates/mercury/src/packet.rs`
- Encrypted message builders: `crates/services/src/mercury_ext.rs`
- BaseApp handler: `crates/services/src/base.rs`
