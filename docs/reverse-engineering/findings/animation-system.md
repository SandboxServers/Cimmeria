# Animation System — Reverse Engineering Findings

> **Diátaxis type**: reference
> **Audience**: engineers implementing animation-related server and client behavior in Cimmeria
> **Last updated**: 2026-05-13
> **Confidence**: HIGH for notify pipeline and anim state machine; MEDIUM for reload event-set sourcing (issue #210)
> **Worker**: W-anim, V5 Documentation Campaign session 4

---

## Overview

Stargate Worlds uses a two-layer animation architecture. The UE3 engine layer (`UAnimTree`, `UAnimNode` subclasses, `UAnimNotify`) drives skeletal mesh playback on the client. On top of that, CME added a custom SGW layer (`USGWAnim_*` node classes, `USGWAnimController`, `USGWAnimNotify_Event`) that bridges animation events into the CME EventSignal bus, selects animations based on entity combat stance read from a server-synchronized field, and reads animation sequences from the ZipStorage cooked data system. The server triggers animation state changes via `Event_NetIn_onKismetEventSetUpdate` (which routes the current event-set ID to the client) and via ability/action outcomes that fire CME events naming specific animation sequences. The client then dispatches these through the anim tree automatically.

---

## Anim Notify Pipeline

### Emitter — `SGWAnimNotifyEvent_Emit` (`0x00e974b0`)

This is the CME emit function called when a UE3 animation notify fires on a character. Source: `SGWAnimNotify_Event.cpp` line 25.

**Call sequence:**
1. Validates that the `UAnimNotify` object is a valid `GameEntity`-derived type via `FUN_00d404b0` — walks the UObject class hierarchy (`+0x34` → inheritance chain, stepping via `+0x3C`) checking against `DAT_01ef26d4` (cached `GameEntity` class descriptor).
2. Reads the actor name from `this+0x44/+0x48` (wstring pointer). Converts to narrow string via `FUN_00423f40`.
3. Reads event type hash at `entity+0x1B4` (a pre-computed hash of the event name).
4. Calls `CmeEventSignal_GetSystem` (`0x0155f790`) + `CmeEventSignal_LookupByName` (`0x00a5c0f0`) to resolve the named CME signal. **The signal name is the actor name — not a compile-time constant.** This means the actor must be named to match the registered CME signal (e.g. `Event_NetOut_UseAbility`).
5. If signal found: `__RTDynamicCast` to `CME::SubjectEvent<void>`, writes event hash at `signal+0x8`.
6. Reads `SequenceName` from `context+0xB4` (FName), converts to narrow string.
7. Calls `FUN_00a4fa60` (SetField variant) for `SequenceName`, then `FUN_0043d490` (SetField) for:
   - `CancelOnMovement` — bit 0 of `this+0x3C`
   - `PlaybackType` — `this+0x40` (int)
   - `HaltAnimTree` — bit 1 of `this+0x3C`
8. Dispatches via `(*signal->vtable[2])(...)` — fires all registered subscribers.

**Plate fields emitted:**

| Field | Source offset | Type |
|-------|--------------|------|
| `SequenceName` | `context+0xB4` (FName) | string |
| `CancelOnMovement` | `this+0x3C` bit 0 | bool |
| `PlaybackType` | `this+0x40` | int |
| `HaltAnimTree` | `this+0x3C` bit 1 | bool |

**Vtable position:** `SGWAnimNotifyEvent_Emit` appears at slot +0x28 (slot 10) of the `USGWAnimNotify_JumpEvent` vtable region at `0x019e7460`. Only one DATA xref to the function itself — no direct callers; always invoked through virtual dispatch.

### Dispatch chain

The UE3 engine calls `UAnimNotify::Notify` (virtual slot, base at `UAnimNotify` vtable `0x018bee44`) when a sequence playback position crosses a notify marker. `USGWAnimNotify_Event` overrides this with `FUN_00e97070`:

```
UE3 anim system
  -> UAnimNotify::Notify (virtual dispatch through UAnimNotify vtable)
    -> FUN_00e97070 (USGWAnimNotify_Event::Notify override, SGWAnimNotify_Event.cpp)
      reads: mesh component (this+0x3C/0x40), bone socket (this+0x50/0x54), flags
      validates: component has valid scale via FUN_00814540
      dispatches: (*entity->vtable[+0xE8])(bone, sequence, filter) [normal]
               or (*entity->vtable[+0xEC])(bone, sequence, filter) [bone-socket variant]
```

`FUN_00e97070` does NOT call `SGWAnimNotifyEvent_Emit` directly — that function is invoked separately when the entity chooses to emit a CME event rather than a direct vtable call. The two paths are:
- Direct vtable `+0xEC/+0xE8`: triggers anim immediately on the entity
- CME emit `SGWAnimNotifyEvent_Emit`: broadcasts to all subscribers of the named signal

### Notify class hierarchy

Three CME-custom `UAnimNotify` subclasses, all at `0x00e96f60`-range:

| Class | Source | Destructor | Purpose |
|-------|--------|-----------|---------|
| `USGWAnimNotify_Script` | `SGWAnimNotify_Script.cpp` | `0x00e96f60` | Executes a CME/Python script on notify |
| `USGWAnimNotify_Event` | `SGWAnimNotify_Event.cpp` | `0x00e97290` | Emits a named CME event (`SGWAnimNotifyEvent_Emit`) |
| `USGWAnimNotify_JumpEvent` | `SGWAnimNotify_JumpEvent.cpp` | `0x00e97ae0` | JumpEvent variant (source at `0x019e74c0`) |

All three destructors are `return 1` stubs — MSVC scalar destructor pattern per campaign convention.

---

## Anim Event Sources — Ability vs Item EventSet (Issue #210)

### The two event-set systems

The client supports **two distinct event-set mechanisms** that the server can use to configure an entity's animation/kismet behavior:

**1. KismetEventSet** — server→client delivery via `Event_NetIn_onKismetEventSetUpdate`:
- Server sends: `kismetEventSetId` (int)
- Client handler: `FUN_00e6fd20` (`GameEntity.cpp` line 0x149)
- Storage: `this+0x98` on the `GameEntity`
- After store: calls `FUN_00d29c90` → `FUN_00d28070` → ZipStorage lookup to actually load the event set data

Subscriber registration (from RTTI): `MemberCallback<GameEntity, Event_NetIn_onKismetEventSetUpdate>` — only `GameEntity` is the subscriber.

**2. BehaviorEventSet** — client→server request/response:
- Client command: `Event_NetOut_AddBehaviorEventSet` / `Event_NetOut_RemoveBehaviorEventSet`
- Handler: `FUN_00c891b0` (`SGWTextCommandManager.cpp` line 0xB84)
- Emits: `Event_NetOut_LoadBehavior` (Pattern B, `FUN_00cbd610`) with field `aBehaviorEventSetId`

**3. ItemEventSet** — archetype field embedded in item cooked data:
- Field: `ItemEventSet` at `itemArchetype+0x0C` (from `FUN_015d47c0`)
- Serialized as `CookedData:ItemEventSetType`
- The serializer (`FUN_015d36f0`) walks the `ItemEventSet` array and writes each entry

### The reload anim path

The reload flow from the client side:
```
UI/slash cmd "reload"
  -> FUN_00c889a0 (SGWTextCommandManager.cpp line 0xB4B)
     reads: reloadType from CME event
     creates: Event_NetOut_RequestReload (Pattern B, FUN_00cbcda0)
     field: aReloadType (int)
     dispatches: FUN_00caf850
       -> SGWNetworkManager -> entity method -> server
```

The server receives `requestReload` with `reloadType`. The Cimmeria implementation (issue #210 context) currently triggers the reload animation from the **ability's** event set. The binary evidence:

- The client does NOT directly look up `ItemEventSet` when processing a reload request. It sends `Event_NetOut_RequestReload` with a `reloadType` integer to the server.
- The server is expected to respond with a `kismetEventSetId` update via `Event_NetIn_onKismetEventSetUpdate`, which the client stores in `GameEntity+0x98` and uses to select the Kismet animation sequences.
- `ItemEventSet` (at `itemArchetype+0x0C`) is the **archetype definition** of what event sets are available for a given item type. This data is loaded from the PAK file. The `kismetEventSetId` the server sends back should refer to an entry within this archetype's `ItemEventSet` list.

**Issue #210 root cause (hypothesis):** The server currently sends an `ability` event set ID rather than the `itemArchetype.ItemEventSet` ID when processing reload. The correct behavior is:
1. Player triggers reload (sends `Event_NetOut_RequestReload` with `reloadType`)
2. Server looks up the **equipped item's archetype** → reads `ItemEventSet` list → selects the appropriate event set entry matching `reloadType`
3. Server sends `Event_NetIn_onKismetEventSetUpdate` with that `ItemEventSet` entry's ID
4. Client stores it at `GameEntity+0x98` → ZipStorage loads the kismet sequences → anim tree plays the correct reload

The function that picks the event set on the server side is **not in the client binary** (it's server logic in `crates/`). The client only tells us what it expects: a `kismetEventSetId` pointing into the item archetype's `ItemEventSet`, not into a generic ability event set.

---

## Anim State Machine — Combat Stance, Holster, Crouch Transitions

### `USGWAnim_TransitionByStance` — the stance-driven anim selector

Source: `SGWAnim_TransitionByStance.cpp`. Class size: 0x138 bytes. StaticClass: `FUN_00e969d0`.

**Key fields (relative to `this`):**

| Offset | Type | Purpose |
|--------|------|---------|
| `+0x11C` | `char[4]` | Current stance code (e.g. `"1HS"` = OneHanded+Stand) |
| `+0x120` | `int` | Current anim table index (-1 = none) |
| `+0x124` | `int` | Sequence playback counter |
| `+0x128` | `entity*` | Pointer to the entity whose combat state drives transitions |
| `+0x12C` | `SGWAnimTransitionEntry*` | Animation table (entries from AnimMap.xml) |
| `+0x130` | `int` | Animation table entry count |
| `+0xC0` bit 1 | `bool` | If set: suppress anim trigger during TickAnim |
| `+0xCC` | `USkeletalMeshComponent*` | Mesh component for scale validation |

**Tick flow** (`FUN_00e968d0`, vfunc_70 override):
1. Calls base `UAnimNodeSequence::Tick`.
2. Reads `entity+0x3D0` — the combat stance code (e.g. `"1HC"` for OneHanded+Crouch).
3. Calls `FUN_00e96720` (lookup): compares current code to table entries. Table entries are 0x14 bytes, with weapon code at `+0x00` (3 chars) and posture code at `+0x06` (3 chars). Returns matching index.
4. On index change: saves new index to `+0x120`, resets counter `+0x124 = 0`, fires `FUN_00e96810` (trigger).
5. Saves current code to `+0x11C`.

**AnimMap.xml mapping:** The XML file at `game/sgw/Working/SGWGame/Config/AnimMap.xml` defines the 6-char key structure directly:
- First 3 chars: weapon type prefix: `RX` (Relaxed), `1H` (OneHanded), `2H` (TwoHanded)
- Next 3 chars: stance: `S` (Stand) or `C` (Crouch)
- The combined code is what the client reads from `entity+0x3D0`

This confirms: **the server must write the correct 3+3 char weapon+posture code to `entity+0x3D0`** (or an equivalent wire field that maps to it) for stance transitions to work.

**Transition trigger** (`FUN_00e96810`):
- Walks the sequence list associated with the matched table entry
- Fires vtable `+0x13C` (anim action callback) for each sequence whose position is within the valid range (defined by `entry+0xC`)
- Calls vtable `+0x10C` (blend weight = 1.0) if mesh scale > 0 (non-zero character)
- Resets to index -1 via vtable `+0x110` when complete

### `USGWAnim_BlendByPosture` (`FUN_00e92170`)

Source: `SGWAnim_BlendByPosture.cpp`. Class size: 0xF0 bytes. Ctor calls `FUN_00e8a6e0` (SGWAnim base ctor) then stamps `USGWAnim_BlendByPosture::vftable`. Drives stand/crouch blending — distinct from the transition (crossfade) system above.

### `USGWAnim_BlendByWeapon` (`FUN_00e925a0`)

Source: `SGWAnim_BlendByWeapon.cpp`. Class size: 0xF4 bytes. Ctor calls `FUN_00e8a6e0` then stamps `USGWAnim_BlendByWeapon::vftable`. Drives Relaxed/OneHanded/TwoHanded blend selection.

### Cross-links

- **W-weap**: `entity+0x3D0` is the weapon+posture code. The weapon subsystem must write this field when weapon equip/holster occurs. The `AnimMap.xml` `<OneHanded>`, `<TwoHanded>`, `<Relaxed>` transition sequences are the client-side half of holster/draw.
- **W-state**: Crouch transitions read from the same `entity+0x3D0` (the `C` suffix). The combat mode system that toggles crouch must update this field.
- The AnimMap.xml encodes the available transitions: `Relaxed→1H` = `HGM_1HS_Relaxedto1H_FINAL`, `1H→Relaxed` = `HGM_1HS_1HtoRelaxed_FINAL`. These are the draw/holster animations.

---

## `USGWAnimController` (UScript-native class)

Source: `SGWAnimController.cpp`. Class size: 0xD4 (212 bytes). Registered in Engine package. StaticClass: `FUN_00e95d10` (`0x00e95d10`).

UScript-native methods (exec function dispatch table at `0x01df10xx`):
- `execClearSecondAnim`
- `execClearAnim`
- `execPlayAnimNode`
- `execPlaySecondNamedAnim`
- `execPlayNamedAnim`

The exec functions are UScript bytecode thunks — they decode parameters via the bytecode dispatch table at `DAT_01edcbd0` and invoke the real implementation via `vtable[+0x184]` on the `USGWAnimController` object.

These are the UScript-accessible methods that Kismet sequences call to drive character animations. `PlayNamedAnim(name)` looks up a sequence by name in the anim tree; `PlayAnimNode` addresses a node directly.

---

## Open Questions

1. **`entity+0x3D0/1/2` write site**: RESOLVED — see section "Posture Block Write Site" below. The write is in `FUN_00ec0840` (CompositedAppearanceProxy::ApplyToPawn), driven by the `Event_NetIn_BeingAppearance` appearance pipeline.

2. **`FUN_00d29c90` full trace**: After storing `kismetEventSetId` at `GameEntity+0x98`, the code calls `FUN_00d29c90` → `FUN_00d28070` → ZipStorage. The exact ZipStorage query (which cooked data file, which key) is not yet traced. This is needed to fully confirm the issue #210 fix path.

3. **`USGWAnim_BlendByWeapon` / `USGWAnim_BlendByPosture` tick functions**: The actual tick/GetBoneAtoms bodies were not decompiled (only ctors analyzed). They likely read `entity+0x3D0` similarly to `USGWAnim_TransitionByStance`. Verify during W-weap or W-state investigation.

4. **`USGWAnimNotify_JumpEvent` Notify override**: `FUN_00e978xx` area — not yet decompiled. Expected to be structurally similar to `FUN_00e97070` but with a different dispatch path for jump/landing events.

5. **`USGWAnimNotify_Script` Notify override**: Expected to call into the CME Python bridge. Not yet decompiled.

6. **`FUN_00caf850` / `FUN_00cafd50`**: The final dispatch call from the reload and behavior-event-set handlers respectively. These route to `SGWNetworkManager` → Mercury entity method. The method index numbers are not yet extracted.

---

## Evidence Trail

| Claim | Evidence |
|-------|----------|
| `SGWAnimNotifyEvent_Emit` fields: SequenceName, CancelOnMovement, PlaybackType, HaltAnimTree | Decompile 0x00e974b0; literal string constants in body |
| Signal name is actor name (dynamic) | 0x00e974b0: `FUN_00423f40(this+0x44/+0x48)` then `LookupByName(result)` |
| Entity+0x3D0 = stance code | `FUN_00e968d0`: `*(param_1+0x128)+0x3D0` passed as `param_2` to `FUN_00e96720` |
| Table entry size = 0x14 bytes | `FUN_00e96720`: `pcVar3 = pcVar3 + 0x14` loop stride |
| ItemEventSet at itemArchetype+0x0C | `FUN_015d47c0`: `FUN_015d36f0(param_1, "ItemEventSet", -1, param_4 + 0xC)` |
| KismetEventSetId stored at GameEntity+0x98 | `FUN_00e6fd20`: `*(this+0x98) = local_2c` after reading `kismetEventSetId` |
| GameEntity is the subscriber for onKismetEventSetUpdate | RTTI: `.?AV?$MemberCallback@XVGameEntity@@P81@AEXPBVEvent_NetIn_onKismetEventSetUpdate...` |
| Event_NetOut_RequestReload is Pattern B | `FUN_00cbcda0`: `NetworkEvent_Ctor` + stamp `Event_NetOut_RequestReload::vftable` |
| reloadType field sent with reload request | `FUN_00c889a0`: `CmeEventData_GetField("reloadType")`, then `SetField("aReloadType", ...)` |
| AnimMap.xml is the anim-sequence lookup table | Confirmed: file content matches 3-char weapon + 3-char posture structure used in `FUN_00e96720` |
| `entity+0x3D2` write site is `0x00ec08e5` | Byte-pattern search `88 ?? D2 03 00 00` returns single hit at `0x00ec08e5`; decompile of containing function `FUN_00ec0840` confirms `*(char*)(param_5+0x3D2) = (char)iVar2` with `iVar2` from `proxy+0x34` |
| `FUN_00ec0840` is CompositedAppearanceProxy::ApplyToPawn | Debug string `"Applying CompositedAppearanceProxy to pawn"` at entry; calls `GameBeing_UpdateCombatStanceWeaponSet` at exit |
| Write is driven by Event_NetIn_BeingAppearance, not BSF flags | Causal chain traced: `FUN_00e01360` (BeingAppearance handler) → `FUN_00e00bc0` (setAppearance) → `FUN_00e69150` (scheduleJob) → `FUN_00ebe840` (set proxy+0x34) → `FUN_00ec0840` (write) |
| Default weapon category = 4 (melee) | `FUN_00ec0840` at body: `if (iVar2 == 0) { iVar2 = 4; }` before the write |
| USGWAnimController UScript class size 0xD4 | `FUN_00e95d10`: `(*GMalloc->Malloc)(0x178, 8)` then `FUN_004b4130(..., 0xD4, ...)` |

---

---

## Posture Block Write Site — `entity+0x3D2` (W-holster-finder, Session 5b)

**Confidence**: HIGH (write site confirmed by byte-pattern search + decompile)

### The writer: `FUN_00ec0840` (CompositedAppearanceProxy::ApplyToPawn)

**Address**: `0x00ec0840`
**Source**: `CompositedAppearanceProxy.cpp`
**Debug string**: `"Applying CompositedAppearanceProxy to pawn"`

The single write instruction in the binary for `entity+0x3D2` is at `0x00ec08e5`:

```c
*(char*)((int)entity + 0x3D2) = (char)weapon_category;
```

Where `weapon_category = *(int*)(proxy + 0x34)`, defaulting to 4 (melee) if the proxy value is 0.

After the write, `FUN_00ec0840` immediately calls `GameBeing_UpdateCombatStanceWeaponSet(entity, animCtrl, ...)` — the stance-update function that reads `animController+0x3D0`.

### Full causal chain (server → write site)

```
Server sends Event_NetIn_BeingAppearance {BodySet, ComponentList}
  -> FUN_00e01360 @ 0x00e01360  (GameBeing::HandleNetIn_BeingAppearance)
     reads BodySet, ComponentList; calls GameBeing::setAppearance
  -> FUN_00e00bc0 @ 0x00e00bc0  (GameBeing::setAppearance)
     schedules appearance job
  -> FUN_00e69150 @ 0x00e69150  (GameAppearanceManager::scheduleAppearanceJob)
     logs "SCHEDULING JOB"; calls FUN_00e998e0 to enqueue job
  -> [async TBB task] FUN_00ebdb50  (CompositingProcess_main.cpp)
     builds compositing task graph; loads body set cooked data
  -> FUN_00ebe840 @ 0x00ebe840
     sets: CompositedAppearanceProxy+0x34 = job[0x1e]  (weapon category from ComponentList)
     sets: CompositedAppearanceProxy+0x38 = job[0x30]  (stance flag from ComponentList)
  -> [job completes] FUN_00eb4be0  (IComposingProcessContinuation::Process)
     gets pawn via FUN_00e69070 (reads entity from listener entry at +0x8)
     calls FUN_00ec0840(proxy, entity, ...)
  -> FUN_00ec0840 @ 0x00ec0840  (CompositedAppearanceProxy::ApplyToPawn)
     WRITES *(char*)(entity+0x3D2) = (char)(proxy+0x34)  ← THE WRITE SITE
     also writes *(byte*)(entity+0x3BC) = *(byte*)(proxy+0x38)
     then calls GameBeing_UpdateCombatStanceWeaponSet(entity, animCtrl, ...)
```

### Weapon category values (partial)

| Value | Meaning |
|-------|---------|
| 0 | Unset (proxy constructor default; FUN_00ec0840 maps this to 4) |
| 4 | WEAP_Melee (confirmed as the fallback in FUN_00ec0840) |
| others | Decoded from BeingAppearance ComponentList cooked data — not fully enumerated |

### Implication for issues #249 / #333 / #339 (BSF_Holster — resolved)

**BSF_Holster (bit 8 of bStateField) does NOT write entity+0x3D2.** The posture byte is exclusively driven by the appearance pipeline. The holster animation blend in `USGWAnim_BlendByPosture` reads `entity+0x3D2` directly — the correct server behavior is to send a `BeingAppearance` with the weapon visual filtered out of `ComponentList` when the player holsters. BSF_Holster is not even a persistence/query bit — the 2009 client stores the full 32-bit `bStateField` at `+0x158` but `GameBeing_OnStateFieldUpdate` (`ghidra://SGW.exe@0x00e01c90`) only dispatches on bits 0-7. See [`docs/architecture/state-field-bits.md`](../../architecture/state-field-bits.md) for the verified bit→side-effect table.

Resolution:

- **#249** (spawn-holstered) — fixed in PR #338. World-entry path emits `BeingAppearance` with `weapon_visual` filtered from `ComponentList`.
- **#333** (BSF_HOLSTER retirement) — fixed in PR #338. Constant removed; dead clear-on-fire / clear-on-reload writes removed. Test-and-docs follow-up in PR #362 refreshed the wire-flow annotations in [`docs/gameplay/weapon-ammo-reload.md`](../../gameplay/weapon-ammo-reload.md) and added this resolution note.
- **#339** (runtime toggle rebroadcast) — fixed in PR #338. `combatant.rs requestHolsterWeapon`, `use_ability.rs` fire-while-holstered queue, and `player/world.rs` reload-while-holstered queue all call `request_appearance_refresh` → base-side `BeingAppearance` rebroadcast to self + AoI witnesses. PR #362 closed the missing test gap (the use_ability fire path's regression guard previously discarded `rx`, so dropping the rebroadcast call would have silently passed).

### Key addresses (appearance system)

| Address | Name | Role |
|---------|------|------|
| `0x00ec08e5` | write site | `MOV [entity+0x3D2], al` — the only byte write to this offset in the binary |
| `0x00ec0840` | `CompositedAppearanceProxy::ApplyToPawn` | Writes entity+0x3D2 from proxy+0x34; calls GameBeing_UpdateCombatStanceWeaponSet |
| `0x00ebe840` | (unnamed) | Sets proxy+0x34 = job[0x1e] (weapon category from BeingAppearance data) |
| `0x00eb4be0` | `IComposingProcessContinuation::Process` | Invoked on job completion; gets pawn; calls ApplyToPawn |
| `0x00e01360` | `GameBeing::HandleNetIn_BeingAppearance` | CME subscriber; reads BodySet/ComponentList |
| `0x00e00bc0` | `GameBeing::setAppearance` | Schedules appearance compositing job |
| `0x00e69150` | `GameAppearanceManager::scheduleAppearanceJob` | Defers/schedules async compositing task |
| `0x00ebdb50` | `CompositingProcess_main` | Async TBB compositing task body |
| `0x00e69070` | EntityListenerEntry→pawn accessor | Returns `*(param_1+8)` as pawn pointer |

---

## Cross-References

- [`../address-map.md`](../address-map.md) — updated below with animation system addresses
- [`combat-wire-formats.md`](combat-wire-formats.md) — `Event_NetOut_RequestReload` wire format
- [`cme-event-signal.md`](cme-event-signal.md) — CME pipeline that `SGWAnimNotifyEvent_Emit` uses
- [`../v5-campaign/CAMPAIGN_STATUS.md`](../v5-campaign/CAMPAIGN_STATUS.md) — W-anim session 4 report
- `game/sgw/Working/SGWGame/Config/AnimMap.xml` — canonical anim sequence name table
- GitHub issue #210 — reload anim sourced from wrong event set
