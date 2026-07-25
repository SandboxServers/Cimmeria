---
name: bm-create-auction-registration-key-bug
description: FUN_00D46F70 hardcodes SellItems vtable — raised as mismatch concern; team-lead SUPERSEDED with FUN_00D6CE00 + SSO structs as the approved registration path
metadata:
  type: project
---

# BM createAuction — Registration Key Mismatch Analysis (2026-06-22)

## STATUS: SUPERSEDED by team-lead authorization

The manual-callback_obj approach proposed here was OVERRIDDEN by the team-lead. The APPROVED registration path is:

**`FUN_00D6CE00(ECX=[0x01EF2264], &SSO_SGWPlayer, 2, &SSO_BMCreateAuction)`**

Called from a main-thread tick one-shot (NOT from Lua). SSO structs baked into fixed cave. After return, rendezvous check verifies subscriber is present at BMCreateAuction bucket before trigger. See approved spec details in `bm-create-auction-dispatch-diagnosis.md`.

The FUN_00D46F70 finding below is still archaeologically correct — but the team-lead has accepted FUN_00D6CE00 as the correct single-call wrapper that handles all of this internally with the right RTTI.

---

# BM createAuction — Registration Key Mismatch (2026-06-22, Ghidra static pass)

## The bug

`FUN_00D46F70 @ 0x00D46F70` is the MemberCallback constructor called by `FUN_00D4EBC0`, which is called by `FUN_00D5A230`. It unconditionally writes `0x019C6380` (SellItems MemberCallback vtable) into `callback_obj[0]`:

```
00d46faf: MOV [EAX], 0x019C6380     ← SellItems vtable hardcoded (FINAL write)
00d46fb5: MOV [EAX+4], ECX          ← EventHandler ptr
00d46fb8: MOV [EAX+8], EDX          ← lambda ptr (0x00D43DC0)
```

`FUN_00A37790` (CME table inserter) then reads `callback_obj->vtable[2]()` to get the TypeDescriptor hash key. SellItems vtable slot[2] returns the SellItems TypeDescriptor — NOT `0x01E660B0` (BMCreateAuction). The subscriber gets inserted under the SellItems bucket, not the BMCreateAuction bucket. Our dispatch thunk calls `FUN_00A372F0` with `param_5 = 0x01E660B0` → finds nothing → crash or silent miss.

**Why:** `FUN_00D5A230` was designed for SellItems only. Its callee chain (FUN_00D4EBC0 → FUN_00D46F70) has no parameterized TypeDesc — it's hardwired.

## The fix: manual callback_obj in fixed cave

Skip `FUN_00D5A230`, `FUN_00D46F70`, `FUN_00418E30`, and `FUN_00C6EA70` entirely.

Build callback_obj manually in the fixed cave at `0x01674420`:

```
callback_obj[0] = &fake_vtable         ← our vtable; slot[2] = TypeDesc getter returning 0x01E660B0
callback_obj[4] = &fake_eh             ← EventHandler: +4=entity_desc, +8=method_node
callback_obj[8] = 0x00D43DC0           ← Mercury send lambda (LAB_00D43DC0)
```

Then call `FUN_00A37790(ECX=CME_singleton, 0, &callback_obj)` directly. No allocation, no throw risk.

`FUN_00A37790` only reads `callback_obj->vtable[2]` for the hash key during registration. `FUN_00A371F0` reads `callback_obj->vtable[5]` at dispatch time for the send path. So fake_vtable needs:
- slot[2] = `0x01674420` (TypeDesc getter: `MOV EAX, 0x01E660B0; RET`)
- slot[5] = `0x00CCC040` (vtable[5] send dispatch — `FUN_00CCC040`)
- other slots can be 0 or copies from SellItems vtable

## Calling convention verification (FUN_00D5A230)

`__thiscall(ECX=this/EventHandler, [ESP+8]=entity_desc, [ESP+0xC]=method_node)` → `RET 0x8`. Callee cleans 2 stack args. But its vtable-hardcode disqualifies it for our use.

## FUN_00C6EA70 is a DUPLICATE-REGISTRATION GUARD

`FUN_00C6EA70 @ 0x00C6EA70` is NOT the registrar — it's the pre-check guard that throws `_CxxThrowException` on duplicate registration. (Ghidra shows the throw on the `*(iVar1+8) == 0` branch — this is the standard Ghidra condition-flip artifact; the actual behavior is: slot occupied → throw, slot empty → return.) Do NOT call `FUN_00C6EA70` from our cave.

## Addresses confirmed this session

- `FUN_00D46F70 @ 0x00D46F70` — MemberCallback ctor, hardcodes SellItems vtable `0x019C6380`
- `FUN_00D4EBC0 @ 0x00D4EBC0` — alloc 0xC + call D46F70 + call A37790; `RET 0xC`
- `FUN_00D5A230 @ 0x00D5A230` — SellItems-specific full init; `RET 0x8`; DO NOT USE for BMCreateAuction
- `FUN_00C6EA70 @ 0x00C6EA70` — duplicate-guard only, throws on duplicate; `RET 0x10`; DO NOT CALL
- `FUN_00A37790 @ 0x00A37790` — CME table INSERT; `__thiscall(CME, 0, &callback_obj)`; reads vtable[2] for TypeDesc hash; safe to call directly

## FUN_00E5CAE0 ground truth (BMSearch vtable[2] — dispatch thunk template)

Exact 9 instructions (27 bytes at dispatch thunk = 0x01674488):
```asm
8B 41 08           MOV EAX,[ECX+0x8]
68 B0 60 E6 01     PUSH 0x01E660B0     ← BMCreateAuction TypeDesc (swapped from 0x01E66224)
68 CC B4 DA 01     PUSH 0x01DAB4CC
51                 PUSH ECX
8B 4C 24 10        MOV ECX,[ESP+0x10]
50                 PUSH EAX
E8 ?? ?? ?? ??     CALL 0x00A372F0     ← rel32 = 0x00A372F0 - (thunk_addr + 5)
C2 04 00           RET 0x4
```
If thunk at `0x01674488`: rel32 = `0x00A372F0 - 0x0167448D` = `0xFF3F2E63`. Full bytes for CALL: `E8 63 2E 3F FF`.

**Why:** The `[ESP+0x10]` trick works because BMSearch's emit site does: PUSH CME_singleton PUSH 0x1 MOV ECX=EventSignal CALL vtable[2]. At vtable[2] entry: ESP→[ret_addr], [ESP+4]=CME_singleton, [ESP+8]=0x1. After 3 pushes (0x01E660B0, 0x01DAB4CC, ECX): [ESP+0x10] = CME_singleton. Confirmed live from memory file and FUN_00E5CAE0 disassembly.

## REVISED CAVE LAYOUT at 0x01674420 (target: ≤208 bytes)

```
+0x00  6   TypeDesc getter:  B8 B0 60 E6 01 C3
+0x06  2   CC CC (padding)
+0x08  4   one_shot_flag (init = 0x00000000)
+0x0C  4   CC CC CC CC (padding)
+0x10  4   fake_vtable[0] = 0x00000000
+0x14  4   fake_vtable[1] = 0x00000000
+0x18  4   fake_vtable[2] = 0x01674420  ← TypeDesc getter
+0x1C  4   fake_vtable[3] = 0x00000000
+0x20  4   fake_vtable[4] = 0x00000000
+0x24  4   fake_vtable[5] = 0x00CCC040  ← vtable[5] send dispatch
+0x28  4   fake_eh[0] = 0x00000000 (vtable not used by A37790/A371F0 for eh)
+0x2C  4   fake_eh[4] = 0 (entity_desc — filled at Phase 2 init)
+0x30  4   fake_eh[8] = 0 (method_node — filled at Phase 2 init)
+0x34  4   callback_obj[0] = 0x01674430  ← ptr to fake_vtable (at +0x10)
+0x38  4   callback_obj[4] = 0x01674448  ← ptr to fake_eh (at +0x28)
+0x3C  4   callback_obj[8] = 0x00D43DC0  ← lambda
+0x40  28  SSO "SGWPlayer": 00 00 00 00 | 53 47 57 50 6C 61 79 65 72 00 00 00 00 00 00 00 | 09 00 00 00 | 0F 00 00 00 | 00 00 00 00
+0x5C  28  SSO "BMCreateAuction": 00 00 00 00 | 42 4D 43 72 65 61 74 65 41 75 63 74 69 6F 6E 00 | 0F 00 00 00 | 0F 00 00 00 | 00 00 00 00
+0x78  (= 0x01674498) dispatch thunk starts here
```

Dispatch thunk (27 bytes at 0x01674498):
```
8B 41 08 68 B0 60 E6 01 68 CC B4 DA 01 51 8B 4C 24 10 50 E8 63 2E 3F FF C2 04 00
```
(CALL rel32 = FF3F2E63 assuming thunk at 0x01674498; recalculate if thunk address changes)

Total bytes used: 0x78 + 27 = 0x93 = 147 bytes. Fits in 208 comfortably.

## Rendezvous check (required before trigger)

After Phase 2 one-shot fires:
- Read `CME_singleton = [0x01EE2678]`
- Read `bucket_base = [CME_singleton + 0x30]`
- Bucket slot for BMCreateAuction = SLOT 2 (static: `0x01E660B0 XOR 0xDEADBEEF = 0xDF4BDE5F` → hash → slot 2)
- Read `bucket_head = [bucket_base + 8]`
- ABORT if `bucket_head == 0` or `bucket_head == 0xFFFFFFFF`

**Why:** Confirms `FUN_00A37790` inserted under `0x01E660B0` key (the key our dispatch thunk uses). This is the "one link not yet proven" the team-lead flagged.

## Session state (2026-06-22 end)

Client: DEAD (crash #9 — raw C-string vs SSO mismatch in FUN_00D6CE00 from prior session).
Process PID 173976 unrecoverable (EIP=0x7B386C89 = msvcr80!terminate).
All heap pages gone. Fixed .text cave at 0x01674420 state: unknown, needs full rewrite on relaunch.
Awaiting team-lead approval of manual-callback_obj approach before next session.

[[bm-create-auction-dispatch-diagnosis]]
[[bm-create-auction-send-wiring]]
[[feedback-x64dbg-session-liveness-check]]
