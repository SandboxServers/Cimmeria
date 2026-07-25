---
title: "Effect System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Effect System

> **Last updated**: 2026-07-25
> **Status**: Implemented — application, removal, pulsing, stacking, absorption shields, and channel cancellation all work. Two gaps: diminishing returns, and **no effect visuals at all** (no `onSequence` is emitted anywhere in the effect system).

## Overview

Effects are the atomic unit of gameplay change. Every ability resolves into one or more effects, each of which modifies entity stats, applies status conditions, or triggers scripts. Effects can be instantaneous, duration-based, or pulsing (repeating at intervals). Each effect instance tracks its own stat changes and can revert them on removal.

The `EffectInstance` class in `deprecated/python/cell/AbilityManager.py` handles all effect lifecycle logic.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Effect application | DONE | `addEffect()` on AbilityManager |
| Effect removal (by ID, by flag, by moniker) | DONE | Multiple removal paths |
| Pulsing effects (periodic) | DONE | Timer-based pulse loop |
| Duration tracking | DONE | `completeTime`, `remainingPulses` |
| Stat modification (absolute) | DONE | `changeStat()` with STAT_Absolute |
| Stat modification (% current) | DONE | `STAT_CurrentPercentage` |
| Stat modification (% max) | DONE | `STAT_MaxPercentage` |
| Stat modification (% min-max) | DONE | `STAT_MinMaxPercentage` |
| Temporary vs permanent changes | DONE | Temporary reverted on removal |
| QR combat damage | DONE | `qrCombatDamage()` using shared or per-effect QR |
| Effect scripts | DONE | Dynamic script loading via `cell.effects.<name>` |
| Kismet sequences (init, pulse, remove, per-QR hit) | NOT IMPL | Nothing under `crates/services/src/cell/effects/` emits `onSequence`. Events 2000–2008 are never sent, so effects have no visual at all — see [cinematic-system.md](cinematic-system.md) |
| Client result reporting | DONE | `onEffectResults` with stat delta list |
| Clear on death/damage/rez/bandolier | DONE | `EF_ClearOnDeath`, `EF_ClearOnDamage`, etc. |
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
| `EF_ClearOnDeath` | -- | YES | Remove effect on entity death |
| `EF_ClearOnDamage` | -- | YES | Remove effect when damage received |
| `EF_ClearOnRez` | -- | YES | Remove effect on revive |
| `EF_RemoveOnBandolierSlotChange` | -- | YES | Remove on weapon swap |
| `EF_OnlySendToSelf` | -- | YES | Don't broadcast to witnesses |
| `EF_DontUseQR` | -- | YES | Skip QR calculation |
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
SecondaryId:          INT32  -- Effect instance ID
TotalTime:            FLOAT  -- Total duration in seconds
BigWorldTimeComplete: FLOAT  -- Game time when effect expires
```

## Data References

- **Effect definitions**: 3,216 in `db/resources/Effects/Seed/effects.sql`
- **Schema**: `Effect.xsd`
- **Effect scripts**: `crates/services/src/cell/effects/scripts.rs`, with a matching `match` arm in `registry.rs`
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
