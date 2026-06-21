# Black Market Client Window — Runtime Patch (Restoration)

**Confidence**: HIGH (owner-confirmed working in-world, 2026-06-21)
**Date**: 2026-06-21
**Sources**: `SGW.exe` — Ghidra static analysis + x64dbg live tracing against the running, server-connected client. Companion to [`black-market-restoration.md`](black-market-restoration.md) (server side) and [`black-market-wire-formats.md`](black-market-wire-formats.md) (wire).

## Summary

The Black Market window never opened in the stock client, even though the server correctly sends `onBMOpen` on every auctioneer interaction. The cause is a **shelved-feature gap in the client**: the BM client-method handlers were never bound into the player entity's dispatch map, so the incoming call is silently dropped. The window UI itself (CEGUI layouts + `BlackMarket.lua`) is fully built.

It is restored with a small **runtime binary patch** of the client process — a *deferred wide-Lua-injection*. Confirmed: interacting with the in-world auctioneer opens the full Black Market window (Search / My Auctions / My Bids).

> **Scope of the current patch:** it revives **only method 90** (`onBMOpen`), so the window *chrome* opens but its tabs render empty — the five data/notification methods (91–95) are still silently dropped. A complete restoration must bind all six (see *The full surface* below).

> The patch is currently applied by hand via x64dbg (in-memory, lost on client close). Shipping it is tracked by the launcher-integration issue (see **Implementation Impact**).

## Root cause

1. Server sends `onBMOpen` = the player's client-method **90** (`SGWBlackMarketManager` is the 10th `<Implements>` interface; its 6 client methods occupy indices **90–95**, calibrated against the working `ContactListManager` at 85–89 and `onDialogDisplay` at 105).
2. Incoming entity methods are routed by the universal client dispatcher **`Client_NetIn_EntityMethodDispatch` @ `0x00c6f8f0`** (renamed from `FUN_00c6f8f0`; runs on a **network thread**, tid ≠ main). It reads the method index and searches the entity description's red-black **method-handler map** at `desc+0xe0`, keyed by `(componentKey = *(desc+0x1e), methodIndex)`, walking the type hierarchy.
3. **The BM methods get array indices but have no map node.** Their `Event_NetIn_BM*` descriptors exist in `.rdata` and are well-formed (identical CME type to the working dialog/contact events) but were **never bound** into the map — the feature was shelved before final wiring. So method 90 falls to the **silent-drop path at `0x00c6fa8a`** (`FUN_01590f30(desc+0xe0, idx)` then `return`). **All six BM methods (90–95) share this gap** — not just 90 (confirmed byte-level below).

**Live proof**: a non-freezing log breakpoint at `0x00c6fa8a` logged `idx=5A` (= 90) exactly once per auctioneer interaction, while `ContactList` (85–89) and `DialogDisplay` (105) dispatch normally through the same machinery.

## The patch — deferred wide-Lua-injection

The window opens from Lua: `BlackMarketMod.onBMOpen()` → `BlackMarketWin:show()` (subscribed to `Events.BMOpen`). So instead of repairing the opaque native binding, we drive that Lua directly. Two constraints force the shape of the patch (each one cost a crash during development):

- **The client's Lua API is WIDE.** `Lua_doString_wide` (`0x00404030`, renamed from `FUN_00404030`) is `luaL_loadbuffer(L, wbuf, len, wname) + lua_pcall(L,0,0,0)` where `wbuf`/`wname` are **UTF-16LE** and `len` is the **character count** (not bytes; the engine's own init call passes an odd `len`). A narrow ASCII buffer is parsed as garbage → corrupt bytecode → a deterministic crash deep in the VM (wild `EIP`).
- **The dispatcher is a network thread.** Calling Lua there races the main thread's VM and crashes (the working events are marshaled to the main thread before their Lua runs). So the actual Lua call must happen on the **main thread**.

Resolution: the network thread only sets a flag; the main thread's per-frame tick consumes it.

### Cave 1 — network (hook at the drop path `0x00c6fa8a`)
Detour (5 bytes) over `mov edx,[esp+0x10]; push edx` → cave:
```asm
cmp dword [esp+0x10], 0x5A      ; method 90 (onBMOpen)?
jne  .passthrough
mov  dword [flag], 1            ; atomic write — safe on any thread
.passthrough:
mov  edx, [esp+0x10]            ; replay overwritten instrs
push edx
jmp  0x00c6fa8f                 ; back into the original drop handler
```

### Cave 2 — main thread (hook at `FEngineLoop::Tick` `0x00416ec0`)
Detour (6 bytes: `jmp rel32` + `nop`) over the first instruction `mov eax, fs:[0]` → cave:
```asm
cmp  dword [flag], 1
jne  .passthrough
mov  dword [flag], 0            ; one-shot per interact
pushad
mov  eax, [0x01ee2a58]          ; g_SGWUIManager_ptr
test eax, eax
jz   .done
mov  eax, [eax+0x10]            ; holder
test eax, eax
jz   .done
mov  eax, [eax]                 ; L (UI lua_State); validate [L+4]==0x08 (LUA_TTHREAD)
test eax, eax
jz   .done
push <wname "bm">              ; UTF-16LE
push 0x19                       ; len = 25 CHARACTERS
push <wscript>                 ; UTF-16LE "BlackMarketMod.onBMOpen()"
push eax                        ; L
mov  eax, 0x00404030            ; Lua_doString_wide
call eax
add  esp, 0x10
.done:
popad
.passthrough:
mov  eax, fs:[0]               ; replay overwritten instr
jmp  0x00416ec6                 ; back into FEngineLoop::Tick
```

`Lua state L = *(*(*(0x01ee2a58) + 0x10))`. `0x01ee2a58` (`g_SGWUIManager_ptr`) holds the `SGWUIManager` singleton; `+0x10` holds a pointer to a 4-byte holder of `L`. This is the VM that owns the `Events`/`Commands`/`Actions` namespaces — i.e. where `BlackMarketMod` lives.

## Key addresses (`SGW.exe`, QA build)

| Address | Symbol | Role |
|---|---|---|
| `0x00c6f8f0` | `Client_NetIn_EntityMethodDispatch` | Universal incoming entity-method dispatcher (network thread) |
| `0x00c6fa8a` | — | Silent-drop path; **network cave hook** (method idx at `[esp+0x10]`) |
| `0x00416ec0` | `FEngineLoop::Tick` | Main-thread per-frame loop; **tick cave hook** |
| `0x00404030` | `Lua_doString_wide` | `luaL_loadbuffer`(wide)+`lua_pcall`; the window-open primitive |
| `0x0166ef4e` / `0x013a7830` | `luaL_loadbuffer` / `lua_pcall` | Lua C API |
| `0x01ee2a58` | `g_SGWUIManager_ptr` | Singleton ptr; `L = *(*(*+0x10))` |
| `0x00d83ec0` | `register_NetIn_BMOpen` | BM `onBMOpen` event descriptor — present, never bound |

Wide script payload: `"BlackMarketMod.onBMOpen()"` (25 chars / 50 bytes UTF-16LE), chunk name `"bm"`. Method indices: `onBMOpen=90 (0x5A)`, `onBMError=91`, `onBMAuctions=92`, `onBMAuctionRemove=93`, `onBMAuctionUpdate=94`, `onBMWatchedItemsUpdate=95`.

## Evidence / RE journey (condensed)

- Index calibration from the client `.def`s + working features pinned `onBMOpen` at 90.
- Found the dispatcher by breakpointing the *working* dialog handler shim and reading its return address (`0x00c6fc05`, inside `Client_NetIn_EntityMethodDispatch`).
- Decompile of the dispatcher exposed the map search + the silent-drop fallback; live log BP confirmed `idx=5A` dropped per interact.
- Disproved two earlier theories: "UI abandoned" (UI is complete) and "`on`-prefix name mismatch" (`Event_NetIn_DialogDisplay` has the same mismatch and works).
- Thread check (`fs:[0x24]` at the drop path vs the main thread) proved the dispatcher is a network thread.
- The wide-string requirement was found by reading the engine's *own* working `Lua_doString_wide` call args (`0x01DE9C70` = UTF-16 `"CEGUI.Point = …"`, name `"tolua: embed"`, odd `len`).

## Critical gotchas (carry forward to any client-patch work)

- **Wide Lua strings**: UTF-16LE buffers, `len` = character count. Narrow → deterministic VM crash.
- **Network vs main thread**: never call Lua / touch the VM from `Client_NetIn_EntityMethodDispatch`; defer to `FEngineLoop::Tick`.
- **x64dbg log BPs**: `fastresume` *suppresses* the log callback — set it to `0`; `breakif 0` alone keeps it non-freezing. Log format `{[esp+0x10]}` works; `{x:[esp]}` does not.

## Implementation impact

- **Ship via launcher (chosen path)**: the launcher applies this patch to the `SGW.exe` process at startup — allocate an RWX cave, write the wide strings + flag + both caves, then write the two detours. Cave-internal absolute addresses and the detour `rel32`s must be computed for the runtime cave base. `L` is resolved at runtime (validate `[L+4]==0x08`). Tracked by the launcher-integration issue.
- **Alternative — on-disk exe patch**: relocate the two caves + strings into unused space inside `SGW.exe` at fixed VAs and write the two detours; the patched exe persists without launcher code.
- **Addresses are build-specific** to this client; re-resolve symbols before applying to any other `SGW.exe` build.
- **Server side** (PR #586 / #571) is independent and proven against the open window; the recovered values still to finalize: `createAuction` decode field order (`item, buyout, length, starting`), `EBlackMarketError` = `1/2`, `auctionLength` = `EBlackMarketTime` 1–5, `nextMinBid` = server-provided.

## The full surface — all six BM methods (90–95)

`SGWBlackMarketManager` contributes six server→client (NetIn) methods. Confirmed live by walking the player method vector and reading each `MethodDescription` name (`std::string` SSO at `+0x4`; ≥16-char names are a heap pointer at `+0x4`):

| Idx | Method | Args | Role | Status |
|---|---|---|---|---|
| 90 | `onBMOpen` | `INT32 entityId` | Open the window; bind to the auctioneer | **revived** (Lua-injection) |
| 91 | `onBMError` | `INT32 errorId` | Show an error (funds / bid-too-low / gone) | dropped |
| 92 | `onBMAuctions` | `ARRAY<AuctionItem>` | The search-result listing — the main data payload | dropped |
| 93 | `onBMAuctionRemove` | `INT32 sequenceId` | Remove a row (sold / cancelled / expired) | dropped |
| 94 | `onBMAuctionUpdate` | `AuctionItem` | Add/update one row (new bid / new listing) | dropped |
| 95 | `onBMWatchedItemsUpdate` | `ARRAY<INT32>` | Update the watched-items set | dropped |

All six are parsed + Exposed but unbound — the *same* gap. The current patch revives only **90**, so the window opens but stays empty: `onBMAuctions` (92) and `onBMAuctionUpdate` (94) — the actual listing data — never reach the client.

**Why 90 is the easy one:** its only arg (`entityId`) is unused by the Lua open, so a **no-arg** Lua call (`BlackMarketMod.onBMOpen()`) suffices. **91–95 carry data** — some complex (`ARRAY<AuctionItem>`) — so they cannot be revived by a no-arg call; their wire args must be **decoded** first. That asymmetry decides the implementation strategy below.

## Byte-level confirmation & native-binding analysis (2026-06-21 dig)

A follow-up dig (Ghidra build-path trace + live reads of the loaded descriptions) pinned the gap exactly and answered "can we bind it natively instead of injecting Lua?"

**`onBMOpen` is byte-identical to the working `onDialogDisplay`.** Reading the player desc's `MethodDescription` for both (`GameEntityManager [0x01ef244c]` → `+0x90` EntityDescriptionMap → desc array `[EDM+0x10..+0x14]`, stride `0x110` → client-Methods object at `desc+0xe0` → vector `[desc+0xf0]`, `0x50` stride):

| Field | `onBMOpen` [90] | `onDialogDisplay` [105] |
|---|---|---|
| name (`std::string` @ +0x4) | `onBMOpen` | `onDialogDisplay` |
| flags (+0x1c) | `4` (**Exposed**) | `4` (**Exposed**) |
| exposed ordinal (+0x44) | `90` | `105` |
| sentinel (+0x48) | `0xFFFFFFFF` | `0xFFFFFFFF` |
| DetailDistance (+0x4c) | `FLT_MAX` | `FLT_MAX` |

They differ only in name / ordinal / arg count — **no per-method flag** distinguishes the dropped method from the working one. (Both player-class descs — array indices 2 and 3, componentKey `*(desc+0x1e)` = 2/3, 157 and 163 client methods — carry `onBMOpen` at index 90.) This eliminates every alternative theory: **not** a parse/`<Implements>` failure (it's parsed + named at 90), **not** a server-side index mismatch (the client's method 90 *is* `onBMOpen`), **not** a malformed/`ServerOnly` descriptor (identical to dialog).

**The `Event_NetIn_BM*` descriptors are inert.** Hardware read-watches on the `onBMOpen` (`0x019c9108`) and `onDialogDisplay` (`0x019bb4e0`) descriptors, armed from process start, **never fired** through login + world-entry. With no static code xrefs either, the descriptors are vestigial type-info — the dispatch binding is **not** descriptor-driven.

**The gap is purely a missing incoming-event subscriber.** A method's dispatch node is created when a handler *subscribes* to its incoming event (`CmeEventSignal_Subscribe @ 0x00a5c150`). `onDialogDisplay` has one; the BM methods do not — matching the separate finding that BM has the `TypedEmitInfo` event-*type* but no `CallbackImpl` subscriber-glue (see [`architectural-anomalies.md`](architectural-anomalies.md)). The BM window was wired to the **Lua** `Events.BMOpen`, never to the NetIn events.

**Native binding — reachable, but not a shortcut for *opening*.** Subscribing a handler would create the dispatch node so the method dispatches natively. For `onBMOpen` alone it is *more* work than the Lua-injection (you'd construct the signal + `CallbackImpl`, and the handler would still have to open the window — exactly what the injection already does). The **general key** is real, though: shelved client methods are "perfect but unsubscribed," so subscribing revives native dispatch for any of them — and crucially, **the native path gets the engine's arg-decoding for free**, which is what the data methods (91–95) need.

### Registry walk (2026-06-21): the BM signals are entirely unregistered

Confirmed the hard way with a live walk of the **CME signal registry** (`BW__unknown_0155f790` → std::map at `0x01f11fc4`, **723 events**, name-sorted; `CmeEventSignal_LookupByName @ 0x00a5c0f0` returns 0 on miss): **no `Event_NetIn_onBM*` signal exists at all.** Tracing toward `onBMOpen` lands in the *empty* gap between `onArchetypeUpdate` and `onBeginAidWait` (a nil child), and the whole `onBM*` family sorts into that same gap — so none of the six are registered. Every non-BM neighbour (`onActiveSlotUpdate`, `onArchetypeUpdate`, `onBeginAidWait`, `onBeingNameIDUpdate`, `onCharacterLoadFailed`, `onErrorCode`, `onPlayerDataLoaded`, …) is present; the registry is healthy, BM was simply never wired in. So it's not "registered without a subscriber" — the **signal itself is absent**.

This sharpens the native-binding cost **and** its risk. The dispatcher's found-path calls the lookup and **dereferences the result unconditionally** (`0x00c6fbf9: EDX=[signal]`), so a bare dispatch node whose eventKey doesn't resolve is a **guaranteed null-deref crash** — there is no inert "found but does nothing" node. Native binding therefore requires, in order:

1. **Register a CME signal** for the method — construct a signal object (a vtable whose `+8` shim opens the window, deferred to the main thread) and **insert it into the `0x01f11fc4` registry** (one red-black-tree splice).
2. **Splice the dispatch node** `(componentKey, methodIndex)` into the per-entity tree at `this+8` (a second red-black-tree splice), with `+0x18` pointing at the registered name.

That's two live RB-tree inserts plus object construction on the server-connected client — the full Pattern-A wiring BM's developers omitted. For `onBMOpen` it is strictly more work and more crash-risk than the proven Lua-injection, for the identical result, so a "quick" native test of method 90 is **not** a quick node-splice: a bare node would null-deref on the next auctioneer interact.

## Shipping the full feature without repeated patching

The manual x64dbg patching is a **development convenience**, not the shipping model. The delivered feature is a **single patch applied automatically at every client launch** — you never hand-patch the running game.

- **One install, every launch.** The launcher (or an on-disk exe patch) writes the cave(s) + detour(s) once at startup; thereafter it just works. The "constant patching" seen during RE is only re-attaching x64dbg each dev session.
- **Cover all six in that one install.** Two shapes:
  - **(a) Native binding — recommended for 91–95.** Subscribe handlers for 90–95 via `CmeEventSignal_Subscribe @ 0x00a5c150`. The engine's normal path then decodes each method's wire args (from the `MethodDescription` arg types that already exist) and hands them to the handler, which forwards to `BlackMarketMod.onBMXxx(args)`. This is the only approach that decodes the data methods correctly without re-implementing the wire parser.
  - **(b) Dispatcher marshaling cave.** Extend the network cave to catch idx ∈ [90, 95]. Trivial for 90 (no args); for 91–95 you'd have to replicate the engine's arg-decode in the cave — much more work and fragile. Best reserved for 90.
  - A **hybrid** is fine: keep the proven Lua-injection for 90, add native subscribers for 91–95.
- **Server side is already real** (PR #586 / #571): all six are sent correctly; only the client binding is missing.

**Bottom line:** yes — implementable as a one-time, auto-applied client patch. The remaining work is (1) extend coverage from method 90 to 90–95 (prefer native binding so the data methods decode), and (2) finish the launcher integration so it applies on every launch.

## Ghidra annotations applied

`Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`) + plate comment; `Lua_doString_wide` (`0x00404030`) + plate comment; `g_SGWUIManager_ptr` label (`0x01ee2a58`); disassembly comments at the two hook sites; `register_NetIn_BMOpen` plate comment ("descriptor present, never bound").
