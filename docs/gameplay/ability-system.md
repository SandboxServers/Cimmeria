---
title: "Ability System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Ability System

> **Last updated**: 2026-07-25
> **Status**: Implemented — direct-target, cone, AoE, ground-target, and channeled abilities all work. Remaining gaps: chain targeting, the combo/response system, and pre-launch ability conditions.

## Overview

Abilities are the primary interaction mechanism in combat. Each ability has a target type, warmup time, cooldown, range, and a list of effects that are applied to targets when the ability resolves. Abilities are organized by monikers (shared cooldown groups).

The `AbilityManager` class (in `deprecated/python/cell/AbilityManager.py`) manages the full lifecycle: validation, warmup, resolution, effect dispatch, and cooldown tracking.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Single-target ability launch | DONE | `TargetSelf`, `TargetTarget` |
| Ability warmup timer | DONE | Speed modifiers applied (grenade, deploy, attack) |
| Ability cooldown timer | DONE | Moniker-based shared cooldowns |
| Effect dispatch on resolve | DONE | Effects applied to all collected targets |
| Auto-cycle (auto-attack) | DONE | Re-fires ability on cooldown expiry |
| Ability interruption | DONE | Cancels warmup, resets cooldown |
| Ammo consumption | DONE | `requiredAmmo`, `consumeAmmo()` |
| Weapon range check | DONE | `UseWeaponRange` flag uses equipped weapon range |
| Position/facing check | DONE | Front/flank/rear mask validation |
| Weapon moniker requirement | DONE | `requiresWeapons()`, `itemMonikers` |
| AoE / cone targeting | DONE | `cell/abilities/cone_aoe/` — geometry, flag categories, and witness fan-out |
| Ground-target abilities | DONE | `useAbilityOnGroundTarget` in `cell/abilities/dispatch.rs`. Note it charges cooldown and ammo even when no enemy is in radius or the nearest target is beyond `max_range` |
| Channeled abilities | DONE | Channel pulsing and cancellation in `cell/effects/pulsing/`, with the `AF_CHANNEL_ALLOWS_MOVEMENT` movement gate |
| Kismet sequences (begin, end) | DONE | `Ability_Begin` (1000) and `Ability_End` (1001) emitted from `use_ability/handle.rs` |
| Kismet sequences (interrupt, failed) | NOT IMPL | `Ability_Interrupt` (1002) and `Ability_Failed` (1003) are never emitted, despite interruption itself working |
| Chain targeting | NOT IMPL | |
| Combo / response system | NOT IMPL | `Response` flag modifies cooldown only |
| Ability conditions | NOT IMPL | Pre-launch condition checks from ability data |

## Entity Definition

### SGWAbilityManager.def Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `warmupTimer` | PYTHON | CELL_PRIVATE | Active warmup timer data |
| `bIsWarmingUp` | INT8 | CELL_PRIVATE | Currently warming up |
| `lastWarmUpInterruptTime` | FLOAT | CELL_PRIVATE | Last warmup interrupt timestamp |
| `warmUpRuntimeParams` | PYTHON | CELL_PRIVATE | Params for warming up ability |
| `pulsedEffects` | PYTHON | CELL_PRIVATE | Active pulsed effects list |
| `durationEffects` | PYTHON | CELL_PRIVATE | Active duration effects list |
| `abilityAdjustments` | PYTHON | CELL_PRIVATE | Runtime ability modifications |
| `abilityCooldowns` | PYTHON | CELL_PRIVATE | Active ability cooldown timers |
| `categoryCooldowns` | PYTHON | CELL_PRIVATE | Moniker/category cooldown timers |
| `effectComponents` | ARRAY\<PYTHON\> | CELL_PRIVATE | Active effect component instances |
| `effectMonikers` | ARRAY\<PYTHON\> | CELL_PUBLIC | (entityId, monikerCRC) tuples |
| `effectSequenceId` | INT32 | CELL_PRIVATE | Next unique effect save key |
| `pendingAbilities` | PYTHON | CELL_PRIVATE | Abilities waiting to be resolved |
| `debugAbilityList` | ARRAY\<INT32\> | CELL_PRIVATE | Debug: abilities being traced |
| `debugEffectList` | ARRAY\<INT32\> | CELL_PRIVATE | Debug: effects being traced |

### Key Cell Methods

| Method | Args | Purpose |
|--------|------|---------|
| `invokeAbility` | abilityId, targetId, subsystemId, location, userData | External ability invocation |
| `resolveAbility` | abilityId, invokerParams, runtimeParams, useVelocity | Resolve ability on target |
| `resolveEffect` | effectId, abilityId, runtimeParams, attackerVars, pulseCount, duration, callerId, callerVars | Resolve effect on target |
| `onHealthZeroed` | INT32, 3x WSTRING | Death notification |
| `onKillCredit` | entitySpecId, dbId, xpAward | Kill credit |
| `clearEffectsByMoniker` | monikerCRC, count, sourceEntity | Remove effects by moniker |
| `removeEffectById` | effectId | Remove specific effect |
| `pulseChanneledEffectOnTarget` | abilityId, effectId, runtimeParams, attackerVars, pulseTime | Channel effect pulse |

## Ability Lifecycle

```
Client: useAbility(abilityId, targetId)
  |
  v
AbilityManager.useAbility()
  |-> canUseAbility() -- checks: alive, not busy, has ability, not cooling down, TCM check
  |-> AbilityInstance.setTarget(targetId)
  |-> AbilityInstance.canUse() -- checks: target exists, alive, in range, facing, weapon monikers
  |-> AbilityInstance.launch()
       |-> Calculate warmup (apply speedGrenade/Deploy/Attack modifiers)
       |-> Calculate cooldown (apply response modifier)
       |-> addCooldownTimer() (ability + moniker cooldowns)
       |-> Send onTimerUpdate for cooldown
       |-> If warmup > 0: start warmup timer, play Ability_Begin sequence
       |-> Else: afterWarmup() immediately
            |-> Consume ammo
            |-> Play Ability_End sequence
            |-> collectTargets()
            |-> For each effect: target.abilities.addEffect(effect, invokerId)
            |-> Fire 'ability.finished' event
            |-> abilityFinished()
```

## Targeting Modes

| Mode | Constant | Status | Description |
|------|----------|--------|-------------|
| Self | `TargetSelf` | DONE | Targets the caster |
| Single target | `TargetTarget` | DONE | Targets selected entity |
| Ground position | `TargetPosition` | NOT IMPL | AoE at position |
| Cone | unknown | NOT IMPL | Frontal cone AoE |
| Chain | unknown | NOT IMPL | Bounces between targets |

## Condition Feedback Codes

| Code | Constant | Meaning |
|------|----------|---------|
| InvalidEntity | `CONDITION_FEEDBACK_InvalidEntity` | Target doesn't exist |
| NotLiving | `CONDITION_FEEDBACK_NotLiving` | Target or self is dead |
| OutsideWeaponRange | `CONDITION_FEEDBACK_OutsideWeaponRange` | Too far / too close |
| WrongWeaponType | `CONDITION_FEEDBACK_WrongWeaponType` | Required weapon moniker not equipped |
| AmmoCountLessThan | `CONDITION_FEEDBACK_AmmoCountLessThan` | Insufficient ammo |
| WeaponCooldownNotReady | `CONDITION_FEEDBACK_WeaponCooldownNotReady` | Ability still on cooldown |
| EntityDoesNotHaveAbility | `CONDITION_FEEDBACK_EntityDoesNotHaveAbility` | Ability not in entity list |
| PositionCheck* | `CONDITION_FEEDBACK_PositionCheck{Above,Below,Front,Flank,Rear}` | Invalid facing direction |

## Reload Ability

`ABILITY_RELOAD_WEAPON = 596` is the well-known reload ability. Its `warmup` and `cooldown` come from the standard `ability_defs` row — no special-cased timing. The `requestReload(EReloadType)` cell method on `SGWPlayer` ([`SGWPlayer.def:794-797`](../../entities/defs/SGWPlayer.def#L794), opcode 86) starts the reload by:

1. Setting `reload_complete_at = now + warmup` on the cell entity.
2. Calling `start_ability_cooldown(596, warmup + cooldown)` so subsequent fires are gated by the standard cooldown timer.
3. Sending `onTimerUpdate` (method 12) so the client renders the cooldown bar.

The warmup deadline gates **magazine refill timing**: a 100 ms `reload_completion_tick` checks `reload_complete_at` and calls `refill_active_slot()` (sets `current_ammo = clip_size`) when the deadline elapses. The fire-path does not promote pending refills — it only reads the current ammo. See [weapon-ammo-reload.md](weapon-ammo-reload.md) for the full sequence.

## Data References

- **Ability definitions**: 1,886 in `db/resources/Abilities/Seed/abilities.sql`
- **Schema**: `Ability.xsd`
- **Enumerations**: `ETargetingMode`, `EAbilityFlag`, `ETargetCollectionMethod`, `EConditionFeedback`
- **Ability flags**: `UseWeaponRange`, `SpeedGrenade`, `SpeedDeploy`, `SpeedAttack`, `Response`, `Deactivate_AutoCycle`

## RE Priorities

1. **AoE targeting** - Decompile `TargetCollectionMethod` handlers in client binary
2. **Channeled abilities** - Understand `channeledAbilityData` format and pulse mechanics
3. **Ability conditions** - Pre-launch condition system from ability XML data
4. **Combo system** - How `Response` flag chains abilities together
5. **Ground targeting** - `useAbilityOnGroundTarget` message format and server handling

## Related Docs

- [combat-system.md](combat-system.md) - Damage pipeline, QR system
- [effect-system.md](effect-system.md) - Effect resolution, stat changes
- [stat-system.md](stat-system.md) - Stats used in ability checks
