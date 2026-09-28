---
title: "Deployables"
type: reference
audience: engineers
last_updated: 2026-09-28
---

# Deployables

> **Last updated**: 2026-09-28
> **Status**: Phase 0 implemented server-side; in-client UAT pending. One deployable works: 1012 "Deployable: Microwave Emitter" (Scientist, Support branch, level 25). The mechanism is generic, so the rest of the family follows as seed rows plus, where needed, an effect script. Ledger: [docs/analysis/deployables/](../analysis/deployables/README.md).

## What a deployable is

A deployable is a stationary object a player places on the ground with a "Deployable:" ability. It stands for a fixed lifetime, applies an effect around itself on a fixed cadence, and then disappears. The player who placed it owns it: its damage, threat, kill XP and mission credit are the owner's.

Two things in the 2009 data look like deployables and are not:

- **The "Kit:" items** (Kit: Turret, Kit: Medical Station and the rest) are crafting components. Their cooked rows carry `IsElementaryComponent="true"`, and nothing links them to an ability. They are out of scope.
- **Grenades** share the client's `KIS-SA_Deployable_Source` Kismet template and the `DeploymentBar` ability flag (2), but they are thrown projectiles with no spawned object. The flag marks the client's deployment bar, not a spawn.

The scoping and the cooked-data evidence are summarised in the [ledger](../analysis/deployables/README.md#evidence).

## 1012 Deployable: Microwave Emitter

Every number comes from the cooked data (`data/cache/CookedDataAbilities.pak` `_1012`, `CookedDataEffects.pak` `_5065` and `_5066`), which the seed transcribes field for field.

| Property | Value | Source |
|---|---|---|
| Target | Ground point (`TargetTypeId=3`), `TCM_AERadius` | ability 1012 |
| Range | `max_range` 500 | ability 1012 |
| Warmup | 2 s, shortened by `speedDeploy` (flag `SpeedDeploy`, 4096) | ability 1012 |
| Cooldown | 30 s (charged at launch, covering the warmup) | ability 1012 |
| Lifetime | 30 pulses, 1 s apart: 30 s | effect 5065 "Pulser": "30 pulses x1 Second duration / Despawn Target on Finish" |
| Pulse effect | effect 5066 "Damage": "Medium Radius AE / Secondary -100F" | effect 5066 |
| Radius | 8 m (the "Medium" range tier; 5066 has no `Radius` NVP) | effect 5066 `tcm_param1` |
| Damage per pulse | 100 Focus (`FocusDamage` NVP 380), no Health term | "-100F", written like the "-100F -10H" rows 641 and 656 |
| Damage model | `RangedPhysicalDamage`: Focus goes first, and once it is gone the overflow bleeds into Health (about 33 Health a pulse) | seed choice, see [open questions](../analysis/deployables/README.md#open-owner-questions) |
| Look | `WP-Human.BS_DeployableLow` (skeletal mesh `DP-Base100`) wearing `WP-Human.Dp_Standard100` | template 400 |

Effect 5066 carries `EF_DontUseQR` (16) and `EF_SequenceOnPulse` (128). The server honours neither yet: the pulse rolls QR like any hit, and no per-pulse sequence plays (1012 and its effects have no event set).

## How it works

The design is decision 27 of [abilities-and-effects-system.md](../architecture/abilities-and-effects-system.md).

1. **The press.** The client sends `useAbilityOnGroundTarget(1012, x, y, z)`. The server checks the point before charging anything: finite coordinates, within 500 of the caster, in line of sight of the caster's eye where the world has a collision occluder, and on the navmesh where the world enforces navmesh containment. A point over the mesh is moved down onto the floor. The point is then held for the cast, and the cast launches with no target.
2. **The warmup.** The ordinary 2 s warmup, with the cooldown and warmup timers sent on the press. Moving, dying or changing space during the warmup cancels it and drops the held point.
3. **The fire.** The object is placed at the point, facing the caster's heading, and the caster's older object from the same ability is removed (one out at a time).
4. **The pulses.** Every second, the object applies 5066 to every entity within 8 m that its owner may hit with an area ability, as if the owner had hit it. That is every hostile NPC, plus the owner's engaged duel partner. It never hits another player, a pet or a friendly NPC.
5. **The end.** After the 30th pulse the object is removed in the same tick, and every player who could see it gets a `LeftAoI`.

The object also goes, within one 100 ms tick and before it can pulse again, when its owner:

| Owner event | `reason` on the despawn row |
|---|---|
| dies | `owner_dead` |
| logs out, disconnects, or is destroyed on any path (gate travel, ring, respawn elsewhere) | `owner_gone` |
| is in another space (zone change, space transfer) | `owner_left_space` |
| casts the same ability again | `replaced` |

A player given the owner's entity id after a logout does not inherit the object: the owner check is the account and character captured when it was placed.

### What the object is on the wire

The object is an `SGWBeing` (wire class 0x01) introduced by the ordinary AoI create path: `CREATE_ENTITY`, then a cascade with `BeingAppearance(WP-Human.BS_DeployableLow, [WP-Human.Dp_Standard100])` and the ability's display name (moniker 5463, "Deployable: Microwave Emitter"). No new message and no client patch.

It is a being, not a pet, on purpose:

- Area and cone attacks collect only `SGWMob` candidates, and a being never gets an AI fight pass (`generate_threat` refuses it). So nothing can target or attack it, and Phase 0 needs no NPC-versus-NPC combat (#1009).
- A pet would bind into the owner's pet bar.

It takes its owner's faction, so a player's single-target attack on it is refused by the #444 gate.

### Refusals

Every refused press gets an `onErrorCode` and a chat line, and charges nothing.

| Case | `onErrorCode` | Chat line |
|---|---|---|
| Point beyond the range | 42 `OutsideWeaponRange` | "That spot is out of range." |
| Point behind a wall | 39 `LOS` | "You cannot see that spot." |
| Point off the navmesh (containment worlds), or not a number | 0 `InvalidEntity` | "You cannot place that there." |
| Pressed during the cooldown | 99 `WeaponCooldownNotReady` | "That deployable is not ready yet." |
| Pressed during another warmup | 99 | "You are already using an ability." |
| A plain `useAbility` naming the deployable (no ground point) | 0 | "Choose a spot on the ground to place that." |
| Placement failed after the warmup | 0, after `Ability_Interrupt` | "Your deployable could not be placed." |

An untrained deployable gets the ordinary not-known answer (167).

## Adding a deployable

1. Add a template in 400-409 (`db/resources/Entities/Seed/entity_templates.sql`): class `being`, a body set and component the client ships, no loot, never in `spawnlist`.
2. Add a `resources.deployables` row (`db/resources/Entities/Seed/deployables.sql`): the ability, the template, the effect whose pulses time it, the effect each pulse applies, and `max_active`.
3. Make the pulse effect do something: damage NVPs, a script name, or both. A heal or threat pulse needs a script that acts on the right targets (see the [ledger](../analysis/deployables/README.md#remaining-phases)).
4. Extend the live-DB guards in `crates/cell-catalog/src/cell/spawner/tests/live_db_deployables.rs`.

## Telemetry

| Target | Level | Events |
|---|---|---|
| `deployables.lifecycle` | INFO | `spawned` (point, radius, `pulses_total`), `despawned` (`reason`, `path`, `pulses`, `hits`, `health_damage`, `focus_damage`, `kills`, `lifetime_secs`) |
| `deployables.lifecycle` | DEBUG | `deploy_launched`; `deploy_refused` at the launch (`reason`, the client's `ground`) |
| `deployables.lifecycle` | WARN | `spawn_failed`, `despawn_failed`, `registry_scrubbed`, `deploy_refused` at the fire |
| `deployables.pulse` | DEBUG | `pulse` (`pulse_index`, `targets`, `health_damage`, `focus_damage`, `kills`) |

Every row carries `entity_id`, `owner_id`, `account_id`, `player_id` and `ability_id`.

## Code

| Piece | Where |
|---|---|
| Registry, spawn, verdict, despawn | `crates/cell-world/src/cell/deployables/` |
| Launch, fire, pulse tick, feedback | `crates/cell-combat/src/cell/abilities/deployable/` |
| Seed binding loader | `crates/cell-catalog/src/cell/spawner/deployables.rs` |
| Tick wiring | `crates/cell/src/cell/service/message_loop.rs` (after the pet sweep) |

## Related

- [pet-system.md](pet-system.md): the summon pipeline this one is modelled on.
- [ability-system.md](ability-system.md), [effect-system.md](effect-system.md), [combat-system.md](combat-system.md).
