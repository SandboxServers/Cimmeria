---
name: bm-create-auction-dispatch-diagnosis
description: createAuction FUN_00A372F0 architecture — vtable-hash CME dispatch table, why thunk-to-A372F0 crashes, reconstruction verdict (2026-06-22)
metadata:
  type: project
---

# BM createAuction — FUN_00A372F0 Dispatch Architecture & Crash Diagnosis (2026-06-22)

## Session objective
Diff BMSearch vs createAuction at FUN_00A372F0 to find why createAuction doesn't transmit.

## Key findings (confirmed live, x64dbg)

### 1. Wrong vtable slot in previous session
Prior session patched `0x019DD39C` thinking it was the EventSignal vtable slot 2.
- ACTUAL EventSignal vtable for createAuction: at `0x019DD360`
  - slot 0 (+0x00) = `0x00DA88E0`
  - slot 1 (+0x04) = `0x00E5C200` (register_NetOut, returns RTTI string)
  - slot 2 (+0x08) = `0x00E5C420` ← the send path (NATIVE, before patching)
  - slot 3 (+0x0C) = `0x00E5C300`
- `0x019DD39C` = slot 2 of a DIFFERENT vtable (TypedEmitInfo vtable at `0x019DD394`)
- The prior thunk at `0x073D0000` was wired to the TypedEmitInfo vtable, which is invoked by the CME dispatch queue — but the dispatch queue never ran → 0 thunk hits

### 2. Root cause: createAuction emit missing a push vs BMSearch
In `BMSearch_NetOut_emit @ 0x00E59F70`:
```
call 0x0054C980        ; EAX = CME singleton (DAT_01EE2678)
mov edx, [esi]
push 0x01
push eax               ← EXTRA push: CME singleton pushed here
mov eax, [edx+0x08]
mov ecx, esi
call eax               ; vtable[2] with [esp+4]=CME_singleton, [esp+8]=0x01
```

In `BMCreateAuction_NetOut_emit @ 0x00E59970`:
```
call 0x0054C980        ; EAX = CME singleton
mov edx, [esi]
push 0x01              ← ONLY push 0x01, no extra push eax
mov eax, [edx+0x08]
mov ecx, esi
call eax               ; vtable[2] with [esp+4]=0x01 only
```

This means createAuction's vfunc_2 (`0x00E5C420`) receives `[esp+0x10]` = 0x01 (not CME singleton) → passes ECX=0x01 (garbage) to `0x00E5C320` → the intermediate emission object is queued into nothing → CME dispatch never fires it.

### 3. Fix attempt: redirect vtable slot 2 + new thunk with direct singleton read
Patched `0x019DD368` (correct EventSignal vtable slot 2) from `0x00E5C420` to `0x073D0000`.

New thunk at `0x073D0000` (29 bytes):
```
8B 41 08                mov eax, [ecx+0x08]      ; EventSignal->field_0x08 (= 0)
68 B0 60 E6 01          push 0x1E660B0            ; createAuction TypedEmitInfo
68 CC B4 DA 01          push 0x1DAB4CC
51                      push ecx                  ; EventSignal
8B 0D 78 26 EE 01      mov ecx, [0x01EE2678]    ; ECX = CME singleton directly
50                      push eax                  ; EventSignal->field_0x08
E8 D6 72 66 F9          call FUN_00A372F0
C2 04 00                ret 0x04
```

Result: thunk FIRES (0x73D0000 = 1 hit, 0xE59C44 = 6 hits) — correct wiring confirmed.

**BUT: client crashes with EIP = 0x1 inside FUN_00A372F0.**

### 4. Root cause of crash: CME dispatch table (vtable-hash lookup)
`FUN_00A372F0 @ 0x00A372F0`:
- ECX = CME singleton (the "channel manager")
- Calls `0x00A36F40` — this is a hash lookup on `[edi]` = EventSignal vtable ptr
- `0x00A36F40`: XORs vtable ptr with `0xDEADBEEF`, hashes into CME singleton's table at `[esi+0x20]/[esi+0x24]`
- If no registered entry for this vtable hash → crash (calls through null/garbage function ptr → EIP=0x1)

For BMSearch EventSignal (vtable `0x019DD410`): entry EXISTS in the CME table (was registered at startup as an active NetOut channel).
For createAuction EventSignal (vtable `0x019DD360`): NO entry in the CME table (shelved feature, never registered).

**This is not a simple thunk fix. FUN_00A372F0 requires the NetOut event to be registered in the CME channel dispatch table.**

### 5. Architecture summary
CME NetOut event dispatch works as follows:
- At startup: each active NetOut event type registers its vtable in the CME singleton's hash table, linking it to the player's network channel
- At emit time: `FUN_00A372F0(CME_singleton, EventSignal_vtable_ptr_hash)` looks up which channel to use and forwards the serialized payload
- createAuction's EventSignal (vtable `0x019DD360`) was never registered → the lookup crashes

### 6. EventSignal object layout (createAuction, freshly allocated)
```
+0x00: 0x019DD360  ← vtable ptr (set by 0xE5C1D8)
+0x04: 0xED6C36C0  ← inner channel list object (from 0x00440D10)
+0x08: 0x00000000  ← null channel ptr (never set — by design, CME table provides it)
+0x0C: 0x00000001  ← state/count
```

### 7. Reversibility bytes for this session's patches
- `0x019DD368`: original = `20 C4 E5 00` (= `0x00E5C420`); patched to = `00 00 3D 07` (= `0x073D0000`)
- `0x073D0000` (thunk): original first session was `8B 41 08 68 B0 60 E6 01 68 CC B4 DA 01 51 8B 4C 24 10 50 E8 D8 72 66 F9 C2 04 00` (27 bytes); now `8B 41 08 68 B0 60 E6 01 68 CC B4 DA 01 51 8B 0D 78 26 EE 01 50 E8 D6 72 66 F9 C2 04 00` (29 bytes)
- `0x00e599a8`: NOP×6 (from prior session, unchanged this session)

Client CRASHED in this session. All patches are memory-only and die with the process.

## COMPLETE ARCHITECTURE (recovered 2026-06-22, static Ghidra pass)

### The real CME dispatch architecture
`FUN_00A372F0` is NOT a "NetOut channel dispatch" — it is the **CME event emitter**. It takes a TypeDescriptor pointer (from the EventSignal vtable via `register_NetOut_*` → slot 1, or directly from `RTTI_Type_Descriptor`) as hash key, XOR's it with 0xDEADBEEF in `FUN_00A36F40`, and looks up the subscriber list in the CME singleton's hash table at `[CME_singleton + 0x1c]`. Found → calls subscriber vtable[5] to dispatch payload. Not found → calls through null → EIP=0x1.

### The SGWNetworkManager registration chain
SGWNetworkManager's monster init function (`register_NetOut_onStrikeTeamResponse @ 0x00db3390`, 10000+ lines of decompile) registers ALL NetOut event subscribers. The pattern for each method:
```
BW__unknown_00438c40(local_NAME_buf, "methodName");    // build method name
BW__unknown_00438c40(local_ENTITY_buf, "SGWPlayer");   // build entity class name
pvVar2 = Mercury__unknown_00c6f870();                   // get Mercury channel
FUN_00d6cXXX(pvVar2, local_ENTITY_buf, 2, (uint)local_NAME_buf);  // register
```
Each `FUN_00d6cXXX` is method-specific. It:
1. Calls `FUN_00c6ef40(entity_buf, arg2)` + `FUN_00c6e810(...)` to resolve method index
2. Allocates 0xC bytes, calls a per-event-type EventHandler constructor (e.g., `FUN_00d5a230` for SellItems)
3. The EventHandler ctor sets vtable to `SGWNetworkManager::EventHandler<class_Event_NetOut_XXX>::vftable`, stores channel + method_index, gets CME singleton, calls a registration function variant (e.g., `FUN_00d4ebc0`) which calls `FUN_00a37790` to insert into CME table
4. `FUN_00c6ea70` also wires the Mercury channel send slot

### What's missing for BM NetOut methods
The SGWNetworkManager init registers **ZERO BM NetOut methods**. Only `Event_NetIn_BM*` (server→client) handlers are registered. The full `searchAuctions` / `createAuction` / `placeBid` / `cancelAuction` subscriber chain was never added. BMSearch's EventSignal vtable (`0x019DD410`) NOT present in init — no `SGWNetworkManager::EventHandler<BMSearch>` vtable exists anywhere in .rdata.

**Key question pending**: why does BMSearch work (prior session confirmed vtable 0x019DD410 IS in CME table at runtime)? Either (a) BMSearch IS registered somewhere else not yet found, or (b) the prior claim that BMSearch worked was based on misreading — the send appeared to work but frame never actually reached server. NEEDS LIVE VERIFICATION when client is back.

### The two-patch approach (REVISED for next session)

**Patch 1 (already applied, NOP at 0x00e599a8)**: entity-guard bypass → emit reaches vtable[2] call

**Patch 2 — the real gap**: createAuction's EventSignal vtable[2] (`0x00E5C420`) calls `FUN_00E5C320` (intermediate queuing path) rather than calling `FUN_00A372F0` directly like BMSearch's vtable[2] (`0x00E5CAE0`). This is the "missing push eax" bug from the dispatch diagnosis.

**Option A (faithful registration)**: Add a `SGWNetworkManager::EventHandler<BMCreateAuction>` subscriber registration to the CME table. This requires:
1. A new EventHandler object with vtable set to match BMCreateAuction TypeDescriptor
2. Calling one of the FUN_00d4xxxx variants with CME_singleton + 0 + handler_obj + &LAB_00d43dc0
The challenge: we need the per-createAuction vtable which doesn't exist. Must synthesize one or reuse the BMSearch one.

**Option B (vtable swap)**: Before calling vtable[2], temporarily swap [ESI+0] from `0x019DD360` → `0x019DD410` (BMSearch's vtable), call vtable[2] which calls `FUN_00E5CAE0` → `FUN_00A372F0`, then restore. This relies on BMSearch being registered. Fragile but surgical.

**Option C (direct Mercury send)**: Skip FUN_00A372F0 entirely. From the thunk that intercepts vtable[2], call the actual Mercury packet-send function directly with the already-packed 13-byte payload. Need to identify the Mercury send fn and get the channel ptr from the CME singleton.

**Option D (emit-path fix in cave)**: Replace the faulty `FUN_00E5C320` call inside vtable[2] with a direct call to `FUN_00A372F0`, providing CME_singleton as first arg. This mirrors what BMSearch's vtable[2] does natively. The thunk at `0x073D0000` (from prior session) already does this for the vtable-redirect path.

### Summary for live session
The thunk from prior session (`0x019DD368` → `0x073D0000`) is the right architecture for Option D. The crash happened because the CME table had no entry for createAuction's TypeDescriptor. The fix: either add the entry (Option A) or use BMSearch's vtable in the call (Option B). Option D + Option B is the safest combination: thunk redirects vtable[2], thunk briefly swaps vtable ptr to BMSearch's before calling FUN_00A372F0, restores. No new subscription objects needed.

## CORRECTIONS FROM BUILD SESSION (2026-06-22 live)

### "Missing push EAX" claim was WRONG
The survey claimed createAuction's emit was missing `PUSH EAX` (CME singleton). Disassembly at `0x00E59C3A`–`0x00E59C44` shows:
```
E59C35: CALL 0x0054c980    ; EAX = CME singleton
E59C3A: MOV EDX,[ESI]      ; vtable
E59C3C: PUSH 0x1           ; arg = 1
E59C3E: PUSH EAX           ; arg = CME singleton  ← IS present
E59C3F: MOV EAX,[EDX+0x08] ; vtable[2]
E59C42: MOV ECX,ESI        ; ECX = EventSignal (thiscall)
E59C44: CALL EAX
```
At vtable[2] entry: ECX=EventSignal, [esp+4]=CME_singleton, [esp+8]=0x1.

### MemberCallback vtable layout (SellItems, used as reference)
At `0x019C6380`: slot[0]=`0x00E53F00`, slot[1]=`0x00428B60`, slot[2]=`0x00D44240` (TypeDesc getter), slot[3]=`0x00D46FE0`, slot[4]=`0x00429700`, slot[5]=`0x00CCC040` (send dispatch).
The send dispatch `0x00CCC040` routes through `[CME_singleton+4]` (the Mercury send fn ptr at `0xE08F04A0` in current session). The `&LAB_00d43dc0` stored at callback_obj+0x08 is NOT vtable[5].

### CME callback object layout (12 bytes, built by FUN_00d46f70)
```
+0x00: vtable ptr (event-type-specific MemberCallback vtable)
+0x04: EventHandler 'this' ptr (stores method_index at +4, channel at +8)
+0x08: &LAB_00d43dc0 (the Mercury send lambda — used by some paths, not vtable[5])
```
`FUN_00a371f0` dispatch: calls `[callback_vtable[2]]()` → TypeDescriptor, then `[callback_vtable[5]](EventSignal, param_3)`.

### FUN_00A37790 calling convention confirmed
`__thiscall(CME_singleton, param_1=0, param_2=&callback_obj)`. Internally calls `(*param_2)[2]()` to get TypeDescriptor for hash key, then inserts node into CME table via `FUN_00a39170` + `FUN_00a38950`.

### Cave built and patched — session 2026-06-22 BUILD
Cave allocated at `0x155F0000` (ASLR-placed, 4096 bytes). Layout:
```
0x155F0000  TypeDesc getter: B8 B0 60 E6 01 C3   (MOV EAX,0x01E660B0; RET)
0x155F0008  Registration-done flag (init=0)
0x155F0010  MemberCallback vtable (6 slots):
              [0]=0x00E53F00, [1]=0x00428B60, [2]=0x155F0000 (getter),
              [3]=0x00D46FE0, [4]=0x00429700, [5]=0x00CCC040 (send dispatch)
0x155F0028  callback_obj: vtable=0x155F0010, +4=0, +8=0x00D43DC0
0x155F0034  vtable[2] thunk (70 bytes):
              PUSH ESI/EDI; save ECX→ESI, [esp+0xC]→EDI (CME singleton);
              CMP [flag],0; JNZ @skip
              ; one-shot: FUN_00A37790(ECX=EDI, 0, &callback_obj); set flag=1
              @skip: FUN_00A372F0(ECX=EDI, 0, EventSignal=ESI, 0x01DAB4CC, 0x01E660B0)
              ADD ESP,0x10; POP EDI/ESI; RET 8
```
Patches applied:
- `0x00E599A8`: `0F 84 A3 02 00 00` → `90 90 90 90 90 90` (entity guard NOP)
- `0x019DD368`: `20 C4 E5 00` → `34 00 5F 15` (vtable[2] → cave thunk)

Non-freezing BPs set (fastresume=1):
- `0x155F0034` (cave_thunk_entry), `0x155F004F` (call_register), `0x155F006D` (call_dispatch)
- `0x00A37790` (CME_register_subscriber), `0x00C6F870` (Mercury_send)

Baseline hit counts before UI trigger: thunk=0, register=0, dispatch=0, A37790=0, Mercury_send=1.

**Self-test**: owner opens BM window → clicks Create → check BP hits and sgw_auction DB row.
Expected: cave_thunk_entry ≥1 (on first trigger: register=1 + dispatch=1; A37790=1; Mercury_send ≥1).

## SESSION 2026-06-22 (TICK CAVE) — CRASH AT FUN_00A37790

### Tick cave approach (new in this session)
Cave at `0x15600000` (new page): flag(0) at +0, UTF-16LE `createAuction(1, 100, 500, 1)` at +8.
Tick cave at `0x15600050`: hooks `0x416EC0` (game tick function prolog), fires `Lua_doString_wide(L, script, 29)` once (one-shot flag), re-executes stolen bytes `64 A1 00 00 00 00`, JMPs back to `0x416EC6`.

Reversibility for tick detour:
- `0x00416EC0` original bytes: `64 A1 00 00 00 00` (first 6)
- Patch applied: `E9 8B 91 1E 15 90`

### BP hit progression (controlled trigger, 2026-06-22)
1. `tick_cave_entry` (0x15600050): 1 hit — detour is live, game ticks through `0x416EC0`
2. `tick_cave_lua_call` (0x15600081): 1 hit — Lua state valid ([L+4]&0xFF==8), all 3 args correct on stack (L=0xEB918280, script=0x15600008, len=0x1D)
3. `cave_thunk_entry` (0x155F0034): 1 hit — Lua `createAuction()` fired, reached the CME thunk
4. `cave_call_register` (0x155F004F): 1 hit — about to call `FUN_00A37790`
5. **CLIENT CRASHED** inside `FUN_00A37790` — second-chance AV at EIP `0x00A37792`, reading address `0xFFFFFFFF`

### Crash analysis
- `cave_call_dispatch` (0x155F006D): 0 hits — crash happened during registration, before dispatch
- `CME_register_subscriber` (0xA37790): BP fired (EIP seen at 0x00A37792) but client died before returning
- Flag cell `[0x15600000]` = 0 at crash — the `mov [flag], 1` after the Lua call never executed
- Exception: `EVENT_EXCEPTION`, code `0xC0000005` (Access Violation), reading `0xFFFFFFFF`
- `ExceptionInformation: [0, 4294967295]` — read access violation on address `0xFFFFFFFF`

### Root cause of crash in FUN_00A37790
`FUN_00A37790` is a CME table INSERT function (confirmed from prior Ghidra static analysis and calling convention `__thiscall(CME_singleton, 0, &callback_obj)`). It calls `(*callback_obj)[2]()` (vtable[2] = TypeDesc getter at `0x155F0000`) to get the TypeDescriptor hash key, then inserts the node into the CME table via `FUN_00A39170` + `FUN_00A38950`.

The crash reading `0xFFFFFFFF` occurs inside the insert path. The most likely cause: `FUN_00A38950` or `FUN_00A39170` is doing a lookup of the PARENT bucket chain to find where to insert, and the initial bucket head for TypeDesc hash `0x1E660B0 XOR 0xDEADBEEF` contains `0xFFFFFFFF` (uninitialized sentinel). Some path in the insert dereferences the bucket head directly rather than checking for the sentinel.

Alternative hypothesis: the callback_obj vtable slot layout is wrong. The stub uses vtable cloned from SellItems (`0x019C6380`), but some slot expected by `FUN_00A37790` is pointing to an incompatible function. Specifically, vfunc_3 (`0x00D46FE0`) or vfunc_4 (`0x00429700`) might be called during insertion and expect state that doesn't exist in our minimal stub.

### What to try next session
**Option 1 (stub vtable correction)**: Ghidra — trace what `FUN_00A37790` calls beyond `[callback_vtable[2]]`. If it calls vfunc_3 or vfunc_4, understand what those need. Likely vfunc_3 (`0x00D46FE0`) is the destructor/ref-release — may need to be NOPped or redirected to a safe stub.

**Option 2 (skip FUN_00A37790 entirely — direct FUN_00A372F0 + pre-built CME entry)**: 
Rather than registering a new subscriber dynamically, pre-scan the CME table at cave-init time for the BMSearch entry (vtable `0x019DD410`), clone it into a new node with createAuction's TypeDesc, and insert the cloned node manually. Then call `FUN_00A372F0` directly. This avoids the `FUN_00A37790` insert path entirely.

**Option 3 (reuse BMSearch subscriber registration path)**: At the time `createAuction` fires, temporarily swap `[ESI+0]` (the EventSignal vtable ptr) from `0x019DD360` to `0x019DD410` (BMSearch vtable), call `FUN_00A372F0` which will use the EXISTING BMSearch registration, then restore. BMSearch IS registered in the CME table (confirmed prior session). This avoids needing a new registration at all.

Option 3 is lowest risk: no new CME table writes, no new vtable objects, just a transient vtable swap on the EventSignal before the dispatch call. Add to the cave: before `FUN_00A372F0` call, `MOV [ESI], 0x019DD410`; after call returns, `MOV [ESI], 0x019DD360`.

## Key addresses
- `BMCreateAuction_NetOut_emit`: `0x00E59970`
- `BMSearch_NetOut_emit`: `0x00E59F70`  
- `FUN_00A372F0` (CME event emitter / dispatch): `0x00A372F0`
- `FUN_00A371F0` (subscriber list walk + vtable[5] invoke): `0x00A371F0`
- `FUN_00A37790` (CME subscriber registration): `0x00A37790`
- `FUN_00A36F40` (Park-Miller hash lookup): `0x00A36F40`
- `FUN_00A39170` (hash-table insert outer): `0x00A39170`
- `FUN_00A38950` (hash-table insert inner): `0x00A38950`
- `FUN_00A38630` (insert-with-rehash): `0x00A38630`
- `FUN_00A381A0` (new-node allocator, 0x18 bytes): `0x00A381A0`
- `FUN_00A38ED0` (insert-or-rehash driver): `0x00A38ED0`
- `FUN_00d46f70` (callback_obj constructor): `0x00D46F70`
- `FUN_00d4ebc0` (EventHandler→callback_obj wrapper): `0x00D4EBC0`
- `FUN_00CCC040` (MemberCallback vtable[5] send dispatch): `0x00CCC040`
- `LAB_00D43DC0` (Mercury send lambda, 22 bytes): `0x00D43DC0`
- CME singleton global: `[0x01EE2678]`
- Mercury channel object global: `DAT_01EF2264` = `[0x01EF2264]`
- ldiv import thunk: `[0x017EFB28]`
- createAuction EventSignal vtable: `0x019DD360`
- BMSearch EventSignal vtable: `0x019DD410`
- SellItems MemberCallback vtable: `0x019C6380`
- BMCreateAuction TypeDescriptor: `0x01E660B0`
- Shared pool ptr: `0x01DAB4CC`
- **FIXED CAVE ADDRESS (permanent, survives restarts): `0x01674420`** — 208 bytes of CC in .text section (SGW.exe base=0x00400000, no ASLR; next function starts at 0x016744F0)

## STATIC PASS 2026-06-22 — DEFINITIVE FINDINGS

### *** SUPERSEDED 2026-06-22 SESSION 2 — SEE CORRECTED SECTION BELOW ***
### (Old "method index lives at EventHandler->+0x04" interpretation was WRONG — see full RE below)

### Path 3 (vtable swap) — DEAD on three independent grounds
1. Hash key is TypeDescriptor (`param_4`), not EventSignal vtable. Swap doesn't change the lookup.
2. Type-equality guard rejects any subscriber whose TypeDescriptor != `0x01E660B0`.
3. Method index comes from the subscriber's EventHandler, not EventSignal vtable.

### Crash site in FUN_00A38ED0 confirmed
`0x00A390E0: MOV EAX,[EDI+0x4]` where `EDI=0xFFFFFFFF` (uninitialized bucket slot). Confirmed as uninitialized bucket crash.

### Bucket slot index for createAuction TypeDescriptor: RUNTIME-DEPENDENT (not static 2)
- Key = `0x01E660B0 XOR 0xDEADBEEF = 0xDF4BDE5F` → Park-Miller → **0x6DF41E32** (confirmed by PowerShell)
- Slot = `0x6DF41E32 & [outer_table+0x3C]` (runtime mask), then threshold-adjust using `[outer_table+0x40]`
- Prior "SLOT 2 static" claim was WRONG — hash constant was miscalculated (0x1385E502 incorrect)
- Bucket base: `[CME_singleton + 0x30]` (= [sub_table+0x14]), sentinel: `[CME_singleton + 0x24]` (= [sub_table+0x08])
- HEAD at `[bucket_base + final_slot*4]`, TAIL at `[bucket_base + final_slot*4 + 4]`

---

## CORRECTED MERCURY-SEND PARAMETER LAYOUT (2026-06-22, session 2 static RE)

### The crash root cause (re-derived)
Crash at `0x00C6FCF9: MOVZX ECX, word ptr [EBX+0x1E]` with EBX = 0x3E (method index integer).
EBX is loaded from param_2 of `0x00C6FC40` = `fake_eh→+0x04`. We had 0x3E (method index) there.
**`0x00C6FC40` expects an entity description pointer at param_2, not a raw integer.**

### LAB_00D43DC0 — corrected byte-verified disassembly
Verified by `mcp__ghidra__disassemble_bytes` on 30 bytes at `0x00D43DC0`:
```
00d43dc0: 8b442404   MOV EAX,[ESP+4]      ; EAX = EventSignal ptr   (first stack arg from FUN_00CCC040)
00d43dc4: 8b5108     MOV EDX,[ECX+8]      ; EDX = EventHandler->+0x08 = method_node ptr
00d43dc7: 50         PUSH EAX             ; push EventSignal
00d43dc8: 8b4104     MOV EAX,[ECX+4]      ; EAX = EventHandler->+0x04 = entity_desc ptr
00d43dcb: 8b4c240c   MOV ECX,[ESP+0xC]    ; ECX = pool ptr (second stack arg from FUN_00CCC040, now at [ESP+0xC] after 1 push)
00d43dcf: 52         PUSH EDX             ; push method_node
00d43dd0: 50         PUSH EAX             ; push entity_desc
00d43dd1: 51         PUSH ECX             ; push pool
00d43dd2: e899baf2ff CALL 0x00C6F870      ; lazy-init channel factory (ignores all 4 args, returns channel in EAX)
00d43dd7: 8bc8       MOV ECX,EAX          ; ECX = channel (irrelevant — 0x00C6FC40 is __stdcall, gets conn from CEGUI singleton)
00d43dd9: e862bef2ff CALL 0x00C6FC40      ; the real send fn — param_1=[ESP+4]=pool, param_2=[ESP+8]=entity_desc, param_3=[ESP+12]=method_node
                                           ; RET 0xC cleans 3 args; EventSignal at [ESP+16] cleaned by LAB_00D43DC0's RET 8
```

Prior annotation was wrong: the old memo labeled `[ECX+4]` = "METHOD INDEX" and `[ECX+8]` = "channel ptr"
and `[ESP+4]` as "pool_ptr". The verified bytes show: `[ECX+4]` = entity_desc, `[ECX+8]` = method_node,
`[ESP+4]` at lambda entry = EventSignal, pool arrives at `[ESP+0xC]` after 1 PUSH.

Also: `0x00C6F870` does NOT consume the 4 pushed args — it has its own SEH frame (0xC) + local frame (0x10)
and pops via `ADD ESP, 0x1C; RET`. Caller's 4 pushes remain on stack, then consumed by `0x00C6FC40 RET 0xC`
(3 args) + the caller of LAB_00D43DC0's `RET 8` (the 4th = EventSignal).

### FUN_00CCC040 (vtable[5] send dispatch) — corrected call chain
Verified by `mcp__ghidra__disassemble_bytes` at `0x00CCC040` (only 23 bytes + INT3s):
```
00ccc040: 8b542408  MOV EDX,[ESP+8]    ; EDX = param_2 of vtable[5] call = pool (0x01DAB4CC)
00ccc044: 8bc1      MOV EAX,ECX        ; EAX = callback_obj ('this' thiscall)
00ccc046: 8b4c2404  MOV ECX,[ESP+4]    ; ECX = param_1 of vtable[5] call = EventSignal ptr
00ccc04a: 51        PUSH ECX           ; push EventSignal
00ccc04b: 8b4804    MOV ECX,[EAX+4]   ; ECX = callback_obj->+0x04 = EventHandler ptr (= &fake_eh)
00ccc04e: 8b4008    MOV EAX,[EAX+8]   ; EAX = callback_obj->+0x08 = lambda address (= LAB_00D43DC0)
00ccc051: 52        PUSH EDX           ; push pool
00ccc052: ffd0      CALL EAX           ; call lambda with ECX=&fake_eh, stack=(EventSignal, pool)
00ccc054: c20800    RET 0x8
```

vtable[5] is called by `FUN_00A371F0` as `(*piVar1 + 0x14)(param_2, param_3)` where
`param_2 = EventSignal` and `param_3 = pool (0x01DAB4CC)` from `FUN_00A372F0`.

### 0x00C6F870 — corrected understanding
NOT just a void factory. It IS a factory (lazily allocates the Mercury channel into `DAT_01EF2264`)
AND returns the channel in EAX. But LAB_00D43DC0 passes its 4 args BEFORE this call,
and `0x00C6F870` ignores them entirely. Its `ADD ESP, 0x1C; RET` pops only its OWN SEH frame.
The 4 caller pushes (pool, entity_desc, method_node, EventSignal) remain on the stack for `0x00C6FC40`.
The `MOV ECX, EAX` after `CALL 0x00C6F870` is unused — `0x00C6FC40` is `__stdcall` and gets
the ServerConnection from `[CEGUI_singleton + 8]` internally, not from ECX.

### 0x00C6FC40 (CEGUI__unknown_00c6fc40) — what it actually is
The "Outgoing entity method send" function. __stdcall, 3 params:
- param_1 = pool (BigWorld packet pool / connection-id context — checked at `[param_1+0xC]` vs local player id)
- param_2 = entity_desc ptr — must have `+0x1E` = componentKey matching current entity desc
- param_3 = method_node ptr — has `+0x44` = wire method index, `+0x1C`&3 = direction flags, `+0x20/+0x30` = arg type descriptor iterators
Internally: calls `ServerConnection_startEntityMessage(conn, *(param_3+0x44))` to write msg_id byte,
then iterates arg type descriptors from method_node to serialize EventSignal args via vtable[8] per arg.

### CORRECT fake_eh field layout
`fake_eh` = the EventHandler struct. Passed into `LAB_00D43DC0` as ECX via `FUN_00CCC040`'s
`MOV ECX, [EAX+4]` where AX = callback_obj. The lambda then reads:
- `[ECX+4]` = `EventHandler->+0x04` = **entity_desc ptr** (result of `FUN_00c6ef40(channel, 2)`)
- `[ECX+8]` = `EventHandler->+0x08` = **method_node ptr** (result of `FUN_00c6e810(entity_desc, 2, str_ptr)`)

These are confirmed from real subscriber construction chain:
- `FUN_00D6CE00` (SellItems registrar): `iVar1 = FUN_00c6ef40(channel, 2)` → entity_desc; `iVar2 = FUN_00c6e810(iVar1, 2, name_buf)` → method_node; calls `FUN_00d5a230(alloc, iVar1, iVar2)`.
- `FUN_00D5A230` (SellItems EventHandler ctor): `*(this+4) = param_1` (entity_desc); `*(this+8) = param_2` (method_node).
- `FUN_00D46F70` (MemberCallback ctor): `*(this+4) = param_1` (EventHandler ptr = &fake_eh); `*(this+8) = param_2` (&LAB_00D43DC0).

So `callback_obj->+0x04` = &fake_eh and `callback_obj->+0x08` = &LAB_00D43DC0 (UNCHANGED — correct).
What changes: the *content* of fake_eh (+0x04 and +0x08).

### WRONG values in prior fake_eh:
- `+0x04 = 0x3E` (raw method index) → WRONG; must be entity_desc ptr
- `+0x08 = channel_ptr [0x01EF2264]` → WRONG; must be method_node ptr

### CORRECT values for fake_eh at one-shot init:
```asm
; Step 1: get entity_desc ptr (FUN_00c6ef40 is __stdcall, returns entity_desc in EAX)
CALL  0x00C6F870           ; ensure channel init; EAX = DAT_01EF2264 (channel object)
PUSH  2                    ; param_2 = cell direction (2)
PUSH  EAX                  ; param_1 = channel object
CALL  0x00C6EF40           ; __stdcall, cleans 2 args; EAX = entity_desc (verified: RET 0xC at 0x00C6F1D6)
; EAX = entity_desc ptr for SGWPlayer cell
MOV   [fake_eh + 0x04], EAX

; Step 2: get method_node ptr for createAuction
; FUN_00c6e810(entity_desc, 2, string_ptr) is __cdecl/stdcall, returns method_node in EAX
; param_3 = ptr to a std::string struct containing "createAuction"
; For SSO (string len=13 < 15): string struct layout at a cave address:
;   [+0x04..+0x10] = "createAuction\0" (14 bytes inline)
;   [+0x14]        = 0x0D (length = 13)
;   [+0x18]        = 0x0F (capacity = 15 = SSO sentinel)
; Build this as a 0x1C-byte static blob in the cave at <cave_string_slot>
; then pass its address as param_3:
PUSH  <cave_string_slot>   ; ptr to "createAuction" std::string struct
PUSH  2                    ; param_2 = cell direction
PUSH  EAX                  ; param_1 = entity_desc (from step 1)
CALL  0x00C6E810           ; __stdcall(?), cleans args; EAX = method_node ptr
MOV   [fake_eh + 0x08], EAX
```

Note: `0x019515F8` holds the raw C-string "createAuction" but `FUN_00c6e810`/`FUN_01591420` expect a
MSVC `std::string`-like struct (reads `[str+0x18]` for capacity, `[str+4]` for inline chars).
A fake std::string struct must be baked into the cave with proper layout.

### std::string struct for "createAuction" (13 chars, SSO-safe)
```
Cave offset  Size  Content
+0x00        4     [ignored / padding / any]
+0x04        13    "createAuction"
+0x11        1     0x00  (null terminator)
+0x12        2     [pad]
+0x14        4     0x0000000D  (length = 13)
+0x18        4     0x0000000F  (capacity = 15 = SSO mode sentinel)
```
Total: 0x1C bytes. FUN_01591420 checks `*(uint*)(str+0x18) < 0x10` → 0xF < 0x10 → TRUE → uses inline chars at `(char*)(str+4)`.

### FUN_00c6e810 calling convention
From `FUN_00D6CE00`: called as `iVar2 = FUN_00c6e810(iVar1, param_2, param_3)` where iVar1 is in EAX
and param_2/param_3 from stack. Ghidra shows `void FUN_00c6e810(int param_1, int param_2, uint param_3)`.
Likely __cdecl or __stdcall with 3 args. Confirm by reading `RET n` at end of function before using.

### Updated cave layout at 0x01674420 (additional slots needed)
The cave at `0x01674420` (208 CC bytes) needs to expand to include:
- `fake_eh` with 12 bytes (unchanged size, changed semantics)
- `cave_string_slot`: 0x1C bytes for the "createAuction" std::string struct
- one_shot_init code: ~30 bytes more than before (2 resolve calls instead of 1 simple MOV)

The fixed cave at `0x01674420` has 208 bytes (ending before `0x016744F0`). Total needed:
6 (TypeDesc) + 4 (flag) + 24 (vtable) + 12 (fake_eh) + 12 (callback_obj) + 28 (string struct) + ~90 (thunk) + ~50 (init code) ≈ 226 bytes — tight. Verify available space or spill string struct to data section.

### Passing fake_eh layout to FUN_00A37790 (still needed for registration)
The FUN_00A37790 registration crash was from uninitialized bucket (fixed by bucket pre-init).
After bucket pre-init, `FUN_00A37790(CME_singleton, 0, &callback_obj)` calls `callback_obj->vtable[2]()`
= TypeDesc getter (returns `0x01E660B0`) for the hash key. This path is correct and unchanged.
The `FUN_00A37790` registration only uses callback_obj's vtable[2] and vtable[3]/[4] (destructor/ref).
It does NOT use fake_eh's +0x04/+0x08. The fake_eh content matters ONLY at dispatch time (vtable[5] call).

### Channel field (fake_eh→+0x08) — confirmation that channel is NOT stored in EventHandler
For real subscribers: `EventHandler→+0x04` = entity_desc (a pointer to entity description)
and `EventHandler→+0x08` = method_node (a pointer to method metadata with +0x44 = wire method index).
The Mercury channel is derived by `0x00C6FC40` from the CEGUI singleton at runtime, NOT from the EventHandler.
`[0x01EF2264]` (the Mercury channel global) is NOT stored in fake_eh. The prior spec storing it at +0x08 was wrong.

### PASS/FAIL on "swap +0x04 and +0x08" hypothesis
**FAIL.** The crash is NOT fixed by swapping the two values. The values themselves are wrong:
- Current +0x04 = 0x3E (integer) — must be a valid entity_desc pointer
- Current +0x08 = channel_ptr — must be a valid method_node pointer  
Swapping would put channel_ptr at +0x04 (also wrong — not an entity_desc) and 0x3E at +0x08 (also wrong — not a method_node). Both fields need entirely different values from runtime resolution calls.

## SESSION 2026-06-22 (7th attempt) — SEH CRASH IN LUA CONTEXT

### Crash summary
Cave at 0x01674420 built correctly (VirtualProtect → RWX confirmed, all 5 CALL targets verified by disassembler). Entity guard NOP and vtable[2] redirect both applied. Tick cave at 0x056F0100 fired correctly (tick_cave_lua_call BP = 1 hit). cave_thunk_entry (0x0167447C) = 1 hit — vtable[2] thunk was reached. But crashed before cave_call_register (0x016744BE) = 0 hits.

### Root cause: SEH manipulation inside Lua execution context
FUN_00C6F870 and FUN_00C6EF40 both install SEH frames via `MOV FS:[0], ESP` at their prologs. When called from within an active Lua coroutine → C callback → our thunk chain, Wine's exception dispatcher at 0x7B47D089 reads a null `next` pointer in the SEH chain → AV (0xC0000005 reading 0x0). Crash at EIP 0x7B386C89 (Wine ntdll).

Evidence: flag at 0x056F0000 was 0 (cleared by tick cave before crash), stack return address 0x00433DCA = ADD ESP,0xC after Lua_doString_wide return site, ExceptionInformation [0, 0] = read from null.

### Fix: pre-populate fake_eh via createthread (not from Lua)
Resolver stub written at 0x056E0100:
```
CALL 0x00C6F870           ; get channel (EAX)
PUSH 2; PUSH EAX
CALL 0x00C6EF40           ; entity_desc (EAX), __stdcall RET 8
MOV [0x0167444C], EAX     ; fake_eh+0x04 = entity_desc
PUSH 0x01674460; PUSH 2; PUSH EAX
CALL 0x00C6E810           ; method_node (EAX)
MOV [0x01674450], EAX     ; fake_eh+0x08 = method_node
RET
```
All CALL targets verified by disassembler (0x056E0100: call 0x00C6F870, call 0x00C6EF40, call 0x00C6E810, mov/mov/ret).

### Next-session protocol (7th shot)
1. VirtualProtect 0x01674000 → 0x40 (RWX) via createthread stub at 0x056E0010
2. Re-write full cave at 0x01674420 (TypeDesc getter, flag, vtable, fake_eh zeroed, callback_obj, cave_str, thunk)
3. NOP 0x00E599A8 (6 bytes: 90×6); redirect 0x019DD368 → 7C 44 67 01
4. Re-write resolver stub at 0x056E0100 (as above)
5. `createthread 0x056E0100` — runs outside Lua context, no SEH conflict
6. Read [0x0167444C] and [0x01674450] — MUST be non-null before proceeding
7. Write [0x01674428] = 1 (pre-mark init done; thunk skips init on first call, goes straight to register+dispatch)
8. Set non-freezing BPs on 0x0167447C, 0x016744BE, 0x016744DC, 0x016744E9, 0x00A37790
9. Re-write tick cave at 0x056F0100, arm tick detour 0x00416EC0 → JMP 0x056F0100
10. Write [0x056F0000] = 1 to trigger
11. Watch: cave_thunk_entry → cave_call_register → cave_call_dispatch → DB row sequence_id=4 seller_id=62

### If A37790 crashes again (bucket issue)
Bucket[2] this session = live chain (head 0xEF63B3C0, next 0xF120FFBD) — NOT 0xFFFFFFFF. Pre-init skip should be correct. But if A37790 still crashes: skip registration, call FUN_00A372F0 directly (the thunk's skip_init path does this). The live bucket means dispatch WILL find the entry without registering.

### Heap pages from this session (die on relaunch)
- 0x056E0000 (VirtualProtect stub + resolver stub) — needs rebuild
- 0x056F0000 (flag + scripts + tick cave) — needs rebuild
- 0x056F0100 (tick cave code) — needs rebuild

## STATIC PASS 2 (post-crash-7) — COMPLETE CRASH RE-ANALYSIS 2026-06-22

### SEH-chain-corruption theory REFUTED by memory map

EIP 0x7B386C89 is in **msvcr80.dll `.text`** (0x7B371000–0x7B3D4000), NOT Wine ntdll.
ExceptionAddress 0x7B47D089 is in **resampledmo.dll** (audio codec, stack red herring).
Environment: **native Windows**. No Wine. SEH semantics are standard Win32.

Memory map evidence (from x64dbg get_memory_map, session 2026-06-22):
```
0x7B370000  msvcr80.dll  (header)
0x7B371000  .text  0x63000 bytes  ← 0x7B386C89 is here (offset +0x15C89)
0x7B3D4000  .rdata
0x7B460000  resampledmo.dll  ← 0x7B47D089 is here (offset +0x1C089, a stack return addr, not crash site)
```
The crash at EIP 0x7B386C89 = `msvcr80!terminate()` or `msvcr80!_CxxThrowException` internal unwinder.
The `0x7B47D089` was on the call stack from an audio thread, not the crashing thread.

### Real crash cause: C++ exception thrown by FUN_00C6EF40 due to wrong param_1

`FUN_00C6EF40(param_1, 2)` received `DAT_01EF2264` (Mercury channel value, 0xECEB7C00) as `param_1`.

Decompile of `FUN_00C6EF40` shows it calls `FUN_00c66c10(param_1)` first. `FUN_00C66C10` does a
GameEntityManager registry lookup using `param_1` as a key:
```c
iVar2 = DAT_01ef244c;  // GameEntityManager singleton
this = *(void**)(iVar2 + 0x90);  // entity description collection
uVar1 = FUN_0158eca0(this, param_1, local_4);  // map lookup: is param_1 a known entity?
if (result == 0xffff) → _CxxThrowException("Cannot register rpc ... entity not found")
```
The Mercury channel object (0xECEB7C00) is NOT a registered entity in GameEntityManager → lookup returns 0xFFFF →
`FUN_00C6EF40` calls `_CxxThrowException`. This C++ exception propagates up through the Lua VM
(which has no `__try` around C callbacks) → reaches `msvcr80!terminate()` → process crash.

### Correct argument to FUN_00C6EF40: entity CLASS NAME STRING

From the mega-registrar (register_NetOut_onStrikeTeamResponse, 0x00DB3390), the sellItems registration:
```c
BW__unknown_00438c40(local_method_buf, "sellItems");   // method name
BW__unknown_00438c40(local_entity_buf, "SGWPlayer");   // entity CLASS NAME (not a pointer, a string)
pvVar2 = Mercury__unknown_00c6f870();                  // get Mercury channel → used as ECX ('this')
FUN_00d6ce00(pvVar2, local_entity_buf, 2, (uint)local_method_buf);
```

`FUN_00D6CE00` signature: `__thiscall(this=mercury_channel, param_1=entity_class_name_str, param_2=direction_int, param_3=method_name_str)`

Inside FUN_00D6CE00:
```c
iVar1 = FUN_00c6ef40(param_1, param_2);         // "SGWPlayer" → entity_desc
iVar2 = FUN_00c6e810(iVar1, param_2, param_3);  // entity_desc, 2, "sellItems" → method_node
// alloc EventHandler, call FUN_00c6ea70 to register
```

We were passing the Mercury channel VALUE as param_1. The correct value is a pointer to the string "SGWPlayer".

### Correct method name string: "BMCreateAuction" (NOT "createAuction")

"createAuction" is the Lua binding name. The entity description method table uses the .def XML name.
From `entities/defs/interfaces/SGWBlackMarketManager.def` line 50:
```xml
<BMCreateAuction>
    <Exposed/>
    ...
</BMCreateAuction>
```
Method name for `FUN_00C6E810` = `"BMCreateAuction"`.
`<Exposed/>` confirms the flag check `*(byte*)(iVar1+0x1c) & 4` will pass. No throw from that path.

### FUN_00A37790 and FUN_00A372F0: SEH-clean, Lua-context-safe

Decompile of FUN_00A37790: no ExceptionList manipulation, no _CxxThrowException.
Decompile of FUN_00A372F0: pure hash-table walk, no exceptions.
Decompile of FUN_00A36F40 (hash fn): pure arithmetic (XOR + Park-Miller), no allocations.
Both are safe to call from any context including Lua C callback context.
**The register+dispatch path is NOT the problem. Only the init path (FUN_00C6EF40) was throwing.**

### Cross-thread safety of FUN_00C6F870/EF40/E810 from createthread

All three functions use FS:[0] for SEH frames but restore it before returning (standard MSVC SEH pattern).
All globals they read (DAT_01EF2264, DAT_01EF244C) are process-wide, not TLS.
Safe to call from createthread if given correct arguments.
Risk: if they throw, the exception is unhandled on the createthread thread → that thread dies, main process survives.
Mitigation: set one_shot_flag AFTER successful return, not before, so a failed init allows retry.

### REVISED CAVE PLAN: call FUN_00D6CE00 directly (single call, all resolution internal)

Old plan: fake_eh + manual entity_desc/method_node resolution + A37790 + A372F0 (7 ops, ~200 bytes)
New plan: one call to `FUN_00D6CE00(mercury_channel, "SGWPlayer", 2, "BMCreateAuction")` (~30 bytes + 2 strings)

FUN_00D6CE00 handles internally: entity_desc lookup, method_node lookup, EventHandler allocation, CME registration via FUN_00c6ea70. After it returns, FUN_00A37790 and FUN_00A372F0 are called by its internals — no need to call them from our cave.

New cave layout at 0x01674420:
```
+0x00  4 bytes   one_shot_flag (init=0)
+0x04  10 bytes  "SGWPlayer\0"
+0x0E  16 bytes  "BMCreateAuction\0"
+0x1E  ~40 bytes thunk code:
    CMP [cave+0x00], 0; JNZ @done
    MOV [cave+0x00], 1         ; set flag
    PUSH cave+0x0E             ; "BMCreateAuction" = param_3
    PUSH 2                     ; direction = param_2
    PUSH cave+0x04             ; "SGWPlayer" = param_1
    MOV ECX, [0x01EF2264]      ; mercury channel = 'this' (ECX for __thiscall)
    CALL FUN_00D6CE00          ; 0x00D6CE00
    @done: [stolen bytes] JMP back
```

The vtable[2] redirect at 0x019DD368 must still be in place (redirects emit vtable[2] to this cave thunk).
Entity guard NOP at 0x00E599A8 must still be in place.
The fake_eh, callback_obj, vtable-6-slot, and "createAuction" SSO string blobs are NO LONGER NEEDED.

### Key addresses confirmed/corrected

- FUN_00D6CE00: 0x00D6CE00 — the correct single-call registration wrapper
- FUN_00C6EF40: takes entity CLASS NAME STRING as param_1, NOT a channel/entity pointer
- FUN_00C6E810: takes method .def name ("BMCreateAuction"), NOT Lua name ("createAuction")
- FUN_00A37790 and FUN_00A372F0: both Lua-safe (no SEH, no exceptions)
- msvcr80.dll: 0x7B370000–0x7B40B000 (crash was here, NOT Wine)
- "SGWPlayer" entity class is the correct entity name for all BM methods

## STATIC ANALYSIS — THROW-SAFE OPTION A (2026-06-22, post crash-8)

### Crash 8 post-mortem (FUN_00D6CE00 still threw)
FUN_00D6CE00 calls FUN_00C6EF40 and FUN_00C6E810 internally. Even with correct string arguments,
if either throws (e.g., entity not yet registered, or method not found), the exception propagates
through the Lua C-callback stack and corrupts the heap. FUN_00D6CE00 is itself NOT throw-safe to
call from a Lua context.

Additionally: the cave had a string/thunk overlap bug (thunk at +0x1E overlapped last 2 bytes of
"BMCreateAuction\0" which needs 16 bytes from +0x10 to +0x1F; thunk must start at +0x20 = 0x01674440).

### RECOMMENDED APPROACH: Bypass both throwing functions, call FUN_00C6EA70 directly

FUN_00D6CE00 decompile:
```c
iVar1 = FUN_00c6ef40(param_1, param_2);        // THROW A: entity not found
iVar2 = FUN_00c6e810(iVar1, param_2, param_3); // THROW B: method not found; THROW C: not Exposed
this_00 = FUN_00418e30(0xc);                   // alloc 12 bytes — no throw
puVar3  = FUN_00d5a230(this_00, iVar1, iVar2); // construct EventHandler — no throw
FUN_00c6ea70(this, iVar1, param_2, iVar2, puVar3); // register — no throw (confirmed Lua-safe)
```

Strategy: resolve entity_desc and method_node as PURE READS before calling any of the above,
validate at each step, then call only the no-throw tail (FUN_00418E30, FUN_00D5A230, FUN_00C6EA70).

### entity_desc computation (pure reads, no throw)

FUN_00C66C10 @ 0x00C66C10: takes entity class name string, does GameEntityManager map lookup.
Returns ushort class_index, or 0xFFFF on miss. Does NOT throw. Safe to call from any context.

FUN_0158E1A0 @ 0x0158E1A0: computes `class_index * 0x110 + *(entity_collection + 0x10)`.
Pure arithmetic. Cannot throw.

Full sequence:
```
gem          = [0x01EF244C]          ; GameEntityManager singleton
entity_coll  = [gem + 0x90]          ; entity collection ptr
coll_base    = [entity_coll + 0x10]  ; base ptr of entity_desc array
class_idx    = FUN_00C66C10("SGWPlayer")   ; ushort, 0xFFFF on miss → gate on this
entity_desc  = class_idx * 0x110 + coll_base
```

### method_node lookup (pure read, no throw)

FUN_01591420 @ 0x01591420: `__thiscall(this=&cell_method_map, param_1=method_name_string)`.
Does std::map::find by name. Returns node ptr or 0. Does NOT throw.

The cell-method map for direction=2 lives at entity_desc + 0x88 (confirmed from FUN_00C6E810 branch).

FUN_01591420 reads method_name_string as MSVC std::string-like:
  - if `*(uint*)(str+0x18) < 0x10` → inline chars at `(char*)(str+4)`
  - else → chars at `*(char**)(str+4)`
Must pass a valid std::string struct, not a raw C string pointer.

### Exposed-bit validation (pure read, no throw)

After FUN_01591420 returns method_node (non-zero):
  `exposed_byte = [method_node + 0x1c]`
  `if (exposed_byte & 4) == 0 → abort` (should never happen: BMCreateAuction is <Exposed/>)

### Registration calls (no throw, Lua-safe)

FUN_00418E30 @ 0x00418E30: `operator new(0xC)` — alloc 12 bytes. Returns NULL on OOM (safe to gate).
FUN_00D5A230 @ 0x00D5A230: EventHandler ctor. Sets `[this+0x04] = entity_desc, [this+0x08] = method_node`.
FUN_00C6EA70 @ 0x00C6EA70: CME register. `__thiscall(this=mercury_channel, entity_desc, direction=2, method_node, EventHandler_ptr)`.
  Confirmed Lua-safe (FUN_00A37790 and FUN_00A372F0 have no SEH frames, no exceptions).

### New cave layout (thunk starts at +0x20 = 0x01674440)

Cave at 0x01674420, thunk at 0x01674440:
```
+0x00  4 bytes   one_shot_flag = 0
+0x04  10 bytes  "SGWPlayer\0"
+0x0E  2 bytes   padding (zeros)
+0x10  16 bytes  "BMCreateAuction\0"   ← ends at +0x1F = 0x0167443F
+0x20  thunk starts here (= 0x01674440)
```

Thunk logic (in cave, fires from vtable[2] redirect):
```asm
PUSH ESI
MOV ESI, ECX                          ; save EventSignal
CMP [0x01674420], 0                   ; check one_shot_flag
JNZ @restore                          ; skip if already registered
; --- one-shot init ---
; get class_idx via FUN_00C66C10
MOV EAX, [0x01EF244C]                 ; GameEntityManager
MOV EAX, [EAX + 0x90]                 ; entity_collection
MOV EAX, [EAX + 0x10]                 ; coll_base
MOVZX ECX, WORD PTR [...]             ; class_idx = FUN_00C66C10("SGWPlayer") — call or pre-read
; ... compute entity_desc = class_idx * 0x110 + coll_base
; ... call FUN_01591420(entity_desc+0x88, &str_struct) → method_node
; ... check method_node != 0
; ... call FUN_00418E30(0xC) → eh_ptr
; ... call FUN_00D5A230(eh_ptr, entity_desc, method_node)
; ... call FUN_00C6EA70(channel, entity_desc, 2, method_node, eh_ptr)
MOV [0x01674420], 1                   ; set one_shot_flag AFTER successful registration
@restore:
MOV ECX, ESI
POP ESI
JMP 0x00E5C420                        ; original vtable[2]
```

NOTE: The cave thunk calls FUN_00C66C10 and FUN_01591420 (both no-throw). It does NOT call
FUN_00C6EF40, FUN_00C6E810, or FUN_00D6CE00. Zero throw risk.

### Addresses for next session (all fixed-VA in SGW.exe)
- FUN_00C66C10  @ 0x00C66C10  ; entity class name → ushort class_index (no throw)
- FUN_01591420  @ 0x01591420  ; cell-method name → method_node ptr (no throw, pure read)
- FUN_00D6CE00  @ 0x00D6CE00  ; single-call registration wrapper (__thiscall, ECX=channel)
- FUN_00A372F0  @ 0x00A372F0  ; CME dispatch (tail-called by thunk, matches BMSearch pattern)
- DAT_01EF244C               ; GameEntityManager singleton ptr
- DAT_01EF2264               ; Mercury channel singleton ptr
- Cave thunk entry: 0x01674440
- Vtable[2] redirect slot: 0x019DD368 → 0x01674440 (LE: 40 44 67 01)
- Event_NetOut_BMCreateAuction::RTTI_TypeDescriptor: 0x01E660B0
- TypeDescriptor_01dab4cc (shared pool TypeDesc): 0x01DAB4CC

## SESSION 2026-06-22 (CRASH #9) — RAW C-STRING vs STD::STRING MISMATCH

### What was applied (all confirmed by disassembly):
- Entity guard NOP 0x00E599A8 ✓
- Vtable redirect 0x019DD368 ← 0x01674440 ✓
- Dispatch thunk at 0x01674440: MOV EAX,[ECX+8]; PUSH 0x01E660B0; PUSH 0x01DAB4CC; PUSH ECX; MOV ECX,[ESP+0x10]; PUSH EAX; CALL 0x00A372F0; RET 0x4 ✓
- Tick cave at 0x05630100, detour 0x00416EC0 → 0x05630100 ✓ (1259 non-freezing hits confirmed)
- FUN_D6CE00_entry: 1 hit — FUN_00D6CE00 called from main-thread tick once
- msvcr80!terminate() → process dead

### Root cause confirmed: raw C-string vs std::string*

FUN_00D6CE00 passes param_1 directly to FUN_00C6EF40. FUN_00C6EF40 expects MSVC std::string*
(SSO struct), not a raw char*. We passed 0x01674424 (char* "SGWPlayer\0") and 0x01674430 (char* "BMCreateAuction\0").

FUN_00C6EF40 reads [param_1+0x18] for SSO capacity check. For our raw pointer:
[0x01674424+0x18] = [0x0167443C] = 0x00000000 (zero, in the zero-padding region of the cave).
Zero fails SSO check → interprets [param_1+4] as char** heap pointer → reads bytes of
"SGWPlayer" string as a pointer → AV → _CxxThrowException → terminate().

Evidence: EBX=0x74637541 ("tcuA"), EDX=0x47574753 ("SGWG") — RTTI name bytes from incorrectly-
dereferenced string. These exact bytes appear when the C6EF40 map lookup iterates RTTI strings.

### Fix: pass SSO std::string structs, not raw char* pointers

"SGWPlayer" SSO at 0x05630300 (from heap page, survives restart needing rebuild):
  +0x00: 00 00 00 00
  +0x04: 53 47 57 50 6C 61 79 65 72 00 00 00 00 00 00 00  ("SGWPlayer\0" + 7 pad)
  +0x14: 09 00 00 00  (length=9)
  +0x18: 0F 00 00 00  (capacity=15, SSO sentinel)

"BMCreateAuction" SSO at 0x05630320:
  +0x00: 00 00 00 00
  +0x04: 42 4D 43 72 65 61 74 65 41 75 63 74 69 6F 6E 00  ("BMCreateAuction\0")
  +0x14: 0F 00 00 00  (length=15)
  +0x18: 0F 00 00 00  (capacity=15, SSO sentinel)

Tick cave call becomes: PUSH 0x05630320; PUSH 2; PUSH 0x05630300; MOV ECX,[0x01EF2264]; CALL 0x00D6CE00

### Other confirmed findings from this session:
- RTTI TypeDescriptors: 0x01E66224 = ".?AVEvent_NetOut_BMSearch@@", 0x01E660B0 = ".?AVEvent_NetOut_BMCreateAuction@@" — both confirmed live via inspect_memory_content
- FUN_00A372F0: __thiscall, 4 stack args, callee RET 0x10 — confirmed by disassembly
- Dispatch thunk's RET 0x4 correctly matches FUN_00E5CAE0's RET 0x4 — cleans CME_singleton from emit caller
- MOV ECX,[ESP+0x10] after 3 pushes correctly picks up CME singleton from emit site (at [ESP+4] at thunk entry → [ESP+0x10] after PUSH RTTI1, PUSH RTTI2, PUSH ECX)
- VirtualProtect stub bug: PUSH ESP pushes value that VirtualProtect overwrites with old-prot (0x20 = PAGE_EXECUTE_READ) — corrupts thread return address → EIP=0x20 after RET. VP DID succeed (EAX=1). Fix: allocate scratch DWORD on heap, push its address instead of PUSH ESP.
- createthread NOT safe for GameEntityManager functions (FUN_00C66C10 throws from non-main-thread too)
- ALL SGW functions touching entity/method maps must run from main-thread tick cave only

### VP stub fix for next session
```asm
; VP stub at 0x05630010 — safe version
PUSH EBP
MOV EBP, ESP
SUB ESP, 4              ; scratch space for lpflOldProtect
LEA EAX, [EBP-4]        ; EAX = &scratch
PUSH EAX                ; lpflOldProtect (not the thread return address)
PUSH 0x40               ; PAGE_EXECUTE_READWRITE
PUSH 0x1000             ; dwSize
PUSH 0x01674000         ; lpAddress
CALL VirtualProtect
MOV ESP, EBP
POP EBP
RET
```

### Next session setup (complete)
1. VP stub (fixed, with LEA scratch): allocate page, write fixed VP stub, createthread to it
2. Write cave at 0x01674420: data+thunk (59 bytes, same as this session — correct)
3. Write SSO structs at 0x05630300 / 0x05630320 (28 bytes each)
4. NOP 0x00E599A8, redirect 0x019DD368 ← 0x01674440
5. Tick cave: flag==1 → FUN_00D6CE00 with SSO ptrs; flag==2 → Lua createAuction(10237,100,500,1)
6. Arm detour 0x00416EC0, set flag=1, confirm D6CE00 hits without crash
7. Set flag=2, confirm dispatch_thunk_entry hits, verify sgw_auction DB row

## FINAL ARCHITECTURE — vtable redirect IS required (2026-06-22, static-pass-3)

### FUN_00E5C420 (native BMCreateAuction vtable[2]) — SHELVED, never dispatches

FUN_00E5C420 → FUN_00E5C320:
  scalable_malloc(0x18)                           ; CAN throw _CxxThrowException on OOM
  FUN_00E5C210(alloc, TypeDesc, ...)              ; inits TypedEmitInfo struct
  FUN_00E6BEB0 → enqueue into this+0x10/0x14      ; deferred UI/compositing queue
  FUN_00A372F0 is NEVER called.

Xref check: FUN_00E5C320 has exactly ONE caller — FUN_00E5C420. FUN_00E5C420 xrefs: only within the emit.
The queue at this+0x10/0x14 is drained by FUN_00E6B480, which calls FUN_00E6CA60 (compositing pipeline),
NOT FUN_00A372F0. Registered subscribers are never notified via this path.

### FUN_00E5CAE0 (BMSearch vtable[2]) — WORKING, direct dispatch

FUN_00E5CAE0:
  FUN_00A372F0(param_1, *(uint*)(this+8), this,
               &TypeDescriptor_01dab4cc,
               &Event_NetOut_BMSearch::RTTI_Type_Descriptor)

One call, no alloc, no queue. This is the live wire path.

### Conclusion

BMCreateAuction's native vtable[2] = broken deferred-queue path (shelved feature).
Even with FUN_00D6CE00 registering a subscriber, the native path will never fire it.
Vtable[2] redirect at 0x019DD368 → 0x01674440 (thunk) IS REQUIRED.

### Thunk dispatch path (after one_shot_flag set by Phase 2 registration)

Tail-call FUN_00A372F0 matching BMSearch pattern exactly:
  PUSH 0x01E660B0              ; &Event_NetOut_BMCreateAuction::RTTI_TypeDescriptor (param_5)
  PUSH 0x01DAB4CC              ; &TypeDescriptor_01dab4cc (param_4)
  PUSH ESI                     ; this = EventSignal (param_3)
  PUSH dword ptr [ESI+8]       ; pool/connection context from EventSignal+8 (param_2)
  PUSH [ESP+original_param_1]  ; EventSignal_ptr passed to vtable[2] (param_1)
  CALL 0x00A372F0

### method_node+0x44 = wire method index (recon check c)
FUN_00C6FC40 reads *(param_3+0x44) to write the msg_id byte.
Recon must confirm [method_node+0x44] == 0x3E (62) before calling FUN_00D6CE00.

### Finalized recon-first plan

Phase 1 (reads only, all must pass before any write):
  A. gem=[0x01EF244C] != 0 AND channel=[0x01EF2264] != 0
  B. class_idx = FUN_00C66C10("SGWPlayer") != 0xFFFF → entity_desc = class_idx*0x110 + coll_base
  C. method_node = FUN_01591420(entity_desc+0x88, &"BMCreateAuction"_sso) != 0
     AND [method_node+0x1c]&4 != 0 (exposed)
     AND [method_node+0x44] == 0x3E (method index)

Phase 2 (main-thread tick, one-shot, no Lua on stack):
  ECX=[0x01EF2264]; PUSH &"BMCreateAuction"_sso; PUSH 2; PUSH &"SGWPlayer"_sso; CALL 0x00D6CE00
  Set one_shot_flag AFTER return.

Phase 3: Write 0x019DD368 ← 40 44 67 01 (vtable redirect)
Phase 4: Write 0x00E599A8 ← 90×6 (entity guard NOP)
Phase 5: onBMOpen → createAuction(10237,100,500,1) → verify sgw_auction DB row

### FUN_01591420 std::string struct for "BMCreateAuction" (15 chars, SSO-safe)
"BMCreateAuction" = 15 chars. 15 == 0xF == SSO capacity sentinel value. Check: does FUN_01591420
check `*(uint*)(str+0x18) < 0x10`? If capacity=0xF < 0x10 → uses inline chars at str+4. YES, SSO mode.
But capacity should be the allocated capacity, not the string length. The real SSO sentinel is capacity field
= 0xF (string length fits inline when length <= 15, which 15 does exactly). The null terminator adds 1,
total inline storage needed = 16 bytes, which fits in the 16-byte inline buffer (str+4 to str+13 = 10 bytes
for small string? — CHECK: MSVC small-string uses a 16-byte buffer at str+4, capacity=15 sentinel).
"BMCreateAuction" (15 chars) + null = 16 bytes. Exactly fills SSO buffer. capacity=0xF.

SSO struct at cave (e.g. at 0x01674480 if space allows, or on a data page):
  +0x00  4  any (vtable ptr placeholder, ignored)
  +0x04  15 "BMCreateAuction"
  +0x13  1  0x00 null terminator
  +0x14  4  0x0000000F  (length = 15)
  +0x18  4  0x0000000F  (capacity = 15 = SSO sentinel)
  Total: 0x1C bytes

For "SGWPlayer" (9 chars) SSO struct:
  +0x04  9  "SGWPlayer"
  +0x0D  1  0x00
  +0x14  4  0x00000009  (length = 9)
  +0x18  4  0x0000000F  (capacity = 15 = SSO sentinel)
  Total: 0x1C bytes

Both can live in a data section or the cave's spare bytes after the thunk.

### Abort conditions (next session)
At any step, if:
- gem == 0 or channel == 0 → STOP
- class_idx == 0xFFFF → STOP (entity not yet registered; must be past world-entry)
- entity_desc == 0 or looks invalid → STOP
- method_node == 0 → STOP (BMCreateAuction not in cell method map)
- eh_alloc == NULL → STOP (OOM)
→ Do NOT set one_shot_flag. Set an error_flag instead so next trigger retries.
→ Report step + live register values before aborting.

## BUCKET PRE-INIT — CORRECTED (session summary, 2026-06-22)

### Context
FUN_00A37790 crashed at `0x00A390E0: MOV EAX,[EDI+0x4]` with EDI=0xFFFFFFFF because the bucket
for BMCreateAuction's TypeDescriptor key was uninitialized. The team-lead prescribed a conditional
bucket pre-init before the A37790 call. Three earlier attempts had wrong offsets.

**Note:** The current Phase 3 plan (FINAL ARCHITECTURE above) calls FUN_00D6CE00 from a main-thread
tick cave without FUN_00A37790 directly. FUN_00D6CE00 calls FUN_00C6EA70 which calls FUN_00A37790
internally. The bucket pre-init is still needed before that chain fires, BUT the Phase 3 tick-cave
approach runs from main-thread context where the game loop may have already initialized the CME table
with enough entries that the bucket is no longer uninitialized. Test FIRST; only add pre-init if
A37790 still crashes (bucket still 0xFFFFFFFF).

### CME hash table object layout (from FUN_00A37710 + FUN_00A38D50 ctors)

`[DAT_01EE2678]` = outer_table (0x44-byte alloc). Nested sub-table starts at outer_table+0x1c.

FUN_00A36F40 takes `ECX = sub_table = outer_table+0x1c`. Offsets from outer_table base:
- mask       = `[outer_table+0x3C]`  (= `[sub_table+0x20]`, init = 1)
- threshold  = `[outer_table+0x40]`  (= `[sub_table+0x24]`, init = 1)
- bucket_arr = `[outer_table+0x30]`  (= `[sub_table+0x14]`, init by FUN_00A378F0 with 9 DWORDs)
- sentinel   = `[outer_table+0x24]`  (= `[sub_table+0x08]` = node_pool_ptr from FUN_00A36D30)

Each bucket entry: HEAD at `[bucket_arr + slot*4]`, TAIL at `[bucket_arr + slot*4 + 4]`.

### Park-Miller result for key 0x01E660B0

Computed by PowerShell (confirmed):
- XOR: `0x01E660B0 XOR 0xDEADBEEF = 0xDF4BDE5F` = -548,676,001 signed
- ldiv(-548676001, 127773): quot = -4294, rem = -18739
- uVar3_64 = (-18739 * 16807) + (-4294 * -2836) = -314,946,373 + 12,177,784 = -302,768,589
- uVar3 (int32) = -302,768,589 → negative → +0x7FFFFFFF → **1,844,715,058 = 0x6DF41E32**

This is the precomputed constant for `MOV EAX` in the pre-init.

### Slot at runtime

After `AND EAX, [ECX+0x3C]` (mask), apply threshold check:
- if `[ECX+0x40]` (threshold) <= slot: `slot += -1 - (mask >> 1)`
- At table init (mask=1, threshold=1): slot=1 wraps to 0. But at runtime the table has grown
  (many NetOut subscriptions added at startup), so mask and threshold will be larger.
- The correct HEAD address = `[ECX+0x30] + final_slot * 4`

### Corrected pre-init bytes (43 bytes, ECX = [DAT_01EE2678])

```
B8 32 1E F4 6D     MOV EAX, 0x6DF41E32       ; Park-Miller for 0x01E660B0
23 41 3C           AND EAX, [ECX+0x3C]       ; & runtime mask
8B 51 40           MOV EDX, [ECX+0x40]       ; threshold
3B D0              CMP EDX, EAX              ; if threshold <= slot
77 0B              JA  +11 (no_wrap)         ; skip if threshold > slot
8B 51 3C           MOV EDX, [ECX+0x3C]       ; mask again
D1 EA              SHR EDX, 1                ; mask >> 1
F7 DA              NEG EDX                   ; -(mask>>1)
4A                 DEC EDX                   ; -1 - (mask>>1)
03 C2              ADD EAX, EDX              ; slot += adjustment
; no_wrap:
C1 E0 02           SHL EAX, 2               ; slot * 4 (byte offset)
03 41 30           ADD EAX, [ECX+0x30]      ; HEAD addr = bucket_base + offset
81 38 FF FF FF FF  CMP dword [EAX], 0xFFFFFFFF
75 0A              JNE +10 (skip_seed)
8B 51 24           MOV EDX, [ECX+0x24]      ; sentinel node ptr
89 10              MOV [EAX], EDX           ; seed HEAD
89 50 04           MOV [EAX+4], EDX         ; seed TAIL
; skip_seed:
```

### Prior wrong values (DO NOT USE)
- `[ECX+0x20]` for mask → WRONG (was outer_table offset 0x20, not sub-table mask)
- `[ECX+0x14]` for bucket_arr → WRONG (outer_table+0x14 is the outer bucket alloc, not sub)
- `[ECX+0x08]` for sentinel → WRONG (outer_table+0x08 = 0 from ctor, not the node pool)
- `SHL EAX, 3` (×8 stride) → WRONG (stride is ×4 per DWORD slot, but each bucket pair is HEAD+TAIL → write both +0x00 and +0x04 relative to the slot address)
- Precomputed hash `0x241B3748` → WRONG (arithmetic error from miscomputed XOR)
- "SLOT 2 (static)" claim → WRONG (slot varies with runtime mask; no static slot)

## FINAL LOCKED PLAN — OPTION B SYNTHESIZE (team-lead locked, 2026-06-22)

### Two questions resolved by team-lead static analysis

**Q1 — ECX for FUN_00A37790 (CONFIRMED):**
`FUN_0054C900` is CME-singleton lazy-init only. `FUN_00D4EBC0` calls `FUN_00A37790(this, 0, callback_obj)`
where `this` is passed from the CME singleton. `FUN_00A37790` navigates to `this+0x1c` (the sub-hash)
internally. Registration call:
```asm
MOV ECX, [0x01EE2678]   ; CME singleton (outer_table ptr)
PUSH 0x01674464         ; &callback_obj in cave
PUSH 0
CALL 0x00A37790         ; __thiscall ECX, RET 8
```
No separate hash-table object to locate.

**Q2 — EH (EventHandler) layout (CONFIRMED):**
`FUN_00D5A230`: `[this+4]=entity_desc, [this+8]=method_node`.
`0x00D43DC0` bytes: `8B 51 08` = `MOV EDX,[ECX+8]` (method_node), `8B 41 04` = `MOV EAX,[ECX+4]` (entity_desc).
Mini-EH struct: `[EH+0]=unused, [EH+4]=entity_desc, [EH+8]=method_node`.
`callback_obj[+4] = &EH_struct` (pointer to the 12-byte EH block, NOT entity_desc directly).

### Cave layout at 0x01674420 (FINAL)

```
0x01674420  TypeDesc getter (6 bytes): B8 B0 60 E6 01 C3
0x01674426  one_shot_flag (4 bytes, init=0)
0x0167442A  [2 bytes pad]
0x0167442C  fake_vtable (24 bytes = 6 slots × 4):
              [+0x00] = 0x00E53F00  slot 0 (SellItems dtor)
              [+0x04] = 0x00428B60  slot 1 (SellItems vfunc_1)
              [+0x08] = 0x01674420  slot 2 (our TypeDesc getter)
              [+0x0C] = 0x00D46FE0  slot 3 (SellItems vfunc_3)
              [+0x10] = 0x00429700  slot 4 (SellItems vfunc_4)
              [+0x14] = 0x00CCC040  slot 5 (shared send dispatch)
0x01674444  callback_obj (12 bytes):
              [+0x00] = 0x0167442C  vtable ptr (fake_vtable above)
              [+0x04] = 0x01674464  EventHandler ptr → &fake_eh
              [+0x08] = 0x00D43DC0  lambda (LAB_00D43DC0)
0x01674450  [20 bytes scratch/pad]
0x01674464  fake_eh (12 bytes):
              [+0x00] = 0x00000000  (unused)
              [+0x04] = 0x00000000  (entity_desc — written at one-shot init)
              [+0x08] = 0x00000000  (method_node — written at one-shot init)
0x01674470  "SGWPlayer" SSO struct (0x1C bytes, for FUN_0158ECA0):
              [+0x00..+0x03] = 0
              [+0x04..+0x0C] = "SGWPlayer\0"
              [+0x0D..+0x13] = zeros
              [+0x14] = 9   (length)
              [+0x18] = 15  (capacity, SSO sentinel)
0x0167448C  "BMCreateAuction" SSO struct (0x1C bytes, for FUN_01591420):
              [+0x00..+0x03] = 0
              [+0x04..+0x12] = "BMCreateAuction\0"
              [+0x13] = 0
              [+0x14] = 15  (length)
              [+0x18] = 15  (capacity, SSO sentinel)
0x016744A8  dispatch thunk (vtable[2] redirect target, ~22 bytes):
              8B 41 08           MOV EAX,[ECX+8]
              68 B0 60 E6 01     PUSH 0x01E660B0
              68 CC B4 DA 01     PUSH 0x01DAB4CC
              51                 PUSH ECX
              8B 4C 24 10        MOV ECX,[ESP+0x10]
              50                 PUSH EAX
              E8 XX XX XX XX     CALL 0x00A372F0
              C2 04 00           RET 0x4
Total cave: ~0xCA bytes — fits before 0x016744F0
```

### Execution sequence (next session)

**Phase 1 — Liveness check**
- `get_debugger_status` → Running:True, PID matches
- `get_latest_event` → no exception, EIP sane
- Second-chance AV = dead; STOP if seen

**Phase 2 — VirtualProtect**
- Allocate scratch DWORD, `LEA EAX,[EBP-4]` as lpflOldProtect address
- VirtualProtect 0x01674000 → 0x40 (RWX); verify EAX=1

**Phase 3 — Recon (reads only; STOP on any fail)**
- A. `gem=[0x01EF244C] != 0` AND `channel=[0x01EF2264] != 0`
- B. `coll=[gem+0x90]`; `coll_base=[coll+0x10]`
- C. `class_idx = FUN_0158ECA0(coll, ?)` — call or probe static data for "SGWPlayer" index
  Alternative (team-lead spec): `FUN_00C66C10("SGWPlayer")` → class_idx (no-throw, confirmed safe)
  Then `entity_desc = FUN_0158E1A0(coll, class_idx)` OR `entity_desc = class_idx * 0x110 + coll_base`
- D. `method_node = FUN_01591420(entity_desc+0x88, &"BMCreateAuction"_sso)` → non-zero
- E. `[method_node+0x1C] & 4 != 0` (exposed bit)
- F. `[method_node+0x44] == 0x3E` (wire method index = 62)

**Phase 4 — Write cave**
- Write all bytes per layout above
- Disassemble-verify fake_vtable, callback_obj, fake_eh, dispatch thunk

**Phase 5 — Write fake_eh fields**
- `MOV DWORD PTR [0x01674468], <entity_desc>`  ; fake_eh+0x04
- `MOV DWORD PTR [0x0167446C], <method_node>`  ; fake_eh+0x08

**Phase 6 — Bucket pre-init + A37790 registration**
- `MOV ECX, [0x01EE2678]`
- **LIVE SANITY-LOG (before A37790):** Read and log:
  - `live_mask    = [ECX+0x3C]`
  - `live_buckets = [ECX+0x30]`
  - `live_sentinel= [ECX+0x24]`
  - Compute `uVar3 = 0x6DF41E32` (pre-mask constant), then `slot = uVar3 & live_mask`; apply threshold wrap if `[ECX+0x40] <= slot`
  - `head_addr = live_buckets + slot*4`
  - `head_val  = [head_addr]`
  - **STOP if any of:** slot >= live_mask+1, head_val == 0xFFFFFFFF and pre-init didn't seed it, head_val is neither live_sentinel nor a plausible heap ptr (i.e. > 0x00400000 and < 0x7FFFFFFF)
- Run 58-byte corrected pre-init (see BUCKET PRE-INIT section below)
- `MOV ECX, [0x01EE2678]; PUSH 0x01674444; PUSH 0; CALL 0x00A37790`

**Phase 7 — Rendezvous check**
- Recompute slot using same Park-Miller logic
- Read HEAD at `[[0x01EE2678]+0x30 + slot*4]`
- STOP if HEAD == `[[0x01EE2678]+0x24]` (still empty = registration failed silently)

**Phase 8 — Set one_shot_flag**
- `MOV DWORD PTR [0x01674426], 1`

**Phase 9 — Patches**
- NOP 0x00E599A8 (6×90) — entity guard bypass
- `0x019DD368` ← `A8 44 67 01` (vtable[2] → dispatch thunk)

**Phase 10 — Non-freezing BPs**
- 0x016744A8 (dispatch thunk entry)
- 0x00A372F0 (CME emit)
- 0x00C6FC40 (entity-method sender)
- 0x00A37790 (CME register, to confirm no re-entry)
- 0x00A390E0 (crash canary: MOV EAX,[EDI+4]; if EDI=0xFFFFFFFF → wrong bucket seeded → STOP)
All: fastresume=1, log=1

**Phase 11 — Trigger**
- Tick cave: `createAuction(10237, 100, 500, 1)` via main-thread Lua
- OR: onBMOpen injection → open BM window → click Create button

**Phase 12 — Verify**
- `sgw_auction` DB row: seller_id=62, item_type_id=10237, starting_bid=100, buyout_price=500

### HOLD
Owner controls relaunch. Do NOT execute until relaunch authorization received from team-lead.

## BUCKET PRE-INIT — FINAL V2 (inline Park-Miller, no hand constants, 2026-06-22)

### Team-lead corrections (third pass)
1. Offsets already correct: +0x24/+0x30/+0x3C/+0x40 all confirmed. NOT changed.
2. Keep pre-init. Uninitialized buckets are real and confirmed by session-3 crash. NOT dropped.
3. Do NOT use a hand-computed Park-Miller constant. Two attempts gave two different answers.
   Replicate `FUN_00A36F40`'s EXACT math inline; the key = `0x01E660B0`, divisor = `0x1F31D`.

### FUN_00A36F40 algorithm (from Ghidra decompile, verbatim)

```c
// this = sub_table = singleton+0x1c
// param_2 = &key (key = 0x01E660B0)
lVar4 = ldiv(*param_2 ^ 0xDEADBEEF, 0x1F31D);       // XOR then signed divide
uVar3 = lVar4.rem * 0x41A7 + lVar4.quot * -0xB14;    // linear combine
if ((int)uVar3 < 0) { uVar3 = uVar3 + 0x7FFFFFFF; }  // adjust if negative
uVar3 = *(uint*)(this+0x20) & uVar3;                 // mask = [singleton+0x3C]
if (*(uint*)(this+0x24) <= uVar3) {                   // threshold = [singleton+0x40]
    uVar3 = uVar3 + (-1 - (*(uint*)(this+0x20) >> 1));
}
// bucket HEAD at: *(int*)(this+0x14) + uVar3*4  = [singleton+0x30] + uVar3*4
// bucket TAIL at: HEAD_addr + 4
```

### Inline pre-init assembly (57 bytes, ECX = [0x01EE2678] throughout)

```asm
PUSH EBX                            ; 53             save caller-saved EBX
MOV  EAX, 0x01E660B0                ; B8 B0 60 E6 01  key = TypeDescriptor
XOR  EAX, 0xDEADBEEF                ; 35 EF BE AD DE  xored = key ^ mask
CDQ                                 ; 99              sign-extend EAX→EDX:EAX
MOV  EBX, 0x1F31D                   ; BB 1D F3 01 00  divisor = 127773
IDIV EBX                            ; F7 FB           EAX=quot, EDX=rem (signed)
IMUL EDX, EDX, 0x41A7               ; 69 D2 A7 41 00 00  rem * 16807
IMUL EAX, EAX, 0x0B14               ; 69 C0 14 0B 00 00  quot * 2836 (positive)
SUB  EDX, EAX                       ; 2B D0           uVar3 = rem*0x41A7 - quot*0x0B14
                                    ; (matches binary's IMUL+SUB, not ADD with -0x0B14)
TEST EDX, EDX                       ; 85 D2
JGE  short no_adj                   ; 7D 05 (5 bytes forward)
ADD  EDX, 0x7FFFFFFF                ; 81 C2 FF FF FF 7F  adjust if negative
no_adj:
AND  EDX, [ECX+0x3C]                ; 23 51 3C        slot = uVar3 & mask
MOV  EAX, [ECX+0x40]                ; 8B 41 40        threshold
CMP  EAX, EDX                       ; 3B C2           if threshold <= slot
JA   short no_wrap                  ; 77 09 (9 bytes forward)
MOV  EAX, [ECX+0x3C]                ; 8B 41 3C        mask
SHR  EAX, 1                         ; D1 E8
NEG  EAX                            ; F7 D8
DEC  EAX                            ; 48              -1-(mask>>1)
ADD  EDX, EAX                       ; 03 D0           slot += adjustment
no_wrap:
SHL  EDX, 2                         ; C1 E2 02        slot*4 = byte offset to HEAD
ADD  EDX, [ECX+0x30]                ; 03 51 30        EDX = &bucket[slot].HEAD
CMP  DWORD PTR [EDX], 0xFFFFFFFF    ; 81 3A FF FF FF FF
JNE  short skip_seed                ; 75 09 (9 bytes forward)
MOV  EAX, [ECX+0x24]                ; 8B 41 24        sentinel node ptr
MOV  [EDX], EAX                     ; 89 02           seed HEAD
MOV  [EDX+4], EAX                   ; 89 42 04        seed TAIL
skip_seed:
POP  EBX                            ; 5B
```

Note on `JA no_wrap` distance: after `CMP EAX,EDX` the wrap body is:
`8B 41 3C` (3) + `D1 E8` (2) + `F7 D8` (2) + `48` (1) + `03 D0` (2) = 10 bytes → JA +10 → `77 0A`.

Note on `JGE no_adj` distance: after `TEST EDX,EDX` the adj body is:
`81 C2 FF FF FF 7F` (6 bytes) → JGE +6 → `7D 06`.

Corrected displacement bytes:
- `JGE no_adj` → `7D 06`
- `JA no_wrap` → `77 0A`
- `JNE skip_seed` → `75 09` (skip_seed body is MOV EAX,3 + MOV [EDX],EAX + MOV [EDX+4],EAX = 3+2+3 = 8... wait: `8B 41 24`=3, `89 02`=2, `89 42 04`=3 = 8 bytes → `75 08`)

Final corrected displacements: JGE=`7D 06`, JA=`77 0A`, JNE=`75 08`.

### Registration call (immediately after pre-init, ECX must still = [0x01EE2678])

```asm
MOV  ECX, [0x01EE2678]    ; reload CME singleton (EBX restored, ECX may be clobbered by prev code)
PUSH 0x01674444           ; &callback_obj
PUSH 0
CALL 0x00A37790           ; __thiscall, RET 8
```

### Non-freezing BP to add (crash canary)

Add BP at `0x00A390E0` (the session-3 crash site: `MOV EAX,[EDI+0x4]`) with fastresume=1, log=1.
If this BP fires with EDI=0xFFFFFFFF, the pre-init seeded the wrong slot or missed it.

### Rendezvous check (after A37790)

Re-run the same inline Park-Miller → EDX = &HEAD slot.
`CMP DWORD PTR [EDX], [ECX+0x24]` — if equal, HEAD is still the sentinel (registration wrote nothing → STOP).
If HEAD differs from sentinel and is a valid-looking pointer → registration succeeded.

## EMULATOR-DERIVED FINAL VALUES (2026-06-22, third static pass)

### Q1 — Pre-mask hash constant for key 0x01E660B0

**CONFIRMED: `0x6DF41E32`**

Derivation (three independent paths agree):
1. PowerShell computation #1 (this session, direct IDIV emulation)
2. PowerShell computation #2 (this session, verified with check: quot*divisor+rem == dividend)
3. Ghidra decompile + manual Java-equivalent arithmetic

Steps:
- xored = 0x01E660B0 ^ 0xDEADBEEF = 0xDF4BDE5F = -548,676,001 (signed int32)
- ldiv(-548,676,001, 127,773): quot=-4,294, rem=-18,739 (check: -4294×127773 + -18739 = -548,676,001 ✓)
- rem×0x41A7 = -18,739×16,807 = -314,946,373
- quot×0x0B14 = -4,294×2,836 = -12,177,784 (note: SUB instruction: rem_part - quot_part)
- uVar3 = -314,946,373 - (-12,177,784) = -302,768,589 = 0xEDF41E33 (signed negative)
- adjust: -302,768,589 + 0x7FFFFFFF = 1,844,715,058 = **0x6DF41E32**

Note: earlier session attempt at `0x241B3748` had an arithmetic error (division step).
Note: `0x6DF41E32` was also produced in a prior session but was marked "unverified."
It is now CONFIRMED by two independently-computed PowerShell derivations + cross-check.

### Q2 — Bucket stride

**CONFIRMED: stride = 4 bytes (`SHL 2`, `×4`)**

Evidence:
- `LEA ECX,[ECX+EAX*4]` at 0x00A36F8D — explicit ×4 multiplier in x86 SIB
- `FUN_00A378F0` init formula: `iVar1 + param_1*4` → 4 bytes per entry
- Initial allocation: 9 entries × 4 bytes = 36 bytes

### Q3 — Bucket structure and seed shape

**CONFIRMED: single flat DWORD array, HEAD+TAIL are adjacent entries**

Layout (from disassembly + FUN_00A38D50 + FUN_00A378F0 ctor chain):
- `bucket_arr[slot]` (at `bucket_arr + slot*4`) = HEAD of linked list for this slot
- `bucket_arr[slot+1]` (at `bucket_arr + slot*4 + 4`) = TAIL = next slot's HEAD (shared boundary)
- Empty slot: HEAD == TAIL == sentinel node address

Sentinel node:
- Heap-allocated 0x34-byte node by FUN_00A36D30
- Self-referential: [node+0x00]=[node+0x04]=node (doubly-linked list sentinel)
- Address stored at [outer_table+0x24] = [singleton+0x24]

Pre-init writes (when HEAD == 0xFFFFFFFF = raw uninitialized):
- `[EDX]   = [ECX+0x24]`  (seed HEAD with sentinel ptr)
- `[EDX+4] = [ECX+0x24]`  (seed TAIL with sentinel ptr)
This produces HEAD==TAIL==sentinel, matching what a freshly-initialized empty slot looks like.

### DO NOT USE (wrong values, archived)
- Hand constant `0x241B3748` → arithmetic error confirmed (earlier session)
- `[ECX+0x08/0x14/0x20]` → all off by 0x1c from outer-table base
- `SHL 3` → wrong; use `SHL 2` (stride is `slot*4` per disasm + ctor)
- "SLOT 2 (static)" → wrong; slot is hash-derived at runtime from mask
