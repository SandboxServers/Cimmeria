# Combat System Wire Formats

> **Date**: 2026-03-01
> **Confidence**: HIGH (3+ sources agree on all formats)
> **Sources**: Ghidra decompilation of `sgw.exe`, `.def` files, `alias.xml`, Python client scripts

---

## Architecture: Universal RPC Dispatcher

### Finding

All outgoing entity method calls from the client are routed through a **single universal RPC dispatcher** at `0x00c6fc40`. There are no per-method serialization functions — the wire format is entirely driven by the `.def` file method signatures and BigWorld's `DataType::addToStream` virtual method.

This means combat wire formats can be derived directly from `.def` files + `alias.xml` type definitions, without decompiling individual event handlers.

### Evidence

**Dispatcher at `0x00c6fc40`** (decompiled, simplified):

```c
void universal_rpc_dispatch(void *stream, void *args, void *methodDesc) {
    int methodType = *(int*)(methodDesc + 0x1c) & 3;

    if (methodType == 2) {
        // Base/proxy method
        ServerConnection_startProxyMessage(conn, methodId);   // header = methodId | 0xC0
    } else {
        // Cell method (type 1 or 3)
        ServerConnection_startEntityMessage(conn, methodId);  // header = methodId | 0x80
    }

    // Serialize arguments in order from .def
    int argCount = *(int*)(methodDesc + 0x24);  // vector size
    for (int i = 0; i < argCount; i++) {
        DataType *dt = argTypes[i];
        dt->addToStream(stream, argValue);  // vtable offset 0x20
    }
}
```

**Key addresses**:
| Function | Address | Role |
|----------|---------|------|
| Universal RPC dispatcher | `0x00c6fc40` | Routes all entity method calls |
| `ServerConnection_startEntityMessage` | `0x00dd6a60` | Writes cell method header (`methodId \| 0x80`) |
| `ServerConnection_startProxyMessage` | `0x00dd6980` | Writes base method header (`methodId \| 0xC0`) |

### Message Header Format

```
Cell method:   [1 byte: methodID | 0x80]  [args...]
Base method:   [1 byte: methodID | 0xC0]  [args...]
```

Method IDs are assigned sequentially per category (CellMethods, ClientMethods, BaseMethods), following the entity description parse order:

```
Parent (recursive) → Implements (in declaration order) → Own methods
```

For server→client messages, the same pattern applies: the server sends a method ID + serialized args, and the client dispatches to the matching ClientMethod handler.

---

## BigWorld DataType Wire Encodings

All argument types serialize using standard BigWorld `DataType::addToStream`. These encodings are confirmed from the BigWorld 2.0.1 engine source (`src/lib/entitydef/data_types.cpp`).

| Type | Wire Size | Encoding |
|------|-----------|----------|
| `INT8` | 1 byte | Signed, raw |
| `UINT8` | 1 byte | Unsigned, raw |
| `INT16` | 2 bytes | Signed, little-endian |
| `UINT16` | 2 bytes | Unsigned, little-endian |
| `INT32` | 4 bytes | Signed, little-endian |
| `UINT32` | 4 bytes | Unsigned, little-endian |
| `INT64` | 8 bytes | Signed, little-endian |
| `UINT64` | 8 bytes | Unsigned, little-endian |
| `FLOAT` / `FLOAT32` | 4 bytes | IEEE 754, little-endian |
| `FLOAT64` | 8 bytes | IEEE 754, little-endian |
| `STRING` | 4 + N bytes | `uint32 length` + UTF-8 data |
| `WSTRING` | 4 + 2N bytes | `uint32 length` (in chars) + UTF-16LE data |
| `VECTOR3` | 12 bytes | 3 x `float32` (x, y, z) |
| `MAILBOX` | N/A | Not sent on wire (server-only) |
| `PYTHON` | N/A | Not sent on wire from client |
| `ARRAY<of>T</of>` | 4 + N*sizeof(T) | `uint32 count` + elements |
| `FIXED_DICT` | sum of fields | Fields serialized in declaration order, no count prefix |

---

## Combat Messages: Client → Server

These are `<Exposed/>` CellMethods that the client can invoke. They travel as cell method calls through the universal RPC dispatcher.

### useAbility

**Source**: `SGWPlayer.def` CellMethods (Exposed)

```xml
<useAbility>
    <Exposed/>
    <Arg> INT32 <ArgName>AbilityID</ArgName></Arg>
    <Arg> INT32 <ArgName>TargetID</ArgName></Arg>
</useAbility>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 4 | int32 | AbilityID | Ability definition ID |
| 5 | 4 | int32 | TargetID | Target entity ID (0 if no target) |

**Total**: 9 bytes

**Discrepancy resolved**: Python `AbilityManager.py` shows `useAbility(abilityId, targetId, targetLoc)` with 3 parameters, but the `.def` file only declares 2 args. The `targetLoc` parameter has a default value of `None` in the Python cell handler and is **not sent on the wire** — it is populated server-side from the target entity's position. Ground targeting uses the separate `useAbilityOnGroundTarget` method below.

---

### useAbilityOnGroundTarget

**Source**: `SGWPlayer.def` CellMethods (Exposed)

```xml
<useAbilityOnGroundTarget>
    <Exposed/>
    <Arg> INT32 <ArgName>AbilityID</ArgName></Arg>
    <Arg> FLOAT <ArgName>LocationX</ArgName></Arg>
    <Arg> FLOAT <ArgName>LocationY</ArgName></Arg>
    <Arg> FLOAT <ArgName>LocationZ</ArgName></Arg>
</useAbilityOnGroundTarget>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 4 | int32 | AbilityID | Ability definition ID |
| 5 | 4 | float32 | LocationX | Target X coordinate |
| 9 | 4 | float32 | LocationY | Target Y coordinate |
| 13 | 4 | float32 | LocationZ | Target Z coordinate |

**Total**: 17 bytes

---

### setAutoCycle

**Source**: `SGWPlayer.def` CellMethods (Exposed)

```xml
<setAutoCycle>
    <Exposed/>
    <Arg> INT8 <ArgName>enabled</ArgName></Arg>
</setAutoCycle>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 1 | int8 | enabled | 0 = disable, 1 = enable auto-cycle |

**Total**: 2 bytes

---

### setCrouched

**Source**: `SGWCombatant.def` CellMethods (Exposed)

```xml
<setCrouched>
    <Exposed/>
    <Arg> INT8 <ArgName>enabled</ArgName></Arg>
</setCrouched>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 1 | int8 | enabled | 0 = standing, 1 = crouched |

**Total**: 2 bytes

---

### requestHolsterWeapon

**Source**: `SGWCombatant.def` CellMethods (Exposed)

```xml
<requestHolsterWeapon>
    <Exposed/>
    <Arg> INT8 <ArgName>aHolster</ArgName></Arg>
</requestHolsterWeapon>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 1 | int8 | aHolster | 0 = draw weapon, 1 = holster weapon |

**Total**: 2 bytes

---

### setTargetID

**Source**: `SGWBeing.def` CellMethods (Exposed)

```xml
<setTargetID>
    <Exposed/>
    <Arg> INT32 <ArgName>aTargetID</ArgName></Arg>
</setTargetID>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 4 | int32 | aTargetID | Entity ID of new target (0 to clear) |

**Total**: 5 bytes

---

### setMovementType

**Source**: `SGWBeing.def` CellMethods (Exposed)

```xml
<setMovementType>
    <Exposed/>
    <Arg> UINT8 <ArgName>aMovementType</ArgName></Arg>
</setMovementType>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 1 | uint8 | aMovementType | Movement state enum |

**Total**: 2 bytes

---

### requestReload

**Source**: `SGWPlayer.def` CellMethods (Exposed)

```xml
<requestReload>
    <Exposed/>
    <Arg> UINT8 <ArgName>aReloadType</ArgName></Arg>
</requestReload>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 1 | uint8 | aReloadType | Reload type enum |

**Total**: 2 bytes

---

### respawn

**Source**: `SGWPlayer.def` CellMethods (Exposed)

```xml
<respawn>
    <Exposed/>
</respawn>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |

**Total**: 1 byte (no args)

---

### callForAid

**Source**: `SGWPlayer.def` CellMethods (Exposed)

```xml
<callForAid>
    <Exposed/>
    <Arg> INT32 <ArgName>respawnerID</ArgName></Arg>
</callForAid>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |
| 1 | 4 | int32 | respawnerID | ID of respawner to call aid from |

**Total**: 5 bytes

---

### toggleHealDebug

**Source**: `SGWCombatant.def` CellMethods (Exposed)

```xml
<toggleHealDebug>
    <Exposed/>
</toggleHealDebug>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | Header | `methodID \| 0x80` |

**Total**: 1 byte (no args, debug command)

---

## Combat Messages: Server → Client

These are ClientMethods sent from the server. The server serializes them identically — method ID + args in `.def` order.

### onEffectResults

**Source**: `SGWBeing.def` ClientMethods

```xml
<onEffectResults>
    <Arg> INT32                  <ArgName>SourceID</ArgName></Arg>
    <Arg> INT32                  <ArgName>AbilityID</ArgName></Arg>
    <Arg> INT32                  <ArgName>EffectID</ArgName></Arg>
    <Arg> INT32                  <ArgName>TargetID</ArgName></Arg>
    <Arg> UINT8                  <ArgName>ResultCode</ArgName></Arg>
    <Arg> ClientEffectResultList <ArgName>ClientEffectResultList</ArgName></Arg>
</onEffectResults>
```

**`ClientEffectResultList`** = `ARRAY<of>ClientEffectResult</of>` (from `alias.xml`)

**`ClientEffectResult`** = `FIXED_DICT` with fields:

| Field | Type | Size |
|-------|------|------|
| StatID | INT8 | 1 byte |
| Delta | INT32 | 4 bytes |
| DamageCode | INT8 | 1 byte |
| StatResultCode | INT8 | 1 byte |

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | SourceID | Entity ID of ability invoker |
| 4 | 4 | int32 | AbilityID | Ability definition ID |
| 8 | 4 | int32 | EffectID | Effect definition ID |
| 12 | 4 | int32 | TargetID | Entity ID of target |
| 16 | 1 | uint8 | ResultCode | Overall result code |
| 17 | 4 | uint32 | ListCount | Number of stat results |
| 21 | 7*N | ClientEffectResult[] | StatResults | Per-stat damage/heal entries |

**Per ClientEffectResult entry** (7 bytes each):

| Offset | Size | Type | Field |
|--------|------|------|-------|
| +0 | 1 | int8 | StatID |
| +1 | 4 | int32 | Delta |
| +5 | 1 | int8 | DamageCode |
| +6 | 1 | int8 | StatResultCode |

**Total**: 21 + 7*N bytes (N = number of stat results)

**Python validation**: `python/client/SGWBeing.py` `onEffectResults(invokerId, abilityId, effectId, targetId, resultCode, statResultList)` — matches exactly. Each entry in `statResultList` has `StatID`, `Delta`, `DamageCode`, `StatResultCode`.

---

### onStatUpdate

**Source**: `SGWCombatant.def` ClientMethods

```xml
<onStatUpdate>
    <Arg> StatUpdateList <ArgName>Stats</ArgName></Arg>
</onStatUpdate>
```

**`StatUpdateList`** = `ARRAY<of>StatUpdate</of>` (from `alias.xml`)

**`StatUpdate`** = `FIXED_DICT` with fields:

| Field | Type | Size |
|-------|------|------|
| StatId | INT32 | 4 bytes |
| Min | INT32 | 4 bytes |
| Current | INT32 | 4 bytes |
| Max | INT32 | 4 bytes |

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | uint32 | Count | Number of stat updates |
| 4 | 16*N | StatUpdate[] | Stats | Updated stat entries |

**Per StatUpdate entry** (16 bytes each):

| Offset | Size | Type | Field |
|--------|------|------|-------|
| +0 | 4 | int32 | StatId |
| +4 | 4 | int32 | Min |
| +8 | 4 | int32 | Current |
| +12 | 4 | int32 | Max |

**Total**: 4 + 16*N bytes

**Python validation**: `python/client/SGWBeing.py` `onStatUpdate(stats)` iterates stat list with keys `StatId`, `Min`, `Current`, `Max` — exact match.

**Note**: `StatId` corresponds to the index of the stat in the `StatList` FIXED_DICT (defined in `alias.xml`). There are 70+ stats defined, from `coordination` (index 0) through `absorbUntypedEnergy` (index ~69).

---

### onStatBaseUpdate

**Source**: `SGWCombatant.def` ClientMethods

```xml
<onStatBaseUpdate>
    <Arg> StatUpdateList <ArgName>Stats</ArgName></Arg>
</onStatBaseUpdate>
```

**Wire format**: Identical to `onStatUpdate` above. The difference is semantic — `onStatBaseUpdate` updates the base (unbuffed) stat values, while `onStatUpdate` updates the current (buffed) values.

**Python validation**: `python/client/SGWBeing.py` `onStatBaseUpdate(stats)` — same format as `onStatUpdate`.

---

### onTimerUpdate

**Source**: `SGWBeing.def` ClientMethods

```xml
<onTimerUpdate>
    <Arg> INT32 <ArgName>ID</ArgName></Arg>
    <Arg> INT8  <ArgName>Type</ArgName></Arg>
    <Arg> INT32 <ArgName>SourceID</ArgName></Arg>
    <Arg> INT32 <ArgName>SecondaryId</ArgName></Arg>
    <Arg> FLOAT <ArgName>TotalTime</ArgName></Arg>
    <Arg> FLOAT <ArgName>BigWorldTimeComplete</ArgName></Arg>
</onTimerUpdate>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | ID | Timer/ability/effect ID |
| 4 | 1 | int8 | Type | Timer category (see below) |
| 5 | 4 | int32 | SourceID | Entity that caused the timer |
| 9 | 4 | int32 | SecondaryId | Secondary identifier (unused, always 0) |
| 13 | 4 | float32 | TotalTime | Total duration in seconds |
| 17 | 4 | float32 | BigWorldTimeComplete | BigWorld timestamp when timer expires |

**Total**: 21 bytes

**Timer categories** (from `python/client/SGWBeing.py`):

| Type Value | Category | ID Field Meaning |
|------------|----------|------------------|
| 0 | `AbilityCooldown` | Ability ID |
| 1 | `CategoryCooldown` | Category ID |
| 2 | `AbilityWarmup` | Ability ID |
| 3 | `DurationEffect` | Effect ID |

**Python validation**: `python/client/SGWBeing.py` `onTimerUpdate(id, timerCategory, entityId, 0, duration, endTime)` — matches. The 4th parameter `SecondaryId` is always passed as 0 in the Python caller.

---

### onMeleeRangeUpdate

**Source**: `SGWCombatant.def` ClientMethods

```xml
<onMeleeRangeUpdate>
    <Arg> INT32 <ArgName>range</ArgName></Arg>
</onMeleeRangeUpdate>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | range | Melee range in game units |

**Total**: 4 bytes

---

### onArchetypeUpdate

**Source**: `SGWCombatant.def` ClientMethods

```xml
<onArchetypeUpdate>
    <Arg> INT32 <ArgName>archetype</ArgName></Arg>
</onArchetypeUpdate>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | archetype | Archetype enum ID |

**Total**: 4 bytes

---

### onAlignmentUpdate / onFactionUpdate

**Source**: `SGWCombatant.def` ClientMethods

```xml
<onAlignmentUpdate>
    <Arg> INT8 <ArgName>alignment</ArgName></Arg>
</onAlignmentUpdate>
<onFactionUpdate>
    <Arg> INT8 <ArgName>faction</ArgName></Arg>
</onFactionUpdate>
```

**Wire format** (same structure for both):

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | int8 | alignment/faction | Alignment or faction enum value |

**Total**: 1 byte each

---

### onLevelUpdate

**Source**: `SGWBeing.def` ClientMethods

```xml
<onLevelUpdate>
    <Arg> INT32 <ArgName>Level</ArgName></Arg>
</onLevelUpdate>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | Level | New level |

**Total**: 4 bytes

---

### onTargetUpdate

**Source**: `SGWBeing.def` ClientMethods

```xml
<onTargetUpdate>
    <Arg> INT32 <ArgName>TargetId</ArgName></Arg>
</onTargetUpdate>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | TargetId | Entity ID of entity's current target |

**Total**: 4 bytes

---

### onTopSpeedUpdate

**Source**: `SGWBeing.def` ClientMethods

```xml
<onTopSpeedUpdate>
    <Arg> FLOAT <ArgName>TopSpeed</ArgName></Arg>
</onTopSpeedUpdate>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | float32 | TopSpeed | Maximum movement speed |

**Total**: 4 bytes

---

### onStateFieldUpdate

**Source**: `SGWBeing.def` ClientMethods

```xml
<onStateFieldUpdate>
    <Arg> INT32 <ArgName>bStateField</ArgName></Arg>
</onStateFieldUpdate>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | bStateField | Bitfield of being states (stunned, rooted, etc.) |

**Total**: 4 bytes

---

### onEffectUserData

**Source**: `SGWBeing.def` ClientMethods

```xml
<onEffectUserData>
    <Arg> INT32                <ArgName>InstanceId</ArgName></Arg>
    <Arg> ARRAY<of>WSTRING</of> <ArgName>UserDataNames</ArgName></Arg>
    <Arg> ARRAY<of>WSTRING</of> <ArgName>UserDataValues</ArgName></Arg>
</onEffectUserData>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | InstanceId | Effect instance ID |
| 4 | 4 | uint32 | NameCount | Number of user data names |
| 8 | var | WSTRING[] | UserDataNames | Array of key names |
| var | 4 | uint32 | ValueCount | Number of user data values |
| var | var | WSTRING[] | UserDataValues | Array of key values |

**Total**: Variable (depends on string lengths)

---

### onBeginAidWait

**Source**: `SGWPlayer.def` ClientMethods

```xml
<onBeginAidWait>
    <Arg> INT32                         <ArgName>TimeToAid</ArgName></Arg>
    <Arg> ARRAY <of> Respawner </of>    <ArgName>respawners</ArgName></Arg>
</onBeginAidWait>
```

**`Respawner`** = `FIXED_DICT` (from `alias.xml`):

| Field | Type | Size |
|-------|------|------|
| respawnerID | INT32 | 4 bytes |
| respawnerName | WSTRING | 4 + 2*len bytes |

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | int32 | TimeToAid | Seconds until auto-respawn |
| 4 | 4 | uint32 | Count | Number of respawners |
| 8 | var | Respawner[] | respawners | Available respawn locations |

**Total**: Variable

---

### onKnownAbilitiesUpdate

**Source**: `SGWPlayer.def` ClientMethods

```xml
<onKnownAbilitiesUpdate>
    <Arg> ARRAY<of>INT32</of> <ArgName>AbilityData</ArgName></Arg>
</onKnownAbilitiesUpdate>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | uint32 | Count | Number of ability IDs |
| 4 | 4*N | int32[] | AbilityData | List of known ability IDs |

**Total**: 4 + 4*N bytes

---

### onErrorCode

**Source**: `SGWPlayer.def` ClientMethods

```xml
<onErrorCode>
    <Arg> UINT8  <ArgName>SystemID</ArgName></Arg>
    <Arg> INT32  <ArgName>InstanceID</ArgName></Arg>
    <Arg> UINT16 <ArgName>ErrorCodeID</ArgName></Arg>
</onErrorCode>
```

**Wire format**:

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 1 | uint8 | SystemID | `EErrorCodeSystem` enum |
| 1 | 4 | int32 | InstanceID | Subsystem instance (e.g. ability ID) |
| 5 | 2 | uint16 | ErrorCodeID | `EConditionHandlerFeedback` enum |

**Total**: 7 bytes

Used for combat error feedback: ability not ready, out of range, insufficient focus, etc.

---

## Cross-Validation Summary

| Message | .def | alias.xml | Python Client | BW Source | Confidence |
|---------|------|-----------|---------------|-----------|------------|
| useAbility | Y | — | Y (discrepancy resolved) | Y | HIGH |
| useAbilityOnGroundTarget | Y | — | Y | Y | HIGH |
| setAutoCycle | Y | — | Y | Y | HIGH |
| setCrouched | Y | — | Y | Y | HIGH |
| onEffectResults | Y | Y (ClientEffectResult) | Y | Y | HIGH |
| onStatUpdate | Y | Y (StatUpdate) | Y | Y | HIGH |
| onStatBaseUpdate | Y | Y (StatUpdate) | Y | Y | HIGH |
| onTimerUpdate | Y | — | Y | Y | HIGH |
| onMeleeRangeUpdate | Y | — | — | Y | HIGH |
| onArchetypeUpdate | Y | — | — | Y | HIGH |
| onAlignmentUpdate | Y | — | — | Y | HIGH |
| onFactionUpdate | Y | — | — | Y | HIGH |
| onLevelUpdate | Y | — | Y | Y | HIGH |
| onTargetUpdate | Y | — | Y | Y | HIGH |
| onStateFieldUpdate | Y | — | Y | Y | HIGH |

## Discrepancies Found

### 1. useAbility Parameter Count (RESOLVED)

**Python** (`AbilityManager.py`): `useAbility(abilityId, targetId, targetLoc)` — 3 parameters

**.def file**: `useAbility(INT32 AbilityID, INT32 TargetID)` — 2 parameters

**Resolution**: The `targetLoc` parameter defaults to `None` in the Python cell handler and is populated server-side from the target's position. It is NOT sent on the wire. Ground-targeted abilities use the separate `useAbilityOnGroundTarget` method with explicit XYZ coordinates.

### 2. CME EventSignal System Misidentification

The Phase 2 plan assumed the `Event_NetOut_*` / `Event_NetIn_*` functions were network serialization handlers. They are actually the **CME (Cimmeria Media Engine) EventSignal system** — a client-side UI event bus that routes events between UI/game subsystems. The actual network serialization goes through the universal RPC dispatcher at `0x00c6fc40`.

Functions like `register_NetOut_UseAbility` at `0x00cb7d90` simply return the string `"Event_NetOut_UseAbility"` — they register a named signal, not a network handler. The `SGWNetworkManager` subscribes to these signals and routes them through the universal dispatcher.

## Related Documents

- [Entity Property Sync Protocol](../../protocol/entity-property-sync.md) — Property sync details
- [Mercury Wire Format](../../protocol/mercury-wire-format.md) — Underlying packet protocol
- [Message Catalog](../../protocol/message-catalog.md) — Full event registry
- [Combat System](../../gameplay/combat-system.md) — Gameplay mechanics

## Implementation Impact

For the Cimmeria server emulator:

1. **No per-method serialization needed** — Use the `.def` file method signatures to auto-generate serialization code, or use BigWorld's built-in `DataType::addToStream` / `createFromStream` pattern.

2. **Method ID assignment must match** — The server must assign method IDs in the exact same order as the client's `EntityDescription` parser: Parent (recursive) → Implements (declaration order) → own methods. A mismatch causes method calls to dispatch to the wrong handler.

3. **FIXED_DICT field order matters** — Fields serialize in XML declaration order with no delimiters. `ClientEffectResult` is `StatID, Delta, DamageCode, StatResultCode` — reordering causes deserialization failure.

4. **ARRAY prefix is always uint32** — Even for small arrays, the count prefix is 4 bytes. No compact encoding for small counts.

5. **Combat flow**: Client sends `useAbility(abilityId, targetId)` → Server processes damage → Server sends `onEffectResults(sourceId, abilityId, effectId, targetId, resultCode, statResults[])` + `onStatUpdate(stats[])` + `onTimerUpdate(...)` for cooldowns.
