---
title: "requestEntityUpdate (0x07) cache-stamp handshake, the corrected GameEntityManager vtable, and the Leave-AoI Path A/B fix"
type: reference
audience: engineers doing RE or wiring AoI / entity-lifecycle server logic
last_updated: 2026-09-28
---

# requestEntityUpdate cache-stamp handshake

> **Last updated**: 2026-09-28
> **Source**: `SGW.exe` read-only headless-Ghidra pass (`tools/re/ghidra-headless/Probe.java`, `-noanalysis -readOnly`, image base `0x00400000`), cross-checked against colo SigNoz `requestEntityUpdate` telemetry for the seven days to 2026-09-27
> **Confidence**: HIGH for the wire shape, the caller identity, and the two doc corrections; MEDIUM for the per-instance `0x0B` callback identity (needs a live trace); OPEN for the deferred-leave drain ordering question
> **Companion docs**: [docs/drafts/spec/mercury-wire-format.md](../../drafts/spec/mercury-wire-format.md) §2.5.2, [docs/drafts/spec/entity-property-sync.md](../../drafts/spec/entity-property-sync.md) §1.10, [docs/audits/entity-property-sync-section2-audit-2026-05-16.md](../../audits/entity-property-sync-section2-audit-2026-05-16.md) Appendix E.4, [docs/audits/mercury-rust-conformance-2026-05-15.md](../../audits/mercury-rust-conformance-2026-05-15.md) row N3
> **Triggered by**: issue #838 (`requestEntityUpdate` dropped) and issue #1000 (client entity enter/leave lifecycle RE)

## Summary

Three findings, in order of how load-bearing they are for the #838 fix:

1. **`requestEntityUpdate` (msg `0x07`) is `[u32 entityId][N × u32 cacheStamp]`**, not `[u32 header][N × u32 entity_id]`. The client fires it once per distinct entity from `EntityManager::onEntityEnter`, with an always-empty cache-stamp vector on this build (`N = 0` in every case checked, static and live). It is a handshake, not a recovery request, and the client does not wait for a reply before treating the entity as usable.
2. **The `GameEntityManager` vtable starts at `0x019aaec4`, not `0x019aaeb8`.** An RTTI Complete Object Locator pointer at `0x019aaec0` (four bytes before the corrected base) proves it. This also means a 2026-05-16 audit correction that called two Ghidra plate comments "doubly wrong" was itself wrong — both plate comments were right.
3. **`EntityManager_LeaveAoI`'s Path A/B condition in the draft chapter was inverted.** An entity *found* in the primary map is dispatched immediately; an entity *not found* is the one that goes into the deferred-leave buffer. The chapter had it backwards.

## 1. The `requestEntityUpdate` emitter

### 1.1 Caller: `EntityManager::onEntityEnter`

`FUN_00dd24f0` is confirmed as `EntityManager::onEntityEnter` by the embedded assert string:

```text
ASSERTION FAILED: vehicleID == 0 && "Client only entities should not start already
aboard a vehicle."
..\..\..\..\Server\bigworld\src\client\entity_manager.cpp(%d)%s%s\n
```

This matches issue #838's citation exactly. It is vtable slot 3 of the corrected `GameEntityManager` vtable (§2 below).

Near the end of the function, after the entity has been registered (and, on the fast path, after `EntityManager_enterWorld` has already been called — the entity is visible before this point, regardless of what happens next), there is one conditional external call:

```c
if (((*(int *)((int)this + 8) != 0) && (param_1 < 0x40000000)) &&
   ((piVar6 == (int *)0x0 ||
    ((param_1 != *(int *)((int)this + 0x14) && (piVar6 != *(int **)((int)this + 0x10))))))) {
  ...
  Mercury_Channel_3(*(void **)((int)this + 8), param_1 /* + an elided vector arg, see 1.2 */);
  ...
}
```

Read as: **if connected to a channel** (`this+8 != 0`), **the id is a normal networked id** (`< 0x40000000`), and **this id/object pair differs from the last one processed** (dedup against `this+0x10`/`this+0x14`), call the emitter. This dedup is why the colo telemetry count (2,275 `requestEntityUpdate` sends over seven days) tracks 1:1 with distinct AoI-enter events rather than every internal enter-count decrement.

**The decompiler's own signature for this tail call is incomplete** — it shows only `(channel, param_1)`, but the callee needs a second argument (§1.2). This is the same class of decompiler artifact as the callee itself; the true call passes the entity id plus a freshly constructed, empty cache-stamp vector, matching issue #838's description of the emitter's inputs.

### 1.2 The emitter itself

The function is at `0x00dd80c0` (entry) / `0x00dd80d3` (the body, right after the guard that skips the whole thing when the channel's feedback-context pointer is null). It carries two wrong names from two separate Ghidra auto-annotation passes:

- `ServerConnection::sendAvatarUpdates` in `docs/drafts/spec/mercury-wire-format.md` (pre-this-pass).
- `Mercury_Channel_3` in the headless-probe Ghidra project used for this pass.

Neither name is right; this is the `annotation-script-shift-bugs.md` pattern recurring on the same function. Decompiling the function with Ghidra's stock signature inference produces a broken read:

```c
void __thiscall Mercury_Channel_3(void *this, int param_1)
{
  ...
  puVar3 = (undefined4 *)(**(code **)(*piVar2 + 0x10))(4);
  *puVar3 = unaff_retaddr;                        // <-- decompiler artifact
  puVar3 = *(undefined4 **)(param_1 + 4);         // treats param_1 as a vector pointer
  ...
  while (true) {
    ...
    puVar4 = (undefined4 *)(**(code **)(*piVar2 + 0x10))(4);
    *puVar4 = uVar1;                               // writes one u32 per loop iteration
    puVar3 = puVar3 + 1;
  }
}
```

`unaff_retaddr` is Ghidra's label for "a value read from a register/stack slot this function never seems to have written" — the standard symptom of a function whose true signature has more parameters than what Ghidra inferred. The raw disassembly resolves it:

```text
00dd80f2: MOV EAX, dword ptr [EDX + 0x10]
00dd80f5: PUSH 0x4
00dd80f7: CALL EAX                          ; (*vtable[4])(4) — reserve 4 bytes in the bundle
00dd80f9: MOV ECX, dword ptr [ESP + 0x18]   ; <-- the entity id (caller's real 2nd argument)
00dd80fd: MOV EBX, dword ptr [ESP + 0x1c]   ; <-- the cache-stamp vector pointer (3rd argument)
00dd8101: MOV dword ptr [EAX], ECX          ; write the entity id into the reserved 4 bytes
00dd8103: MOV EDI, dword ptr [EBX + 0x4]    ; vector.begin()
00dd8106: CMP EDI, dword ptr [EBX + 0x8]    ; vs vector.end()
```

So the real signature is `(this=channel, entityId, cacheStampVectorPtr)`, and the function:

1. Calls `Bundle::startMessage(slot_7)` (u16-length-prefixed message).
2. Writes the **entity id** as the message's first 4 bytes.
3. Loops the vector from `begin` to `end`, writing one `u32` **cache stamp** per element.

### 1.3 Wire shape and the always-empty vector

**Wire shape**: `[msg_id=0x07][u16 length-prefix][u32 entityId][N × u32 cacheStamp]`.

In every case checked, `N = 0`:

- The vector `onEntityEnter` constructs and passes is never observed populated in this pass (no write site found that stores anything into it before the call).
- Colo SigNoz: 2,275/2,275 `requestEntityUpdate` captures over the seven days to 2026-09-27 were exactly 4 bytes — a bare entity id, e.g. `52 88 01 00` = `100434`.

This is consistent with the rest of the codebase's cache-stamp evidence: `entity-creation-wire-formats.md` and `space-viewport-wire-formats.md` both note the server-side `cacheStamp` field is "always 0" in every capture. SGW never exercises BigWorld's cache-stamp versioning in either direction.

**Practical read for the server**: treat the body as `[u32 entityId]` (4 bytes) in the common case, but parse it generically as `[u32 entityId][N × u32 cacheStamp]` with N derived from the message's length prefix, in case a future client build (or a patched one) ever populates stamps.

### 1.4 What this means for #838's acceptance criteria

- **Known entity** (the id is already in the witness's AoI, i.e. the base already sent a `CREATE_ENTITY` for it to this witness): the client already has full state from that create — this message is a formality, not a request for anything specific. The fix answers with **nothing**.
- **Unknown entity** (the id is not one this witness's AoI has produced a create for): treat it as the recovery case PR #390's cell handler already implements — send a full `CREATE_ENTITY` re-create.
- **The client never blocks on a reply.** `onEntityEnter` calls `EntityManager_enterWorld` synchronously (on the fast path) independent of whatever happens with this message, so a slow or missing server response does not freeze the entity at spawn by itself — see the invisible-Cellblock-guard note in §4.

## 2. Corrected `GameEntityManager` vtable

### 2.1 The proof

`docs/audits/entity-property-sync-section2-audit-2026-05-16.md` Appendix E.4 numbered this vtable from `0x019aaeb8`. That base is wrong. Dumping 16 slots from `0x019aaeb8`:

| Address | Value | What it is |
|---|---|---|
| `0x019aaeb8` | `0x00dd0b00` | A function pointer, but not part of this vtable (see below) |
| `0x019aaebc` | `0x00000000` | Null |
| `0x019aaec0` | `0x01b988f4` | **A data pointer, not code** |
| `0x019aaec4` | `0x00dd20b0` | First real vtable slot |

`0x019aaec0` is exactly `0x019aaec4 - 4` — the offset where MSVC's RTTI Complete Object Locator pointer always sits, immediately before a polymorphic class's vtable. A COL pointer is data (it points at the `RTTICompleteObjectLocator` structure), never code, which is exactly what `0x01b988f4` is. This is the standard, unambiguous marker for "the vtable starts here." The two slots before it (`0x019aaeb8`, `0x019aaebc`) belong to whatever object precedes `GameEntityManager` in memory, not to it.

### 2.2 The corrected table

Dumped from the true base, `0x019aaec4` (a parallel copy exists at `0x019ce980`, offset by the same three-slot removal):

| Index | Address | Target | Identity |
|---|---|---|---|
| 0 | `0x019aaec4` | `0x00dd20b0` | `EntityManager__vfunc_0` — not yet decompiled |
| 1 | `0x019aaec8` | `0x00dd11b0` | Not yet decompiled |
| 2 | `0x019aaecc` | `0x00dd0c10` | `GameEntityManager_SetPlayerControlTarget` (audit's name; not yet decompiled this pass) |
| 3 | `0x019aaed0` | `0x00dd24f0` | **`EntityManager::onEntityEnter`** — increments enter count, conditionally emits `requestEntityUpdate` (§1) |
| 4 | `0x019aaed4` | `0x00dd2800` | `EntityManager_EnterAoI` — decrements enter count; on reaching zero, tears down via `FUN_00dd1fb0` |
| 5 | `0x019aaed8` | `0x00dd0bb0` | `GameEntityManager_RemoveEntityListener` (audit E.4, confirmed: `lower_bound` on the listener map + `FUN_00e68df0` refcount release) |
| 6 | `0x019aaedc` | `0x00dd2270` | `EntityManager_HandleEntityCreate` |
| 7 | `0x019aaee0` | `0x00dd0d00` | Not yet decompiled |
| 8 | `0x019aaee4` | `0x00dd29d0` | **`EntityManager_LeaveAoI`** (§3) |
| 9 | `0x019aaee8` | `0x00dd2b80` | `GameEntityManager_DispatchEntityRpc` |
| 10 | `0x019aaeec` | `0x00dd1650` | Named `BW_client_entity_manager_6` in a prior session; not further characterized this pass |
| 11 | `0x019aaef0` | `0x00dd0620` | Not yet decompiled |

Re-deriving the two plate-comment slot numbers the 2026-05-16 audit disputed, from this true base: `0x00dd0c10` is index **2**, and `0x00dd0bb0` is index **5** — exactly what the original Appendix D plate comments said. **Both were correct.** The audit's "doubly wrong" / "systematic vtable slot numbering error" conclusion (Appendix E.4) is retracted; see the correction note added there in this PR.

## 3. Leave-AoI Path A/B correction

`docs/drafts/spec/entity-property-sync.md` §1.10 described `EntityManager_LeaveAoI @ 0x00dd29d0`'s branch on the primary-map lookup backwards. The decompile:

```c
ServerConnection__unknown_00e221a0((void *)((int)this + 0x18), (int *)&local_18, &param_1);
...
if (iStack_14 != iVar4) {          // iterator != end --> FOUND
  ...
  (**(code **)(*piVar2 + 8))();    // dispatch immediately
  return;
}
// iterator == end --> NOT FOUND
iVar4 = (**(code **)(*piVar2 + 8))();
piVar5 = (int *)scalable_malloc(0x20);
...
this_00 = FUN_00dd5c70((void *)(unaff_ESI + 0x3c), (int *)&puStack_8);
FUN_0046eef0(this_00, puVar7);      // enqueue into the deferred-leave slot at +0x3c
```

**Found in the primary map (`this+0x18`) → dispatched immediately. Not found → buffered into the deferred-leave slot at `GameEntityManager+0x3C`.** The chapter had this exactly backwards (labeled "not found" as the immediate-dispatch path). Fixed in this PR; see the correction note in §1.10.

## 4. `0x0B` (`entityInvisible`) semantics — partially settled

The shared trampoline `0x00de1c90` is now fully decompiled:

```c
void __thiscall FUN_00de1c90(void *this, undefined4 param_1, undefined4 param_2, int *param_3)
{
  undefined4 uVar1;
  uVar1 = (**(code **)(*param_3 + 4))(5);
  (**(code **)((int)this + 4))(uVar1);
  return;
}
```

This confirms the #1000 description: it is a thin forwarder. It calls `vfunc_4(5)` on its third argument to obtain a value, then forwards that value into a **per-instance callback stored at `this+4`**. The trampoline's own mechanics are no longer a mystery — but the identity of `this` (and therefore the per-instance callback) is a runtime property: it depends on which object instance calls through the two data xrefs at `0x019d13e4` / `0x019d15ac`, and that requires walking live objects. This is unresolved in this (static, read-only) pass, consistent with #1000's own note that a live x64dbg breakpoint would settle it.

## 5. Open questions (unchanged from #1000)

Not settled by this pass — both explicitly need a live client, not static RE:

- **Q1 — deferred-leave drain ordering.** Whether the `+0x3c` deferred-leave buffer drains before or after a subsequent create for the same id. The enqueue mechanics (§3) are confirmed; the drain call site relative to `EntityManager_HandleEntityCreate` was not located in this pass — `HandleEntityCreate`'s decompile does not reference `+0x3c` at all, so the drain must happen elsewhere (plausibly a per-tick flush), not synchronously at create time.
- **Q3 — `CREATE_ENTITY` for an id still in the client's cache map after a `LEAVE_AOI`.** Not exercised in this static pass.

## Ghidra anchors

- `ghidra://SGW.exe@0x00dd24f0` — `EntityManager::onEntityEnter` (vtable slot 3)
- `ghidra://SGW.exe@0x00dd80c0` — `requestEntityUpdate` emitter (misnamed `Mercury_Channel_3` / `ServerConnection::sendAvatarUpdates`)
- `ghidra://SGW.exe@0x019aaec4` — `GameEntityManager` vtable (corrected base)
- `ghidra://SGW.exe@0x019aaec0` — RTTI Complete Object Locator pointer proving the base above
- `ghidra://SGW.exe@0x00dd29d0` — `EntityManager_LeaveAoI` (vtable slot 8)
- `ghidra://SGW.exe@0x00de1c90` — shared `0x0B` trampoline
