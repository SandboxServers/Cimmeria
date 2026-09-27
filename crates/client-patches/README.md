# cimmeria-client-patches

An injected DLL that restores client features the 2009 `SGW.exe` shipped
unfinished. It is built as a `cdylib` for the **32-bit client process**, plus
an `rlib` so the host can run the unit tests of everything that is not raw
FFI. It is separate from `cimmeria-client-telemetry` on purpose: gameplay must
not depend on the telemetry opt-in. The decision record is
[docs/architecture/client-patches.md](../../docs/architecture/client-patches.md).

**Status:** the Black Market **receive** path (build check, receive hooks,
decode, main-thread delivery to Lua). Sending the cell methods 61–66 comes
next. Nothing loads this DLL yet: the launcher's always-inject step is a
later packet, and nothing here has run inside a live client. The live
checks still owed are listed at the end.

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
   everything after the jump still matches, the hook chains on top.
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
| `0x00dd6a60` | `ServerConnection::startEntityMessage` | fingerprinted only; the send side will call it |
| `0x01ef244c` | `GameEntityManager*`; local player id at `+0x14` | compared with `Entity + 0x0c` |
| `0x01ee2a58` | `g_SGWUIManager_ptr`; UI `lua_State` = `*(*(*(p) + 0x10))`, tag byte `+4 == 8` | delivery |

## The Lua contract (for the UI overlay)

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

## Log

Each line goes to `OutputDebugStringW` (visible in Sysinternals DebugView)
and to `cimmeria-client-patches.log` next to `SGW.exe`. The file is rewritten
at each launch, when the directory is writable. The log records:

- the fingerprint result for each site;
- each hook installed;
- the first, tenth, hundredth and further powers-of-ten occurrence of each
  outcome: claimed, delivered, dropped (queue full, no overlay, no handler),
  handler errors, and decode failures, each with its reason.

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

These map to V2 and V3 in the evidence doc, and belong to the live-spike
packet:

- The mangled `lua51.dll` exports resolve inside the running client. They
  were read from its export table, not exercised.
- The dispatcher and drop-callee pair sees `onBMOpen` with the stream
  attached, and `CimmeriaBM.onOpen` runs.
- The argument stream holds exactly one message's arguments. The claim
  refuses a call with bytes left over (`TrailingBytes` in the log), so if
  every call fails that way, the stream spans more than one message and the
  trailing-byte rule must go.
