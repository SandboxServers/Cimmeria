# cimmeria-client-patches

An injected DLL that restores client features the 2009 `SGW.exe` shipped
unfinished. It is built as a `cdylib` for the **32-bit client process**, plus
an `rlib` so the host can run the unit tests of everything that is not raw
FFI. It is separate from `cimmeria-client-telemetry` on purpose: gameplay must
not depend on the telemetry opt-in. The decision record is
[docs/architecture/client-patches.md](../../docs/architecture/client-patches.md).

**Status:** the Black Market **receive** path (build check, receive hooks,
decode, main-thread delivery to Lua) and **send** path (native Lua
functions for the cell methods 61–66). Nothing loads this DLL yet: the
launcher's always-inject step is a later packet, and nothing here has run
inside a live client. Both paths are verified statically and by host and
i686 unit tests only. The live checks still owed are listed at the end.

## Build, lint and test against the i686 target

Every hook, detour and Lua call is behind
`#[cfg(all(windows, target_arch = "x86"))]`. A host build compiles only the
portable logic, so a host-only check passes while the gated code stays
unchecked. Run the i686 checks before pushing, through the build lane:

```sh
rustup target add i686-pc-windows-msvc   # one-time

bash tools/build-lane/lane.sh cargo clippy -p cimmeria-client-patches \
  --target i686-pc-windows-msvc --all-targets -- -D warnings
bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-client-patches \
  --target i686-pc-windows-msvc
cargo fmt -p cimmeria-client-patches -- --check
```

The DLL lands at `<target dir>/i686-pc-windows-msvc/<profile>/cimmeria_client_patches.dll`.
CI runs the same three in
[.github/workflows/client-patches-build.yml](../../.github/workflows/client-patches-build.yml).

`build.rs` embeds an `asInvoker` manifest in the 32-bit binaries. Windows'
installer detection refuses to start an unmanifested 32-bit `.exe` with
"patch" in its name unless it is elevated (os error 740), and the test
harness is `cimmeria_client_patches-<hash>.exe`.

## What it does

1. **Build fingerprint gate** (`fingerprint.rs`). Before any hook goes in, it
   compares the first bytes at every address it uses with this build's
   bytes. On any mismatch it installs nothing and logs which site differed.
   `FEngineLoop::Tick` and the drop callee may already start with `E9 rel32`,
   because the telemetry DLL hooks them with MinHook too. In that case, if
   everything after the jump still matches and the jump lands inside the
   loaded telemetry DLL's image, the hook chains on top. A jump anywhere
   else fails the gate.
2. **Receive**, on the Mercury network thread (`receive/`). A detour on the
   entity-method dispatcher records `(entity, stream)` in a thread-local for
   the length of the call. A detour on the drop callee, which the dispatcher
   calls only when no handler is bound, calls the original first. It then
   claims the call if the returned `MethodDescription`'s **name** is one of
   `onBMOpen`, `onBMError`, `onBMAuctions`, `onBMAuctionRemove`,
   `onBMAuctionUpdate` and `onBMWatchedItemsUpdate`, and the entity is the
   local player. It decodes the arguments with `cimmeria-patch-wire`
   straight from the client's `BinaryIStream` and pushes the result onto a
   bounded queue of 256. When the queue is full, the call is dropped and
   counted. Every other dropped method behaves exactly as before.
3. **Deliver**, on the main thread (`deliver/`). A detour on
   `FEngineLoop::Tick` drains up to 16 calls a frame into the UI Lua through
   `lua51.dll`, as described in the contract below. An empty queue costs one
   atomic load per frame. Until the UI's `lua_State` exists, calls stay
   queued.
4. **Send**, on the main thread (`send/`). Every 30 frames the same `Tick`
   detour makes sure the UI Lua has the global table `CimmeriaBMNative`
   (see the send contract below). Its native functions validate their
   arguments, encode the cell method with `cimmeria-patch-wire`, check that
   the client is connected, and call `ServerConnection::startEntityMessage`
   with the extended method id, then write the sub-index byte and the
   payload through the bundle's `reserve`. The engine calls run under a
   structured-exception guard (`microseh`), so a fault or C++ exception in
   the engine becomes an `engine_error` return.

Safety rules:

- Every client pointer is read through `ReadProcessMemory` on the DLL's own
  process, behind a `VirtualQuery` pre-check, so a bad pointer, or a page
  freed by another thread mid-read, means "not ours", never a fault.
- There is a `catch_unwind` at every Rust-owned FFI edge.
- The queue lock is never held while calling into the game.
- Lua runs only on the main thread, and every Lua call, including building
  the arguments, inside `lua_cpcall`; handlers run under a nested
  `lua_pcall`.

The dispatcher and `Tick` detours use the `thiscall-unwind` ABI. A C++
exception from the original function then reaches the game's own handler
instead of aborting inside the detour.

### Addresses

`SGW.exe` QA build, image base `0x00400000`, ASLR off. Details and evidence
are in `src/addresses.rs` and
[black-market-client-io.md](../../docs/reverse-engineering/findings/black-market-client-io.md).

| Address | Function | Use |
|---|---|---|
| `0x00c6f8f0` | `Client_NetIn_EntityMethodDispatch` | hooked (receive) |
| `0x01590f30` | `EntityDescription_GetExposedClientMethodByIndex` (drop callee) | hooked (receive), chainable |
| `0x00416ec0` | `FEngineLoop::Tick` | hooked (deliver), chainable |
| `0x00dd6a60` | `ServerConnection::startEntityMessage` | called (send) |
| `0x00dd8010` | `ServerConnection::startAvatarMessage` | fingerprinted only: pins the `(conn, idx, 0)` call |
| `0x00dd6130` | `ServerConnection::isOnline` | fingerprinted only: pins the online flag at `conn + 0x30c` |
| `0x00dd05a0` | `GameEntityManager` getter | fingerprinted only: pins `0x01ef244c` |
| `0x00c701a0` | the engine's write-one-byte helper | fingerprinted only: pins `reserve` at bundle vtable `+0x10` |
| `0x01ef244c` | `GameEntityManager*`; `ServerConnection*` at `+0x08`, local player id at `+0x14` | receive: compared with `Entity + 0x0c`; send: the connection, and "no player yet" |
| `0x01ee2a58` | `g_SGWUIManager_ptr`; UI `lua_State` = `*(*(*(p) + 0x10))`, tag byte `+4 == 8` | delivery |

## The Lua contract (for the UI overlay)

The overlay that implements this contract, a patched `BlackMarket.lua` and
`BlackMarket.layout`, is in [overlay/](overlay/README.md).

The overlay defines one global table, `CimmeriaBM`, and a plain function on
it for each call. The DLL calls them with no `self`:

| Server method | Lua call |
|---|---|
| `onBMOpen(entityId)` | `CimmeriaBM.onOpen(entityId)` |
| `onBMError(errorId)` | `CimmeriaBM.onError(errorId)` |
| `onBMAuctions(auctionItems, totalResults, clientKey)` | `CimmeriaBM.onAuctions(items, totalResults, clientKey)` |
| `onBMAuctionRemove(sequenceId)` | `CimmeriaBM.onAuctionRemove(sequenceId)` |
| `onBMAuctionUpdate(auctionItem)` | `CimmeriaBM.onAuctionUpdate(item)` |
| `onBMWatchedItemsUpdate(itemList)` | `CimmeriaBM.onWatchedItems(itemList)` |

- **Numbers** are Lua numbers: `entityId`, `errorId`, `totalResults`,
  `clientKey` (`0` search results, `1` my auctions, `2` my bids),
  `sequenceId` and every numeric item field.
- **`items`** is a 1-based array of item tables, in server order.
  **`itemList`** is a 1-based array of item definition ids.
- **An item table** has exactly these keys, the `.def` names: `sequenceId`,
  `itemDefId`, `stackSize`, `durability`, `charges`, `currentBid`,
  `buyoutPrice`, `endTimeValue` (time-left bucket 1–5, for the row timer),
  `nextMinBidPrice` (all numbers) and `sellerName` (a string).
- **`errorId`** is the raw id. `0` is `InvalidSortType`, `1` is
  `BMUnavailable`, and the server's own rejection ids follow them. The
  overlay maps ids to text.

How the calls are made:

- **Lookup is raw.** `CimmeriaBM` and each function are read with
  `lua_rawget`, so no metamethod runs outside a protected call. The functions
  must be plain fields of the table, not inherited through `__index`.
- **Every call runs protected.** The lookups and the argument tables are
  built inside `lua_cpcall`, so running out of Lua memory drops the call
  instead of exiting the client, and the handler runs under `lua_pcall`.
  Either error is logged with its message and does not propagate. The Lua
  stack is restored whatever happens.
- **A missing overlay drops the call.** If `CimmeriaBM` is not a table, or it
  lacks the function, the call is dropped and counted. The DLL keeps no Lua
  state, so the overlay should request what it needs when it opens, for
  example by searching again.
- **Calls arrive between frames,** in the order the server sent them, at up
  to 16 a frame.

## The send contract (for the UI overlay)

The DLL creates one global table, `CimmeriaBMNative`, and never touches the
overlay's `CimmeriaBM`. Its functions are plain fields, called with no
`self`:

| Lua call | Cell method |
|---|---|
| `CimmeriaBMNative.search(opts)` | `BMSearch` (61) |
| `CimmeriaBMNative.create(itemInstanceId, startingPrice, buyoutPrice, auctionLength)` | `BMCreateAuction` (62) |
| `CimmeriaBMNative.bid(sequenceId, bidAmount)` | `BMPlaceBid` (63) |
| `CimmeriaBMNative.cancel(sequenceId)` | `BMCancelAuction` (64) |
| `CimmeriaBMNative.watch(itemDefId, enable)` | `BMStartWatchingItem` (65) if `enable` is truthy, else `BMStopWatchingItem` (66) |
| `CimmeriaBMNative.techCompetency(itemDefId)` | none; returns `nil` in this build (see below) |
| `CimmeriaBMNative.version` | the DLL's version string, not a function |

**Return values.** A send function returns `true` once the message is on
the engine's outgoing bundle; the server's answer arrives later through
`CimmeriaBM`. Otherwise it returns `nil, reason`, with `reason` one of:

| `reason` | Meaning |
|---|---|
| `"bad_args"` | An argument is missing, has the wrong type, is out of range, or a string is over 255 bytes |
| `"offline"` | Not connected to the server, or no player entity yet |
| `"not_main_thread"` | Called from a thread other than the game's main thread |
| `"engine_error"` | The engine failed: a null bundle, or a fault or C++ exception caught by the guard |

It never raises a Lua error on bad input, and nothing is sent when it
returns `nil`.

**Arguments.**

- **Numbers** are Lua numbers holding an integer in the field's range:
  -2^31 to 2^31-1 for `INT32` fields, 0 to 255 for `UINT8`. A numeric
  string such as `"5"` is not converted, and `2.5` is refused.
- **Strings** are Lua strings, sent as UTF-8, at most 255 bytes.
- **`search(opts)`**: `opts` is a table, or `nil` for all defaults. Keys:
  `sortId` (0–255), `clientKey` (`0` search, `1` my auctions, `2` my bids;
  anything else is `bad_args`), `sequenceId`, `bForward` (a number 0–255 or
  a boolean), `sellerName`, `bidderName`, `itemName`, `minTC`, `maxTC`,
  `quality`, `filterFlags`. A missing number is `0` and a missing string
  `""`, except `quality`, which defaults to `2000`, the client's own
  default. Unknown keys are ignored, and fields are read raw, so `opts`
  cannot rely on `__index`.
- **`create`**: `buyoutPrice` may be `nil` for no buyout. `auctionLength`
  is a `UIAuctionTime` value, `1` to `5` (the create form offers `3`, `4`
  and `5`). The DLL sends the `.def` order (item, buyout, length,
  starting), whatever order the Lua call takes them in.
- **`watch`**: `enable` follows Lua truthiness; a missing `enable` stops
  watching. The server answers watch calls with `WatchUnavailable` for now
  (D4).

**When the table appears.** The DLL checks for the table every 30 frames,
about twice a second, once the UI `lua_State` is up, and rebuilds it if a
UI reload cleared it. The check leaves an existing, current table alone, so
a reference the overlay keeps stays valid. The overlay should look
`CimmeriaBMNative` up when a button is pressed, not once when its file
loads: at load time the table may not exist yet. If it is still missing, the
DLL is not loaded, or its build check failed; the log says which.

**`techCompetency`** always returns `nil`. The plan's decision D7 asked for
a native getter, but the item-definition lookup behind the client's
`getItemDefInfo` goes through the cooked-data cache, can start a load for a
definition the client has not seen, and returns a reference-counted
pointer. None of that has been verified on a running client, so this build
does not call it. The overlay shows the column blank when the value is
`nil`.

## Log

Each line goes to `OutputDebugStringW` (visible in Sysinternals DebugView)
and to `cimmeria-client-patches.log` next to `SGW.exe`. The file is rewritten
at each launch, when the directory is writable. The log records:

- the fingerprint result for each site;
- each hook installed;
- the first, tenth, hundredth and further powers-of-ten occurrence of each
  outcome: claimed, delivered, dropped (queue full, no overlay, no handler),
  handler errors, and decode failures, each with its reason;
- each registration of `CimmeriaBMNative`, with the `lua_State` and whether
  it replaced a stale value, and any registration failure;
- each send, with the method, its cell index, the sub-index and the payload
  size, and each refusal, with the native, the reason and the detail (for
  example `offline (no ServerConnection)` or which argument was bad). Each
  of the first 100 sends and the first 100 refusals is logged, then powers
  of ten.

It stops after 2,000 lines.

## Running beside the telemetry DLL, and beside the old hand patch

Both DLLs hook `FEngineLoop::Tick` and the drop callee through MinHook. The
second DLL to hook chains onto the first, in either order. Before enabling a
hook, the DLL re-reads the prologue after MinHook has copied it, and
rebuilds the hook if the bytes changed in between. That narrows the race
between two DLLs hooking the same site at the same moment. The launcher
should still inject the two DLLs one after the other.

The hand-applied x64dbg Black Market patch (a `Tick` cave plus a cave at
`0x00c6fa8a`) should not be combined with this DLL. Both would open the
window on `onBMOpen`, and the network cave changes the drop path this DLL
relies on.

## Live checks owed

These map to V1–V4 in the evidence doc, and belong to the live-spike
packet (BM-00):

- The mangled `lua51.dll` exports resolve inside the running client. They
  were read from its export table, not exercised.
- The dispatcher and drop-callee pair sees `onBMOpen` with the stream
  attached, and `CimmeriaBM.onOpen` runs.
- The argument stream holds exactly one message's arguments. The claim
  refuses a call with bytes left over (`TrailingBytes` in the log), so if
  every call fails that way, the stream spans more than one message and the
  trailing-byte rule must go.
- `CimmeriaBMNative` appears in the UI Lua within a second of the UI
  coming up, and again after a UI reload.
- `CimmeriaBMNative.cancel(1)` reaches the server's cell dispatch as
  `0xBD`, sub-index 3 and a 4-byte payload, for the player's entity; the
  server logs the decode. The message's length field is filled in when the
  bundle closes, which was not traced statically.
- `search`, `create`, `bid` and `watch` each reach the server and decode
  with no trailing bytes.
- Called at character select or while disconnected, a native returns
  `nil, "offline"` and nothing is sent.
