# Combat Damage System — Client Binary Analysis

> **Last updated**: 2026-03-08
> **Source**: SGW.exe Ghidra decompilation
> **Confidence**: HIGH — exhaustive search, no formulas found

---

## Key Finding: All Damage Calculations Are Server-Side Only

**SGW.exe does NOT contain combat damage formulas, QR formulas, or armor/resistance math.** The client receives pre-computed damage deltas from the server and applies them directly. No verification, no recalculation.

If Cimmeria's QR formula needs validation, the formulas must come from server-side code/data, not from SGW.exe.

---

## Combat Enums (from Lua bindings in `FUN_00acbb10`)

### HitType Enum (`UIHitType`)

| Name | Value |
|------|-------|
| None | 0 |
| Normal | 1 |
| Miss | 2 |
| Glance | 3 |
| Critical | 4 |
| CriticalX2 | 5 |

### DamageType Enum (`UIDamageType`)

| Name | Value |
|------|-------|
| Untyped | 0 |
| Psionic | 1 |
| Physical | 2 |
| Energy | 3 |
| Hazmat | 4 |

### StatResultType Enum (`UIStatResultType`)

| Name | Value |
|------|-------|
| None | 0 |
| Immune | 1 |
| Absorb | 2 |
| Mortal | 3 |

### CombatState Enum (`UICombatantState`)

Blind, Alive, KnockDown, Fear, Stun, Snare, Disease, Confuse, Disorient, KnockBack, Suppression, Dead, Slow

### Complete Combat Stat List (`UIStatType`)

| Stat | Category |
|------|----------|
| Coordination, Engagement, Fortitude, Morale, Perception, Intelligence | Base attributes |
| MovementSpeedMod | Movement |
| Health, Focus, HealthRegen, FocusRegen | Resources |
| Accuracy, Defense | Hit determination |
| **QrMod** | QR modifier |
| PhysicalAF, EnergyAF, HazmatAF, PsionicAF | Armor Factor (per damage type) |
| KineticRes, MentalRes, HealthRes | Resistance |
| StealthRating, DisguiseRating, RevealRating | Stealth |
| RangeModifier, **CoverQRModifier** | Modifiers |
| AmmoSlot1–5, DeploymentAmmo | Ammunition |
| **Damage**, **Penetration** | Weapon stats |

---

## Effect Result Codes (data table `0x01e6ce00`)

| Code | Meaning |
|------|---------|
| ABILITY_INTERRUPT | Ability interrupted |
| ABILITY_FAILED | Ability failed |
| EFFECT_INIT | Effect applied |
| EFFECT_REMOVED | Effect removed |
| EFFECT_HIT_NORMAL | Normal hit |
| EFFECT_HIT_CRIT | Critical hit |
| EFFECT_HIT_DOUBLE_CRIT | Double critical |
| EFFECT_HIT_GLANCING | Glancing blow |
| EFFECT_HIT_MISS | Miss |
| EFFECT_PULSE_BEGIN | DOT/HOT tick start |
| EFFECT_PULSE_END | DOT/HOT tick end |
| ENTITY_SPAWN | Entity spawned |
| ENTITY_DEATH | Entity died |
| ENTITY_ALERT | Entity alert |
| ENTITY_MAKEDEAD | Force death |

### Kismet Sequence Event IDs (`0x00be32d0`)

| Hit Type | Event ID |
|----------|----------|
| HitNormal | 0x7D2 (2002) |
| HitCrit | 0x7D3 (2003) |
| HitDoubleCrit | 0x7D4 (2004) |
| HitGlancing | 0x7D5 (2005) |
| HitMiss | 0x7D6 (2006) |

---

## Client-Side Combat Data Pipeline

### 1. RPC Arrival — `onEffectResults`

Server sends via universal RPC dispatcher (`0x00c6fc40`). Three handlers:

| Class | RTTI | Role |
|-------|------|------|
| CombatQueue | `0x01e6d600` | Combat text display |
| GameEntityManager | `0x01df2238` | Entity stat updates |
| SequenceManager | `0x01e21e30` | Kismet visual triggers |

### 2. CombatQueue Handler (`0x00eb1630`, `Src\CombatQueue.cpp`)

Extracts: SourceID, TargetID, AbilityID, EffectID, ResultCode, ClientEffectResultList

Each result entry has 4 fields (16 bytes):
| Offset | Field | Type |
|--------|-------|------|
| +0x00 | StatID | int |
| +0x04 | StatResultCode | int |
| +0x08 | DamageType | int |
| +0x0C | Delta | float |

**Delta is the final computed damage value — no recalculation.**

### 3. Stat Update (`0x00e00e60`)

`onStatUpdate` arrives with: StatId (int), Current (float), Max (float), Min (float). Applied directly.

### 4. Stat Data Structure (`0x00aebf80` / `getUnitStat`)

Each stat per GameBeing has 7 fields (28 bytes):

| Offset | Field |
|--------|-------|
| +0x00 | current (float) |
| +0x04 | max (float) |
| +0x08 | min (float) |
| +0x0C | (unknown) |
| +0x10 | baseCurrent (float) |
| +0x14 | baseMax (float) |
| +0x18 | baseMin (float) |

---

## QR Formula Hints (from stat names)

Based on the stat names exposed, the server-side QR formula likely uses:
- **Accuracy** vs **Defense** → hit/miss determination
- **QrMod** + **CoverQRModifier** → modifiers to QR roll
- **Damage** + **Penetration** → weapon output stats
- **PhysicalAF / EnergyAF / HazmatAF / PsionicAF** → damage reduction per type
- **KineticRes / MentalRes / HealthRes** → resistance
- **RangeModifier** → range-based accuracy adjustment

QR outcomes: Normal, Critical, CriticalX2, Glancing, Miss.
Stat results: None, Immune, Absorb, Mortal.

---

## Implications for Cimmeria

1. **Cannot validate QR formulas from client binary** — they don't exist here.
2. **The complete stat enum is now known** — server stat IDs must match this order.
3. **4 damage types confirmed**: Physical, Energy, Hazmat, Psionic (each with its own Armor Factor).
4. **3 resistance types**: Kinetic, Mental, Health.
5. **CoverQRModifier** is a real stat — cover affects QR rolls, not just a damage reduction.
6. **Penetration** is a weapon stat — likely reduces target's Armor Factor.
7. **Delta values are floats** — the server should send float damage, not int.

---

## Session 5 Deep-Dive — Full Damage Pipeline Verification

> **Date**: 2026-05-13
> **Source**: SGW.exe Ghidra decompilation — `Src\CombatQueue.cpp`, `0x00be32d0` Kismet parser
> **Confidence**: HIGH — all fields confirmed from assert strings and RTTI

### CombatQueue Internal Data Structures

#### Combat Entry Struct (`FUN_00eb0ef0` at `0x00eb0ef0`) — 0x14 bytes

One entry is created per `onEffectResults` event that passes the filter:

| Offset | Field | Type | Notes |
|--------|-------|------|-------|
| +0x00 | SourceID | int32 | Casting entity ID |
| +0x04 | TargetID | int32 | Target entity ID |
| +0x08 | AbilityID | int32 | Ability definition ID |
| +0x0C | ResultCode | uint8 (stored as uint32) | QR result code byte |
| +0x10 | EffectResultList | shared_ptr | Ref-counted pointer to per-stat results |

#### Per-Stat Result Entry (`FUN_00eb0f70` / `FUN_00eb1230` at `0x00eb0f70`) — 0x14 bytes

One entry per `ClientEffectResult` in the result list:

| Offset | Field | Type | Source field name |
|--------|-------|------|-------------------|
| +0x00 | StatID | int32 (from int8 widened) | `"StatID"` — byte, widened to int for internal use |
| +0x04 | DamageCode | int32 (from int8 widened) | `"DamageCode"` |
| +0x08 | StatResultCode | int32 (from int8 widened) | `"StatResultCode"` |
| +0x0C | Delta | float | `"Delta"` — via `FUN_00438b50` float accessor |
| +0x10 | RefCountList | uint32 | Ref-count for inner shared-ptr list at +0x10 |

**Note on field order**: The assert strings at `CombatQueue.cpp:0x51–0x54` read fields in this order:
`StatID` → `Delta` → `DamageCode` → `StatResultCode`. The internal struct layout after widening
places them as: `StatID, DamageCode, StatResultCode, Delta`. The `FUN_00d361b0` call at `0x00d361b0`
packs them as `{StatID, DamageCode, StatResultCode, Delta}` (4 words × 4 bytes = 16 bytes per entry
into the vector, with a 5th word for the ref-count stub).

### Visibility Filter (confirmed from `CombatQueue_HandleOnEffectResults`)

Before processing stat results, the function calls:

```c
iVar4 = FUN_005757f0();           // spectator/debug mode check
cVar1 = FUN_00574430(iVar4);      // returns true if spectator mode active
if (!spectatorMode) {
    // check combat state flags at iVar5+0x61, iVar5+0xb1, iVar5+0xb2, iVar5+0xb3
    // These appear to be "is local player" + "is local player's target" flags
}
// Filter: skip event if neither SourceID nor TargetID matches local player or their current target
```

The filter reads entity pointers from `FUN_00c66ad0()` (local GamePlayer accessor) at offsets
`+0x8c+4` (player entity) and `+0x8c+8` (player's target entity). Unmatched events are silently
discarded.

**Gate: AbilityID > 0** — events with `AbilityID <= 0` skip the stat-result processing loop
entirely (confirmed by `if (0 < (int)local_a8)` at `CombatQueue.cpp:~0x45`).

### CombatQueue Ring Buffer / Linked List (`FUN_00eb0fc0` at `0x00eb0fc0`)

After building the combat entry, the function calls `FUN_00eb0fc0(iVar5, *(iVar5+4), &entry)` which
inserts the entry into a doubly-linked list anchored at `CombatQueue+0x4`. This is a ring buffer
with head/tail pointers — the UI reads from this buffer on the next frame.

`FUN_00eb14d0` (`0x00eb14d0`) then drains the ring buffer:

```c
while (buffer not empty):
    entry = dequeue()
    bVar3 = FUN_00eb11a0(entry)    // ability data lookup (FUN_00ae6b50 → ability cache)
    if (bVar3):
        FUN_00eb1410(...)            // link combat entry to list
        FUN_00eb1230(entry)          // emit combat text event (FUN_00eb0a70)
```

`FUN_00eb1230` allocates 0x14 bytes, copies the entry fields, then calls `FUN_00eb0a70` which emits
`Event_CombatText` (or equivalent) to the CME bus via `FUN_00e6beb0`.

### Complete QR Result Code Table (confirmed from `0x01e6ce00` pointer table)

The table at `0x01e6ce00` contains 20 pointers to UTF-16LE strings. The existing findings had
codes 0–4 mislabeled — the correct mapping (confirmed by reading all table entries):

| Code | Hex | Name (UTF-16LE string) | Kismet Event ID |
|------|-----|------------------------|-----------------|
| 0 | 0x00 | `ABILITY_INTERRUPT` | — |
| 1 | 0x01 | `ABILITY_FAILED` | — |
| 2 | 0x02 | `EFFECT_INIT` | 2000 (0x7D0) |
| 3 | 0x03 | `EFFECT_REMOVED` | 2001 (0x7D1) |
| 4 | 0x04 | `EFFECT_HIT_NORMAL` | 2002 (0x7D2) |
| 5 | 0x05 | `EFFECT_HIT_CRIT` | 2003 (0x7D3) |
| 6 | 0x06 | `EFFECT_HIT_DOUBLE_CRIT` | 2004 (0x7D4) |
| 7 | 0x07 | `EFFECT_HIT_GLANCING` | 2005 (0x7D5) |
| 8 | 0x08 | `EFFECT_HIT_MISS` | 2006 (0x7D6) |
| 9 | 0x09 | `EFFECT_PULSE_BEGIN` | 2007 (0x7D7) |
| 10 | 0x0A | `EFFECT_PULSE_END` | 2008 (0x7D8) |
| 11 | 0x0B | `ENTITY_SPAWN` | 5000 (0x1388) |
| 12 | 0x0C | `ENTITY_DEATH` | 5001 (0x1389) |
| 13 | 0x0D | `ENTITY_ALERT` | 5003 (0x138B) |
| 14 | 0x0E | `ENTITY_MAKEDEAD` | 5004 (0x138C) |
| 15 | 0x0F | `DESIGNER_1` (index 1) | 6000 (0x1770) |
| 16 | 0x10 | `DESIGNER_2` (index 2) | 6001 (0x1771) |
| ... | ... | `DESIGNER_N` (N ≤ 14) | 5999+N |
| 29 | 0x1D | `STARGATE_N` (N ≥ 1) | 0x17D3+N |

**Correction to prior W-abilities doc**: Codes 0–4 were listed as `ABILITY_INTERRUPT` through
`EFFECT_HIT_NORMAL`, but code 4 maps to `EFFECT_HIT_NORMAL` (hit result), not a lifecycle code.
Codes 0 and 1 (`ABILITY_INTERRUPT` / `ABILITY_FAILED`) do not carry stat deltas — the result list
will be empty for those codes.

Source: `FUN_00be32d0` at `0x00be32d0` — Kismet console command parser, confirmed by
`FUN_004195f0(list, L"HitNormal")` → `(*GEngine->vf_0x22c)(0x7D2)`.

### Complete Kismet Sequence Event ID Table (from `FUN_00be32d0`)

| Kismet name | Event ID | Decimal | Notes |
|-------------|----------|---------|-------|
| `Begin` | 0x3E8 | 1000 | Ability cast start |
| `Interrupt` | 0x3EA | 1002 | Cast interrupted |
| `End` | 0x3E9 | 1001 | Cast completed |
| `Failed` | 0x3EB | 1003 | Cast failed |
| `Init` | 0x7D0 | 2000 | Effect applied |
| `Removed` | 0x7D1 | 2001 | Effect removed |
| `HitNormal` | 0x7D2 | 2002 | Normal hit |
| `HitCrit` | 0x7D3 | 2003 | Critical hit |
| `HitDoubleCrit` | 0x7D4 | 2004 | Double crit |
| `HitGlancing` | 0x7D5 | 2005 | Glancing blow |
| `HitMiss` | 0x7D6 | 2006 | Miss |
| `PulseBegin` | 0x7D7 | 2007 | DOT/HOT tick start |
| `PulseEnd` | 0x7D8 | 2008 | DOT/HOT tick end |
| `Spawn` | 0x1388 | 5000 | Entity spawned |
| `Death` | 0x1389 | 5001 | Entity died |
| `Alert` | 0x138B | 5003 | Entity alert |
| `MakeDead` | 0x138C | 5004 | Force dead |
| `Designer N` (N=1..14) | 0x1770+N-1 | 5999+N | Designer debug channels |
| `Stargate N` (N=1..14) | 0x17D3+N | 6100+N | Stargate-specific sequences |

**Prior docs listed only HitNormal–HitMiss (0x7D2–0x7D6).** The cast lifecycle events (Begin/End/
Interrupt/Failed at 1000–1003) and entity lifecycle events (Spawn/Death/Alert/MakeDead at 5000–5004)
were missing. These are now fully confirmed.

### Damage Application Ordering (client-side only)

The client applies the server's pre-computed delta directly with no recalculation:

```
Server sends onEffectResults → CombatQueue extracts fields → emit to combat text display
                             → GameEntityManager reads Delta → applies to entity stat[StatID].current
                             → SequenceManager fires Kismet event by ResultCode
```

**Armor/absorption/mitigation math**: Completely absent from client. The server computes:
`rawDamage → armorReduction (PhysicalAF etc.) → absorption (absorbUntypedEnergy stat) → final Delta`
and sends only the final value. The `StatResultCode` field tells the client whether the result was
`Immune`/`Absorb`/`Mortal`/`None` for display purposes only.

### Key New Addresses

| Address | Function | Notes |
|---------|----------|-------|
| `0x00eb1630` | `CombatQueue_HandleOnEffectResults` | Full handler; `CombatQueue.cpp:0x2b–0x54` |
| `0x00eb0ef0` | Combat entry struct constructor | 5 fields: SourceID, TargetID, AbilityID, ResultCode, EffectList |
| `0x00eb0f70` | Per-stat entry initializer | 5 fields: StatID, DamageCode, StatResultCode, Delta, RefCount |
| `0x00eb14d0` | CombatQueue drain loop | Drains ring buffer; calls ability lookup + emit |
| `0x00eb11a0` | Ability-data gate check | `FUN_00ae6b50` ability cache lookup |
| `0x00eb1230` | Combat text event emitter | Allocs 0x14, copies entry, emits via `FUN_00eb0a70` |
| `0x00eb0a70` | CME combat text emit | Dispatches to `FUN_00e6beb0` (list insert + signal) |
| `0x00be32d0` | Kismet console command parser | Full `TestSequence` keyword→EventID mapping |
| `0x01e6ce00` | QR result code string table | 20-entry pointer table; UTF-16LE strings |
| `0x019e913c` | String: `ABILITY_INTERRUPT` | Index 0 in result code table |
| `0x019e91b8` | String: `EFFECT_HIT_NORMAL` | Index 4 in result code table |
| `0x005757f0` | Spectator/debug mode check | Called as visibility gate in CombatQueue |
| `0x00574430` | Spectator mode flag accessor | Returns bool for filter branch |

### Open Questions

1. **`FUN_00ae6b50` ability cache lookup**: Called from `FUN_00eb11a0` to gate whether a combat text
   entry is emitted. If the ability data is not yet in cache (async load), the entry is silently
   dropped. This could cause missed combat text on first use of a new ability.
2. **`ENTITY_ALERT` (code 13) vs `ENTITY_ALERT` (code 11 in effect table)**: The Kismet parser
   maps `Alert` → 5003 (0x138B), but the original effect result type table (at `0x01e6ce00`) has
   `ENTITY_ALERT` at index 13. These are consistent — verify that the server sends code 13 (not
   code 11) for alert events.
3. **`DESIGNER_N` and `STARGATE_N`**: `Stargate N` event IDs (0x17D3+N) suggest stargate-specific
   Kismet sequences. The N parameter comes from `_wtoi()` on the command token. Range check:
   `N-1 < 0xD` (i.e., N=1..14) for Designer; no upper bound shown for Stargate.
