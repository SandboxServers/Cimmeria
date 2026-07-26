---
name: bm-create-auction-registration-key-bug
description: FUN_00D46F70 hardcodes SellItems vtable — CONFIRMED CORRECT 2026-07-25 via fresh Ghidra decompile; this is the approved registration path (manual callback_obj + direct FUN_00A37790), NOT FUN_00D6CE00
metadata:
  type: project
---

# BM createAuction — Registration Key Mismatch Analysis (2026-06-22, RE-CONFIRMED 2026-07-25)

## STATUS: UN-SUPERSEDED — this file's diagnosis is CORRECT, confirmed by independent Ghidra re-verification

A prior team-lead override adopted `FUN_00D6CE00` + SSO structs as the "approved" registration path (see the superseded note that used to be here, and `bm-create-auction-dispatch-diagnosis.md`'s "FINAL LOCKED PLAN"/"FINAL ARCHITECTURE" sections). **That override was based on a misunderstanding and is wrong.** A 2026-07-25 read-only Ghidra research pass (triggered by a genuine contradiction between this file and `bm-create-auction-next-session-plan.md`) re-decompiled the entire chain from scratch and confirmed:

- `FUN_00D5A230` (called by `FUN_00D6CE00`) hardcodes `*this = SGWNetworkManager::EventHandler<class_Event_NetOut_SellItems>::vftable` directly in its own body, unconditionally, regardless of what entity/method strings are passed to `FUN_00D6CE00`.
- `FUN_00D46F70` (reached via `FUN_00D4EBC0`) hardcodes `*this = CME::EventSignal::MemberCallback<NoSubject, EventHandler<Event_NetOut_SellItems>, ..., Event_NetOut_SellItems>::vftable` — confirmed via Ghidra's own recovered RTTI/template symbol names, not inferred from offsets.
- `FUN_00A37790` derives its CME-table hash key by calling `callback_obj->vtable[2]()` — for the SellItems-templated object this returns SellItems' TypeDescriptor, never BMCreateAuction's (`0x01E660B0`).
- `FUN_00A372F0` (the emit-time dispatcher) hashes on the literal `type_info*` the caller passes in (BMCreateAuction's, from the thunk) — a different address, different bucket.

**Conclusion: `FUN_00D6CE00` is fundamentally a SellItems-specific thin wrapper (one of many near-identical `FUN_00D5Axxx`/`FUN_00D6Cxxx` template instantiations SGWNetworkManager's mega-registrar uses, one per NetOut method). There is no dedicated wrapper for BMCreateAuction/BMSearch in the shipped binary — none was ever registered by the native init.** Calling `FUN_00D6CE00("SGWPlayer", 2, "BMCreateAuction")` resolves the correct entity_desc/method_node (so the *unrelated* per-entity RPC map that `FUN_00C6EA70` populates gets the right wire index) but inserts the CME-dispatch subscriber under SellItems' bucket — a silent, non-crashing correctness bug, not just a crash risk. `FUN_00D6CE00` (and everything downstream of it — `FUN_00D5A230`, `FUN_00D4EBC0`, `FUN_00D46F70`, `FUN_00C6EA70`) must NOT be used for the CME-table subscription. The manual-callback_obj approach below is the only viable path given the binary's actual structure.

**Also corrected 2026-07-25**: the rendezvous-check "SLOT 2 (static)" claim in this file (below) is WRONG — the bucket index is hash-derived at runtime. See `bm-create-auction-dispatch-diagnosis.md`'s "EMULATOR-DERIVED FINAL VALUES" section (hash constant `0x6DF41E32`, confirmed independently by direct decompile of `FUN_00A36F40` + Python re-derivation) and `[[bm-fork-b-session-crash-notes]]`-adjacent notes for the full arithmetic. Use `bucket_slot = 0x6DF41E32 & [CME_singleton+0x3C]`, wrap-adjusted if `[CME_singleton+0x40] <= slot`, HEAD at `[CME_singleton+0x30] + slot*4`.

The `FUN_00D46F70` finding below remains archaeologically correct as originally written.

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

**CORRECTED 2026-07-25 — the slot below is NOT static; recompute at runtime:**

After Phase 2 one-shot fires:
- Read `CME_singleton = [0x01EE2678]`
- Compute `pre_mask = 0x6DF41E32` (confirmed Park-Miller hash of `0x01E660B0 XOR 0xDEADBEEF`, independently re-derived 2026-07-25 — see `bm-create-auction-dispatch-diagnosis.md`)
- `mask = [CME_singleton + 0x3C]`; `slot = pre_mask & mask`
- `threshold = [CME_singleton + 0x40]`; if `threshold <= slot`: `slot += (-1 - (mask >> 1))`
- `bucket_base = [CME_singleton + 0x30]`
- `bucket_head = [bucket_base + slot*4]`
- ABORT if `bucket_head == 0` or `bucket_head == 0xFFFFFFFF` or `bucket_head == [CME_singleton+0x24]` (still the sentinel = registration wrote nothing)

**Why:** Confirms `FUN_00A37790` inserted under the `0x01E660B0` (BMCreateAuction) key — this requires the manual-callback_obj path with a synthetic vtable[2] returning `0x01E660B0`, NOT `FUN_00D6CE00` (which would insert under SellItems' key instead, landing in a different bucket entirely).

## Session state (2026-06-22 end)

Client: DEAD (crash #9 — raw C-string vs SSO mismatch in FUN_00D6CE00 from prior session).
Process PID 173976 unrecoverable (EIP=0x7B386C89 = msvcr80!terminate).
All heap pages gone. Fixed .text cave at 0x01674420 state: unknown, needs full rewrite on relaunch.
Awaiting team-lead approval of manual-callback_obj approach before next session.

[[bm-create-auction-dispatch-diagnosis]]
[[bm-create-auction-send-wiring]]
[[feedback-x64dbg-session-liveness-check]]
