# Ability Resolution Pipeline

**Session**: W-abilities, V5 Documentation Campaign, 2026-05-13
**Binary**: SGW.exe (32-bit x86 PE, MSVC 8.0 / VC80, image base `0x00400000`)
**Scope**: Button press → AcquireTarget → useAbility emit → onEffectResults → animation play → cooldown lock

---

## Summary

The ability resolution pipeline in SGW.exe is a data-driven, CME-event-bussed system. Abilities are defined
in PAK files as `AbilityType` structs, exposed to Lua via runtime accessor functions, and activated through
a branch that splits on whether the ability needs a ground-target reticle. The server resolves all combat
math; the client receives results via `onEffectResults` and distributes them to UI through CME signals.
Cooldowns are managed entirely client-side by `CooldownManager`, which tracks server-sent timestamps and
fires `Event_UI_AbilityCooldown` transitions. Channeled abilities are cancelled through a dedicated
`ConfirmEffect` path distinct from the normal activation flow.

---

## Phase 1 — Call Trace: Button Press to Network Emit

```
[Lua UI action bar click]
    ↓ "useAbility(abilityId)" Lua call
    ↓ FUN_00aa2910  (Lua wrapper thunk)
           — string at 0x01940b70: "#ferror in function 'useAbility'"
    ↓ AbilitySet_InvokeAbility
           — RTTI-casts local entity to GameBeing
           — resolves AbilityType for abilityId
    ↓ AbilitySet_GetSlotByIndex (0x00d2a000)
           — looks up slot in ability array by zero-based index
    ↓ AbilitySet_EmitUseAbilityOrGroundTarget (0x00d2ae40)
           — reads targetType from pAbilityData+0x48
           — BRANCHES:
               if targetType == 3 (TargetGround):
                   AbilitySet_ActivateGroundTargetReticle (0x00dea330)
                       — asserts TCM == TCM_AERadius (2)
                       — shows AE reticle, subscribes to Event_Player_GroundTargetingEnd
                   [player clicks ground]
                   → Event_NetOut_UseAbilityOnGroundTarget (17 bytes wire format)
               else (targeted ability):
                   Pattern B: scalable_malloc(0xC) + vtable stamp
                   SetField "AbilityID" (int)
                   SetField "TargetID"  (int)
                   FUN_00cacd50 (CME emit)
                   → Event_NetOut_UseAbility (9 bytes wire format)
```

Wire formats confirmed in `findings/combat-wire-formats.md`:
- `useAbility`: 9 bytes — `[1B methodIdx][4B abilityId][4B targetId]`
- `useAbilityOnGroundTarget`: 17 bytes — `[1B methodIdx][4B abilityId][4B X][4B Y][4B Z]`

---

## Phase 2 — AbilityType Definitions

Abilities are defined as `AbilityType` structs deserialized from PAK files. Two struct layouts exist:

### PAK/Serialized Layout (AbilityType_DeserializePak — 0x015d51c0)

| Offset | Field                    | Type   |
|--------|--------------------------|--------|
| +0x34  | WarmupSeconds            | float  |
| +0x38  | CooldownSeconds          | float  |
| +0x44  | Target_Collection_Method | int (ETargetCollectionMethod) |
| +0x48  | TCM_Param1               | float  |
| +0x4C  | TCM_Param2               | float  |
| +0x54  | Flags (low word)         | uint   |
| +0x58  | Flags (high word)        | uint   |
| +0x60  | MinRange                 | float  |
| +0x64  | MaxRange                 | float  |

EffectIds array serialized at param_4+4 via `FUN_015d3e60`.

### Runtime/Lua Layout (AbilityType_GetLuaAbilityInfo — 0x00adb670)

| Offset | Field                    | Lua key                   |
|--------|--------------------------|---------------------------|
| +0x50  | warmUpSeconds            | "WarmupSeconds"           |
| +0x54  | coolDownSec              | "CooldownSeconds"         |
| +0x60  | icon                     | "Icon"                    |
| +0x94  | targetCollectionMethod   | "TargetCollectionMethod"  |
| +0x98  | flags                    | bit0=isWeaponAbility, bit1=isDeployAbility, bit16=isPetCommand |

**Note**: PAK and runtime layouts have different offsets for the same fields. This is typical of a 2009
engine that had separate data loading and runtime-cache paths. The PAK offsets are serialization order;
the runtime offsets are memory layout after initialization.

### ETargetCollectionMethod (TCM) Enum Values

| Value | Name         | Evidence |
|-------|--------------|----------|
| 2     | TCM_AERadius | Assert at 0x00d29d40: "getTargetCollectionMethod() == SGW::ETargetCollectionMethod::TCM_AERadius" |
| 3     | TargetGround | Assert at 0x00dea330: "abilityInfo->getTargetType()==SGW::ETargetType::TargetGround" |

### ETargetType Enum Values

| Value | Name         | Evidence |
|-------|--------------|----------|
| 3     | TargetGround | Used as branch condition at 0x00d2ae40 (param_1[0x12] == 3 → ground targeting flow) |

---

## Phase 3 — Effect Resolution: onEffectResults

### CombatQueue_HandleOnEffectResults (0x00eb1630)

Source confirmed: `Src\CombatQueue.cpp` lines 0x2b–0x54.

**Subscriber**: `Event_NetIn_onEffectResults`

**Wire format** (21 + 7×N bytes):
```
[4B SourceID][4B TargetID][4B AbilityID][4B EffectID][1B ResultCode]
  × N entries:
  [1B StatID][4B Delta (float)][1B DamageCode][1B StatResultCode]
```

**QR (Quality Rating) ResultCode values** (byte at position 20):

| Code | Name                    |
|------|-------------------------|
| 0    | ABILITY_INTERRUPT       |
| 1    | ABILITY_FAILED          |
| 2    | EFFECT_INIT             |
| 3    | EFFECT_REMOVED          |
| 4    | EFFECT_HIT_NORMAL       |
| 5    | EFFECT_HIT_CRIT         |
| 6    | EFFECT_HIT_DOUBLE_CRIT  |
| 7    | EFFECT_HIT_GLANCING     |
| 8    | EFFECT_HIT_MISS         |

**Visibility filter**: In non-debug mode, skips entries where neither SourceID nor TargetID matches
the local player or local target. This reduces UI processing for out-of-range combat.

**All damage calculations are server-side.** The client receives pre-computed deltas. See
`findings/combat-damage-analysis.md` for the HitType/DamageType/StatResultType enums.

---

## Phase 4 — Active Effects: EffectType

### EffectType Runtime Struct (from EffectSet serializer at 0x015ceeb0)

| Offset | Field                    | Type  | Notes |
|--------|--------------------------|-------|-------|
| +0x0c  | TotalTime                | float | Baked into def; NOT from timer event |
| +0x18  | isChanneled              | bool  | Key signal for cast-bar cancel UI |
| +0x28  | Target_Collection_Method | int   | TCM enum |
| +0x2c  | TCM_Param1               | float |
| +0x30  | TCM_Param2               | float |

### EffectType_GetLuaEffectInfo (0x00aec290)

Lua-callable. Returns a table with:
- `Name`, `Description`, `IconLoc` — from wstring accessors `FUN_00d2d010/030/050`
- `TargetCollectionMethod` — from `FUN_00d2d140` (low 16 bits → float)
- `Beneficial` (bool), `Hidden` (bool)
- `Channeled` (bool) — from `FUN_00d2d0e0(iVar3) & 0xFF` ← **isChanneled flag exposed to UI**
- `Flags` — from `FUN_00d2d150`
- `TotalTime` — read directly from EffectType+0x0c
- `TimeRemaining` — computed live via `FUN_00e085e0`

**Parameter**: 1-based slot index; function subtracts 1 before lookup.

---

## Phase 5 — Channeled Ability Cancellation

### SGWTextCommandMgr_HandleConfirmEffect (0x00c8c820)

This is the **only** client path for cancelling a channeled ability mid-cast.

```
Lua: cancelEffect(effectId)       → CME event with EffectId + Response=0
Lua: acceptEffect(effectId)       → CME event with EffectId + Response=1
     ↓
SGWTextCommandMgr_HandleConfirmEffect (0x00c8c820)
     — reads EffectId (int), Response (byte)
     — Pattern B: scalable_malloc(0xC) + vtable stamp
     — SetField "aEffectId" (int)   = EffectId
     — SetField "aAccepted" (bool)  = (Response == 1)
     — emit Event_NetOut_ConfirmEffect
     ↓
[server receives ConfirmEffect: aAccepted=false → interrupt channel]
[server receives ConfirmEffect: aAccepted=true  → confirm channel (rare, for confirmation prompts)]
```

---

## Phase 6 — Timer System: All 5 Handlers

All five handlers subscribe to the **same** `Event_NetIn_TimerUpdate` signal. They dispatch by the
`Type` byte field (read from the string "Type" at `0x019ba868`).

| Type (decimal) | Char | Handler | Function |
|----------------|------|---------|----------|
| 0 | `\0` | CooldownManager — warmup start | `CooldownManager_HandleOnTimerUpdate` (0x00ea6af0) |
| 1 | `\1` | CooldownManager — cooldown start AND GameEntityManager — entity arrival | `CooldownManager_HandleOnTimerUpdate`; `FUN_00c68110` (0x00c68110) |
| 2 | `\2` | CooldownManager — warmup end | same |
| 3 | `\3` | CooldownManager — cooldown end | same |
| 4 | `\4` | CooldownManager — pass-through (see note) | `CooldownManager_HandleOnTimerUpdate` (0x00ea6af0) |
| 5 | `\5` | EffectSet — active effect duration | `EffectSet_HandleOnTimerUpdate` (0x00e09160) |
| 6 | `\6` | DialogController — NPC interaction timer | `FUN_00d26380` (0x00d26380) |
| 7 | `\7` | CooldownManager — pass-through (see note) | `CooldownManager_HandleOnTimerUpdate` (0x00ea6af0) |
| 8 | `\8` | CooldownManager — pass-through (see note) | `CooldownManager_HandleOnTimerUpdate` (0x00ea6af0) |
| 9 | `\t` | MissionSet — mission timer start/reset | `MissionSet_HandleOnTimerUpdate` (0x00d18a30) |
| 10 | `\n` | MissionSet — mission progress timer | same |
| 11 | `\v` | MissionSet — mission completion timer | same |
| 12 | `\f` | GameBeing — weapon reload | `GameBeing_HandleOnTimerUpdate_Reload` (0x00e02380) |
| 13 | `\r` | GameBeing — deployment reload | same |
| 14 | `\x0E` | GameProxyPlayer — BigWorld time-complete | `SGWBeing_onBigWorldTimeComplete` (0x00dec9e0) |
| 16 | `\x10` | SGW::Crafting — crafting job timer | `FUN_00e47800` (0x00e47800) |

**Note on types 4, 7, 8**: `CooldownManager_HandleOnTimerUpdate` has NO type-based early-return — it processes
any type that passes the `SourceID == entityId` gate. These type values likely correspond to secondary cooldown
sub-states (e.g., shared cooldown group, weapon-slot cooldown, pet ability cooldown) that the server sends using
the same CooldownManager infrastructure. The exact semantic distinctions of 4, 7, 8 are inferred from context;
there are no explicit branch comparisons in the binary for them.

**Note on type 1 dual handling**: Both `CooldownManager` (cooldown start, filtered by SourceID = own entity)
and `GameEntityManager` (`FUN_00c68110`, explicit `Type == 1` branch, filtered by entity ID lookup) handle
type 1. The CooldownManager handles timers for the local player's own abilities; the GameEntityManager handler
tracks entity arrival/spawn timers for other entities in the world (AoI system).

**Subscriber count correction**: Eight subscribers to `Event_NetIn_TimerUpdate` (not five as previously
documented). The four previously undocumented subscribers are:
- `GameProxyPlayer` → type 14, handler `SGWBeing_onBigWorldTimeComplete` (0x00dec9e0)
- `DialogController` → type 6, handler `FUN_00d26380` (0x00d26380)
- `SGW::Crafting` → type 16, handler `FUN_00e47800` (0x00e47800)
- `GameEntityManager` → type 1 (entity arrival), handler `FUN_00c68110` (0x00c68110)

### CooldownManager_HandleOnTimerUpdate (0x00ea6af0)

**Gate**: Reads SourceID first. If `SourceID != this->entityId`, returns immediately.

**Fields read**: `SourceID` (int), `Type` (byte), `ID` (int), `TotalTime` (float), `BigWorldTimeComplete` (float).

**Calls**:
- `FUN_00ea6120` — resolves time range from sorted cooldown interval tree
- `FUN_00c6d1c0` — time utility
- `FUN_00ea62b0` — updates cooldown state; **fires `Event_UI_AbilityCooldown`** when state transitions

**UI Bridge**: Subscriber to `Event_UI_AbilityCooldown` lives at `0x01e0c458`. This is what drives
the cooldown overlay on the action bar.

### EffectSet_HandleOnTimerUpdate (0x00e09160)

**Timer type 5 only.** Reads `SecondaryId` (links to active effect instance), `BigWorldTimeComplete`,
`TotalTime`. Updates the stored `[startTime, endTime]` window for the matching active effect.
Used by the UI to display effect duration bars and ticking status indicators.

### GameBeing_HandleOnTimerUpdate_Reload (0x00e02380)

**Type 12 (`'\f'`)**: Emits `Event_UI_EntityReload` via `FUN_00e063b0`.
**Type 13 (`'\r'`)**: Emits `Event_UI_EntityReloadDeployment` via `FUN_00e064b0`.
Remaining time = `BigWorldTimeComplete - FUN_00c6e220()` (server clock subtraction).
Source confirms event format fields at `0x019d63f8` ("Type").

### MissionSet_HandleOnTimerUpdate (0x00d18a30)

**Type 9 (`'\t'`)**: Clears ID counter — mission timer start/reset.
**Type 10 (`'\n'`)**: Walks `this+0x58` (mission list), updates matching entry via `FUN_00c6d1c0 + FUN_00d16dd0`.
**Type 11 (`'\v'`)**: Walks `this+0x64` (completion list), same final calls.
Final: `FUN_00d16dd0(this, uVar2)` fires mission timer callback (likely triggers mission UI update).

### DialogController_HandleOnTimerUpdate (0x00d26380) — NEW (W-misc-gaps)

**Type 6 (`'\x06'`) only.** Registered in `FUN_00d26850` (DialogController constructor).
Constructor wrapper: `FUN_00d27460` (0x00d27460). MemberCallback ctor: `FUN_00d26ee0` (0x00d26ee0).

**Fields read**:
- Field name string from `0x019bb218` → "Type" byte
- Field name string from `0x019bb220` → `SecondaryId` (uint) — links to NPC interaction slot
- `"BigWorldTimeComplete"` (float) — absolute server time when interaction window expires

**Logic**:
1. If `BigWorldTimeComplete <= 0.0` (expired): emits `Event_UI_InteractionTimer` with remaining=0,
   clears `this+0x48` (active interaction ID).
2. If `BigWorldTimeComplete > 0` and SecondaryId > 0: stores SecondaryId to `this+0x48`,
   fetches interaction data from `CacheLibrary` (`FUN_00cfe680`).
3. If `BigWorldTimeComplete > 0` and SecondaryId <= 0: computes elapsed+remaining via
   `FUN_00c6bc20/FUN_00c6bca0` and emits `Event_UI_InteractionTimer`.

**Purpose**: Drives the countdown timer shown during NPC dialog interactions (e.g., "press F within
10 seconds to continue conversation"). SecondaryId identifies the interaction dialog set.

### SGWBeing_onBigWorldTimeComplete (0x00dec9e0) — NEW (W-misc-gaps)

**Type 14 (`'\x0E'`) only.** Registered on `GameProxyPlayer` (and `SGWMob`/`SGWBeing`) in
`SGWBeing_RegisterCallbacks` (0x00df3ab0) and `SGWMob_RegisterCallbacks` (0x00df3cc0).
Wrapper: `FUN_00dfb2a0` (GameProxyPlayer path) / `FUN_00dfaaf0` (SGWMob path).

**Fields read**:
- `"BigWorldTimeComplete"` (double) — absolute BW server time when interval ends
- `"SourceID"` (uint) — entity ID the timer belongs to

**Logic**:
1. Computes `delta = (float)(BigWorldTimeComplete - currentBWTime)`.
2. Clamps delta to 0.0f if negative.
3. `scalable_malloc(4)` → stores delta as a float.
4. Publishes via `FUN_00dfdcb0` (CME emit for a BW-time-remaining signal).

**Purpose**: Converts the server's absolute "BigWorld time complete" timestamp into a client-local
float countdown (seconds remaining). Used for zone timers, match timers, and event countdowns that
are driven by BigWorld server time rather than entity-specific cooldown logic.

### SGW::Crafting_HandleOnTimerUpdate (0x00e47800) — NEW (W-misc-gaps)

**Type 16 (`'\x10'`) only.** Registered in `FUN_00e49850` (SGW::Crafting constructor) via
`FUN_00e460e0` (0x00e460e0). MemberCallback ctor: `FUN_00e45c70` (0x00e45c70).

**Fields read**:
- `"SourceID"` (uint) — crafting job entity ID
- `"TotalTime"` (uint) — total crafting duration
- `"BigWorldTimeComplete"` (double) — absolute server time when crafting completes

**Logic**:
1. Reads all three fields.
2. Computes `remaining = (float)(BigWorldTimeComplete - currentBWTime)`.
3. Allocates 12-byte struct: `[SourceID (4B), remaining_float (4B), TotalTime (4B)]`.
4. Emits via `FUN_00e4a520` — a crafting progress event.

**Purpose**: Drives the crafting timer UI bar (job in progress). SourceID identifies which crafting
job is ticking; TotalTime enables the progress bar fraction calculation.

### GameEntityManager_HandleOnTimerUpdate (0x00c68110) — NEW (W-misc-gaps)

**Type 1 (`'\x01'`) only.** Registered in `FUN_00c69120` (GameEntityManager constructor) via
`FUN_00c6b2d0` (0x00c6b2d0). MemberCallback ctor: `FUN_00c6aaa0` (0x00c6aaa0) — note: this
constructor takes 3 data params (not 2) and stores them at `this+4`, `this+8`, `this+0xC`.
Allocated object is 0x10 bytes (vs 0x0C for other subscribers).

**Fields read**:
- Field name from `0x019aa7dc` → "Type" byte (gated on `== 1`)
- `"SourceID"` (uint stored as float) — BigWorld entity ID for the spawning entity
- Field from `0x019aa7f0` → "ID" (uint) — entity type or template ID
- `"BigWorldTimeComplete"` (double) — absolute time when entity becomes available

**Logic**:
1. Calls `FUN_00c6d1c0(this+0x9C, SourceID, ...)` — time utility for AoI scheduling.
2. Looks up entity template by ID in `CacheLibrary` (`FUN_00ae6b50`).
3. If template NOT found: calls `FUN_00d2b020(this+200, ID)` and
   `FUN_00c6df00/FUN_00c6bd20` — queues entity for deferred creation once data arrives.
4. If template FOUND: inserts SourceID into list at `this+0xBC`, calls `FUN_00c67920` — creates
   or activates the entity in the AoI immediately.

**Purpose**: Handles deferred entity arrival when the server sends a "this entity will exist at
BigWorldTimeComplete" timer. This is the AoI (Area of Interest) pre-announcement mechanism —
the server tells the client that entity SourceID of type ID will appear at a future BW timestamp,
allowing the client to pre-fetch data (`CacheLibrary`) before the entity materializes.

---

## Recommended Rust Fixes

### Issue — CooldownManager not filtering by SourceID (Cimmeria server-side)

The binary confirms that `CooldownManager_HandleOnTimerUpdate` gates on `SourceID == local entityId`
before processing. The server must therefore send `SourceID` correctly in every `onTimerUpdate` message
for the client to route it to the right entity's cooldown state.

**Recommended Rust fix** (module: `crates/services/src/` — wherever `onTimerUpdate` is serialized):
Ensure the `SourceID` field is always populated with the entity's BigWorld entity ID when sending
`onTimerUpdate` timer types 0–3. An empty or zero `SourceID` will cause the cooldown handler to silently
discard the update.

### Issue — ConfirmEffect must use "aEffectId" and "aAccepted" field names

The binary at `0x00c8c820` sets exactly two fields: `"aEffectId"` (int) and `"aAccepted"` (bool).
The server-side handler must read these exact field names (case-sensitive). Using `"effectId"` or
`"accepted"` will produce a read-of-zero silently.

---

## Open Questions

1. ~~**What is the 5th `Event_NetIn_TimerUpdate` subscriber?**~~ **CLOSED** (W-misc-gaps,
   2026-05-13): There are 8 subscribers total, not 5. All identified — see Timer Type Map above.
   Types 4, 7, 8 pass through CooldownManager without type-gating. Type 6 = DialogController,
   type 14 = BigWorldTimeComplete (GameProxyPlayer), type 16 = Crafting.

2. **Where does `AbilitySet_InvokeAbility` live?** Confirmed to exist (called from Lua thunk at
   `0x00aa2910`) but its exact address was not pinned this session. It performs the RTTI cast from
   `GameEntityBase` to `GameBeing` before resolving the ability.

3. ~~**What is the `GameProxyPlayer_HandleOnTimerUpdate`?**~~ **CLOSED** (W-misc-gaps, 2026-05-13):
   Confirmed. `GameProxyPlayer` subscribes to `Event_NetIn_TimerUpdate` using handler
   `SGWBeing_onBigWorldTimeComplete` (0x00dec9e0), which processes type 14 (BigWorld
   time-complete events — `BigWorldTimeComplete` double + `SourceID` uint field).

4. **Emit path for `useAbilityOnGroundTarget`**: The ground-target flow involves
   `Event_Player_GroundTargetingEnd`. The precise coordinates-capture and emit sequence between
   `AbilitySet_ActivateGroundTargetReticle` and the final emit was not traced to byte level.

5. **EffectType struct confirmation**: Offsets at `+0x18` (isChanneled), `+0x28` (TCM), `+0x2c/+0x30`
   (TCM params) were recovered from the PAK serializer `0x015ceeb0`. The runtime struct may have
   different offsets — cross-reference needed if Lua scripts access these directly.

---

## Cross-References

| Finding | Cross-reference |
|---------|-----------------|
| QR result codes | `findings/combat-damage-analysis.md` — HitType enum, Kismet event IDs |
| Wire format byte layouts | `findings/combat-wire-formats.md` — useAbility, onEffectResults |
| State-flag broadcast | `findings/state-flag-broadcast.md` — BSF_* flag table, FUN_00e01c90 dispatch |
| Weapon reload timers | `findings/weapon-ammo-pipeline.md` — reload emitters, RequestReload wire format |
| CME event signal pipeline | `findings/cme-event-signal.md` — Pattern B NetworkEvent, MemberCallback layout |
| Address map | `address-map.md` — "Ability resolution" subsection |
