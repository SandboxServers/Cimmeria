# System Protocol Handler Decompilation Evidence

> **Confidence**: HIGH
> **Date**: 2026-03-05
> **Sources**: Ghidra decompilation of sgw.exe (32-bit, MSVC)
> **Companion**: See [`space-viewport-wire-formats.md`](space-viewport-wire-formats.md) for the complete wire format reference.

This document provides **Ghidra decompilation evidence** for the system/connection protocol message handlers in the SGW client binary. It supplements the wire format tables in `space-viewport-wire-formats.md` with the actual decompiled C and assembly code from each handler function.

---

## Handler Function Index

| msg_id | Message | Handler Function | Address |
|--------|---------|-----------------|---------|
| 0x00 | AUTHENTICATE (server) | `ServerConnection_authenticate` | `0x00dd8510` |
| 0x04 | RESET_ENTITIES | `Mercury__unknown_00dda0e0` (resetEntities) | `0x00dda0e0` |
| 0x05 | CREATE_BASE_PLAYER | `ServerConnection_createBasePlayer` | `0x00dddca0` |
| 0x06 | CREATE_CELL_PLAYER | `ServerConnection_createCellPlayer` | `0x00dda2e0` |
| 0x07 | SPACE_DATA | `ServerConnection_spaceData` | `0x00dda540` |
| 0x08 | VIEWPORT_INFO | `ServerConnection_spaceViewportInfo` | `0x00dda6c0` |
| 0x0D | TICK_SYNC | Static `ClientMessageHandler<tickSyncArgs>` | (descriptor at `0x017bb720`) |
| 0x03 | SET_GAME_TIME | Static `ClientMessageHandler<setGameTimeArgs>` | (descriptor at `0x017bb180`) |
| 0x31 | FORCED_POSITION | `ServerConnection_forcedPosition` | `0x00dd9ee0` |
| 0x34 | RESTORE_CLIENT | `ServerConnection_restoreClient` | `0x00dd8ae0` |
| 0x36 | RESOURCE_FRAGMENT | `ServerConnection_resourceFragment` | `0x00dddd80` |
| 0x37 | LOGGED_OFF | `ServerConnection_loggedOff` | `0x00dd8c20` |
| - | enableEntities (internal) | `ServerConnection_enableEntities` | `0x00dd9280` |
| - | enableEntitiesRejected | `Detail_EnableEntitiesRejectedMessage__vfunc_0` | `0x01561da0` |
| - | startEntityMessage | `ServerConnection_startEntityMessage` | `0x00dd6a60` |
| - | startProxyMessage | `ServerConnection_startProxyMessage` | `0x00dd6980` |

---

## AUTHENTICATE (0x00) -- Server-to-Client Key Exchange

**Handler**: `ServerConnection_authenticate` @ `0x00dd8510`
**Length type**: DWORD_LENGTH (u32 LE prefix)

### Decompiled Code

```c
void __thiscall ServerConnection_authenticate(void *this, int *param_1)
{
    // Read packed string from stream (session key)
    FUN_00de3770(param_1);  // packed string reader

    // Compare received key with stored key at this+0x08
    if (this->keyLen < 0x10) {
        pcVar1 = (char*)(this + 8);  // inline string
    } else {
        pcVar1 = *(char**)(this + 8);  // heap string
    }
    uVar2 = FUN_004242c0(received_key, 0, received_len, pcVar1, this->keyLen);
    if (uVar2 != 0) {
        // Mismatch!
        LOG("ServerConnection::authenticate: Unexpected key! (%s, wanted %s)\n");
    }
}
```

### Packed String Reader (FUN_00de3770 @ 0x00de3770)

This utility function reads a length-prefixed string from a Mercury stream:

```c
int* FUN_00de3770(int *stream)
{
    byte *pbVar1 = (byte*)stream->read(1);  // read 1 byte
    uint len = (uint)*pbVar1;
    if (len == 0xff) {
        uint3 *puVar2 = (uint3*)stream->read(3);  // read 3 bytes
        len = (uint)*puVar2;  // 24-bit extended length
    }
    char *data = (char*)stream->read(len);  // read string bytes
    // copy into output buffer
    return stream;
}
```

**Wire encoding**: `[1 byte: len or 0xFF][if 0xFF: 3 bytes extended len][len bytes: data]`

### Assembly Evidence

```asm
; 0x00dd855d: Call packed string reader with stream
PUSH EAX       ; param_1 (stream)
CALL 0x00de3770

; 0x00dd8562-0x00dd857a: Get stored key pointer
MOV ECX, [ESI+0x18]    ; this->keyLen
CMP [ESI+0x1c], EDI    ; compare with 0x10 (SSO threshold)
JC short_string
MOV EAX, [ESI+0x8]     ; heap-allocated key pointer
JMP compare
short_string:
LEA EAX, [ESI+0x8]     ; inline key buffer

; 0x00dd8586: String comparison
CALL 0x004242c0         ; std::string::compare equivalent
TEST EAX, EAX
JZ ok                   ; match -> continue
; mismatch -> log error
```

---

## RESET_ENTITIES (0x04) -- Entity System Teardown

**Handler**: `Mercury__unknown_00dda0e0` @ `0x00dda0e0`
**Length type**: CONSTANT(1)

### Decompiled Code

```c
void __thiscall resetEntities(void *this, char *param_1)
{
    // Assert entitiesEnabled_
    if (*(char*)(this + 0x316) == '\0') {
        ASSERT("entitiesEnabled_", "servconn.cpp");
    }

    // Flush channel
    Mercury_Nub_7(this);

    // Clear all entity tracking structures:
    // - Entity list at this+0xf84 (linked list, freed via FUN_00de1d10)
    // - Entity map at this+0xf90 (freed via FUN_01606150)
    // - Space ref-counts at this+0xf9c (freed via FUN_01606150)
    // - Physics map at this+0xfac (freed via FUN_01606150)
    *(byte*)(this + 0xfa8) = 0;  // Clear active viewport

    // Reset buffered cell player message
    *(uint32*)(this + 0xfec) = *(uint32*)(this + 0xfe8);
    *(uint32*)(this + 0xff4) = *(uint32*)(this + 0xfe8);

    // Read keepBase byte
    if (*param_1 == '\0') {  // keepBase == false
        *(uint32*)(this + 0x16c) = 0;  // Clear player entity ID

        // Free all resource request objects in vector at this+0xfd0
        for (each request in vector) {
            scalable_free(request);
        }
        // Compact vector
    }

    // Disable entities, then re-enable
    *(byte*)(this + 0x316) = 0;  // entitiesEnabled_ = false
    ServerConnection_enableEntities(this);  // immediately re-enables

    // Notify handler
    if (*(int*)(this + 0x168) != 0) {
        handler->onResetEntities(*param_1);  // vtable[0x3c]
    }
}
```

### Key Finding

After `RESET_ENTITIES` is received, the client **automatically** calls `ServerConnection_enableEntities`, which sends the client-to-server `ENABLE_ENTITIES` (msg_id 0x08) back. The server should wait for this before sending entity creation messages.

### Assembly Evidence

```asm
; 0x00dda1ea: Read keepBase byte
MOV EAX, [ESP+0x18]    ; param_1 (args struct)
CMP byte ptr [EAX], BL ; compare *param_1 with 0
JNZ skip_full_reset     ; if keepBase != 0, skip

; 0x00dda1f6: Clear player entity ID
MOV [ESI+0x16c], EBX   ; this->playerEntityID_ = 0

; 0x00dda2b1: Disable entities
MOV byte ptr [ESI+0x316], BL  ; entitiesEnabled_ = false

; 0x00dda2b7: Re-enable entities (sends ENABLE_ENTITIES to server)
CALL ServerConnection_enableEntities  ; 0x00dd9280
```

---

## enableEntities (Internal Client Action)

**Handler**: `ServerConnection_enableEntities` @ `0x00dd9280`
**Not a wire message** -- called internally after RESET_ENTITIES.

### Decompiled Code

```c
void __fastcall ServerConnection_enableEntities(void *param_1)
{
    // Get channel and prepare message
    void *channel = Mercury_Channel_2((int)param_1);
    ServerConnection__unknown_0157ad80(channel, DAT_01ef2500);  // write msg header

    // Allocate body space (8 bytes of dummy data)
    int *buf = *(int*)(channel + 8);
    int available = 0x5ad - buf[0x2c] - buf[0x28] - buf[0x24];
    int bodySize = *(int*)(DAT_01ef2500 + 4);
    if (available < bodySize) {
        FUN_0157a5d0(channel, bodySize);  // flush + allocate
    } else {
        buf[0x24] += bodySize;  // reserve space
    }

    // Optional profiling
    if (DAT_01ef2224 != 0) { /* profiler calls */ }

    LOG("ServerConnection::enableEntities: Enabling entities %d\n");

    // Flush the bundle
    Mercury_Nub_7(param_1);

    // Mark entities as enabled
    *(byte*)((int)param_1 + 0x316) = 1;  // entitiesEnabled_ = true
}
```

This sends an 8-byte constant-length `ENABLE_ENTITIES` message to the server, containing dummy payload (the client-to-server msg_id 0x08 table entry specifies `CONSTANT_LENGTH = 8`).

---

## LOGGED_OFF (0x37) -- Server Disconnect

**Handler**: `ServerConnection_loggedOff` @ `0x00dd8c20`
**Length type**: CONSTANT(1)

### Decompiled Code

```c
void __fastcall ServerConnection_loggedOff(void *param_1)
{
    LOG("ServerConnection::loggedOff: The server has disconnected us. reason = %d\n");
    Mercury__unknown_00dd8630(param_1, '\0');  // disconnect(sendMsg=false)
}
```

### Assembly Evidence

```asm
; 0x00dd8c2f: Read reason byte for logging
MOVZX EDX, byte ptr [ECX]  ; reason = *(u8*)args
PUSH EDX                     ; push for printf format

; 0x00dd8c51-0x00dd8c55: Call disconnect handler
PUSH 0x0                     ; sendDisconnectMsg = false
MOV ECX, ESI                 ; this
CALL 0x00dd8630              ; disconnect handler
```

The disconnect handler (`Mercury__unknown_00dd8630` @ `0x00dd8630`) tears down the connection:
- If `sendDisconnectMsg == true`: sends a DISCONNECT message to the server, flushes channel
- Destroys the connection object at `this+0x30c`
- Frees all resource requests
- Clears handler pointer at `this+0x168`

---

## RESTORE_CLIENT (0x34) -- Client State Restore

**Handler**: `ServerConnection_restoreClient` @ `0x00dd8ae0`
**Length type**: WORD_LENGTH

### Decompiled Code

```c
void __thiscall ServerConnection_restoreClient(void *this, int *param_1)
{
    float position[3] = {0};
    float direction[3] = {0};

    uint32 entityId  = *(uint32*)stream->read(4);
    uint32 spaceId   = *(uint32*)stream->read(4);
    uint32 vehicleId = *(uint32*)stream->read(4);
    // Read direction Vec3 (12 bytes as one block)
    void *dirData = stream->read(0xc);
    memcpy(direction, dirData, 12);
    // Read position Vec3 (3 x 4 bytes via FUN_015846a0)
    FUN_015846a0(param_1);  // reads 3 x read(4) into position

    if (*(int*)(this + 0x168) == 0) {
        LOG("ServerConnection::restoreClient: No handler. Maybe already logged off.\n");
    } else {
        handler->onRestoreClient(entityId, spaceId, vehicleId, direction, position, stream);
    }

    // Send restoreClientAck if connected
    if (*(int*)(this + 0x30c) != 0) {
        channel = Mercury_Channel_2(this);
        channel->writeHeader(DAT_01ef250c);  // restoreClientAck message
        *(uint32*)channel->reserve(4) = 0;   // ack payload = 0
        Mercury_Nub_7(this);                  // flush
    }
}
```

### Assembly Evidence

```asm
; 0x00dd8af5: read entityId (4 bytes)
PUSH 0x4 / CALL EDX -> stream.read(4)
MOV EBX, [EAX]        ; entityId

; 0x00dd8b26: read spaceId (4 bytes)
PUSH 0x4 / CALL EDX -> stream.read(4)
MOV EBP, [EAX]        ; spaceId

; 0x00dd8b33: read vehicleId (4 bytes)
PUSH 0x4 / CALL EDX -> stream.read(4)
MOV EAX, [EAX]        ; vehicleId

; 0x00dd8b44: read direction (12 bytes as one block)
PUSH 0xC / CALL EAX -> stream.read(12)
MOVQ XMM0, qword ptr [EAX]  ; first 8 bytes
MOV ECX, [EAX+0x8]           ; last 4 bytes

; 0x00dd8b61: read position (3 x 4 bytes)
CALL FUN_015846a0     ; reads 3 x stream.read(4)
```

---

## FORCED_POSITION (0x31) -- Rotation Order Evidence

**Handler**: `ServerConnection_forcedPosition` @ `0x00dd9ee0`
**Length type**: CONSTANT(49)

The key finding from this handler is the **rotation field order** on the wire.

### Decompiled Code (rotation mapping)

```c
void __thiscall ServerConnection_forcedPosition(void *this, int *param_1)
{
    // param_1 is a pointer to the 49-byte message body, cast as int*
    // param_1[0]  = entityId  (offset 0)
    // param_1[1]  = spaceId   (offset 4)
    // param_1[2]  = vehicleId (offset 8)
    // param_1[3..5] = position (offset 12-23, 3 floats)
    // param_1[6..8] = velocity (offset 24-35, 3 floats)
    // param_1[9]    = rotation field 1 (offset 36)
    // param_1[10]   = rotation field 2 (offset 40)
    // param_1[11]   = rotation field 3 (offset 44)
    // param_1[12]   = physics (offset 48, byte)

    ServerConnection_addMove(this,
        *param_1,           // entityId
        param_1[1],         // spaceId
        param_1[2],         // vehicleId
        (undefined8*)(param_1 + 3),  // position ptr
        (float*)(param_1 + 6),       // velocity ptr
        (float)param_1[0xb],  // <<< rotY passed as 3rd rotation arg
        (float)param_1[10],   // <<< rotZ passed as 2nd rotation arg
        (float)param_1[9],    // <<< rotX passed as 1st rotation arg
        (char)param_1[0xc],   // physics
        ...);
}
```

### Wire Rotation Order Confirmed

The `addMove` signature expects `(rotY, rotZ, rotX)`, and it receives:
- `param_1[11]` as rotY (wire offset 44)
- `param_1[10]` as rotZ (wire offset 40)
- `param_1[9]`  as rotX (wire offset 36)

Therefore the **wire order at offsets 36-44 is: rotX, rotZ, rotY** (Y and Z are swapped compared to the logical yaw/pitch/roll order).

This matches the C++ server source in `client_handler.cpp` which writes: `rotX << rotZ << rotY`.

---

## CREATE_BASE_PLAYER (0x05) -- Stream Read Details

**Handler**: `ServerConnection_createBasePlayer` @ `0x00dddca0`
**Length type**: WORD_LENGTH

### Assembly Evidence for classId Size

```asm
; 0x00dddcea: read classId
PUSH 0x2            ; request 2 bytes
MOV ECX, ESI        ; stream
CALL EAX            ; stream.read(2)
MOVZX EAX, word ptr [EAX]  ; read as uint16

; 0x00dddcfc: passed to handler
PUSH ESI            ; stream (remaining = properties)
PUSH EAX            ; classId (u16)
MOV EAX, [EDI+0x16c]
PUSH EAX            ; entityId
CALL EDX            ; handler->onCreateBasePlayer(entityId, classId, stream)
```

The `classId` is read as **2 bytes (u16)** from the stream, even though the server writes it as `(uint8_t)classDef()->index()` followed by `(uint8_t)0`. The client reads both bytes as a single u16, where the high byte (property count) is part of the value passed to the handler. The handler callback separates them internally.

---

## TICK_SYNC (0x0D) and SET_GAME_TIME (0x03) -- RTTI Evidence

These messages use templated `ClientMessageHandler` classes. The handler functions are not standalone named functions but are inlined into the message dispatch system.

### RTTI Strings

| Message | RTTI String | Address |
|---------|-------------|---------|
| tickSync | `.?AV?$ClientMessageHandler@UtickSyncArgs@ClientInterface@@@@` | `0x01e52270` |
| setGameTime | `.?AV?$ClientMessageHandler@UsetGameTimeArgs@ClientInterface@@@@` | `0x01e52138` |
| resetEntities | `.?AV?$ClientMessageHandler@UresetEntitiesArgs@ClientInterface@@@@` | `0x01e52180` |

### Registration Name Strings

| Message | Name String | Address | Data Xref |
|---------|-------------|---------|-----------|
| resetEntities | `"resetEntities"` | `0x019d09f8` | `0x017bb210` |
| tickSync | `"tickSync"` | `0x019d0a8c` | `0x017bb720` |
| setGameTime | `"setGameTime"` | `0x019d09ec` | `0x017bb180` |

The data xref addresses (0x017bbXXX) are within static initialization data in the `.rdata` section, not within functions. These are part of BigWorld's `InterfaceMinder` message registration system, where each message descriptor contains: `{name_ptr, handler_vtable_ptr, length_type, length, flags}`.

Since these are simple CONSTANT_LENGTH messages with trivial arg structs, their handlers are generated by the `ClientMessageHandler<T>` template and consist of direct struct copies without complex logic.

---

## enableEntitiesRejected -- CME Custom Message

**Handler**: `Detail_EnableEntitiesRejectedMessage__vfunc_0` @ `0x01561da0`
**RTTI**: `.?AVEnableEntitiesRejectedMessage@Detail@@` at `0x01e91210`

### Decompiled Code

```c
undefined4* __thiscall
Detail_EnableEntitiesRejectedMessage__vfunc_0(void *this, byte param_1)
{
    *(undefined***)this = ISGWMessage::vftable;  // Set vtable
    if ((param_1 & 1) != 0) {
        scalable_free(this);  // Destructor with dealloc
    }
    return (undefined4*)this;
}
```

This is only the destructor (vfunc_0). The class inherits from `ISGWMessage`, which is part of the CME (Cheyenne Mountain Entertainment) custom message framework layered on top of BigWorld's Mercury protocol. The actual rejection handling logic is dispatched through the CME event system rather than through a standalone Mercury message handler.

---

## startEntityMessage / startProxyMessage -- Client-to-Server Entity RPC

These are the client-side functions for **sending** entity method calls to the server.

### startEntityMessage (cell methods, 0x80+ range)

**Function**: `ServerConnection_startEntityMessage` @ `0x00dd6a60`

```c
void __thiscall ServerConnection_startEntityMessage(void *this, uint32 param_1)
{
    if (*(int*)(this + 0x30c) == 0) {
        ASSERT("Called when not connected to server!");
    }

    // Initialize message descriptor (lazy, once)
    static MessageDesc entityMsg;
    if (!initialized) {
        entityMsg = *DAT_01ef2514;  // copy base message descriptor
    }

    // Set msg_id = methodIndex | 0x80
    entityMsg.id = (param_1 & 0xFF) | 0x80;

    // Write to channel
    void *channel = Mercury_Channel_2(this);
    channel->writeHeader(entityMsg);

    // Write entity ID
    *(uint32*)channel->reserve(4) = param_1;

    // Channel remains open for caller to write method arguments
    Mercury_Channel_2(this);
}
```

### startProxyMessage (base methods, 0xC0+ range)

**Function**: `ServerConnection_startProxyMessage` @ `0x00dd6980`

```c
void __thiscall ServerConnection_startProxyMessage(void *this, uint8 param_1)
{
    if (*(int*)(this + 0x30c) == 0) {
        ASSERT("Called when not connected to server!");
    }

    static MessageDesc proxyMsg;
    if (!initialized) {
        proxyMsg = *DAT_01ef2514;
    }

    // Set msg_id = methodIndex | 0xC0
    proxyMsg.id = param_1 | 0xC0;

    void *channel = Mercury_Channel_2(this);
    channel->writeHeader(proxyMsg);
    // Note: NO entity ID written (proxy messages target the player's own base entity)
    Mercury_Channel_2(this);
}
```

**Key difference**: Entity messages (cell) write a 4-byte entity ID after the header; proxy messages (base) do NOT write an entity ID (they implicitly target the player's own proxy/base entity).

---

## ServerConnection Field Map (Partial)

From the decompiled handlers, the following `ServerConnection` object offsets are used:

| Offset | Type | Field | Notes |
|--------|------|-------|-------|
| 0x008 | std::string | sessionKey | SSO string (inline if <= 15 chars) |
| 0x018 | uint32 | sessionKeyLen | String length |
| 0x01c | uint32 | sessionKeyCapacity | String capacity |
| 0x168 | void* | handler_ | Callback handler object (nullable) |
| 0x16c | uint32 | playerEntityID_ | Current player entity ID (0 if none) |
| 0x30c | void* | connection_ | Active connection object (nullable) |
| 0x316 | bool | entitiesEnabled_ | Entity system active flag |
| 0x317 | bool | sentDisconnect_ | Whether disconnect msg was sent |
| 0x36c | uint8 | sendIndex_ | Circular buffer write index |
| 0x36e | bool | hasPendingMove_ | Pending movement flag |
| 0x370 | bool | sameSpace_ | Same-space optimization flag |
| 0xf84 | LinkedList | viewportEntities_ | Entity-to-viewport mapping |
| 0xf90 | Map | spaceFollowMap_ | Space chain follow map |
| 0xf9c | Map | spaceRefCounts_ | Viewport ref-count per space |
| 0xfa8 | uint8 | activeViewport_ | Current active viewport ID |
| 0xfac | Map | entityPhysics_ | Per-entity physics mode |
| 0xfb0 | uint32 | entityPhysicsEnd_ | End sentinel for physics map |
| 0xfd0 | vector | resourceRequests_ | Active resource request list |
| 0xfdc | void* | cellPlayerBuffer_ | Buffered createCellPlayer stream |
| 0xfe0 | Stream | createCellPlayerMsg_ | Buffered cell player message |
| 0xfe8 | uint32 | streamBegin_ | Stream buffer start marker |

---

## Related Documents

- [`space-viewport-wire-formats.md`](space-viewport-wire-formats.md) -- Complete wire format tables for all 0x00-0x37 messages
- [`entity-property-sync.md`](entity-property-sync.md) -- Property sync, method IDs
- [`entity-types-wire-formats.md`](entity-types-wire-formats.md) -- Account, SGWEntity types
- [`../../protocol/message-catalog.md`](../../protocol/message-catalog.md) -- Full 975-message catalog
