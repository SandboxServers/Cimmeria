# Effect Execution Model — Client Binary Analysis

> **Last updated**: 2026-03-08
> **Source**: SGW.exe Ghidra decompilation
> **Confidence**: HIGH — decompiled handlers, data structures, RTTI classes, Lua API

---

## Verdict: Effects Are Fully DATA-DRIVEN

**The client does NOT need per-effect scripts.** There is no effect dispatch table, no switch on effect type, and no per-effect script callbacks (`onApply`, `onRemove`, `onTick`, `onExpire` — none found). The server computes stat deltas and sends them; the client just applies them generically.

The 4 server-side Python scripts (out of 3,217 effects) exist only because those effects need **custom server-side logic** beyond standard stat modification (e.g., GateTravel effect carrying destination UserData). The other 3,213 effects work entirely through data parameters.

---

## Effect Data Structure

`CookedData::EffectType` / `DBEffect` — flat data structure deserialized from `CookedDataEffects.pak` via SOAP/XML. ONE effect type class, not a hierarchy of subclasses.

**RTTI**: `CookedData__EffectType` at `0x01e24e60`, `DBEffect` at `0x01e24e90`

### EffectType Fields (serializer `0x015ceeb0`, deserializer `0x015dfd60`)

| Offset | Field | Type | Description |
|--------|-------|------|-------------|
| +0x04 | AbilityId | int | Parent ability ID |
| +0x08 | EffectId | int | Unique effect ID |
| +0x0C | EffectName | string | Display name |
| +0x10 | EffectDesc | string | Description text |
| +0x14 | targetCollectionId | uint | Target collection reference |
| +0x18 | isChanneled | bool | Whether effect is channeled |
| +0x1A | EffectSequence | ushort | Kismet sequence index |
| +0x1C | PulseCount | ushort | Number of damage/heal pulses |
| +0x20 | PulseDuration | float | Time between pulses |
| +0x24 | Delay | float | Initial delay before effect starts |
| +0x28 | Target_Collection_Method | ushort | How targets are collected (AoE, single, etc.) |
| +0x2C | TCM_Param1 | string | Target collection parameter 1 |
| +0x30 | TCM_Param2 | string | Target collection parameter 2 |
| +0x34 | Use_Ability_Velocity | bool | Use parent ability's projectile velocity |
| +0x38 | Flags | int64 | Bitfield (bit 0 = Beneficial, bit 12 = Hidden) |
| +0x40 | IconLocation | string | UI icon path |

Runtime active-effect fields:
- `+0x0C`: TotalTime (float)
- `+0x08`: TimeRemaining (computed from timer)

---

## Effect Result Type Enum (table at `0x01e6ce00`)

16-entry pointer table:

| Index | Name | Meaning |
|-------|------|---------|
| 0 | `EFFECT_INIT` | Effect applied |
| 1 | `EFFECT_REMOVED` | Effect removed |
| 2 | `EFFECT_HIT_NORMAL` | Normal hit |
| 3 | `EFFECT_HIT_CRIT` | Critical hit |
| 4 | `EFFECT_HIT_DOUBLE_CRIT` | Double crit |
| 5 | `EFFECT_HIT_GLANCING` | Glancing blow |
| 6 | `EFFECT_HIT_MISS` | Miss |
| 7 | `EFFECT_PULSE_BEGIN` | DOT/HOT tick started |
| 8 | `EFFECT_PULSE_END` | DOT/HOT tick ended |
| 9 | `ENTITY_SPAWN` | Entity spawned |
| 10 | `ENTITY_DEATH` | Entity died |
| 11 | `ENTITY_ALERT` | Entity alert |
| 12 | `ENTITY_MAKEDEAD` | Entity forced dead |
| 13-15 | `DESIGNER_1` through `DESIGNER_5` | Debug channels |

---

## Client-Side Event Flow

1. Server sends `onEffectResults` with SourceID, TargetID, AbilityID, EffectID, ResultCode, and stat deltas
2. Three handlers process the event generically (no type-specific dispatch):

| Class | Source File | Handler Address | Role |
|-------|------------|-----------------|------|
| `CombatQueue` | `Src\CombatQueue.cpp` | `0x00eb1630` | Combat text display (damage numbers, hit/miss/crit) |
| `GameEntityManager` | `Src\GameEntityManager.cpp` | — | Apply stat deltas to target entity |
| `SequenceManager` | `Src\SequenceManager.cpp` | — | Fire Kismet visual sequences |

3. Kismet events triggered:
   - `USeqEvent_EffectInit` at `0x006b1270`
   - `USeqEvent_EffectRemoved` at `0x006b1340`
   - `USeqEvent_EffectPulse` at `0x006b1410`

### `onEffectResults` Event Format

| Field | Type | Description |
|-------|------|-------------|
| SourceID | int | Casting entity |
| TargetID | int | Target entity |
| AbilityID | int | Parent ability |
| EffectID | int | Effect definition ID |
| ResultCode | enum | Hit type / lifecycle stage (see enum above) |
| ClientEffectResultList | list | Per-stat changes |

Each `ClientEffectResultList` entry:

| Field | Type | Description |
|-------|------|-------------|
| StatID | byte | Which stat is modified |
| Delta | value | Amount to change |
| DamageCode | byte | Type of damage |
| StatResultCode | byte | Result qualifier |

---

## Special Case: EffectUserData

`Event_NetIn_EffectUserData` / `onEffectUserData` is a separate channel for effects carrying custom data. `GateTravel` subscribes to `Event_Effect_EffectWithUserDataApplied` and `Event_Effect_EffectWithUserDataRemoved` — used for Stargate travel effects that carry destination data. This is the ONE case where a specific system reacts to a specific effect, but via a generic "user data" event subscription, not per-effect scripts.

---

## Ability → Effect Relationship

`CookedData::AbilityType` (serializer `0x015d51c0`, deserializer `0x015e5840`):
- `EffectIds` array (int[]) at +0x04 — list of effect IDs the ability triggers
- Also has: `Target_Collection_Method`, `TCM_Param1`, `TCM_Param2`, `Velocity`, `Flags`

The server resolves EffectIds, runs effect logic, sends results. Client never needs to know which effects an ability has.

---

## Lua API for Effects (registered at `0x00acbb10`)

| Function | Purpose |
|----------|---------|
| `getEffectCount` | Number of active effects on a unit |
| `getEffectInfo(unit, index)` | Returns table: Name, Description, IconLoc, TargetCollectionMethod, Beneficial, Hidden, Channeled, Flags, TotalTime, TimeRemaining |
| `cancelEffect` | Sends `Event_NetOut_ConfirmEffect` to server |

UI events: `UnitEffectsUpdate`, `UnitEffectsTooltipsUpdate`

---

## Implications for Cimmeria

1. **No need for 3,213 individual effect scripts.** The generic effect engine reads PulseCount, PulseDuration, stat modifiers from .def/data files and computes results.
2. **Only effects with custom server logic need Python scripts** — currently 4, likely <20 total (GateTravel, possibly some unique combat mechanics).
3. **The server's job**: Compute stat deltas based on effect data parameters, send them via `onEffectResults`. The client handles all visual/UI display generically.
4. **Effect parameters that drive behavior**: PulseCount, PulseDuration, Delay, Target_Collection_Method, TCM_Param1/2, Flags (Beneficial/Hidden/Channeled), stat modifiers.

---

## Session 5 Deep-Dive — Effect Stacking and Diminishing Returns

> **Date**: 2026-05-13
> **Source**: SGW.exe Ghidra decompilation — `EffectSet.cpp`, `GameBeing.cpp`
> **Confidence**: MEDIUM-HIGH — client-side mechanics confirmed; server-side DR absent from binary

### Finding: All Stacking and DR Logic Is Server-Side

The client contains **no diminishing-return formulas, no CC-immunity timers, and no stack-count
limits**. The client receives effect application/removal events and updates a generic active-effects
list. Stack resolution is performed on the server before sending `onEffectResults`.

### Active Effect Instance — Runtime Struct (`FUN_00d2d740` at `0x00d2d740`)

When `EffectSet_HandleOnTimerUpdate` (`0x00e09160`) installs a new effect, it allocates a 0x20-byte
struct (via `FUN_00418e30(0x20)`):

| Offset | Field | Type | Notes |
|--------|-------|------|-------|
| +0x00 | SecondaryId | int32 | Links timer event to this effect instance |
| +0x04 | RefCount | int32 | Starts at 0; managed by `FUN_00e0a6f0` ref-count logic |
| +0x08 | SourceID | int32 | Entity that cast the effect |
| +0x0C | TotalTime | float | Total duration in seconds (from `"TotalTime"` field) |
| +0x10 | BigWorldTimeComplete | float | Abs server timestamp when effect expires |
| +0x14 | UserData | varies | Populated by `FUN_00a55720` (wstring/key-value store for EffectUserData) |

The struct is stored in the `EffectSet`'s active-effects list at `this+0x10`/`this+0x14`/`this+0x1c`
(vector-of-shared-ptrs). `FUN_015fbd50` performs the push-back.

### Effect Timer Update Logic (`EffectSet_HandleOnTimerUpdate` — `0x00e09160`)

Gate: reads `"Type"` byte from the CME event. Only processes if `Type == 5` (EffectSet timer type).

Fields consumed:

| Field | Type | Usage |
|-------|------|-------|
| `"SecondaryId"` | int32 | Links to active-effect instance in the list |
| `"BigWorldTimeComplete"` | float | New expiry timestamp |
| `"TotalTime"` | float | Total duration (only read if current time < BigWorldTimeComplete) |
| `"SourceID"` | int32 | Read but used only for effect data construction |
| `"ID"` | int32 | Effect definition ID |

**Stacking behavior** (confirmed by `FUN_00e0a620` / `FUN_00e0a3b0`):

```
FUN_00e0a6f0(this, &result, &SecondaryId)
  calls FUN_00e0a620:
    FUN_00d283e0(this+0x28, &iterator, &SecondaryId)  // search existing list by SecondaryId
    if (found):
        update ref-count; return existing entry
    else:
        FUN_00e0a3b0(this, &SecondaryId, &result)      // INSERT NEW effect entry
```

The insert path (`FUN_00e0a3b0`) builds a wstring key of the form `"_<SourceID>"` and calls
`FUN_00479210` (scan existing entries) + `FUN_00439600` (list insert). This means:

- **Same SecondaryId** (same timer event linkage) → updates existing entry (refresh / extend)
- **Different SecondaryId** → inserts a new entry (stacking)

**The client permits unlimited stacking** — there is no cap check in `FUN_00e0a3b0` or the insert
path. The server controls maximum stack count by not sending timer events beyond the cap.

### Effect Removal Path (`FUN_00e0a810` at `0x00e0a810`)

When `FUN_00e0a9e0` (`0x00e0a9e0`) removes an effect from the active list, it emits a CME event
with `"CategoryId"` = **9** (confirming `EFFECT_PULSE_END` or similar lifecycle code) and a secondary
key from the effect instance. The actual removal is via `FUN_00e0a810` which fires a CME signal.

### Pulsing / DOT-HOT Duration Math

From `EffectSet_HandleOnTimerUpdate` (`0x00e09160`):

```c
float currentTime = FUN_00c6e220();    // BigWorld server clock (0x00c6e220)
if (currentTime < BigWorldTimeComplete) {
    // Effect still active — update [startTime, endTime] window
    FUN_00c6d1c0(this+0x38, SecondaryId, SecondaryId>>31, &BigWorldTimeComplete)
} else {
    // TotalTime / SourceID / ID read for new effect entry construction
    FUN_00d2d740(newEntry, SecondaryId, SourceID, TotalTime, BigWorldTimeComplete)
    // Push into active-effects vector
    FUN_015fbd50(this, &newEntry)
    FUN_015fbd50(this+0x1c, &newEntry)
    FUN_00e0a9e0(...)    // emit removal/expiry event
}
```

The `this+0x38` field is an interval-tree or sorted list tracking `[startTime, endTime]` windows per
effect instance, used by the UI to render duration bars smoothly.

**PulseDuration math**: `PulseCount × PulseDuration` = total DOT/HOT window, sent from server as
`TotalTime`. Each `EFFECT_PULSE_BEGIN` / `EFFECT_PULSE_END` pair covers one `PulseDuration` slice.
The client does NOT compute individual pulse timestamps — the server sends each pulse as a separate
`onEffectResults` with `ResultCode = EFFECT_PULSE_BEGIN` (9) or `EFFECT_PULSE_END` (10).

### CC State Flags and Client-Side State

CC immunity and the "CC state" (stunned, rooted, etc.) reach the client via `onStateFieldUpdate`
(`GameBeing_OnStateFieldUpdate` at `0x00e01c90`), which reads `"bStateField"` (INT32) and applies
bit-dispatch:

| Bit | Mask | State Name | Client Side-Effect |
|-----|------|------------|-------------------|
| 0 | `0x01` | `BSF_Dead` | `GameBeing_OnDeadStateChanged` + interaction reset |
| 1 | `0x02` | `BSF_AutoCycling` | `GameBeing_EmitAutoCycleStateChanged` → UI auto-cycle indicator |
| 2 | `0x04` | `BSF_Crouching` | `GameBeing_UpdateMovementSpeed` |
| 3 | `0x08` | `BSF_InCombat` | `GameBeing_UpdateCombatStanceWeaponSet` |
| 4 | `0x10` | `BSF_PlayingMinigame` | `FUN_00e31aa0` (minigame UI lock) |
| 5 | `0x20` | `BSF_InStealth` | `GameBeing_EmitStealthStateChanged` |
| 6 | `0x40` | (movement lock) | `GameBeing_UpdateMovementSpeed` (with bit 2,7) |
| 7 | `0x80` | `BSF_Walking` | `GameBeing_UpdateMovementSpeed` |

**IMPORTANT — BSF_Holster (bit 8, mask 0x100) is NOT processed here.** The dispatch uses `TEST BL`
(low byte only). Bit 8 requires `TEST EBX,0x100`, which is absent. The holster state is handled by a
separate client mechanism (confirmed by open issue #249).

**No CC immunity window is tracked client-side.** The server manages DR/immunity and simply does not
apply effects that are immune. The client receives only the final resolved result — if a CC was
immune, `StatResultCode = 1 (Immune)` in the `ClientEffectResult` entry.

### Confirmed Absence: No DR in Binary

Exhaustive search returned no functions matching: `DiminishingReturn`, `Immunity`, `Stacking`,
`CCImmunity`, `CCTimer`, `ImmunityWindow`. All CC immune periods, DR categories, and DR ratios are
server-only concepts not present in SGW.exe. Evidence: the `StatResultCode` byte (field 3 of
`ClientEffectResult`) communicates the resolved result (None/Immune/Absorb/Mortal) to the client
post-hoc; the client never makes this determination.

### Key Addresses

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e09160` | `EffectSet_HandleOnTimerUpdate` | Timer type 5; manages active-effect window |
| `0x00d2d740` | Active-effect struct constructor | 5 fields: SecondaryId, RefCount, SourceID, TotalTime, BigWorldTimeComplete |
| `0x00e0a620` | Effect lookup/insert dispatcher | Finds by SecondaryId; inserts new if not found |
| `0x00e0a3b0` | Effect insert path | Builds `"_<SourceID>"` key; `FUN_00479210` scan + `FUN_00439600` insert |
| `0x00e0a9e0` | Effect removal / expiry emitter | Fires CME event with CategoryId=9 |
| `0x00e0a810` | Effect-removed CME emitter | Emits `"CategoryId"=9` + secondary key field |
| `0x00c6e220` | BigWorld server clock accessor | Returns current server time as float |
| `0x00c6d1c0` | Interval-tree updater | Updates `[startTime, endTime]` at `EffectSet+0x38` |
| `0x015fbd50` | Active-effect vector push_back | Appends new effect instance to vector |
| `0x00e01c90` | `GameBeing_OnStateFieldUpdate` | CC/state bit dispatcher; assert at `GameBeing.cpp:0x341` |
| `0x00e05db0` | `GameBeing_EmitStateFieldChanged` | Fires `Event_Entity_StateFieldChanged` with old/new/delta |

### Open Questions

1. **DR categories and ratios**: Entirely server-side. What DR category each CC effect belongs to
   (if any) must come from effect data files or server configuration.
2. **Stack cap enforcement**: Server controls this. Unknown whether per-effect or per-source.
3. **`this+0x38` structure**: The interval tree / sorted list structure used for effect duration
   windows was not fully reverse-engineered. May be a `std::map<SecondaryId, pair<float,float>>`.
4. **CategoryId 9 meaning**: `FUN_00e0a810` emits `"CategoryId"=9` when removing an effect. This
   matches `EFFECT_PULSE_END` (result code 10) in the enum — but that is a result code, not a
   CategoryId. The CategoryId namespace may be separate. Needs cross-reference with CME event schema.
