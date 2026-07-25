---
name: bm-fork-b-session-crash-notes
description: Black Market fork-B session crash postmortem + Phase 2a safe read-repoint proof (2026-06-22)
metadata:
  type: project
---

# BM Fork-B Session Crash Notes (2026-06-21) + Phase 2a (2026-06-22)

## What crashed

Client crashed during Phase A/B work on #571. A flag3 handler at `0x34CA0400` called `0x00403EC0` (partial lua_register helper) WITHOUT first pushing `_G` onto the Lua stack. `lua_rawset(L, -3)` inside `0x00403EC0` accessed an invalid stack index → access violation at `0x61233988` in lua51.dll.

**Why:** `0x00403EC0` is NOT a standalone lua_register. It is a partial helper that requires the caller to have already pushed a namespace/global table onto the Lua stack at position -3 relative to the key+value it will push.

## Correct C-function registration sequence

To register a C function as a Lua global from native code:

```asm
; Step 1: push _G onto Lua stack
push 0x00000000   ; null name = "use global table"
push L            ; push L
call 0x00403BB0   ; lua_getglobal_wide(L, NULL) → pushes _G; add esp,8

; Step 2: for each function to register:
push fn_addr      ; cfunc pointer
push wide_name    ; wide string pointer (L"functionName")  
push L
call 0x00403EC0   ; rawset _G[name] = cfunc; add esp,0xC

; Step 3: pop _G from Lua stack
push 1            ; n=1
push L
call lua_pop      ; lua_pop(L, 1); add esp,8
```

`lua_pop(L, n)` = `lua_settop(L, -(n)-1)`. Need `lua_settop` address from thunk table. 
Likely near `0x013a77xx` range. Search: `lua_settop` takes (L, idx) = 2 args.

## What was built (persists in memory even after client crash)

All cave contents at:
- `0x34C90000`: store header + test record (count=1, auctionId=99, buyout=100)
- `0x34C90800`: method-92 callback (reads wire stream, writes to store)
- `0x34C90900`: method-92 dispatch node (linked as right child of method-90 node at 0x157B0000)
- `0x34C90940`: CustomMD for method-92 
- `0x34CA0000`: fn1 = getAuctionTotalCount/getAuctionVisibleCount
- `0x34CA0049`: fn2 = getAuctionViewItems
- `0x34CA0100`: fn3 = getAuctionItemInfo (693 bytes, returns table from store record)
- `0x34CA0400`: flag3 handler (BROKEN — disabled by patching cmp target to 0xFF at 0x34CA0406)
- `0x157C02F2`: redirected from tick_back to 0x34CA0400 (flag3 handler)

## JMP patches applied to game exe

| Address | Original | Patched to |
|---------|----------|-----------|
| 0x00aac260 (getAuctionItemInfo) | 83EC0C568B | E99B3E1F34 → fn3 |
| 0x00aac360 (getAuctionTotalCount) | 83EC0C568B | E99B3C1F34 → fn1 |
| 0x00aac3f0 (getAuctionVisibleCount) | 83EC0C568B | E90B3C1F34 → fn1 |
| 0x00aac2e0 (getAuctionViewItems) | 83EC0C568B | E9643D1F34 → fn2 |

## Lua shadow problem (STILL UNSOLVED)

Phase A wscript2 Lua script set getAuctionViewItems/TotalCount/VisibleCount/ItemInfo as Lua closures in `_G`. These closures shadow the C function registrations. The JMP patches only affect the C function bodies — when Lua looks up `_G["getAuctionViewItems"]`, it finds the closure, NOT the C function.

## Fix for next session

Option A (recommended): Fix flag3 handler to use correct sequence:
1. `push 0; push L; call 0x00403BB0` (push _G)
2. x4 `push fn; push name; push L; call 0x00403EC0` (register each fn)
3. `push 1; push L; call lua_settop(L, -2)` (pop _G)
4. Then optionally run a Lua refresh script

First need: lua_settop address. Check thunk table near 0x013a77xx.

Option B (simpler): Instead of using the C registration APIs, run a Lua doString that calls the ORIGINAL registration function. The original registration code ran at engine startup. But we can't call it again easily.

Option C (simplest for testing): Skip shadow fix entirely. Use wscript2 update approach: when flag_refresh=1, run a new Lua script that re-overrides the closures WITH NEW CLOSURES that read from our store at known addresses. Pass auctionIds etc. as Lua literals. This sidesteps the entire C registration issue.

## Key addresses confirmed

- 0x00403BB0: lua_getglobal_wide(L, name) — when name=NULL pushes _G
- 0x00403EC0: partial lua_rawset helper — requires _G already on stack at -3
- 0x0166ef42: lua_pushcclosure(L, cfunc, 0) = lua_pushcfunction(L, fn)
- 0x0166eeee: lua_setfield-like (called with -2 idx in setglobal wrapper)
- 0x013a780c: UNKNOWN — tail-called by 0x00403BB0 when name=NULL
- 0x34C90020: flag_refresh (set by method-92 callback, cleared by tick cave)
- 0x34CA0406: cmp target byte patched to 0xFF to disable broken flag3 handler

## Phase 2a — Root cause found + fixed (2026-06-22, AWAITING OWNER CONFIRM)

### Root cause of empty render (CONFIRMED via Ghidra + math)

**`UIAuctionView.SearchResults == 0` (confirmed).** Registered via `CEGUI__unknown_00403f20` at `0x00ad3a5e`; getter at `0x00aa2160` executes `FLDZ` (push +0.0), calls `lua_pushnumber(L, 0.0)`. The `v==0` guard in the old wscript2 was therefore CORRECT for the enum value.

**Actual bug: `len` mismatch in `push 0x1E8` (488) vs actual script length of 485 chars.** The old wscript2 ended at `0x074B07D9`, making it 485 chars (970 bytes). But the cave pushed `len=488`. `luaL_loadbuffer` read 3 extra null bytes past the script content. Lua 5.1 treats `\0` in source as an invalid character outside string literals — the extra nulls caused a parse error. The chunk never compiled, `lua_pcall` never ran, the closures were never defined in `_G`, and `resetView` ran against the original unmodified C bindings which returned empty data. The flag was consumed (no crash) because the cave's Lua call returned an error code silently.

**UIAuctionView enum members (all from `CEGUI_ButtonBase_3` @ `0x00ad3a2c`):**
| Member | Getter addr | FPU insn | Lua value |
|--------|-------------|----------|-----------|
| `SearchResults` | `0x00aa2160` | `FLDZ` | `0` |
| `MyAuctions` | `0x00aa2360` | `FLD1` | `1` |
| `MyBids` | `0x00aa???? ` | `FLD1+1?` | `2` (inferred) |

**Accessors are NOT viewType-agnostic by necessity** — `UIAuctionView.SearchResults == 0` so the old `v==0` check was correct. The fix was NOT removing the viewType guard; the fix was correcting the `len` mismatch.

### Fix applied (2026-06-22, this session)

New wscript2 written to `0x074B0410`, **501 chars** (`0x1F5`), null terminator at `0x074B07FA`. The viewType guard was made unconditional (`return {99}` for all v) as an extra safety measure, and a diagnostic `BlackMarketMod.x=tostring(UIAuctionView.SearchResults)` was added. Extension cave `push len` patched: `0x074B0833`: `E8` → `F5` (488 → 501). `flag_refresh` fired and consumed.

### Current cave layout (live, this session):

- `0x074B0400` — `flag_refresh` (dword, currently 0)
- `0x074B0408` — wname2 = wide "bm2"
- `0x074B0410`–`0x074B07F9` — wscript2 = 501-char wide Lua script (1002 bytes, NO trailing nulls inside buffer window)
- `0x074B07FA` — wide null terminator (2 bytes, outside the `len=501` window)
- `0x074B0800`–`0x074B0852` — extension cave: push `0x074B0408`, push `0x1F5` (501), push `0x074B0410`, push L, call `0x00404030`, then replay `mov eax,fs:[0]` + jmp `0x00416EC6`
- `0x074B0833` — patched to `F5` (was `E8`): `push 0x1F5` (was `push 0x1E8`)

### Reversibility:
- `0x074B0833`: write `E8` to revert len back to 488 (re-introduces the parse bug, don't do this)
- `0x074B029B`: write `E9 26 6C F6 F8` to disconnect the extension cave from the tick chain

### BlackMarket.lua accessor field names (canonical from client source):
- `getAuctionViewItems(viewType)` returns 1-indexed array of auctionId ints
- `getAuctionTotalCount(viewType)` / `getAuctionVisibleCount(viewType)` return integers
- `getAuctionItemInfo(id)` returns table with fields: `auctionId`, `itemId`, `icon`, `stackSize`, `name`, `techCompentancy` (with typo), `currentBid`, `buyoutPrice`, `nextBidPrice`, `bidderName`, `sellerName`, `bidCount`
- View refresh entry point: `BlackMarketMod.resetView(UIAuctionView.SearchResults)` — calls getAuctionViewItems(0) + refreshes rows
- `rowWin` is `_G['BlackMarket_Search' .. rowId .. 'MainContainer']`; gate is `if itemInfo.itemId and rowWin`

### Owner visual confirmation pending.

## Next session: CONNECT FIRST

After client restart, ALL cave addresses (0x074B0000, 0x074B0400, 0x074B0800) survive only while the process runs. The tick cave page 0x074B0000 was allocated via x64dbg in an earlier session and is the parent page. If the client is restarted, all caves are lost and the onBMOpen patch + all Phase 2a data must be rewritten from scratch.

**Why:** `0x00403EC0` is NOT a standalone lua_register — it is a partial registration helper that requires the caller to have already pushed a namespace table onto the Lua stack first.
**How to apply:** Never call `0x00403EC0` without first calling `0x00403BB0(L, NULL)` to push `_G`.
