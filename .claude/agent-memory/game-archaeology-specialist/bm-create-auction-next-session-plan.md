---
name: bm-create-auction-next-session-plan
description: FINAL approved execution plan for createAuction send-path — byte-exact cave layout, sequence, abort gates, rendezvous check; execute on next relaunch
metadata:
  type: project
---

# BM createAuction — Final Execution Plan (team-lead approved 2026-06-22)

## Status
Client: DEAD (crash #9 — msvcr80!terminate from raw C-string vs SSO mismatch). PID 173976 unrecoverable.
All heap pages gone on relaunch. Fixed .text cave at 0x01674420 needs full rewrite.

## BLOCKER — awaiting team-lead decision on FUN_00C6EA70

Full disassembly of FUN_00C6EA70 @ 0x00C6EA70 confirmed it THROWS on a first registration (EMPTY slot → `_CxxThrowException`). It only returns cleanly when the slot is ALREADY occupied (duplicate guard). Calling it for BMCreateAuction on a fresh client = crash.

The team-lead's Option A spec uses FUN_00C6EA70 as the final registration call — this is wrong. Proposed substitute: call FUN_00A37790 directly with a manually-built callback_obj in the fixed cave (fake_vtable with BMCreateAuction TypeDesc getter at slot[2], fake_eh with entity_desc+method_node, lambda=0x00D43DC0). Report sent to team-lead 2026-06-22. Awaiting approval to proceed.

## SSO layout — VERIFIED against FUN_004242C0 comparator

Traced through FUN_00C66C10 → FUN_0158ECA0 → FUN_0158EA90 → FUN_004242C0. The comparator reads:
- [struct+0x18]: capacity — if < 0x10, SSO inline; else heap ptr
- [struct+0x14]: length
- [struct+0x04]: inline chars (SSO) or char** (heap)

Both FUN_00C66C10 and FUN_01591420 use the same layout through the same comparator. No differences.

SSO "SGWPlayer": 00 00 00 00 | 53 47 57 50 6C 61 79 65 72 00 00 00 00 00 00 00 | 09 00 00 00 | 0F 00 00 00
SSO "BMCreateAuction": 00 00 00 00 | 42 4D 43 72 65 61 74 65 41 75 63 74 69 6F 6E 00 | 0F 00 00 00 | 0F 00 00 00

## Approved sequence (do not deviate)

### Step 0 — Liveness
- `get_debugger_status`: Debugging=True, Running=True, fresh PID (not 173976)
- `get_latest_event`: must NOT be EVENT_PAUSE_DEBUG or any exception
- `get_all_registers`: EIP must be in game code (not msvcr80, not ntdll exception handlers)
- Clear ALL stale BPs (`clear_breakpoint` with no address, type=software)
- Non-freezing BPs only: `bpcnd addr,"0"` or set with fastresume — never freezing

### Step 1 — VirtualProtect cave page (FIRST, before any write)
Allocate a small page for the VP stub (heap alloc, not fixed). Write VP stub with LEA scratch:
```asm
PUSH EBP
MOV EBP, ESP
SUB ESP, 4
LEA EAX, [EBP-4]       ; scratch DWORD — NOT PUSH ESP (that crashes)
PUSH EAX               ; lpflOldProtect
PUSH 0x40              ; PAGE_EXECUTE_READWRITE
PUSH 0x1000            ; dwSize
PUSH 0x01674000        ; lpAddress
CALL [VirtualProtect_IAT]
MOV ESP, EBP
POP EBP
RET
```
VirtualProtect IAT address: look up via `eval_expression "VirtualProtect"` or known import address.
Run via `create_thread <stub_addr>`. Verify page is now RWX by reading old-protect scratch.
WITHOUT THIS STEP: runtime write to 0x01674420 (one_shot_flag) → write-AV → crash #10.

### Step 2 — Write fixed cave at 0x01674420 (208 CC bytes in .text, no ASLR)

**Complete byte string to write at 0x01674420 (107 bytes):**

```
Offset  Content (hex)
+0x00   B8 B0 60 E6 01 C3 CC CC   TypeDesc getter: MOV EAX,0x01E660B0; RET; CC CC
+0x08   00 00 00 00               one_shot_flag = 0
+0x0C   00 00 00 00               padding
+0x10   SSO "SGWPlayer" (28 bytes):
        00 00 00 00
        53 47 57 50 6C 61 79 65 72 00 00 00 00 00 00 00
        09 00 00 00
        0F 00 00 00
+0x2C   padding: 00 00 00 00
+0x30   SSO "BMCreateAuction" (28 bytes):
        00 00 00 00
        42 4D 43 72 65 61 74 65 41 75 63 74 69 6F 6E 00
        0F 00 00 00
        0F 00 00 00
+0x4C   padding: 00 00 00 00
+0x50   Dispatch thunk (27 bytes) at 0x01674470:
        8B 41 08
        68 B0 60 E6 01
        68 CC B4 DA 01
        51
        8B 4C 24 10
        50
        E8 6B 2E 3C FF
        C2 04 00
```

SSO struct for "SGWPlayer" lives at `0x01674430` (cave+0x10):
- chars at +0x04: "SGWPlayer\0" + 7 pad
- length at +0x14: 0x09
- capacity at +0x18: 0x0F (SSO sentinel: capacity<0x10 → inline)

SSO struct for "BMCreateAuction" lives at `0x01674450` (cave+0x30):
- chars at +0x04: "BMCreateAuction\0" (exactly 16 bytes with null)
- length at +0x14: 0x0F
- capacity at +0x18: 0x0F

Dispatch thunk at `0x01674470` (cave+0x50):
- Clone of FUN_00E5CAE0 with TypeDesc swapped 0x01E66224 → 0x01E660B0
- CALL rel32: 0x00A372F0 - (0x01674475 + 5) = 0xFF3C2E6B → bytes `E8 6B 2E 3C FF`
- VERIFY: disassemble 27 bytes at 0x01674470 after write, confirm CALL target = 0x00A372F0

After writing, disasm-verify every instruction in the thunk. Check CALL displacement.

### Step 3 — Phase 1 Recon (all read-only; ABORT at first failure)

A. `[0x01EF2264]` (Mercury channel) != 0
B. `[0x01EF244C]` (GameEntityManager) != 0
C. class_idx = FUN_00C66C10(0x01674434) — `__cdecl`, one stack arg = ptr to "SGWPlayer" SSO
   Call: `PUSH 0x01674430; CALL 0x00C66C10; ADD ESP, 4`
   Result in AX. STOP if AX == 0xFFFF
D. entity_desc = class_idx * 0x110 + [[0x01EF244C+0x90]+0x10]
   Read [0x01EF244C] → gem; read [gem+0x90] → entity_coll; read [entity_coll+0x10] → coll_base
   entity_desc = coll_base + (class_idx * 0x110)
   STOP if entity_desc == 0 or implausible
E. method_node = FUN_01591420(ECX=entity_desc+0x88, &SSO_BMCreateAuction)
   `__thiscall(ECX=entity_desc+0x88, [ESP+4]=ptr_to_sso)` where SSO at 0x01674450
   RET 0x4 (callee cleans 1 stack arg)
   STOP if EAX == 0
F. [method_node+0x1C] & 4 — STOP if zero (not Exposed)
G. [method_node+0x44] == 0x3E (62) — STOP if mismatch (wrong wire method index)

### Step 4 — Phase 2 Registration (tick cave, main-thread one-shot)

Tick cave: allocate heap page, write flag (0) + script + tick detour.
Detour hooks 0x00416EC0 (game tick prolog), steals first 6 bytes `64 A1 00 00 00 00`.

Registration one-shot code (fires once when flag==0, sets flag=1 after return):
```asm
CMP [flag], 0
JNZ @done
; FUN_00D6CE00(__thiscall, ECX=channel, param1=&SSO_SGWPlayer, param2=2, param3=&SSO_BMCreateAuction)
PUSH 0x01674450        ; &SSO "BMCreateAuction" = param_3 (rightmost stack arg pushed first)
PUSH 2                 ; direction = param_2
PUSH 0x01674430        ; &SSO "SGWPlayer" = param_1
MOV ECX, [0x01EF2264]  ; ECX = Mercury channel = 'this'
CALL 0x00D6CE00
MOV [flag], 1          ; set AFTER successful return
@done:
; stolen bytes + JMP back to 0x00416EC6
```

Set non-freezing BP on `FUN_00D6CE00` entry (0x00D6CE00) before arming.
Arm detour: write `E9 xx xx xx xx 90` at 0x00416EC0.
Confirm: BP at 0x00D6CE00 fires 1 time, flag becomes 1, NO crash.

### Step 5 — Rendezvous Check (read-only, before trigger)

After reg one-shot fires and flag==1:
- `CME_singleton = [0x01EE2678]`
- `bucket_base = [CME_singleton + 0x30]`
- BMCreateAuction bucket = SLOT 2 (static: key `0x01E660B0 XOR 0xDEADBEEF` → Park-Miller → slot 2)
- `bucket_head = [bucket_base + 8]`  (bucket_base + 2*4)
- STOP if bucket_head == 0 or bucket_head == 0xFFFFFFFF

If STOP: registration succeeded but under wrong key (register↔dispatch mismatch). Report.

### Step 6 — Patches

Only AFTER recon passed AND reg done AND rendezvous confirmed:

**Entity guard NOP:**
Write at 0x00E599A8: `90 90 90 90 90 90`
Original bytes: `0F 84 A3 02 00 00`
Disasm-verify: 6 NOPs.

**Vtable[2] redirect:**
Write at 0x019DD368: `70 44 67 01` (little-endian 0x01674470)
Original bytes: `20 C4 E5 00` (= 0x00E5C420)
Disasm-verify: `[0x019DD368]` == 0x01674470.

**onBMOpen injection** (tick cave, separate branch):
Lua_doString_wide(L, L"BlackMarketMod.onBMOpen()", ...) — from existing session recipe.

### Step 7 — Trigger

Separate tick cave branch (flag2): Lua `createAuction(10237,100,500,1)` via doString_wide.

Non-freezing BPs to arm before trigger:
- 0x01674470 (dispatch_thunk_entry)
- 0x00A372F0 (CME_dispatch)
- 0x00C6F870 (Mercury_send)

Expected hit sequence on trigger:
1. 0x01674470: 1 hit
2. 0x00A372F0: 1 hit
3. 0x00C6F870: baseline+1

### Step 8 — Verify

`psql -c "SELECT * FROM sgw_auction WHERE seller_id=62 ORDER BY id DESC LIMIT 1"`
Expected: seller_id=62, starting_bid=100, buyout_price=500, item_instance_id=10237

OR watch server log for "createAuction: listing opened" confirmation.

## Key addresses (all fixed .text, no ASLR)

| Symbol | Address |
|--------|---------|
| Cave base | 0x01674420 |
| Cave TypeDesc getter | 0x01674420 |
| Cave one_shot_flag | 0x01674428 |
| Cave SSO "SGWPlayer" | 0x01674430 |
| Cave SSO "BMCreateAuction" | 0x01674450 |
| Cave dispatch thunk | 0x01674470 |
| EventSignal vtable[2] slot | 0x019DD368 |
| Entity guard JZ | 0x00E599A8 |
| FUN_00D6CE00 | 0x00D6CE00 |
| FUN_00C66C10 | 0x00C66C10 |
| FUN_01591420 | 0x01591420 |
| FUN_00A372F0 (CME dispatch) | 0x00A372F0 |
| FUN_00A37790 (CME insert) | 0x00A37790 |
| FUN_00C6F870 (Mercury send) | 0x00C6F870 |
| CME singleton | [0x01EE2678] |
| Mercury channel | [0x01EF2264] |
| GameEntityManager | [0x01EF244C] |
| Game tick prolog | 0x00416EC0 |
| BMCreateAuction TypeDesc | 0x01E660B0 |
| Pool TypeDesc | 0x01DAB4CC |

## Reversibility bytes

- 0x00E599A8: original `0F 84 A3 02 00 00`
- 0x019DD368: original `20 C4 E5 00` (= 0x00E5C420)
- 0x00416EC0: original first 6 bytes `64 A1 00 00 00 00` (stolen by tick detour)

## Known crash causes to avoid

1. Writing cave without VirtualProtect → write-AV on runtime flag write
2. Passing raw char* to FUN_00D6CE00 instead of SSO struct ptr → crash in FUN_00C6EF40
3. Calling FUN_00D6CE00 from Lua C-callback context → _CxxThrowException through Lua = terminate()
4. Using FUN_00D5A230 directly → registers under SellItems TypeDesc key, not BMCreateAuction
5. Freezing BPs → server disconnect
6. VirtualProtect stub using PUSH ESP instead of LEA EAX,[EBP-4] → VP overwrites return address

[[bm-create-auction-dispatch-diagnosis]]
[[bm-create-auction-registration-key-bug]]
[[bm-create-auction-send-wiring]]
[[feedback-x64dbg-session-liveness-check]]
