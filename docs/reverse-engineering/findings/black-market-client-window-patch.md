# Black Market Client Window — Runtime Patch (Restoration)

**Confidence**: HIGH (owner-confirmed working in-world, 2026-06-21)
**Date**: 2026-06-21
**Sources**: `SGW.exe` — Ghidra static analysis + x64dbg live tracing against the running, server-connected client. Companion to [`black-market-restoration.md`](black-market-restoration.md) (server side) and [`black-market-wire-formats.md`](black-market-wire-formats.md) (wire).

## Summary

The Black Market window never opened in the stock client, even though the server correctly sends `onBMOpen` on every auctioneer interaction. The cause is a **shelved-feature gap in the client**: the BM client-method handlers were never bound into the player entity's dispatch map, so the incoming call is silently dropped. The window UI itself (CEGUI layouts + `BlackMarket.lua`) is fully built.

It is restored with a small **runtime binary patch** of the client process — a *deferred wide-Lua-injection*. Confirmed: interacting with the in-world auctioneer opens the full Black Market window (Search / My Auctions / My Bids).

> The patch is currently applied by hand via x64dbg (in-memory, lost on client close). Shipping it is tracked by the launcher-integration issue (see **Implementation Impact**).

## Root cause

1. Server sends `onBMOpen` = the player's client-method **90** (`SGWBlackMarketManager` is the 10th `<Implements>` interface; its 6 client methods occupy indices **90–95**, calibrated against the working `ContactListManager` at 85–89 and `onDialogDisplay` at 105).
2. Incoming entity methods are routed by the universal client dispatcher **`Client_NetIn_EntityMethodDispatch` @ `0x00c6f8f0`** (renamed from `FUN_00c6f8f0`; runs on a **network thread**, tid ≠ main). It reads the method index and searches the entity description's red-black **method-handler map** at `desc+0xe0`, keyed by `(componentKey = *(desc+0x1e), methodIndex)`, walking the type hierarchy.
3. **The BM methods get array indices but have no map node.** Their `Event_NetIn_BM*` descriptors exist in `.rdata` and are well-formed (identical CME type to the working dialog/contact events) but were **never bound** into the map — the feature was shelved before final wiring. So method 90 falls to the **silent-drop path at `0x00c6fa8a`** (`FUN_01590f30(desc+0xe0, idx)` then `return`).

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

## Ghidra annotations applied

`Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`) + plate comment; `Lua_doString_wide` (`0x00404030`) + plate comment; `g_SGWUIManager_ptr` label (`0x01ee2a58`); disassembly comments at the two hook sites; `register_NetIn_BMOpen` plate comment ("descriptor present, never bound").
