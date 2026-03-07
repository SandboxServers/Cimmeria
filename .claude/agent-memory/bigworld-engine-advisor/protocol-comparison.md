# Stock BigWorld 2.0.1 vs SGW Protocol Comparison

Full analysis performed 2026-03-04 against engine source in `external/engines/BigWorld-Engine-2.0.1/`.

## EntityID
- Stock: `typedef int32 EntityID` (signed), NULL_ENTITY = 0, client-only IDs > 1<<30
- SGW: Same type on wire. Rust uses `EntityId(pub i32)` -- correct.
- Allocation: Stock uses IDClient batch allocation from DBMgr. SGW emulator uses hardcoded IDs.

## Encryption
- Stock: Blowfish ECB with XOR chaining, 8-byte blocks, 0xdeadbeef magic, wastage byte
- SGW: AES-256-CBC (32-byte key from SOAP), zero IV, PKCS7 padding, HMAC-MD5 (16 bytes)
- Completely different -- stock BW encryption code is irrelevant.

## Packet Flags
- Stock: uint16 (2 bytes) in network byte order. 0x0040=HAS_SEQUENCE, 0x0010=RELIABLE, etc.
- SGW: uint8 (1 byte). 0x40=HAS_SEQUENCE, 0x10=RELIABLE, 0x08=ON_CHANNEL, 0x04=HAS_ACKS
- Flag values are the same when comparing the low byte; SGW just doesn't use the high byte.

## Footer Byte Order
- Stock: Network byte order (big-endian) via BW_HTONS/BW_HTONL in packFooter()
- SGW: Little-endian (confirmed by pcap and working Rust implementation)

## Direction3D
- Stock: Serialized as roll, pitch, yaw (basictypes.cpp:38-50)
- Stock struct: `struct Direction3D { float roll; float pitch; float yaw; }`
- SGW: Comments in messages.cpp say "Yaw, Pitch, Roll" for createCellPlayer
- MEMORY.md says "rotX, rotZ, rotY (Y/Z swapped)" -- needs pcap confirmation

## forcedPosition
- Stock: 36 bytes = entityID(4) + spaceID(4) + vehicleID(4) + pos(12) + direction(12)
- SGW: 49 bytes = entityID(4) + spaceID(4) + vehicleID(4) + pos(12) + velocity(12) + rotation(12) + flags(1)
- SGW adds velocity Vec3 and flags uint8

## createBasePlayer
- Stock: [entityID:int32][playerType:uint16][...base_properties]
- SGW: [entityID:uint32][classID:uint8][propertyCount:uint8]
- ClassID is 1 byte in SGW vs 2 bytes in stock

## createCellPlayer
- Stock: [spaceID:int32][vehicleID:int32][pos:Vec3][dir:Direction3D(roll,pitch,yaw)][...cell_props]
- SGW: [instanceID:uint32][vehicleID:uint32][pos:Vec3][rot:Vec3(yaw?,pitch?,roll?)][...cell_props?]

## ENABLE_ENTITIES
- Stock: struct { uint8 dummy; } -- 1 byte payload
- SGW: CONSTANT_LENGTH = 8 bytes (uint64 dummy per messages.cpp)

## Entity Method Dispatch
- Same scheme: 0xC0+index = base methods, 0x80+index = cell/client methods
- PROPERTY_FLAG = 0x40 (property update vs method call, server-to-client)
- Exposed-only ordering for client-callable methods
- Sub-slot mechanism for > 62 exposed methods (unlikely needed in SGW)

## resetEntities Flow
- Server sends RESET_ENTITIES (0x04, keepPlayerOnBase: bool)
- Client auto-responds with ENABLE_ENTITIES (0x08)
- Client clears entity state between reset and enable
- Server should wait for ENABLE_ENTITIES before sending new entity creation messages
