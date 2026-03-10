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
