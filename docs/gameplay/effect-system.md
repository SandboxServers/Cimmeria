---
title: "Effect System"
type: reference
audience: engineers
last_updated: 2026-10-03
---

# Effect System

> **Last updated**: 2026-10-03 (ability mechanics AB-04: single-pulse timed buffs and debuffs on the timed effect ledger; duration tracking was only true for multi-pulse effects, audit B-34)
> **Status**: Implemented — application, removal, pulsing, stacking, absorption shields, and channel cancellation all work. Gaps: diminishing returns, most of the effect-clear flags (only `EF_ClearOnDeath` is honoured, and only by the timed stat-buff ledger), and **no effect visuals at all** (no `onSequence` is emitted anywhere in the effect system).

## Overview

Effects are the atomic unit of gameplay change. Every ability resolves into one or more effects, each of which modifies entity stats, applies status conditions, or triggers scripts. Effects can be instantaneous, duration-based, or pulsing (repeating at intervals). Each effect instance tracks its own stat changes and can revert them on removal.

The `EffectInstance` class in `deprecated/python/cell/AbilityManager.py` handles all effect lifecycle logic.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Effect application | DONE | `addEffect()` on AbilityManager |
| Effect removal (by ID, by flag, by moniker) | DONE | Multiple removal paths |
| Pulsing effects (periodic) | DONE | Timer-based pulse loop |
| Duration tracking | PARTIAL | Multi-pulse effects: `active_effects` (`cell/effects/pulsing/`). Single-pulse timed effects (`pulse_count = 1`, `pulse_duration > 0`, the shape of every authored buff and debuff) never register there; since ability mechanics AB-04 the ones with stat NVPs live on the timed effect ledger (`CellEntity::stat_buffs`, one entry per effect and invoker, expired by the stat-buff tick, icon via `onTimerUpdate` type 5 with SecondaryId = effect id). The generated stat effects and the stimpacks use it. Since AB-08 it also holds entries with no expiry: a toggle's (a stance, on with one press, off with the next, a new stance replacing the old by `EFFECT_Stance`) and a stat passive's (`EF_AlwaysPersist`, held while the ability is known). CC, regen buffs and shields do not use it yet (AB-05, AB-09, AB-10). [ADR decision 28](../architecture/abilities-and-effects-decisions-23-33.md#28-native-consumables-the-base-consumes-before-the-cell-applies-and-timed-stat-buffs-live-in-their-own-ledger) |
| Stat modification (absolute) | DONE | `changeStat()` with STAT_Absolute |
| Stat modification (% current) | DONE | `STAT_CurrentPercentage` |
| Stat modification (% max) | DONE | `STAT_MaxPercentage` |
| Stat modification (% min-max) | DONE | `STAT_MinMaxPercentage` |
| Temporary vs permanent changes | PARTIAL | A timed effect ledger entry records exactly what it moved and takes back exactly that on expiry or removal (`StatBuff`, `TimedStat`). Otherwise a stat change is reverted only by its script's `on_remove` (AbsorbShield, Stun, RemoveCoverStance); direct HEALTH/FOCUS writes are one-way |
| QR combat damage | DONE | `qrCombatDamage()` using shared or per-effect QR |
| Effect scripts | DONE | Dynamic script loading via `cell.effects.<name>` |
| Kismet sequences (init, pulse, remove, per-QR hit) | NOT IMPL | Nothing under `crates/cell-world/src/cell/effects/` or `crates/cell-combat/src/cell/effects/` emits `onSequence`. Events 2000–2008 are never sent, so effects have no visual at all — see [cinematic-system.md](cinematic-system.md) |
| Client result reporting | DONE | `onEffectResults` with stat delta list |
| Clear on death/damage/rez/bandolier | PARTIAL | On death, `resolve_death` ends the timed stat buffs whose effect carries `EF_ClearOnDeath` (`EF_CLEAR_ON_DEATH`, `cell/effects/stat_buffs/`). Pulsing effects ignore the flag: pulses on a dead target are skipped (the instances stay and age out) and a dying channeller's channels are cancelled (`cell/abilities/death/mod.rs`). `EF_ClearOnDamage`, `EF_ClearOnRez` and `EF_RemoveOnBandolierSlotChange` have no Rust constant, and nothing clears effects on damage, revive, or bandolier swap |
| Effect stacking rules | DONE | Refcounted via `state_flag_counts`; shipped in PR #420 |
| Absorption shields | DONE | Absorption pool with defined drain ordering; shipped in PR #420 |
| Channeled effect pulses | DONE | `cell/effects/pulsing/`, including channel cancellation and the `AF_CHANNEL_ALLOWS_MOVEMENT` gate |
| Diminishing returns | NOT IMPL | `diminishingReturns` property exists |
| Confirmation dialog | NOT IMPL | `confirmationResponse` is a stub |

## Effect Instance Lifecycle

```
AbilityManager.addEffect(effect, invokerId)
  |-> Remove existing instance of same effect (no stacking)
  |-> Create EffectInstance(manager, effect, invokerId, instanceId)
  |-> Load script from cell.effects.<scriptName> if defined
  |-> updateEffectTimer() -- send onTimerUpdate to client/witnesses
  |-> instance.init()
       |-> doAction("onEffectInit", Effect_Init)
       |-> pulse()
            |-> If pulseCount != 0 and remainingPulses == 0: removeEffect(), return
            |-> remainingPulses--
            |-> Schedule next pulse timer (pulseDuration interval)
            |-> doAction("onPulseBegin", Effect_Pulse_Begin)
            |-> playPulseSequence(resultCode)
            |-> doAction("onPulseEnd", Effect_Pulse_End)

  ... (repeats for each pulse) ...

  instance.remove()
  |-> Cancel pulse timer
  |-> Revert all temporary stat changes
  |-> doAction("onEffectRemoved", Effect_Removed)
```

## Stat Change Types

| Constant | Value | Description |
|----------|-------|-------------|
| `STAT_Absolute` | 0 | Add/subtract fixed amount |
| `STAT_CurrentPercentage` | 1 | Change by % of current value |
| `STAT_MaxPercentage` | 2 | Change by % of max value |
| `STAT_MinMaxPercentage` | 3 | Change by % of (max - min) range |

## Effect Flags

| Flag | Constant | Implemented | Purpose |
|------|----------|-------------|---------|
| `EF_ClearOnDeath` | `EF_CLEAR_ON_DEATH` (4, `crates/entity/src/abilities/defs.rs`) | PARTIAL | Remove effect on entity death. Only the timed stat-buff ledger reads it; see the clear row above |
| `EF_ClearOnDamage` | -- | NO | Remove effect when damage received |
| `EF_ClearOnRez` | -- | NO | Remove effect on revive |
| `EF_RemoveOnBandolierSlotChange` | -- | NO | Remove on weapon swap |
| `EF_OnlySendToSelf` | -- | NO | Don't broadcast to witnesses |
| `EF_DontUseQR` | `EF_DONT_USE_QR` (16, `crates/entity/src/abilities/defs.rs`) | YES | Skip QR calculation: the effect never misses, and a hit whose every effect carries it takes no roll (`damage_apply/qr_gate.rs`, AB-06) |
| `EF_Beneficial_Effect` | -- | NO | AI: is this hostile? |
| `EF_Offline_Time_Counts` | -- | NO | Count cooldown while offline |
| `EF_HasInductionBar` | -- | NO | Show deploy/grenade bar |
| `EF_RemoveOnDisguiseZeroed` | -- | NO | Needs stealth system |
| `EF_AlwaysPersist` | -- | NO | Survives death/rezone |
| `EF_RemoveOnStealthZeroed` | -- | NO | Needs stealth system |
| `EF_CalculateQRFromTarget` | -- | NO | QR from target location |
| `EF_PromptConfirmationDialog` | -- | NO | Prompt before applying |

## Result Codes to Kismet Events

| Result Code | Kismet Event |
|-------------|-------------|
| `RC_None` | `Effect_Pulse_End` |
| `RC_Hit` | `Effect_Hit_Normal` |
| `RC_Miss` | `Effect_Hit_Miss` |
| `RC_Critical` | `Effect_Hit_Crit` |
| `RC_DoubleCritical` | `Effect_Hit_Double_Crit` |
| `RC_Glancing` | `Effect_Hit_Glancing` |

## Wire Format

### onEffectResults

```
SourceID:    INT32   -- Entity that launched the effect
AbilityID:   INT32   -- Ability that caused it
EffectID:    INT32   -- Effect definition ID
TargetID:    INT32   -- Entity that received the effect
ResultCode:  UINT8   -- RC_Hit, RC_Miss, RC_Critical, etc.
ResultList:  ClientEffectResultList
  Each entry:
    StatID:          INT32  -- Which stat changed
    Delta:           INT32  -- Amount of change
    DamageCode:      INT32  -- Damage type (EDamageType)
    StatResultCode:  INT32  -- SRC_None, SRC_Absorb, SRC_Mortal, SRC_Immune
```

### onTimerUpdate (for duration effects)

```
ID:                   INT32  -- Effect ID
Type:                 INT8   -- DurationEffect (timer type enum)
SourceID:             INT32  -- Entity with effect
SecondaryId:          INT32  -- Effect timer lookup key (the effect ID in Cimmeria)
TotalTime:            FLOAT  -- Total duration in seconds
BigWorldTimeComplete: FLOAT  -- Game time when effect expires
```

## Data References

- **Effect definitions**: 3,216 in `db/resources/Effects/Seed/effects.sql`
- **Schema**: `Effect.xsd`
- **Effect scripts**: the `cimmeria-cell-effect-scripts` crate (`crates/cell-effect-scripts/src/cell/effects/`), one row per script in its `EFFECT_SCRIPTS` table (`registry.rs`); the composition root registers the table with the cell at startup (decision 33 of the ADR below)
- **Stat result codes** (`EStatResultCode`): `SRC_None`, `SRC_Absorb`, `SRC_Mortal`, `SRC_Immune`
- **Cross-cutting ADR**: [abilities-and-effects-system.md](../architecture/abilities-and-effects-system.md)

## Remaining Work

1. **Effect visuals** — the largest gap. No `onSequence` is emitted for init, removal, pulse begin/end, or any of the per-QR `Effect_Hit_*` events, so the "Result Codes to Kismet Events" table above describes a mapping nothing currently walks
2. **Diminishing returns** — understand the dict format in `diminishingReturns` and the application algorithm
3. **Confirmation dialog** — `EF_PromptConfirmationDialog` flow and `confirmationResponse` handling
4. **Unimplemented effect flags** — see the NO rows in [Effect Flags](#effect-flags); several are blocked on a stealth system that does not exist

## Related Docs

- [combat-system.md](combat-system.md) - Damage pipeline using effects
- [ability-system.md](ability-system.md) - Abilities that dispatch effects
- [stat-system.md](stat-system.md) - Stats modified by effects
