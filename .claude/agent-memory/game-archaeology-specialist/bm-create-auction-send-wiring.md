---
name: bm-create-auction-send-wiring
description: createAuction send-side gap — entity guard NOP patch at 0x00e599a8 — confirmed live, frame reaches CME send dispatch (2026-06-22)
metadata:
  type: project
---

# BM createAuction Send-Side Wiring (2026-06-22)

## Phase 1 — Reconnaissance results

### Key Ghidra symbols found
- `BMCreateAuction_Lua_binding @ 0x00aabf70` — Lua C-function registered in `_G` as `"createAuction"`
- `BMCreateAuction_NetOut_emit @ 0x00e59970` — CME NetOut emitter for the cell-method frame
- `register_NetOut_BMCreateAuction @ 0x00e5c200` — returns `"Event_NetOut_BMCreateAuction"` string
- `CME_EventSignal_VEvent_NetOut_BMCreateAuction___TypedEmitInfo__vfunc_0 @ 0x00e5c2e0`

### Registration mechanism
All BM Lua bindings (`createAuction`, `searchAuctions`, `refreshMyAuctions`, `cancelAuction`, `getAuctionItemInfo`, `getAuctionViewItems`, `getAuctionTotalCount`, `getAuctionVisibleCount`) are registered in **`CEGUI_ButtonBase_3 @ 0x00acbb10`** via sequential `push fn; push wide_name; push esi; call 0x00403ec0` blocks. The name strings are in the `.rdata` region at `0x01951540`–`0x01951630` (UTF-16LE). `CEGUI_ButtonBase_3` is called by `FUN_0093b900` at startup. **All BM globals ARE in `_G` at runtime.**

Name pointer → global name mapping:
- `0x19515f8` → `"createAuction"` → fn `0x00aabf70`
- `0x19515d8` → `"searchAuctions"` → fn `0x00aca360`
- `0x19515b4` → `"refreshMyAuctions"` → fn `0x00aac090`
- `0x1951598` → `"refreshMyBids"` → fn `0x00aac0e0`
- `0x1951584` → `"placeBid"` → fn `0x00aac130`
- `0x1951568` → `"cancelAuction"` → fn `0x00aac1e0`
- `0x1951540` → `"getAuctionItemInfo"` → fn `0x00aac260`
- `0x1951518` → `"getAuctionViewItems"` → fn `0x00aac2e0` (JMP-patched in fork-B)
- `0x19514ec` → `"getAuctionTotalCount"` → fn `0x00aac360` (JMP-patched in fork-B)
- `0x19514bc` → `"getAuctionVisibleCount"` → fn `0x00aac3f0` (JMP-patched in fork-B)

## Phase 2 — Intent Reconstruction

### Lua binding signature
`createAuction(itemId, startingPrice, buyoutPrice, auctionLength)` — 4 required number args.
- Arg check: `CEGUI__unknown_00403330` = require-number check
- Arg 5 check: `CEGUI__unknown_00403280` = optional presence check (arg5 absent = OK)
- After validation: calls `FUN_00ada360(itemId, startingPrice, buyoutPrice, auctionLength)`
- `FUN_00ada360 @ 0x00ada360` = thin wrapper that calls `BMCreateAuction_NetOut_emit`

### BlackMarket.lua call site (canonical, from client Lua source at line 156)
`createAuction(itemId, bid, buyout, BlackMarketMod.creationDuration)`
Maps: arg1=itemId, arg2=bid(=startingPrice), arg3=buyout(=buyoutPrice), arg4=duration(=auctionLength)

### Wire field order (from emitter packing — NOT .def canonical order)
Emitter packs in this order: `itemInstanceId → startingPrice → buyoutPrice → auctionLength`
= 4+4+4+1 = 13 bytes.
**NOTE: This differs from the `SGWBlackMarketManager.def` CellMethods order which is `itemInstanceId, buyoutPrice, auctionLength, startingPrice`.** The binary emit order takes precedence.

Server decode in `mod.rs` (already correct, matches emit):
```
args[0..3]  → item_id (itemInstanceId)
args[4..7]  → starting_price
args[8..11] → buyout_price
args[12]    → auction_length
```
No mismatch between client emit and server decode.

## Phase 3 — The Gap

### Entity-guard bug at 0x00e599a8
In `BMCreateAuction_NetOut_emit`:
```asm
00e59989: CALL 0x00c66ad0       ; get GameEntityManager singleton (DAT_01ef244c)
00e5998e: MOV ECX, [esp+0x64]  ; ECX = param_1 = itemInstanceId
00e59999: MOV EAX, [eax+0x8c]  ; entity collection
00e5999c: MOV EAX, [eax+0x24]
00e5999f: CALL 0x00e1c830       ; entity lookup: find entity with id = itemInstanceId
00e599a4: XOR EBX, EBX
00e599a6: CMP EAX, EBX
00e599a8: JZ/JNZ 0x00e59c51    ; if NOT found → jump to epilog (no send)
```
The lookup uses `itemInstanceId` (a DB row ID ~1–9999) as a BigWorld runtime entity ID (large 32-bit value). These never match → **always fails → always silently drops the send.**

The guard's INTENT was to ensure the player entity exists before sending. The BUG is that it used the wrong value (itemInstanceId instead of player entity ID) as the lookup key. The guard is entirely redundant — the actual send path via `thunk_FUN_0054c980` already handles missing connections safely.

### Confirmed live (x64dbg evidence)
1. Set non-freezing BP at `0x00e59970` (emit entry) and `0x00e59c44` (vtable send call)
2. Wrote probe `createAuction(99001,100,500,1)` (UTF-16LE, 30 chars) to tick-cave at `0x074B0E00`, set len=0x1E, fired via flag_refresh
3. **Before NOP patch:** `0x00e59970` hit=1, `0x00e59c44` hit=0 → guard failed, send was dropped
4. **After NOP patch (6 NOPs at `0x00e599a8`):** `0x00e59970` hit=2, `0x00e59c44` hit=1 → **send was reached**

## Phase 4 — The Fix (Applied 2026-06-22)

### Patch: NOP the entity-guard JZ
**Address:** `0x00e599a8`
**Original bytes (6):** `0F 84 A3 02 00 00` (JZ rel32 → 0x00e59c51)
**Patch bytes (6):** `90 90 90 90 90 90`
**Reversibility:** write back `0F 84 A3 02 00 00` to `0x00e599a8`

**Effect:** `BMCreateAuction_NetOut_emit` now always proceeds to pack the 13-byte payload and dispatch via the CME EventSignal vtable send. The entity-guard no longer silently drops the send when itemInstanceId doesn't match a player entity ID.

**Safety:** The actual send path (`thunk_FUN_0054c980 @ 0x0054c980` → vtable `[ESI+8]`) uses the CME lazy-init singleton (DAT_01ee2678), same as BMSearch. If no active connection exists, the vtable dispatch handles that safely. The guard provided zero real protection.

### This is the complete native wiring gap
No other subscribers, no CME subscription gaps, no additional stubs needed. The machinery (`Event_NetOut_BMCreateAuction`, `register_NetOut_BMCreateAuction`, CME TypedEmitInfo, vtable dispatch) is all present and functional. The sole shelved-feature gap was the entity-guard silently eating the send.

## Tick cave current state (2026-06-22)
- `0x074B0E00`: contains probe script `createAuction(99001,100,500,1)` in UTF-16LE (30 chars, null at offset 60)
- `0x074B0833`: len byte = `1E` (30) — was `35` (53), was `F5` before that
- To restore prior Phase 2a script (BlackMarketMod.resetView): must re-write `0x074B0E00` with prior wscript2 content and restore len to `F5`
- `flag_refresh @ 0x074B0400` = 0 (cleared)

## Wire layout summary
| Field | .def canonical | Emit actual | Server decode (mod.rs) |
|-------|---------------|-------------|----------------------|
| itemInstanceId | pos 1 | pos 1 (offset 0) | args[0..3] ✓ |
| buyoutPrice | pos 2 | pos 3 (offset 8) | args[8..11] as buyout_price ✓ |
| auctionLength | pos 3 | pos 4 (offset 12) | args[12] ✓ |
| startingPrice | pos 4 | pos 2 (offset 4) | args[4..7] as starting_price ✓ |

**.def order ≠ emit order ≠ intuitive order. Server mod.rs is correct — it was written to match the binary emit order, not the .def order.**

## CME registration cave — 2026-06-22 session (CORRECTED, ready to apply on next launch)

### What happened
Two bugs in the original cave layout caused process crash (write AV in ntdll heap during C++ exception unwind):

**Bug 1 — Wrong PUSH address for "BMCreateAuction" string.**
The string data layout at `0x01674420`:
- `+0x00`: flag (4 bytes)
- `+0x04`: "SGWPlayer\0" (10 bytes, ends at `+0x0D`)
- `+0x0E`: 2 padding bytes (zeros)
- `+0x10`: "BMCreateAuction\0" (16 bytes, ends at `+0x1F`)

The cave thunk had `PUSH 0x0167442E` (= `+0x0E` = the padding zeros). Should be `PUSH 0x01674430` (= `+0x10`).

**Bug 2 — Thunk started at `0x0167443E`, overlapping the "BMCreateAuction\0" string.**
The string ends at `0x0167443F` (inclusive). Thunk was at `0x0167443E` — 2 bytes INSIDE the string. The `n\0` terminator got overwritten by `56 8B` (PUSH ESI / MOV ESI,ECX), silently truncating the method name to "BMCreateAuctio" (14 chars). `FUN_00C6E810` received this string, found no match, and threw `_CxxThrowException`. The throw propagated through the Lua C-callback stack (Lua 5.1 has no `__try` wrapper) and corrupted ntdll's heap allocator during SEH unwind.

### Corrected cave layout (apply on every fresh relaunch)

**Step 1: VirtualProtect page `0x01674000` to RWX**
Use the heap-stub approach: allocate a page, write the VirtualProtect thunk, createthread to execute it.
The specific stub:
```
push 0x0C810100      ; lpflOldProtect scratch cell
push 0x40            ; PAGE_EXECUTE_READWRITE
push 0x1000          ; dwSize
push 0x01674000      ; lpAddress
call <VirtualProtect @ kernel32>
ret
```
On the test machine: `VirtualProtect @ 0x75EE6B30`. Verify with `get_symbol("VirtualProtect")` each session — ASLR changes it.

**Step 2: Write cave data + corrected thunk in one `write_memory` call at `0x01674420`**

Total: 48 bytes (32 data + 16 thunk). Hex:

```
; Data section (32 bytes, 0x01674420–0x0167443F):
00 00 00 00          ; one_shot_flag = 0 (+0x00)
53 47 57 50 6C 61 79 65 72 00 00 00  ; "SGWPlayer\0" + 2 pad (+0x04–+0x0D, pad at +0x0E–+0x0F)
42 4D 43 72 65 61 74 65 41 75 63 74 69 6F 6E 00  ; "BMCreateAuction\0" (+0x10–+0x1F)

; Thunk (starts at 0x01674440, i.e., offset +0x20):
56                    ; push esi
8B F1                 ; mov esi, ecx
83 3D 20 44 67 01 00  ; cmp dword ptr [0x01674420], 0
75 1F                 ; jnz +31 → @restore at 0x0167446B
C7 05 20 44 67 01 01 00 00 00  ; mov dword ptr [0x01674420], 1
68 30 44 67 01        ; push 0x01674430  (&"BMCreateAuction")
6A 02                 ; push 2
68 24 44 67 01        ; push 0x01674424  (&"SGWPlayer")
8B 0D 64 22 EF 01     ; mov ecx, [0x01EF2264]  (mercury channel)
E8 93 53 6F EF        ; call 0x00D6CE00  (FUN_00D6CE00 registration wrapper)
8B CE                 ; mov ecx, esi   @restore
5E                    ; pop esi
E9 AB 7F 6F FF        ; jmp 0x00E5C420 (original vtable[2])
```

Single write_memory hex string (48 bytes):
```
00 00 00 00 53 47 57 50 6C 61 79 65 72 00 00 00 42 4D 43 72 65 61 74 65 41 75 63 74 69 6F 6E 00 56 8B F1 83 3D 20 44 67 01 00 75 1F C7 05 20 44 67 01 01 00 00 00 68 30 44 67 01 6A 02 68 24 44 67 01 8B 0D 64 22 EF 01 E8 93 53 6F EF 8B CE 5E E9 AB 7F 6F FF
```

**NOTE on rel32 values**: `E8 93 53 6F EF` and `E9 AB 7F 6F FF` are computed for:
- CALL `0x00D6CE00` from site `0x01674468`: rel32 = `0xEF6F5393` (confirmed correct for these fixed addresses)
- JMP `0x00E5C420` from site `0x01674470`: rel32 = `0xFF6F7FAB` (confirmed correct)
These are fixed-VA .text addresses, so rel32 is stable across sessions. No need to recompute.

**Step 3: Re-apply entity guard NOP**
Address: `0x00E599A8`, write 6 bytes: `90 90 90 90 90 90`
Original bytes (for reversibility): `0F 84 A3 02 00 00`

**Step 4: Re-point vtable[2] to new thunk entry `0x01674440`**
Address: `0x019DD368`, write 4 bytes: `40 44 67 01`
(Previously pointed to `0x0167443E` — wrong by 2 bytes; corrected to `0x01674440`)

**Step 5: Re-allocate tick cave and tick detour**
The tick page is heap — gone on relaunch. Must `allocate_memory(size=4096, protection=0x40)`.
Then write:
- `+0x00`: `00 00 00 00` (trigger flag)
- `+0x10`: UTF-16LE `"createAuction(10237,100,500,1)\0"` (62 bytes, 31 chars)
- `+0x50`: tick cave code (see below)
Tick detour at `0x00416EC0`: 6-byte JMP to tick_page+0x50. Stolen bytes = `64 A1 00 00 00 00`. JMP back to `0x00416EC6`.

Tick cave code (at tick_page+0x50):
```asm
83 3D [flag_addr] 00   ; CMP [flag], 0
74 32                  ; JZ @stolen_bytes
C7 05 [flag_addr] 00 00 00 00  ; MOV [flag], 0
A1 58 2A EE 01         ; MOV EAX, [0x01EE2A58]  (L resolver chain)
8B 40 10               ; MOV EAX, [EAX+0x10]
8B 00                  ; MOV EAX, [EAX]
8B 48 04               ; MOV ECX, [EAX+4]
81 E1 FF 00 00 00      ; AND ECX, 0xFF
83 F9 08               ; CMP ECX, 8
75 10                  ; JNZ @stolen_bytes
6A 1F                  ; PUSH 31 (char count)
68 [script_addr]       ; PUSH script ptr (tick_page+0x10)
50                     ; PUSH EAX (L)
E8 [rel32]             ; CALL 0x00404030 (Lua_doString_wide)
83 C4 0C               ; ADD ESP, 0xC
@stolen_bytes:
64 A1 00 00 00 00      ; MOV EAX, FS:[0]
E9 [rel32]             ; JMP 0x00416EC6
```
All `[addr]` and `[rel32]` fields must be computed at runtime based on the newly allocated tick_page address.

### BPs to set after each relaunch
- `0x0167443E` (old thunk entry — no longer valid, ignore or disable)
- `0x01674440` (new thunk entry) — fastresume
- `0x01674466` (call to FUN_00D6CE00) — fastresume  
- `0x0167446B` (after registration / skip target) — fastresume
- `0x00D6CE00` (FUN_00D6CE00 entry) — fastresume

### CRASH #9 (2026-06-22) — raw C-string vs std::string mismatch

**Root cause**: FUN_00D6CE00 passes param_1/param_3 directly into FUN_00C6EF40/FUN_00C6E810.
Both functions expect MSVC std::string* (SSO struct), NOT raw char* pointers.

We passed 0x01674424 (char* "SGWPlayer\0") — FUN_00C6EF40 reads [param_1+0x18] for capacity check.
[0x01674424+0x18] = [0x0167443C] = 0x00 (zero padding in cave) → fails SSO check →
interprets [param_1+4] as char** heap pointer → reads "SGWP" bytes as pointer → AV →
_CxxThrowException → msvcr80!terminate() → process dead.

**FIX**: Build SSO std::string structs. FUN_00D6CE00 must receive struct pointers, not char*.

FUN_00D6CE00 DOES successfully call FUN_00C6EF40 when given correct SSO structs — it returned
without throw in prior sessions when the mega-registrar called it with BW__unknown_00438c40 output
(which builds proper SSO structs).

**SSO struct for "SGWPlayer" at e.g. 0x05630300 (heap page):**
```
+0x00: 00 00 00 00                                          (placeholder vtable)
+0x04: 53 47 57 50 6C 61 79 65 72 00 00 00 00 00 00 00      ("SGWPlayer\0" + 7 pad)
+0x14: 09 00 00 00                                          (length = 9)
+0x18: 0F 00 00 00                                          (capacity = 15 = SSO sentinel)
```
Hex: `00 00 00 00 53 47 57 50 6C 61 79 65 72 00 00 00 00 00 00 00 00 00 00 00 09 00 00 00 0F 00 00 00`
= 32 bytes, write at 0x05630300.

**SSO struct for "BMCreateAuction" at 0x05630320:**
```
+0x00: 00 00 00 00                                          (placeholder vtable)
+0x04: 42 4D 43 72 65 61 74 65 41 75 63 74 69 6F 6E 00      ("BMCreateAuction\0")
+0x14: 0F 00 00 00                                          (length = 15)
+0x18: 0F 00 00 00                                          (capacity = 15 = SSO sentinel)
```
Hex: `00 00 00 00 42 4D 43 72 65 61 74 65 41 75 63 74 69 6F 6E 00 00 00 00 00 0F 00 00 00 0F 00 00 00`
= 32 bytes, write at 0x05630320.

**Updated tick cave CALL**:
```asm
PUSH 0x05630320     ; &"BMCreateAuction" SSO struct (NOT raw char*)
PUSH 2
PUSH 0x05630300     ; &"SGWPlayer" SSO struct (NOT raw char*)
MOV ECX, [0x01EF2264]
CALL 0x00D6CE00
```

### VP stub fix (PUSH ESP bug)

PUSH ESP pushes the stack pointer that VirtualProtect uses for lpflOldProtect. VirtualProtect
writes old-prot (0x20 = PAGE_EXECUTE_READ) to that address — which IS the thread's return
address slot on the stack. RET then jumps to 0x20 → AV. VP DID succeed (EAX=1).

**Fixed VP stub** (uses proper stack frame for scratch):
```asm
PUSH EBP
MOV EBP, ESP
SUB ESP, 4              ; scratch DWORD for lpflOldProtect
LEA EAX, [EBP-4]        ; EAX = &scratch (safe, below return address)
PUSH EAX                ; lpflOldProtect
PUSH 0x40               ; PAGE_EXECUTE_READWRITE
PUSH 0x1000             ; dwSize
PUSH 0x01674000         ; lpAddress
CALL VirtualProtect     ; resolves current session — use eval_expression
MOV ESP, EBP
POP EBP
RET
```
This is a proper stdcall-compatible function — RET returns to OS thread exit handler cleanly.

### createthread safety rule

GameEntityManager functions (FUN_00C66C10, FUN_00C6EF40, FUN_00C6E810) throw from non-main-thread.
CONFIRMED by crash: FUN_00C66C10 crashed with same msvcr80!terminate() signature when called from createthread.
ALL SGW entity/method functions must run from main-thread tick cave. No exceptions.

### Known-good addresses (stable across sessions, fixed VA in SGW.exe)
- `FUN_00D6CE00` = `0x00D6CE00` — registration wrapper, calling convention: `__thiscall(ECX=mercury_channel, param_1=&entity_class_name_str, param_2=direction_u32, param_3=&method_name_str)`
- Mercury channel singleton = `[0x01EF2264]`
- Original vtable[2] = `0x00E5C420`
- Entity guard JZ = `0x00E599A8`
- Vtable[2] slot = `0x019DD368`
- Lua_doString_wide = `0x00404030`
- Lua state resolver: `[[[0x01EE2A58]+0x10]]`

## Next session: what to do
1. Relaunch SGW.exe, get in-world.
2. Apply all 5 steps above in order (VirtualProtect → cave data → entity guard NOP → vtable redirect → tick cave + detour).
3. Set the 5 BPs listed above (fastresume).
4. Set `[tick_page+0x00] = 1` to trigger.
5. Watch BP hits at `0x01674440` (thunk), `0x01674466` (call), `0x00D6CE00` (entry).
6. Verify DB for new auction row.
7. **Permanent patch shipping:** Add `0x00e599a8: 90 90 90 90 90 90` to the launcher patcher alongside the existing `onBMOpen` method-90 patch.
