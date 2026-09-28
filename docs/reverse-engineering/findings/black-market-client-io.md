# Black Market Client I/O: Send Path, Receive Hook, Lua Surface

**Confidence**: HIGH for the addresses, signatures, enum values and UI defects, which were read directly from `SGW.exe` in Ghidra and from the shipped client Lua and layout. MEDIUM for two inferences, marked where they appear. Nothing here was exercised live. The live checks are listed at the end.
**Date**: 2026-09-26
**Sources**: `SGW.exe` (Ghidra), `lua51.dll` export table, `SGWGame/Content/UI/Core/BlackMarket/{BlackMarket.lua, BlackMarket.layout, BlackMarket_ItemRow.layout, BlackMarket_YourAuctionRow.layout}`, `entities/defs/interfaces/SGWBlackMarketManager.def`, `entities/defs/alias.xml`, `db/resources/Social/Types/EBlackMarket*.sql`.
**Plan that uses this**: [docs/analysis/black-market/README.md](../../analysis/black-market/README.md).
**Earlier findings this refines**: [black-market-client-window-patch.md](black-market-client-window-patch.md), [black-market-restoration.md](black-market-restoration.md), [black-market-wire-formats.md](black-market-wire-formats.md).

## Summary

The earlier Black Market work proved the window can be opened by a hand-applied x64dbg patch. It then spent four crashes trying to revive the client's own NetOut (client-to-server) path through CME event registration. This pass finds a lower and much simpler cut for both directions:

- **Sending** (cell methods 61–66). The generic entity-method sender calls `ServerConnection::startEntityMessage`. That is BigWorld's plain "start a message on the bundle" call, and it returns the bundle as a `BinaryOStream`. Native code on the main thread can call it directly and write the payload bytes itself. No CME event, subscriber, vtable redirect or registration is involved.
- **Receiving** (client methods 90–95). The universal dispatcher's signature, its two callers, its index decoder and its stream vtable are now pinned. The drop callee hands back the `MethodDescription`, so a hook can check the method *name* before acting instead of trusting an index.
- **Lua**. `lua51.dll` is a wide-character Lua 5.1 that exports the full C API under C++-mangled names. An injected DLL can register native Lua functions and push tables without code caves.
- **Contract gaps closed**: the view enum, the duration enum, `clientKey` semantics, the `BMCreateAuction` wire order, the `onBMAuctions` argument order, and the exact table `getAuctionItemInfo` returns.
- **The shipped Lua and layout are unfinished.** Beyond the unbound network methods, bidding was never wired in the UI, several functions reference undefined variables, and two of the four tabs have no row widgets. A client patch has to fix the UI as well as the binding.

## 1. Sending: `startEntityMessage`, not CME

`CEGUI__unknown_00c6fc40` is the engine's generic outgoing entity-method sender (`__stdcall(entity, entityDesc, methodDesc)`). For a cell method on the local player it does:

```text
conn   = [[0x01ef244c] + 0x08]            ; GameEntityManager -> ServerConnection
online = [conn + 0x30c] != 0              ; FUN_00dd6130
bundle = FUN_00dd8010(conn, [md + 0x44])  ; = startEntityMessage(conn, idx, 0)
if [md + 0x48] >= 0:  FUN_00c701a0(bundle) ; writes the extended sub-index byte
for each arg type in md+0x20:  argType->vtable[+0x20](bundle, &props, argInfoOffset)
```

| Address | Symbol | Signature / role |
|---|---|---|
| `0x01ef244c` | GameEntityManager singleton | `[+0x08]` = ServerConnection, `[+0x14]` = local player entity id, `[+0x90]` = entity-description map |
| `0x00dd05a0` | — | `mov eax,[0x01ef244c]; ret` |
| `0x00dd6130` | — | `bool isOnline(conn)` = `[conn+0x30c] != 0` |
| `0x00dd6a60` | `ServerConnection_startEntityMessage` | `__thiscall(conn, u8 idx, u32 entityId)`, `RET 8`. Sets message id `idx \| 0x80`, starts it on the channel bundle, writes the 4-byte entity id, returns the bundle in `EAX` |
| `0x00dd8010` | (startAvatarMessage) | `startEntityMessage(conn, idx, 0)`: entity id 0 means the local avatar |
| `0x00dd6980` | `ServerConnection_startProxyMessage` | base-method counterpart |
| `0x00c701a0` | — | writes one byte via `bundle->vtable[+0x10](1)` |
| bundle vtable `+0x10` | `BinaryOStream::reserve(n)` | `__thiscall`, returns a writable pointer to `n` bytes |

Framing matches the server's decoder ([cell-method-dispatch-table.md](../../protocol/cell-method-dispatch-table.md) "Wire Encoding"). Methods at index 61 and up go out as `0xBD` (`0x3D | 0x80`) followed by a one-byte sub-index (`index − 61`). The six BM methods are exactly sub-indices 0–5. The word-length field comes from the interface-element template `startEntityMessage` copies (`0x01ef2514`). By standard BigWorld bundle behavior it is filled in when the message closes; that was not traced in this binary and is covered by live check V4.

So a native sender for, say, `BMPlaceBid(sequenceId, bidAmount)` is:

```text
bundle = startEntityMessage(conn, 0x3D, 0)     ; on the MAIN thread only
*reserve(bundle, 1) = 2                        ; sub-index: 63 - 61
memcpy(reserve(bundle, 8), le32(seq) ++ le32(bid), 8)
```

**Why this replaces the CME path.** The prior sessions tried to make `Event_NetOut_BMCreateAuction` reach `0x00c6fc40` by synthesising the CME subscription the shelved feature never registered ([bm-create-auction-registration-key-bug](../../../.claude/agent-memory/game-archaeology-specialist/bm-create-auction-registration-key-bug.md)). Every step of that chain is heap-layout and template-instantiation specific, which is where the crashes came from. `startEntityMessage` sits below all of it and is the function every working NetOut ultimately calls.

**Thread.** The game's own sends run on the main thread. Call this only from the main thread, for example from a Lua C function (Lua runs on the main thread) or the `FEngineLoop::Tick` drain. Never call it from the network thread.

**Side note on the old "entity-guard bug".** `BMCreateAuction_NetOut_emit` (`0x00e59970`) bails when `thunk_FUN_00e1c450([[GEM+0x8c]+0x24], itemInstanceId)` returns null. The earlier diagnosis called that a mis-keyed entity lookup, but the live probe used a fabricated item id (`99001`). `[GEM+0x8c]+0x24` is more plausibly the inventory, and the guard more plausibly a legitimate "do you own this item" check. This is unverified, and it no longer matters, because the native sender bypasses the emitter.

## 2. Receiving: the dispatcher contract

| Address | Symbol | Signature / role |
|---|---|---|
| `0x00dd2b80` | `EntityManager::onEntityMethod` (unnamed) | `__thiscall(this, entityId, msgId, BinaryIStream* stream)`, `RET 0xC`. Looks up the entity. If found, calls the dispatcher. If not, copies the payload into a per-entity queue |
| `0x00dd1e40` | queued-message replay | replays the queue through the same dispatcher once the entity exists |
| `0x00c6f8f0` | `Client_NetIn_EntityMethodDispatch` | `__thiscall(this, Entity* entity, msgId, BinaryIStream* stream)`. `entity+0x0c` = entity id, `entity+0x14` = entity type id |
| `0x01590bb0` | method-index decoder | `__cdecl(msgId, exposedCount, stream)`. `threshold = 0x3E − (exposedCount + 0xC0) / 0xFF` (61 for the player). If `msgId ≥ threshold`, reads one more byte: `idx = byte + (msgId − threshold) * 256 + threshold` |
| `0x01590f30` | `EntityDescription_GetExposedClientMethodByIndex` | the drop callee. `__thiscall(desc+0xe0, idx)` returns the `MethodDescription*`. Name is a `std::string` at `md+0x04` |
| stream vtable `+0x04` | `BinaryIStream::retrieve(n)` | returns a pointer to `n` bytes and consumes them |
| stream vtable `+0x08` | `BinaryIStream::remainingLength()` | used by `0x00dd2b80` to size the queued copy |

On the drop path the dispatcher has already consumed the message id and sub-index, but **not the arguments**. The arguments are still unread in `stream` when `0x01590f30` returns. Given that, a hook pair can claim the six BM methods without touching the method map:

1. A detour on `0x00c6f8f0` records `(entity, stream)` for the duration of the call, in a thread-local.
2. A detour on `0x01590f30` calls the original to get the `MethodDescription*`. If the recorded entity is the local player (`entity+0x0c == [GEM+0x14]`) and the name is one of `onBMOpen` … `onBMWatchedItemsUpdate`, it reads the arguments through `retrieve` and queues a decoded event for the main thread.

The telemetry DLL already hooks `0x01590f30` as its "silent drop oracle" (`crates/client-telemetry/src/hooks/inline_hooks/mercury_dispatch.rs`) and `FEngineLoop::Tick` for its main-thread drain. Any patch DLL has to coexist with those hooks; see the plan.

## 3. The Lua C API is reachable

`lua51.dll` (150,560 bytes, 2009-06-30) exports 119 functions. All of them use **C++-mangled names with `wchar_t` strings**, for example `?lua_pushcclosure@@YAXPAUlua_State@@P6AH0@ZH@Z`, `?lua_setfield@@YAXPAUlua_State@@HPB_W@Z`, `?lua_tolstring@@YAPB_WPAUlua_State@@HPAI@Z` and `?lua_pushstring@@YAXPAUlua_State@@PB_W@Z`. `lua_Number` is `double`. The standard pseudo-indices are present in `index2adr` (`-10000`/`-10001`/`-10002` at file offsets `0x101b`–`0x104c`), so `LUA_GLOBALSINDEX` is the stock `-10002`.

Consequences:

- An injected DLL can register native Lua functions (`lua_pushcclosure` + `lua_setfield(L, LUA_GLOBALSINDEX, name)`) and build argument tables, with `GetProcAddress` on the mangled names. `SGW.exe` itself imports from this DLL in the same way.
- **Adjacent bug:** `crates/client-telemetry/src/bridge/lua_capture.rs` resolves the *unmangled* names (`"luaL_loadbuffer"`, …). They do not exist in this build, so the lab's return-value capture always degrades to fire-and-forget. The fix is to resolve the mangled names and treat strings as wide.
- The UI `lua_State` resolver and the wide `Lua_doString_wide` (`0x00404030`) are unchanged from [black-market-client-window-patch.md](black-market-client-window-patch.md).

## 4. The contract, corrected

### Enum values (read from the client's tolua constant getters)

Each constant is registered with `0x00403f20` and a getter that pushes a `double` literal.

| Lua enum | Values |
|---|---|
| `UIAuctionView` | `SearchResults = 0`, `MyAuctions = 1`, `MyBids = 2` |
| `UIAuctionTime` | `VeryShort = 1`, `Short = 2`, `Medium = 3`, `Long = 4`, `VeryLong = 5` |

The seeded server enums agree: `EBlackMarketSearchType = {Search, MyAuctions, MyBids}` and `EBlackMarketTime = {VeryShort … VeryLong}`.

The create form's three duration buttons send `Medium`, `Long` and `VeryLong`, so **`auctionLength` on the wire is 3, 4 or 5**, the 1-based `UIAuctionTime` value. The default is 5.

### `clientKey` is the view the reply belongs to

`BMSearchOptions` in client memory (constructor `FUN_00adebc0`, emitter `0x00e59f70`):

| Offset | Field | Default |
|---|---|---|
| `+0x00` | sellerName (`std::wstring`) | "" |
| `+0x1c` | bidderName (`std::wstring`) | "" |
| `+0x38` | itemName (`std::wstring`) | "" |
| `+0x54` | filterFlags / `.def` `monikerCRC` | 0 |
| `+0x58` | sortId (u8) | 0 |
| `+0x5c` | sequenceId | 0 |
| `+0x60` | bForward (u8) | 1 |
| `+0x64` | minTC | 0 |
| `+0x68` | maxTC | 0 |
| `+0x6c` | quality | 2000 |
| `+0x70` | clientKey | 0 |

`refreshMyAuctions()` (`0x00aac090` → `0x00ae3130`) sends a search with `sellerName = <my name>` and **`clientKey = 1`**. `refreshMyBids()` (`0x00aac0e0` → `0x00ae31f0`) sends `bidderName = <my name>` and **`clientKey = 2`**. A plain search leaves it at 0. So `clientKey` is the `UIAuctionView` / `EBlackMarketSearchType` the client wants filled, and `onBMAuctions` echoes it back:

```text
onBMAuctions(ARRAY<AuctionItem> auctionItems, INT32 totalResults, INT32 clientKey)   ; .def order
```

**Correction:** the server on `feat/571-black-market-phase1` sends `(items, view, total)` with `view` taken from `sortId`. The argument order is swapped, and the key is the wrong field. My Auctions and My Bids results would land in the Search view.

### `BMCreateAuction` wire order is the `.def` order

The engine sender (§1) serializes one argument per `MethodDescription` argument, and the emitter stores each value as a **named** property (`"itemInstanceId"`, `"startingPrice"`, `"buyoutPrice"`, `"auctionLength"`, `0x00e59970`). All four names match the `.def`. The wire order is therefore the `.def` order:

```text
BMCreateAuction: INT32 itemInstanceId, INT32 buyoutPrice, UINT8 auctionLength, INT32 startingPrice   ; 13 bytes
```

The "emitter order" reading in [black-market-restoration.md](black-market-restoration.md) and the ADR confused property-insertion order with wire order. The by-name lookup inside the arg-type writer (`vtable[+0x20]`) is inferred (MEDIUM), not traced. In practice the question is moot: the client patch serializes this message itself, so the server only has to agree with the patch, and the `.def` order is the faithful choice.

`BMSearchOptions` has the same issue in its last field. The emitter names it `"filterFlags"`, but the `.def` names it `monikerCRC`, so a by-name writer finds nothing for it. That is more evidence the stock emitters were never run against the real definitions. The shipped UI always sends 0 there.

### `onBMError` carries an id, and the Lua wants text

`EBlackMarketError = {InvalidSortType, BMUnavailable}` is the whole shipped enum. `BlackMarketMod.onBMError(this, errorText)` sets a label from a **string**. No client code maps the id to text, because the C++ subscriber was never written. The patch must supply that mapping, and the server's larger placeholder vocabulary (`NOT_ENOUGH_FUNDS`, `BID_TOO_LOW`, …) is a design choice the patch then has to mirror.

### What `getAuctionItemInfo(auctionId)` returns

From `FUN_00ae1ad0`, in field order: `auctionId`, `itemId`, `name` (itemDef `+0x10`), `icon` (itemDef `+0x48`), `techCompentancy` (itemDef `+0x78`, misspelling included), `timeLeft` (u8), `charges`, `currentBid`, `buyoutPrice`, `durability`, `nextBidPrice`, `sellerName`, `stackSize`, `bidderName`, `bidCount` (always 0). A miss returns an empty table, and callers test `itemInfo.auctionId` / `itemInfo.itemId`.

`getItemDefInfo(itemDefId)` (`0x00aa6370` → `FUN_00ae7180`) returns `{ID, Name, Description, Icon, Tier}`. That covers name and icon but **not** tech competency (`+0x78`), so a Lua-side replacement needs a small native getter for it or has to go without it.

**The item-definition lookup** (added 2026-09-27 for BM-04; read from the disassembly without Ghidra, MEDIUM). `FUN_00ae7180` gets the definition through the cooked-data cache, not a plain map read:

```text
FUN_004786f0()                      ; no arguments: lazily built singleton at [0x01ea56d8]
FUN_00ae6c10(this = singleton, SmartPtr* out, key)
  FUN_00ae0200(...)                 ; per-type cache, looked up by a descriptor at 0x017f94c8
  FUN_00ae6670 -> FUN_00ae5470(cache + 8, out, key)
    FUN_00d283e0(cache + 0x28, ...) ; cache fetch
    miss: FUN_00ae4110(...)         ; not traced; plausibly a load request
```

The result is a pointer whose reference count is the dword at `+0x04`. The callers decrement it and, at zero or below, call `vtable[0](1)`, the scalar deleting destructor. The field reads in `FUN_00ae7180` (`+0x10` name, `+0x2c`, `+0x48` icon, `+0x70`) are made while that reference is held. A native getter would have to call this chain on the main thread, read `+0x78`, and release the reference the same way. The exact argument shape of `FUN_00ae6c10` (the key is passed by address and is also released as if it were a reference-counted object) and the side effects of the miss path are not settled, so BM-04 ships `techCompetency` as always `nil`. V6 covers it.

## 5. The shipped UI is unfinished

`BlackMarket.lua` (759 lines) and `BlackMarket.layout` were shelved mid-build. Fixing only the network binding would still leave a UI that cannot bid and throws errors.

| # | Defect | Where |
|---|---|---|
| U1 | No bidding at all. `BlackMarket_SearchBidButton` and `BlackMarket_SearchBuyoutButton` are enabled and disabled but never subscribed to a click. Nothing calls the registered `placeBid(seq, amount)` binding (`0x00aac130`) | Lua 78–90, layout 267–279 |
| U2 | `refreshSearchView` uses an undefined global `viewType`, and its page count is unfloored and 0-based | Lua 286–294 |
| U3 | `onSearchNextClicked` / `onSearchPrevClicked` use undefined `viewType`. Prev also uses an undefined `currentPage` and raises an arithmetic-on-nil error | Lua 556–595 |
| U4 | `updateSearchNavButtons` enables Next only when `currentPage * visCount >= totalCount`, which is inverted | Lua 551 |
| U5 | `selectRow` indexes `BlackMarketMod.auctionToRow`, which does not exist, so the first row click raises an error | Lua 255 |
| U6 | `initRows` ignores its `viewType` argument and always uses the My Bids format, so Search and My Auctions rows never get a click handler | Lua 482–492 |
| U7 | My Auctions rows are looked up as `BlackMarket_MyAuctions<n>…`, but the layout imports them as `BlackMarket_MyAuction<n>` | Lua 739 vs layout 802–837 |
| U8 | The My Bids tab (`Tab3`) and the Watched tab (`Tab4`) have headers and a scrollbar but **no row widgets**. Tab button 4 has no ID or handler | layout 853–1191 |
| U9 | `refreshMyAuctionsView` / `refreshMyBidsView` are empty, and nothing calls `refreshMyAuctions()` / `refreshMyBids()` when a tab opens | Lua 297–304 |
| U10 | The row timer image switches on `techCompentancy` instead of `timeLeft` | Lua 438–445 |
| U11 | `populateRow` calls `rowWin:hide()` when `rowWin` is nil, which raises an error for any view whose rows are missing | Lua 476–478 |
| U12 | Create gives no acknowledgement: the form is not cleared and the My Auctions view is not refreshed | Lua 144–158 |

The Lua expects the store to fire `Events.BMViewUpdate(viewType)` after data arrives. That event would have come from the unwritten C++ subscriber, so a patch can call `BlackMarketMod.onBMViewUpdate(nil, view)` directly instead.

UI Lua and layouts are loose files under `SGWGame/Content/UI/`. The server cannot deliver them, but the launcher's signed overlay-patch pipeline (`crates/launcher/src/install.rs`, `manifest.rs`) can.

## 6. Live checks still owed

| ID | Check | Why it matters |
|---|---|---|
| V1 | Read the `MethodDescription` for `BMCreateAuction`. Expect `+0x44 == 0x3D`, `+0x48 == 1`, and args in `.def` order | Confirms the framing and the arg-order inference |
| V2 | From an injected DLL, resolve the mangled `lua51.dll` exports, register a test global, and call it from Lua | Proves the native-registration path |
| V3 | Hook pair (§2) sees `onBMOpen` with the stream attached. Decode the INT32 and open the window | Replaces the hand-built dispatch node |
| V4 | `startEntityMessage(conn, 0x3D, 0)` + sub-index + payload reaches a local server's cell dispatch for 61–66 | Proves the sender |
| V5 | `getItemDefInfo` for an item the player has never held | Confirms rows can show names and icons for arbitrary items |
| V6 | Break on `FUN_00ae6c10` from a `getItemDefInfo` call: record the key argument, the returned pointer, its refcount before and after, and `[def + 0x78]`, for a cached item and an uncached one | Decides whether BM-04's `techCompetency` can call the chain (D7) |

## Ghidra annotations

None applied in this pass. Suggested renames: `0x00dd8010` → `ServerConnection_startAvatarMessage`, `0x00dd2b80` → `EntityManager_onEntityMethod`, `0x00dd1e40` → `EntityManager_replayQueuedMethods`, `0x01590bb0` → `MethodIndex_decodeExtended`, `0x00ae1ad0` → `BM_buildAuctionItemInfoTable`, `0x00ae7180` → `buildItemDefInfoTable`, `0x00ae31f0` → `BMRefreshMyBids_emit`, `0x00c6fc40` → `Entity_sendServerMethod`.
