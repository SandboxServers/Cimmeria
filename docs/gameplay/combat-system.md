# Combat System

> **Last updated**: 2026-03-01
> **Status**: ~70% implemented

## Overview

The combat system in Stargate Worlds is ability-driven, with a Quality Rating (QR) system that determines hit/miss/crit outcomes using a beta distribution random model. Combat involves stat-based damage calculation, armor mitigation, absorption, and a threat/aggro system for NPC targeting.

The server handles all combat resolution; the client sends ability requests and receives effect results. Auto-cycling (auto-attack) is supported.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Ability activation (single target) | DONE | `AbilityManager.useAbility()` |
| QR hit/miss/crit calculation | DONE | Beta distribution model in `DamageCalc` |
| Damage pipeline (base -> resist -> QR -> AF -> absorb) | DONE | `calculateDamage()` |
| Warmup / cooldown timers | DONE | Timer-based with client sync |
| Auto-cycle (auto-attack) | DONE | Loops ability on cooldown expiry |
| Effect application / removal | DONE | `EffectInstance` class |
| Death / revive | DONE | `PLAYER_STATE_Dead` flag, `onDead()` / `onRevived()` |
| Crouch / cover stance | PARTIAL | State flag set, affects QR, cover sets tracked |
| Successive shots bonus | STUB | Properties exist, not calculated |
| Threat / aggro system | NOT IMPL | `threatenedMobs`, `invokeThreatFromAbility` defined |
| AoE / cone targeting | NOT IMPL | Only `TargetSelf` and `TargetTarget` work |
| Channeled abilities | NOT IMPL | `channeledAbilityData` property exists |
| Diminishing returns | NOT IMPL | `diminishingReturns` property exists |
| Stealth detection | NOT IMPL | Properties and methods defined |

## Entity Definitions

### SGWCombatant.def Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `Alignment` | UINT8 | CELL_PUBLIC | Entity alignment (faction side) |
| `faction` | UINT8 | CELL_PUBLIC | Faction ID |
| `Archetype` | UINT8 | CELL_PUBLIC | Class/archetype of combatant |
| `threatenedMobs` | ARRAY\<INT32\> | CELL_PUBLIC | Mobs that have this entity on their threat list |
| `lastCombatTime` | FLOAT | CELL_PRIVATE | Timestamp of last combat enter |
| `lastRegenTime` | FLOAT | CELL_PRIVATE | Timestamp of last regen pulse |
| `regenTimerID` | INT32 | CELL_PRIVATE | Timer for health/focus regen |
| `statsBaseMin` .. `statsMax` | StatList | CELL_PUBLIC | 6-tier stat dictionary (see [stat-system.md](stat-system.md)) |
| `successiveShots` | INT8 | CELL_PRIVATE | Consecutive shots on current target |
| `currentAmmoType` | INT32 | CELL_PRIVATE | Active ammo type |
| `reloadTimerId` | CONTROLLER_ID | CELL_PUBLIC | Weapon reload timer |
| `NearCoverSetIDs` | PYTHON | CELL_PRIVATE | Cover set entities nearby |
| `entitiesDetectedStealth` | PYTHON | CELL_PRIVATE | Entities that detected this combatant's stealth |

### SGWAbilityManager.def Properties (combat-related)

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `bDmgOff` | INT8 | CELL_PRIVATE | Debug: disable incoming damage |
| `bGodMode` | INT8 | CELL_PUBLIC | Debug: invulnerability |
| `bInfiniteAmmo` | INT8 | CELL_PUBLIC | Debug: infinite ammo |
| `bNoAggro` | INT8 | CELL_PUBLIC | Debug: no aggro generation |
| `channeledAbilityData` | PYTHON | CELL_PRIVATE | Active channeled ability state |
| `diminishingReturns` | PYTHON | CELL_PRIVATE | DR tracking per effect |
| `immuneToEffects` | INT8 | CELL_PRIVATE | Blocks all effect application |

## Network Events

### NetOut (Client -> Server)

| Event | Method | Args | Status |
|-------|--------|------|--------|
| Use ability | `useAbility` (on SGWPlayer) | abilityId, targetId | DONE |
| Set auto-cycle | via ability system | toggles `autoCycle` | DONE |
| Set crouched | `setCrouched` | INT8 enabled | DONE |
| Holster weapon | `requestHolsterWeapon` | INT8 holster | STUB |
| Toggle heal debug | `toggleHealDebug` | (none) | STUB |
| Toggle combat debug | `toggleCombatDebug` | (none) | STUB |
| Confirmation response | `confirmationResponse` | effectId, accepted | STUB |

### NetIn (Server -> Client)

| Event | Method | Args | Status |
|-------|--------|------|--------|
| Stat update | `onStatUpdate` | StatUpdateList | DONE |
| Stat base update | `onStatBaseUpdate` | StatUpdateList | DONE |
| Effect results | `onEffectResults` | sourceId, abilityId, effectId, targetId, resultCode, resultList | DONE |
| Timer update | `onTimerUpdate` | id, type, sourceId, secondaryId, totalTime, completeTime | DONE |
| Melee range | `onMeleeRangeUpdate` | INT32 range | DONE |
| Archetype update | `onArchetypeUpdate` | INT32 | DONE |
| Alignment update | `onAlignmentUpdate` | INT8 | DONE |
| Faction update | `onFactionUpdate` | INT8 | DONE |
| Level update | `onLevelUpdate` | INT32 | DONE |
| State field update | `onStateFieldUpdate` | INT32 | DONE |
| Target update | `onTargetUpdate` | INT32 | DONE |

### Cell Methods (inter-entity)

| Method | Args | Purpose |
|--------|------|---------|
| `onAttacked` | MAILBOX, healthChange, focusChange, damageType | Notify entity of incoming damage |
| `onAddedToThreatList` | INT32 mobId | Mob added us to threat |
| `onRemovedFromThreatList` | INT32 mobId | Mob removed us from threat |
| `invokeThreatFromAbility` | MAILBOX, abilityType, threatValue, tauntAdj, points | Generate threat from ability |
| `adjustStat` | statName, damageType, cur/min/max/baseCur/baseMin/baseMax, mitigation, runtimeDict | Modify a single stat |
| `adjustStats` | ARRAY\<PYTHON\>, sourceId, causeId, causeSourceId, causeType | Batch stat modification |
| `onHealthZeroed` | INT32, 3x WSTRING | Entity health reached zero |
| `onKillCredit` | entitySpecId, dbId, xpAward | Kill credit notification |

## Wire Format

### StatUpdateList

```
TODO: Verify packed format from client binary
Each entry: { StatId: INT32, Min: INT32, Current: INT32, Max: INT32 }
```

### ClientEffectResultList

```
Each entry: { StatID: INT32, Delta: INT32, DamageCode: INT32, StatResultCode: INT32 }
```

## Damage Pipeline

The damage calculation in `DamageCalc.calculateDamage()` follows this pipeline:

```
baseDamage
  * qrRand (randomized from beta distribution)
  * QR_DAMAGE_MULTIPLIER
  * (1 + damage% stat)
  * (1 - statResistance)
  * (1 + qr)
  - armorFactor * max(0, mitigation - penetration) / 100
  - absorption (physical/energy/hazmat/psionic/untyped)
  = final damage
```

### QR Result Codes (EResultCode)

| Code | Constant | Threshold |
|------|----------|-----------|
| Miss | `RC_Miss` | qrRand < QR_MISS |
| Glancing | `RC_Glancing` | qrRand < QR_GLANCING |
| Hit | `RC_Hit` | qrRand < QR_CRITICAL_HIT |
| Critical | `RC_Critical` | qrRand < QR_DOUBLE_CRITICAL_HIT |
| Double Critical | `RC_DoubleCritical` | qrRand >= QR_DOUBLE_CRITICAL_HIT |

## Data References

- **Abilities**: 1,887 defined in `db/resources.sql`
- **Effects**: 3,217 defined in `db/resources.sql`
- **Damage types** (`EDamageType`): Untyped, Energy, Hazmat, Physical, Psionic
- **Stat enumerations**: See [stat-system.md](stat-system.md)

## RE Priorities

1. **Threat system** - Decompile mob AI to understand threat table management, `invokeThreatFromAbility` handling
2. **Channeled abilities** - Find `channeledAbilityData` usage in client for warmup/channel patterns
3. **AoE targeting** - Decompile `TargetCollectionMethod` handlers beyond `TCM_Single`
4. **Diminishing returns** - Understand `diminishingReturns` dict format and application rules
5. **Cover system** - Decompile cover set interaction with QR modifiers

## Related Docs

- [ability-system.md](ability-system.md) - Ability activation, targeting, warmup/cooldown
- [effect-system.md](effect-system.md) - Effect application, pulsing, stat modification
- [stat-system.md](stat-system.md) - Stat types, 6-tier dictionary, regen
