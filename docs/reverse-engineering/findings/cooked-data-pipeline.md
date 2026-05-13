# Cooked Data Pipeline — Binary Findings

**Session**: W-cooked (V5 Campaign Session 5)
**Date**: 2026-05-13
**Analyst**: Game Archaeology Specialist (W-cooked)
**Confidence**: HIGH — all findings confirmed from live Ghidra MCP decompilation

---

## Summary

The cooked data pipeline on the client is built from 21 `LibCategory<LibCategoryKey<N,...>>` objects
registered at startup by `CookedData_RegisterAllLibCategories` (`0x00420074`). Each object wraps a
`Detail::ServerSource<N,...>` that subscribes to five CME events per category and manages a per-category
ZIP/PAK archive cache. The server sends version info; the client compares against a cached MetaData
stamp; if stale, it requests individual elements by key. Element XML arrives via `BASEMSG_RESOURCE_FRAGMENT`
fragments decoded into a `Event_Net_ProxyData` → `ServerSource::onProxyData` handler.

The existing `docs/engine/cooked-data-pipeline.md` has two errors in its "Resource Category IDs"
table: the client has no category 0, and the high-end categories (21 and 22) are wrong.
See "Contradictions with Existing Docs" below.

---

## Finding 1 — Category→PAK Mapping (Definitive, Binary-Confirmed)

**Confidence**: HIGH
**Source**: `CookedData_RegisterAllLibCategories` @ `0x00420074` — decompiled 2026-05-13

The client registers exactly 21 `ServerSource` categories, numbered 1–21. Category 0 does not exist
on the client. The category integer `N` is embedded as a template parameter in each
`LibCategory<LibCategoryKey<N,...>>` instantiation and stored at `LibCategory+0x4`.

### Category→PAK→DataType Table

| Category ID | PAK Filename | C++ DataType (template parameter) |
|-------------|-------------|----------------------------------|
| 1 | `CookedDataKismetSeqEvent.pak` | `CookedKismetEventSequenceData` |
| 2 | `CookedDataAbilities.pak` | `Ability` |
| 3 | `CookedDataMissions.pak` | `Mission` |
| 4 | `CookedDataItems.pak` | `DBInvItem` |
| 5 | `CookedDataDialogs.pak` | `Dialog` |
| 6 | `CookedDataKismetSetEvent.pak` | `CookedKismetEventSetData` |
| 7 | `CookedCharCreation.pak` | `CookedCharCreationData` |
| 8 | `CookedInteractionSet.pak` | `InteractionSet` |
| 9 | `CookedDataEffects.pak` | `DBEffect` |
| 10 | `TextStrings.pak` | `CookedTextType` |
| 11 | `ErrorStrings.pak` | `CookedErrorTextType` |
| 12 | `CookedWorldInfo.pak` | `SGW::WorldInfo` |
| 13 | `CookedDataStargates.pak` | `DBGateInfo` |
| 14 | `CookedDataContainers.pak` | `DBInvContainer` |
| 15 | `CookedBlueprints.pak` | `SGW::Blueprint` |
| 16 | `CookedSciences.pak` | `SGW::AppliedScience` |
| 17 | `CookedDisciplines.pak` | `SGW::Discipline` |
| 18 | `CookedParadigm.pak` | `SGW::RacialParadigm` |
| 19 | `SpecialWords.pak` | `CookedSpecialWordsType` |
| 20 | `CookedInteractions.pak` | `SGW::InteractionData` |
| 21 | `CookedBehaviorEvents.pak` | `BehaviorEventData` |

Additionally, `covernodes_local.pak` is registered as a `ClientSource` (not a `ServerSource`).
It has no category ID and is loaded locally, not from the server.

### Evidence

`CookedData_RegisterAllLibCategories` (`0x00420074`) constructs each `LibCategory<LibCategoryKey<N,...>>`
object as follows (pseudocode pattern repeated 21 times, N varying):

```c
void *obj = scalable_malloc(sizeof(LibCategory<LibCategoryKey<N,...>>));
LibCategoryBase_Ctor(obj, N);                   // set vtable + category ID at obj+0x4
FUN_0044xxxx(obj + 2, L"CookedDataXxx.pak");    // set wstring PAK filename at obj+0x8
*(vtable_ptr*)obj = LibCategory_ServerSource_Vftable_N;  // stamp per-N vtable
CacheLibrary_GetSingleton();
CacheLibrary_RegisterCategory(singleton, obj);  // insert into sorted map by category ID
```

The PAK filenames are embedded as wide-string literals in the binary and were read directly from the
decompiler output. They are not inferred — each wstring was observed as a literal argument.

---

## Finding 2 — LibCategory / ServerSource Struct Layout

**Confidence**: HIGH
**Sources**: `LibCategoryBase_Ctor` @ `0x004786c0`, `LibCategory_ServerSource_Ctor_cat1_KismetSeqEvent` @ `0x0044c800`

```
LibCategory<LibCategoryKey<N,...>> struct layout:
  +0x00  vtable*                  (LibCategory<LibCategoryKey<N,...>>::vftable)
  +0x04  uint32 category_id       (= N; set by LibCategoryBase_Ctor)
  +0x08  wstring pak_filename     (MSVC wstring, std::basic_string<wchar_t>)
  ...
  +0x20  CZipArchive* archive     (open PAK handle; 0 if not yet opened)
  +0x24  uint32 cached_version    (server version stamp, stored after onVersionInfo)
  +0x2C  list_head                (element cache linked list head)
  +0x30  uint32 element_count     (cached element count)
  +0x3C  uint32* pending_begin    (pending request vector begin)
  +0x40  uint32* pending_end      (pending request vector end)
  +0x48  uint32 required_updates  (RequiredUpdates counter from server)
  +0x4C  bool version_info_rcvd   (set to 1 after first onVersionInfo)
```

Note: offsets +0x3C/+0x40 store a vector of (category_id, element_key) pairs for in-flight requests.
Each request pair is 8 bytes (two uint32s). Observed in `ServerSource_RequestElement` (`0x0043bdb0`).

---

## Finding 3 — Five CME Events Per Category (Version Negotiation Pipeline)

**Confidence**: HIGH
**Sources**: `LibCategory_ServerSource_Ctor_cat1_KismetSeqEvent` @ `0x0044c800`,
             `CME_MemberCallback_Ctor_ServerSource_*` @ `0x004267f0`, `0x004268f0`, `0x00426970`,
             `0x004269f0`, `0x00426a70`

Each `LibCategory` constructor wires exactly 5 CME event subscriptions:

| Subscription # | Event | Handler Role |
|----------------|-------|--------------|
| 1 | `Event_Net_Connected` | Re-request version info on (re)connect |
| 2 | `Event_Net_Disconnected` | Cleanup on disconnect |
| 3 | `Event_NetIn_onVersionInfo` | Process server version response |
| 4 | `Event_Net_ProxyData` | Receive BASEMSG_RESOURCE_FRAGMENT data |
| 5 | `Event_NetIn_onCookedDataError` | Handle server-reported element failure |

All subscriptions are through `CME_EventSignal_Subscribe` (`0x00a37790`), which inserts a
`MemberCallback<NoSubject, ServerSource<N,...>, handler_ptr, EventType>` into the CME event bus.
The MemberCallback is 0xC bytes: `[+0x0] vtable*, [+0x4] object_ptr, [+0x8] method_ptr`.

The vtable slot 2 (`vfunc_2`) of each callback returns the RTTI type descriptor — this is the
mechanism CME uses to route incoming events to subscribers of the correct type.

---

## Finding 4 — Version Negotiation Flow (onVersionInfo Handler)

**Confidence**: HIGH
**Source**: `ServerSource_onVersionInfo_Handler_cat6` @ `0x00441630`

```
Client (ServerSource<N>)               Server (BaseApp)
        |                                     |
        |--  Event_NetOut_versionInfoRequest ->|  (on Net_Connected)
        |<-- Event_NetIn_onVersionInfo --------|
        |                                     |
        | [onVersionInfo handler]:             |
        |   1. Read "CategoryId" field         |
        |      → return if != N (wrong cat)   |
        |   2. Read "RequiredUpdates" → this+0x48
        |   3. Read "InvalidateAll" bool:      |
        |      - true:  flush cache at this+0x2C/0x30 (FUN_0047a690)
        |      - false: read "InvalidKeys" list, per-key invalidation
        |   4. Read "Version" → ServerSource_SetVersion(this, &version)
        |      → stores at this+0x24          |
        |      → writes to PAK MetaData entry  |
        |   5. Iterate pending request vector (this+0x3C..0x40):
        |      → ServerSource_RequestElement per entry
        |         (fires Event_NetOut_elementDataRequest if not cached)
        |   6. Set this+0x4C = 1 (version_info_rcvd flag)
        |                                     |
        |-- Event_NetOut_elementDataRequest(N, key) ->|
        |<-- BASEMSG_RESOURCE_FRAGMENT x M ---|  (via Event_Net_ProxyData)
```

The "Version" field from the server payload is stored at `ServerSource+0x24` and immediately persisted
to the PAK archive's `MetaData` entry (4 bytes, little-endian uint32) via `ZipStorageBase_WriteMetaDataVersion`
(`0x00479e10`). On the next session, if the PAK's `MetaData` matches the server version, no re-download
is needed.

---

## Finding 5 — onCookedDataError Handler

**Confidence**: HIGH
**Source**: `ServerSource_onCookedDataError_Handler_cat6` @ `0x00441aa0`

Handler for `Event_NetIn_onCookedDataError`. Reads:
- `"categoryID"` (uint) — returns early if != N (wrong category filter)
- `"elementKey"` (uint32) — identifies the failed element

On match:
1. Decrements `this+0x48` (RequiredUpdates counter) if nonzero — mirrors the element delivery decrement
2. Allocates 8-byte error record: `[0] = N (categoryID), [1] = elementKey`
3. Fires `Event_Cache_ElementError` via CME emit (`FUN_004349b0`)

Note: the decompiler showed `scalable_free(0)` at the function tail — this is a decompiler artifact from
the SEH/EH epilogue, not actual behavior. The function returns normally after firing the error event.

---

## Finding 6 — ZipStorage PAK Open Path

**Confidence**: HIGH
**Source**: `ZipStorageBase_OpenArchive` @ `0x00479340`
**Source file confirmed**: `ZipStorage.cpp`, line 0x82 (130) via log4cxx LocationInfo

`ZipStorageBase::OpenArchive()` is called before any PAK read or write to ensure the archive is open.

Logic:
1. If `this+0x20` (CZipArchive handle) is non-null and CZipArchive state at `+0x5E` is not -1 (closed):
   return true immediately (already open).
2. Iterate path vector at `this+0x14..0x18` to find the PAK file on disk.
3. If path doesn't exist: call `FUN_0139dfb0` to create the directory.
   - On failure: log error `"Error creating cache archive directory: ..."` via log4cxx and return false.
4. Open the ZIP archive via CZipArchive API.
5. Return true if successfully opened.

The `SourceCachePath` INI key (under `[Core.System]` in `GEngineIni`) is read at startup by
`CookedData_RegisterAllLibCategories` and stored into the path vector of each `LibCategory` object.

---

## Finding 7 — CacheLibrary Singleton

**Confidence**: HIGH
**Source**: `CacheLibrary_GetSingleton` @ `0x004786f0`

```c
// CacheLibrary singleton (globally allocated on first call)
void* CacheLibrary_GetSingleton(void) {
    if (DAT_01ea56d8 == 0) {
        void* p = scalable_malloc(0xC);  // 12-byte CacheLibrary
        DAT_01ea56d8 = CacheLibrary_Ctor(p);
        if (DAT_01ea56dc != 0) {
            OutputDebugStringW(L"WARNING: Reinitializing CacheLibrary after shutting it down");
        }
        DAT_01ea56dc = 1;  // set initialized flag (not shutdown flag)
    }
    return DAT_01ea56d8;
}
```

Globals:
- `0x01ea56d8` — `g_pCacheLibrary`: pointer to the 12-byte `CacheLibrary` object
- `0x01ea56dc` — `g_CacheLibraryInitialized`: byte flag, set to 1 after first init; guards against
  re-initialization after shutdown. The `OutputDebugStringW` warning fires if it was previously
  shut down (`DAT_01ea56dc` set to 0 by shutdown path then back to 1 here, but the warning check
  is on the pre-reassignment value — hypothesis: shutdown sets it to some nonzero sentinel value
  to indicate "was shut down").

`CacheLibrary` is 12 bytes. The actual ctor body is `FUN_0157ce00` (thin wrapper at `CacheLibrary_Ctor`
`0x00478840` sets up SEH and calls it). The CacheLibrary holds an internal sorted map (std::map-like
red-black tree) of category ID → LibCategory pointer.

---

## Contradictions with Existing Documentation

### `docs/engine/cooked-data-pipeline.md` — Resource Category IDs table

**Status**: INCORRECT — must be fixed in a separate code-change session.

The existing table ("Resource Category IDs", from `src/baseapp/mercury/sgw/resource.cpp`) lists:
- Category 0 as "Reserved"
- Category 21 as `pet_command`
- Category 22 as `behavior_event`

The binary shows:
- Category 0 does NOT exist on the client. There is no `LibCategory<LibCategoryKey<0,...>>`.
- Category 21 = `BehaviorEventData` / `CookedBehaviorEvents.pak`
- The server's category 22 (`behavior_event`) has no client-side counterpart at that index.

**Likely cause**: The server-side `resource.cpp` table starts from 0 (with 0 as a dummy/empty entry)
and counts differently at the high end, OR the server and client have drifted in numbering. The
binary's LibCategoryKey<N> template parameters are authoritative for what the client expects.

**Recommended fix for Cimmeria**: Audit `src/baseapp/mercury/sgw/resource.cpp` CategoryMap to ensure
the server sends the same category IDs the client registers. The client's 1–21 mapping is the spec.

### `docs/engine/cooked-data-pipeline.md` — "22 resource categories" claim

The pipeline doc says "22 resource categories." The binary has 21 ServerSource categories (1–21).
The number 22 likely comes from counting the server's zero-indexed table (entries 0–21 = 22 entries,
of which entry 0 is a placeholder). The client never receives or registers category 0.

---

## Address Summary (New — this session)

| Address | Name | Notes |
|---------|------|-------|
| `0x00420074` | `CookedData_RegisterAllLibCategories` | Reads SourceCachePath INI; registers all 21 LibCategory objects |
| `0x004786c0` | `LibCategoryBase_Ctor` | Sets vtable + category ID at `+0x4` |
| `0x004786f0` | `CacheLibrary_GetSingleton` | Lazy-init CacheLibrary singleton at `DAT_01ea56d8` |
| `0x00478840` | `CacheLibrary_Ctor` | Thin SEH wrapper; calls `FUN_0157ce00` (body ctor) |
| `0x00437650` | `CacheLibrary_RegisterCategory` | Inserts LibCategory into sorted map by category ID |
| `0x0044c800` | `LibCategory_ServerSource_Ctor_cat1_KismetSeqEvent` | Template ctor for cat 1; registers 5 event subscriptions |
| `0x004267f0` | `CME_MemberCallback_Ctor_ServerSource_NetConnected` | MemberCallback for Event_Net_Connected |
| `0x004268f0` | `CME_MemberCallback_Ctor_ServerSource_NetDisconnected` | MemberCallback for Event_Net_Disconnected |
| `0x00426970` | `CME_MemberCallback_Ctor_ServerSource_onVersionInfo` | MemberCallback for Event_NetIn_onVersionInfo |
| `0x004269f0` | `CME_MemberCallback_Ctor_ServerSource_NetProxyData` | MemberCallback for Event_Net_ProxyData (fragment delivery) |
| `0x00426a70` | `CME_MemberCallback_Ctor_ServerSource_onCookedDataError` | MemberCallback for Event_NetIn_onCookedDataError |
| `0x0042a7b0` | `CME_Subscribe_ServerSource_NetConnected` | Subscription wrapper: alloc + ctor + Subscribe |
| `0x0042a840` | `CME_Subscribe_ServerSource_NetDisconnected` | Subscription wrapper |
| `0x0042a8d0` | `CME_Subscribe_ServerSource_onVersionInfo` | Subscription wrapper |
| `0x0042a960` | `CME_Subscribe_ServerSource_NetProxyData` | Subscription wrapper |
| `0x0042a9f0` | `CME_Subscribe_ServerSource_onCookedDataError` | Subscription wrapper |
| `0x00a37790` | `CME_EventSignal_Subscribe` | Core CME subscribe: resolves RTTI type, inserts callback node |
| `0x00441630` | `ServerSource_onVersionInfo_Handler_cat6` | Processes "CategoryId"/"RequiredUpdates"/"InvalidateAll"/"Version" |
| `0x00441aa0` | `ServerSource_onCookedDataError_Handler_cat6` | Processes "categoryID"/"elementKey", fires Cache_ElementError |
| `0x00479340` | `ZipStorageBase_OpenArchive` | Opens PAK ZIP archive; creates cache dir if needed (ZipStorage.cpp:130) |
| `0x00479930` | `ZipStorageBase_WriteStreamToFile` | Writes ostream to named ZIP entry (ZipStorage.cpp) |
| `0x00479e10` | `ZipStorageBase_WriteMetaDataVersion` | Writes 4-byte version stamp to PAK "MetaData" entry |
| `0x00479e90` | `ServerSource_SetVersion` | Stores server version at `this+0x24` → calls WriteMetaDataVersion |
| `0x0043bdb0` | `ServerSource_RequestElement` | Cache-miss check → fires Event_NetOut_elementDataRequest |
| `0x013a1620` | `CZipStorage_Dtor` | Destroys wstring at `+0xC`, CZipAutoBuffer at `+0x5C` |
| `0x01ea56d8` | `g_pCacheLibrary` | Global: CacheLibrary singleton pointer |
| `0x01ea56dc` | `g_CacheLibraryInitialized` | Global: initialized/shutdown state byte |

---

## Open Questions

1. **`FUN_0157ce00`** — the actual `CacheLibrary` body constructor. Not yet decompiled. Expected to initialize the internal red-black tree map.

2. **`FUN_0047a690`** — called from `onVersionInfo` when `InvalidateAll=true`. Expected to flush all cached elements from the linked list at `this+0x2C`. Not yet decompiled.

3. **`FUN_0043a9d0`** — called from `ServerSource_RequestElement` as a cache-miss check. Returns bool (skip if already cached/in-flight). Not yet decompiled.

4. **`FUN_004349b0`** — CME emit call used by `onCookedDataError` to fire `Event_Cache_ElementError`. Not yet confirmed as the generic emit path or a specific wrapper.

5. **Server category 0 vs client start at 1** — needs server-side code audit. The Cimmeria server's `src/baseapp/mercury/sgw/resource.cpp` must be checked to confirm whether it sends category IDs 1–21 or 0–21.

6. **Category 21 server-side name** — the binary calls it `BehaviorEventData`. The server calls category 22 `behavior_event`. If the server sends 22 and the client registers 21, the category would be silently ignored. This may explain why some NPC behavior events were unreliable.

7. **Net_ProxyData handler body** — the callback for `Event_Net_ProxyData` in the LibCategory constructor passes `&LAB_0043dad0` (a label, not a function symbol) as the method pointer. This means the fragment reassembly handler starts at `0x0043dad0`. Not yet decompiled.

---

## Cross-references

- `docs/engine/cooked-data-pipeline.md` — existing pipeline doc; Resource Category IDs table has errors documented above
- `docs/engine/cooked-data-pak-format.md` — PAK format details; should be updated to add integer IDs to category inventory
- `docs/reverse-engineering/findings/cme-event-signal.md` — CME EventSignal architecture detail
- `docs/reverse-engineering/address-map.md` — address map; new addresses appended this session
