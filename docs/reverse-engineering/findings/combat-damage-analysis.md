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
