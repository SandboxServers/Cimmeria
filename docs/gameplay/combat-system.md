---
title: "Combat System"
type: reference
audience: engineers
last_updated: 2026-05-27
---

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
| Auto-cycle (auto-attack) | DONE | Loops ability on cooldown expiry; toggle persists across relog via `sgw_player.state_field` (#412 — see [state-field-bits.md](../architecture/state-field-bits.md)) |
| Effect application / removal | DONE | `EffectInstance` class |
| Death / revive | DONE | `PLAYER_STATE_Dead` flag, `onDead()` / `onRevived()` |
| Crouch / cover stance | PARTIAL | Cover is a 10-60% damage reduction rated by the node, when it faces the attacker (NA32, see [Cover as damage reduction](#cover-as-damage-reduction-na32)); crouch terms not read yet |
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

## Ammo Gating

Ranged abilities declare a `required_ammo` cost. On every `useAbility`, the cell fire-gate ([`crates/services/src/cell/abilities.rs:259-281`](../../crates/services/src/cell/abilities.rs#L259)) checks:

```text
if required_ammo > 0 && entity.is_player && active_ammo() < required_ammo:
    log "useAbility: not enough ammo"
    return  (fire aborts; no effect dispatch, no cooldown)
```

If the check passes, the server decrements via `set_slot_ammo(active_slot, ammo - required_ammo)`, which mirrors the new value to `Stat[AMMO_SLOT_1+slot]` and emits `onStatUpdate` (method 20) so the bandolier UI refreshes the meter and count. NPCs (`is_player == false`) skip the gate entirely — they currently fire without consuming ammo.

Full server-authoritative ammo model, reload flow, persistence cadence, and client UI subscription chain: [weapon-ammo-reload.md](weapon-ammo-reload.md).

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

### The qrRand distribution (NA32)

`qrRand` is drawn from `Beta(1.4 + 2qr, 1.4)` for `qr >= 0` and `Beta(1.4, 1.4 - 2qr)` for `qr < 0` ([`combat/damage/qr.rs`](../../crates/services/src/cell/combat/damage/qr.rs) `calculate_result`). The mean rises with QR: a stronger attacker crits more and misses less.

> [!NOTE]
> These are the python reference's branches **swapped**. `AbilityManager.py:181-184` wrote `betavariate(α, α + qr * mult)` for `qr >= 0`, which pulled the mean *down* as QR rose. At QR +1.5 about 45% of rolls fell in the Miss/Glancing bands, and the `(1 + qr)` damage term only just cancelled the lower `qrRand`, so expected damage was nearly flat in QR. Every QR stat (accuracy, defense, cover) was inert on damage and inverted on the result code. The client's own units say the opposite: `accuracy` "modifies outgoing ranged and melee QR by +0.01 per point" and `defense` "modifies incoming ranged and melee QR by -0.01 per point" (`alias.xml:204-205`, [combat-formulas-client-evidence.md](../reverse-engineering/findings/combat-formulas-client-evidence.md)). At QR 0 nothing changes: the distribution is the symmetric `Beta(1.4, 1.4)` either way.

### Cover as damage reduction (NA32)

A defender in cover against its attacker takes a percentage off each hit, rated by the cover node it stands at ([`combat/damage/cover_damage.rs`](../../crates/services/src/cell/combat/damage/cover_damage.rs), applied by [`abilities/damage_apply/cover_roll.rs`](../../crates/services/src/cell/abilities/damage_apply/cover_roll.rs)). The owner's rule (D-NA15a, 2026-09-25): "Cover rating should depend on material. Cement walls are better cover than lunch tables. Not all cover is created equally. It should probably range from 10-60% damage reduction. We don't want npcs in cover to be complete bullet sponges."

```text
in cover:  final_pct = clamp(base(quality, height) + stance - penetration, 10, 60)
flanked or not at a node:  final_pct = 0
damage = ... * (1 + qr) * (1 - final_pct / 100) - armorFactor - absorption
```

The reduction applies after the `(1 + qr)` term and before armour and shields (`calculate_damage_scaled`). Cover does not touch the QR roll, so the result code is the one the stats give.

**The rating (`COVER_RATING`, one const).** The material signal is what the level designers authored on each node, `CoverQuality` and `CoverHeight` (NA21's extraction). The seed carries no prop mesh name, so there is no mesh-based refinement.

| Quality | Base | | Height | Adjust |
|---|--:|---|---|--:|
| None | 10 | | Low | -5 |
| Good | 20 | | Mid | 0 |
| Better | 25 | | High | +5 |
| Best | 45 | | Los | +10 |

The world-12 and world-8 seeds (4,024 nodes):

| Height / quality | World 12 | World 8 | Base % |
|---|--:|--:|--:|
| Mid / Better | 130 | 2,238 | 25 |
| High / Best | 97 | 1,243 | 50 |
| Low / Good | 7 | 203 | 15 |
| Mid / Best | | 43 | 45 |
| Low / Best | | 16 | 40 |
| High / Better | 2 | 16 | 30 |
| Mid / None | | 10 | 10 |
| Low / Better | | 8 | 20 |
| Low / None | | 6 | 10 (floor) |
| High / None | | 2 | 15 |
| Mid / Good, High / Good | | 3 | 20 / 25 |

The 16 guard spawns authored at a node hold 13 `Mid`/`Better` slots and 3 `High`/`Best`. So a typical guard slot is 25%, 35% with Cover Stance; tall best-quality cover is 50%, 60% (the cap) with the stance; waist-high `Low`/`Good` is 15%.

**Modifiers inside the band.** The cover stats keep the client's point scale and convert at the one point-to-percent rate the shipped data states, 10 points = 1 percentage point (effect 2004 "+50 (5%) Mental Resist", effect 2005 "Subtlety -100 (10% increase to threat)"):

| Term | Scale | Evidence |
|---|---|---|
| `coverDefense` (defender) | +0.1 points per point: Cover Stance's +100 is +10 | `alias.xml:235`; Cover Stance is effect 4565's +100, not the ability text's +200 (D-NA15) |
| `coverQRModifier` (defender) | +10 points per point | `alias.xml:216`, 1 QR per point; 1 QR = 100 points (ability 1729 "-1 QR" is effect 4299 "-100 DEF") |
| `coverAccuracy` (attacker) | -0.1 points per point | `alias.xml:234`; "Cover Penetration" is `coverAccuracy` (ability 1450 -> effect 1741). Penetration only takes cover away: the floor stays at 10 |
| `coverQRModifier` (attacker behind cover) | +1 QR per point, a QR term | `alias.xml:216`, the attack half of "both attack and defend QR" |

**In cover** means the geometry agrees: the defender stands within 1.5 u of a cover node (its held slot for an NPC, the nearest node on its floor for a player) and the attacker is not more than 20 degrees past the node's side-on line, the same `is_flanked` test that makes an NPC give its slot up. A flanked defender gets 0%, whatever its stat says. Players get the same reduction from the nearest node's rating; they get no Cover Stance.

**Never a bullet sponge.** The cap is 60%, so a covered guard takes at least 40% of every hit, and the test `a_covered_guard_always_takes_at_least_forty_percent` pins it for every rating and the heaviest stat stack. The damage-effect scripts (`RangedPhysicalDamage` and friends) do not go through this path, so their part of a hit is unreduced.

**Tuning.** Change `COVER_RATING` (or the band and scale constants beside it) in `cover_damage.rs`; the unit tests there pin the lunch-table, wall and typical-slot bands.

**Telemetry.** `abilities.qr event=cover_resolved` (DEBUG), written only when one side stands at a cover node: `defender_cover` / `attacker_cover` (`in_cover` \| `flanked` \| `exposed`), `flanked`, `cover_quality`, `cover_height`, `base_pct`, `stance_pct`, `penetration_pct`, `final_pct`, `attacker_cover_qr`.

## Kill credit (quest objective progression)

The cell-side ability path has **two** entry-point shapes, and the
distinction is load-bearing for mission progression.

| Caller shape | Entry point | When to use |
|---|---|---|
| Player-driven, single target | `handle_use_ability_with_kill_credit` | Every player-initiated single-target attack |
| NPC AI, tests, AoE caller layer | `handle_use_ability` (bare) | NPC attacks; ability-mechanic tests; AoE primary cast (the AoE caller fires `fire_entity_death` per-death itself) |

Both entry points share `handle_use_ability`'s single-target validation,
which gates on target validity: the target must be alive and in range, and
— **for player attackers** — a hostile NPC (`entity.is_player &&
(target.is_player || target.faction != HOSTILE_FACTION)` → rejected with a
`warn!`). This mirrors the AoE/cone faction filters and is the
server-authority guard against forged `useAbility` packets that would
otherwise grief vendors, quest NPCs, party members, or other players
(#444 / CAT-C-03). The check is scoped to player attackers because NPC AI
fight calls the same entry point to attack a *player*, which is
legitimate. Single-target abilities resolve as damage unconditionally
today; supportive single-target abilities (heal/buff an ally) will need
the inverse gate once an offensive/supportive ability field exists.

`handle_use_ability_with_kill_credit` wraps `handle_use_ability` with
an alive→dead transition detector that fires the content-engine
`EntityDeath` event. KillCount-style mission chains (e.g., "kill 5
Hallway_Guards") subscribe to that event via
`Trigger::OnEntityDeath { entity_tag }` and progress on each tagged
kill. Skipping the wrapper makes the kill happen on the wire AND in
the cell entity state, but the chain never fires — the player sees
the NPC die, the corpse is lootable, XP is granted, but the
"3 of 5" counter never moves.

### Why two entry points

NPC AI also calls `handle_use_ability`. NPC kills shouldn't fire
`EntityDeath` — the killer has no `player_id`, no mission to credit.
Baking the credit hook into `handle_use_ability` itself would either
require an `Option<player_id>` branch inside the helper (verbose and
easy to miss at call sites) or a separate `engine: Option<&ChainEngine>`
parameter on every caller. Keeping the bare function callable from
NPC AI + tests, and wrapping it explicitly at player entry points,
keeps both invariants visible at the call site.

### Every player-driven attack path routes through the wrapper

Cell-method dispatch sites (`USE_ABILITY`, `INTERACT`, `SET_AUTO_CYCLE`'s
immediate fire) and tick-driven re-fire paths (`auto_cycle_tick`,
`pending_attack_tick`) all route through
`handle_use_ability_with_kill_credit`. The cell-method `USE_ABILITY_ON_GROUND`
(AoE) is the one player path that calls `handle_use_ability_on_ground`
+ fires `fire_entity_death` per-death at the caller layer, because
the AoE flow returns a `Vec<entity_id>` of every NPC that died during
the cast (primary + secondaries).

## Data References

- **Abilities**: 1,886 in `db/resources/Abilities/Seed/abilities.sql`
- **Effects**: 3,216 in `db/resources/Effects/Seed/effects.sql`
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
