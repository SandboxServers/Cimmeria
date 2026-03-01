# BigWorld Technology Version Analysis

Forensic identification of the BigWorld Technology version embedded in sgw.exe, based on string fingerprinting, RTTI analysis, and cross-referencing against open-source BigWorld releases.

## Conclusion

**BigWorld ~1.9.x (late 1.9, post-1.9.1) with CME custom modifications**

The binary contains features present in BigWorld 2.0.1 but absent from 1.9.1, AND features present in neither public release — indicating a custom licensee build from a BigWorld version between 1.9.1 (2008) and 2.0.1 (2012), consistent with SGW's 2007-2009 active development period.

## Reference Sources

| Version | Year | Source |
|---------|------|--------|
| BigWorld 1.9.1 | 2008 | [v2v3v4/BigWorld-Engine-1.9.1](https://github.com/v2v3v4/BigWorld-Engine-1.9.1) |
| BigWorld 2.0.1 | 2012 | [v2v3v4/BigWorld-Engine-2.0.1](https://github.com/v2v3v4/BigWorld-Engine-2.0.1) |
| BigWorld 14.4.1 (OSE) | 2014 | [v2v3v4/BigWorld-Engine-14.4.1](https://github.com/v2v3v4/BigWorld-Engine-14.4.1) |

## Feature Comparison Matrix

| Feature | sgw.exe | BW 1.9.1 | BW 2.0.1 | Interpretation |
|---------|---------|----------|----------|----------------|
| 32 avatarUpdate variants | YES | YES | YES | Baseline (1.8+) |
| Ward variants (Implicit/Explicit) | YES | YES | YES | Baseline (1.8+) |
| tickSync | YES | YES | YES | Baseline |
| voiceData | YES | YES | YES | Baseline (1.7+) |
| restoreClient | YES | YES | YES | Baseline |
| resourceFragment | YES | YES | YES | Baseline |
| MF_ASSERT system | YES | YES | YES | Baseline |
| **FixedDictDataType** | **YES** | **NO** | **YES** | **Post-1.9.1 feature** |
| **UserDataType** (implementedBy) | **YES** | **NO** | **YES** | **Post-1.9.1 feature** |
| **spaceViewportInfo** | **YES** | **NO** | **NO** | **CME custom or private BW branch** |
| **entityInvisible** | **YES** | **NO** | **NO** | **CME custom or private BW branch** |
| **setSpaceViewport** | **YES** | **NO** | **NO** | **CME custom or private BW branch** |
| switchBaseApp | NO | NO | YES | Missing — confirms pre-2.0.1 |

## Evidence Details

### 1. Build Paths (3 files)

BigWorld library source was compiled from CME's build system:

| Address | Build Path |
|---------|-----------|
| 0x019ce768 | `c:\build\qa\sgw\server\src\lib\cstdmf\memory_stream.ipp` |
| 0x019d0f98 | `c:\BUILD\QA\SGW\Server\src\lib\math/sma.hpp` |
| 0x019d1118 | `c:\BUILD\QA\SGW\Server\src\lib\cstdmf/concurrency.hpp` |

The path `c:\build\qa\sgw\server\src\lib\` matches BigWorld's standard `src/lib/` directory layout. CME compiled BigWorld from source (as expected for a licensee).

### 2. Source Files Referenced (2 BigWorld .cpp files)

| Address | Source File | Assertion Count |
|---------|-----------|----------------|
| 0x019cd8e0 | `..\..\..\..\Server\bigworld\src\client\entity_manager.cpp` | 9 |
| 0x019cec60 | `..\..\..\..\Server\bigworld\src\common\servconn.cpp` | 16 |

Both files exist in BW 1.9.1 and 2.0.1. The relative path `..\..\..\..\Server\bigworld\` confirms the source was located at `Server\bigworld\` relative to the SGW game source tree.

### 3. Assertion Strings from BigWorld Code

#### entity_manager.cpp assertions (9 sites)

| Address | Assertion Expression |
|---------|---------------------|
| 0x019cdf80 | `vehicleID == 0 && "Client only entities should not start already aboard a vehicle."` |
| 0x019cd8e0–0x019ce580 | 8 additional assertion sites (standalone file references) |

#### servconn.cpp assertions (16 sites)

| Address | Assertion Expression |
|---------|---------------------|
| 0x019cec60 | `0 == mPendingAttempts` |
| 0x019cf2a0 | `replyHandler.getObject() != NULL` |
| 0x019cf438 | `replyHandler->status() != LogOnStatus::LOGGED_ON` |
| 0x019cf5b8 | `pChannel_` |
| 0x019cf620 | `pChannel_` |
| 0x019cfef8 | `sentPhysics_[ args.id ] == args.physics` |
| 0x019d0080 | `entitiesEnabled_` |
| 0x019d01d0 | `createCellPlayerMsg_.remainingLength() == 0` |
| 0x019d0408 | `id != ObjectID( -1 )` |
| 0x019d05c8 | `mosPtr == mosPtrBeg` |
| 0x019d06e0 | `this->findRURequest( args.rid, true ) == rur` |
| 0x019ced04–0x019cf14c | 5 additional assertion sites |

#### memory_stream.ipp assertions (4 sites)

| Address | Assertion Expression |
|---------|---------------------|
| 0x019ce768 | `aPos <= size() && aPos >= 0 && (pointer + aBytesNeeded) <= pEnd_` |
| 0x019ce800 | `shouldDelete_` |
| 0x019ce868 | `pRead_ + nBytes <= pCurr_` (MF_ASSERT_DEV) |
| 0x019ce8e0 | `pRead_ < pCurr_` (MF_ASSERT_DEV) |

### 4. ServerConnection Debug Strings (22 methods)

Complete roster of ServerConnection methods with debug logging:

| Address | Debug String |
|---------|-------------|
| 0x019cf1f8 | `ServerConnection::logOnBegin: server:%s username:%s protocol_digest: %s` |
| 0x019cf310 | `ServerConnection::logOn: status==LOGGED_ON` |
| 0x019cf33c | `ServerConnection::logOn: from: %s` |
| 0x019cf388 | `ServerConnection::logOnComplete: BaseApp address on login reply (%s) differs from winning BaseApp reply (%s)` |
| 0x019cf3f8 | `ServerConnection::logOnComplete: Logon failed (%s)` |
| 0x019cf4b8 | `ServerConnection::logOnComplete: Logon failed` |
| 0x019cf548 | `ServerConnection::enableEntities: Enabling entities %d` |
| 0x019cf680 | `ServerConnection::addMove: Tried to add a move for unseen entity id %d` |
| 0x019cf6c8 | `ServerConnection::addMove: Tried to add a move for entity id %d that we do not control` |
| 0x019cfc40 | `ServerConnection::startProxyMessage: Called when not connected to server!` |
| 0x019cfc90 | `ServerConnection::startEntityMessage: Called when not connected to server!` |
| 0x019cfcdc | `ServerConnection::processInput: Got a nub exception (%s)` |
| 0x019cfd18 | `ServerConnection::processInput: Dropped corrupted incoming packet` |
| 0x019cfd60 | `ServerConnection::processInput: Disconnecting due to nub exception (%s)` |
| 0x019cfdb0 | `ServerConnection::processInput: There were %d ms between packets` |
| 0x019cfdf8 | `ServerConnection::svidFollow: Viewport for entity %d (immed vehicle %d, top vehicle %d) is unknown` |
| 0x019cfe5c | `ServerConnection::send: calling Nub::send` |
| 0x019cfe88 | `ServerConnection::forcedPosition: Received forced position for entity %d that we do not control` |
| 0x019cff70 | `ServerConnection::bandwidthFromServer: Cannot comply since no mutator set with 'setBandwidthFromServerMutator'` |
| 0x019d0030 | `ServerConnection::authenticate: Unexpected key! (%s, wanted %s)` |
| 0x019d00e0 | `ServerConnection::createBasePlayer: id %d` |
| 0x019d0110 | `ServerConnection::createBasePlayer: Playing buffered createCellPlayer message` |
| 0x019d0160 | `ServerConnection::createCellPlayer: Got createCellPlayer before createBasePlayer. Buffering message` |
| 0x019d024c | `ServerConnection::createCellPlayer: id %d` |
| 0x019d0278 | `ServerConnection::spaceData: space %d key %d` |
| 0x019d02a8 | `ServerConnection::spaceData: Received space data before any space viewport info` |
| 0x019d02fc | `ServerConnection::spaceViewportInfo: space %d svid %d` |
| 0x019d0338 | `ServerConnection::spaceViewportInfo: Server wants us to re-use space viewport %d changing space from %d to %d!` |
| 0x019d03a8 | `ServerConnnection::spaceViewportInfo: Server wants us to close nonexistent viewport %d` |
| 0x019d0490 | `ServerConnection::voiceData: Got voice data before a handler has been set.` |
| 0x019d04e8 | `ServerConnection::restoreClient: No handler. Maybe already logged off.` |
| 0x019d0630 | `ServerConnection::resourceFragment: Unmatched response to request rid %d` |
| 0x019d0680 | `ServerConnection::resourceFragment: Error writing %d bytes of received data to file` |
| 0x019d0768 | `ServerConnection::loggedOff: The server has disconnected us. reason = %d` |
| 0x019cffe0 | `ServerConnection::send: Disconnecting since send failed for %s.` |

Note: `svidFollow` references "Viewport for entity" — part of the spaceViewport system absent from public BigWorld releases.

### 5. avatarUpdate Variants (36 total)

#### 32 Standard Optimized Variants (0x019d0ab8–0x019d0ee4)

Generated by `{Alias/NoAlias} × {FullPos/OnChunk/OnGround/NoPos} × {YawPitchRoll/YawPitch/Yaw/NoDir}`:

| Address Range | Pattern | Count |
|--------------|---------|-------|
| 0x019d0ab8–0x019d0cc8 | `avatarUpdateNoAlias{Pos}{Dir}` | 16 |
| 0x019d0ce8–0x019d0ee4 | `avatarUpdateAlias{Pos}{Dir}` | 16 |

This 32-variant optimized system was introduced in BigWorld 1.8 and is present in both 1.9.1 and 2.0.1.

#### 4 Additional Variants (0x019d08a0–0x019d08ec)

| Address | Name |
|---------|------|
| 0x019d08a0 | `avatarUpdateImplicit` |
| 0x019d08b8 | `avatarUpdateExplicit` |
| 0x019d08d0 | `avatarUpdateWardImplicit` |
| 0x019d08ec | `avatarUpdateWardExplicit` |

Ward variants handle position updates for non-player controlled entities. Present in both BW 1.9.1 and 2.0.1.

#### RTTI Confirmation (32 ClientMessageHandler templates, 0x01e52350–0x01e52f08)

Each avatarUpdate variant has a corresponding `ClientMessageHandler<avatarUpdate*Args@ClientInterface>` RTTI entry, confirming registration in the ClientInterface message dispatch table.

### 6. ClientInterface Message Handlers (46 RTTI entries)

Complete list of ClientInterface message handler RTTI at 0x01e52088–0x01e53050:

| Address | Handler | Notes |
|---------|---------|-------|
| 0x01e52088 | `bandwidthNotification` | Standard |
| 0x01e520e0 | `updateFrequencyNotification` | Standard |
| 0x01e52138 | `setGameTime` | Standard |
| 0x01e52180 | `resetEntities` | Standard |
| 0x01e521d0 | **`spaceViewportInfo`** | **Not in BW 1.9.1 or 2.0.1** |
| 0x01e52220 | **`entityInvisible`** | **Not in BW 1.9.1 or 2.0.1** |
| 0x01e52270 | `tickSync` | Standard |
| 0x01e522b8 | **`setSpaceViewport`** | **Not in BW 1.9.1 or 2.0.1** |
| 0x01e52308 | `setVehicle` | Standard |
| 0x01e52350–0x01e52f08 | 32× avatarUpdate variants | Standard |
| 0x01e52f60 | `detailedPosition` | Standard |
| 0x01e52fb0 | `forcedPosition` | Standard |
| 0x01e53000 | `controlEntity` | Standard |
| 0x01e53050 | `loggedOff` | Standard |

### 7. DataType Hierarchy (RTTI at 0x01e92070–0x01e92974)

| Address | Class | In BW 1.9.1? | In BW 2.0.1? |
|---------|-------|-------------|-------------|
| 0x01e92090 | `DataType` (base) | YES | YES |
| 0x01e922e0 | `IntegerDataType<E>` (uint8) | YES | YES |
| 0x01e92340 | `IntegerDataType<G>` (uint16) | YES | YES |
| 0x01e923a0 | `IntegerDataType<D>` (int8) | YES | YES |
| 0x01e92400 | `IntegerDataType<F>` (int16) | YES | YES |
| 0x01e92460 | `IntegerDataType<J>` (int32) | YES | YES |
| 0x01e924c4 | `LongIntegerDataType<K>` (uint32) | YES | YES |
| 0x01e9252c | `LongIntegerDataType<_J>` (int64) | YES | YES |
| 0x01e92594 | `LongIntegerDataType<_K>` (uint64) | YES | YES |
| 0x01e925f0 | `FloatDataType` | YES | YES |
| 0x01e92644 | `StringDataType` | YES | YES |
| 0x01e926a0 | `WideStringDataType` | YES | YES |
| 0x01e926fc | `PythonDataType` | YES | YES |
| 0x01e92764 | `VectorDataType<Vector2>` | YES | YES |
| 0x01e92814 | `VectorDataType<Vector3>` | YES | YES |
| 0x01e92884 | `VectorDataType<Vector4>` | YES | YES |
| 0x01e92920 | `BlobDataType` | YES | YES |
| 0x01e92974 | `MailBoxDataType` | YES | YES |
| 0x01e92070 | `SequenceDataType` | YES | YES |
| 0x01e9226c | `ArrayDataType` | YES | YES |
| 0x01e92250 | `TupleDataType` | YES | YES |
| 0x01e920c8 | `ClassDataType` | YES | YES |
| 0x01e920a8 | **`FixedDictDataType`** | **NO** | **YES** |
| 0x01e92288 | **`UserDataType`** | **NO** | **YES** |

MetaDataType hierarchy (factory registration):

| Address | Class |
|---------|-------|
| 0x01e921ac | `MetaDataType` |
| 0x01e921c8 | `SequenceMetaDataType` |
| 0x01e921ec | `ClassMetaDataType` |
| 0x01e9220c | `FixedDictMetaDataType` |
| 0x01e92230 | `UserMetaDataType` |

### 8. Additional Message Strings

| Address | Name | Context |
|---------|------|---------|
| 0x019d093c | `setSpaceViewportAck` | Client→Server acknowledgment |
| 0x019d0960 | `restoreClientAck` | Client→Server acknowledgment |
| 0x019d0998 | `ClientInterface` | Namespace identifier |
| 0x019d0a3c | `spaceViewportInfo` | Server→Client message |
| 0x019d0a70 | `entityInvisible` | Server→Client message |
| 0x019d0a8c | `tickSync` | Server→Client message |
| 0x019d0a98 | `setSpaceViewport` | Server→Client message |
| 0x019d0f34 | `voiceData` | Server→Client message |
| 0x019d0f40 | `restoreClient` | Server→Client message |
| 0x019d046c | `resourceFragment` | Server→Client message |

### 9. BigWorld Entity Integration

| Address | String/Class | Purpose |
|---------|-------------|---------|
| 0x01dc873c | `.?AVABigWorldEntity@@` | UE3 Actor wrapping BigWorld entity |
| 0x01dcd6c4 | `.?AVUBigWorldInfo@@` | BigWorld configuration UObject |
| 0x018cacb8 | `BigWorld` | Package/module name |
| 0x018cad14 | `.\Src\BigWorldEntity.cpp` | Source file |
| 0x018f1230 | `game-ini:Engine.BigWorldInfo.DefaultBigWorld` | INI config key |
| 0x0181e428 | `intABigWorldEntityexecAttachComponent` | UnrealScript exec stub |
| 0x0181e474 | `intUBigWorldInfoexecInit` | UnrealScript exec stub |
| 0x019ad5e8 | `BigWorld Position: %.3f, %.3f, %.3f` | Debug output |
| 0x019ad630 | `BigWorld Yaw, Pitch, Roll: %.3f, %.3f, %.3f` | Debug output |
| 0x01a9746c | `SaveBigWorldChunks` | Editor command |

### 10. UserDataType Debug Strings

| Address | String |
|---------|--------|
| 0x01b1bb20 | `UserMetaDataType::getType: Invalid implementedBy %s - must contain a '.'` |
| 0x01b1bb6c | `UserMetaDataType::getType: implementedBy is not specified.` |

The `implementedBy` mechanism for `UserDataType` is present in BW 2.0.1 but NOT in BW 1.9.1, confirming a post-1.9.1 feature.

## Version Timeline

```
BigWorld 1.7   (2006)  — voiceData added
BigWorld 1.8   (2007)  — 32 avatarUpdate variants, Ward support
BigWorld 1.9.1 (2008)  — Open-source reference, NO FixedDictDataType/UserDataType/spaceViewport
    ↓
  SGW build  (~2008-2009)  ← HERE: has FixedDict + UserDataType + spaceViewport (custom)
    ↓
BigWorld 2.0.1 (2012)  — Has FixedDictDataType/UserDataType, NO spaceViewport, adds switchBaseApp
BigWorld 14.4.1 (2014) — OSE release, NO spaceViewport
```

## Interpretation

1. **Post-1.9.1 core**: The presence of `FixedDictDataType` and `UserDataType` (absent from 1.9.1, present in 2.0.1) indicates CME's BigWorld source was newer than the public 1.9.1 release.

2. **Pre-2.0.1**: The absence of `switchBaseApp` (a 2.0.1 feature) confirms the build predates BigWorld 2.0.

3. **CME custom modifications**: `spaceViewportInfo`, `entityInvisible`, and `setSpaceViewport` appear in NO public BigWorld release. These are either:
   - CME custom additions to BigWorld (most likely — CME compiled from source)
   - Features from a private BigWorld point release between 1.9.1 and 2.0 that were removed/refactored before 2.0.1

4. **The spaceViewport system** appears to be a multi-space rendering system (viewport per space, with server-directed open/close of viewports). The debug strings reference "space viewport ID" (svid) and viewport reuse/close operations. This may have been needed for SGW's Stargate zone transitions.

5. **entityInvisible** is a server→client notification about entity visibility. This may relate to SGW's cover system or stealth mechanics.

## Implications for Server Emulation

- The **BW 1.9.1 source** provides a strong starting point for understanding the Mercury protocol, entity system, and ServerConnection flow
- The **BW 2.0.1 source** provides the `FixedDictDataType` and `UserDataType` implementations needed for entity property serialization
- **Custom features** (spaceViewport, entityInvisible) must be reverse-engineered from the sgw.exe binary since no reference source exists
- The `ServerConnection` class in sgw.exe has the same core API as BW 1.9.1/2.0.1, meaning protocol-level compatibility is high
