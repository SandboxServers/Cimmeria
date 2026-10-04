---
title: "Ability System"
type: reference
audience: engineers
last_updated: 2026-09-26
---

# Ability System

> **Last updated**: 2026-09-26
> **Status**: Implemented — direct-target, cone, AoE, ground-target, and channeled abilities all work. Remaining gaps: chain targeting, the combo/response system, and pre-launch ability conditions.

## Overview

Abilities are the primary interaction mechanism in combat. Each ability has a target type, warmup time, cooldown, range, and a list of effects that are applied to targets when the ability resolves. Abilities are organized by monikers (shared cooldown groups).

The `AbilityManager` class (in `deprecated/python/cell/AbilityManager.py`) manages the full lifecycle: validation, warmup, resolution, effect dispatch, and cooldown tracking.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Single-target ability launch | DONE | `TargetSelf`, `TargetTarget`. A beneficial cast (a heal or buff) lands on the caster or an ally, never a hostile (AB-01). See [beneficial casts](#beneficial-casts-ab-01) |
| Ability warmup timer | DONE | AT-10 (2026-09-26). A warmup ability sends `Ability_Begin` and fires after the warmup, not at launch. The speed stats (grenade, deploy, attack) shorten it. See [Warmup in the Rust server](#warmup-in-the-rust-server) |
| Ability cooldown timer | DONE | Moniker-based shared cooldowns |
| Toggles and stances | PARTIAL | AB-08. A player's `AF_TOGGLED` Self ability holds its stat effects with no expiry: the first press puts them on (with an effect-bar icon), the next takes them off, each press charged its cooldown. A new stance takes the old one off (`EFFECT_Stance`); other buffs, such as Aim, stay. Bound: the stances whose effects the `stat` family can read (1642 Soldier, 1458 Ranged Specialist, 859 Concentration, 857 Leading the Target, 714 Mobility, 2064, 2067, 2072, 2863). Target toggles, shields, stealth and disguise toggles are not. See [ADR decision 28](../architecture/abilities-and-effects-decisions-23-33.md#28-native-consumables-the-base-consumes-before-the-cell-applies-and-timed-stat-buffs-live-in-their-own-ledger) |
| Passive abilities | PARTIAL | `EF_AlwaysPersist` effects hold while the ability is known (login, purchase, off at a respec): 2852 Heed Our Calling's pet speed, and since AB-08 the stat passives 1450 Cover Penetration, 1731 Warrior's Resilience and 1574 Create Density: Basic. Mini-game passives (809 Mental Fortitude) and passives without the flag (1457 Steadfast) are not applied. A passive is never cast: a `useAbility` of a `passive_yn` ability, or of one whose every effect is `EF_AlwaysPersist`, is refused at launch with `onErrorCode` 167 and a feedback line, no cooldown (`use_ability/no_mechanics.rs`, `reason = passive_ability`). A weapon swap that revokes an ability takes off its held entries |
| Effect dispatch on resolve | DONE | Effects applied to all collected targets |
| Auto-cycle (auto-attack) | DONE | Re-fires ability on cooldown expiry |
| Ability interruption | DONE | AT-10. Death, a bandolier slot change, moving 0.5 m, and fire-time target, range, line-of-sight and ammo checks. Refunds the cooldown |
| Ammo consumption | DONE | `requiredAmmo`, `consumeAmmo()` |
| Weapon range check | DONE | #1017. An ability flagged `UseWeaponRange` (4) uses the equipped weapon's reach, both bounds, ranged or melee by the ability's `is_ranged` (`resources.items.*_range`, metres). With no weapon, or none with a reach of that kind, it uses its own `max_range` (30 m for 0): NPCs and pets carry no weapon item. The starter pistol reaches 20 m and the 40 m rifles 40 m. See [the ADR, decision 30](../architecture/abilities-and-effects-decisions-23-33.md#30-a-players-cast-honours-min_range-and-a-useweaponrange-ability-reaches-as-far-as-the-weapon-1016-1017) |
| Minimum range check | DONE | #1016. A player's targeted cast closer than the ability's `min_range` is refused at launch and at warmup fire with `onErrorCode` 42 (`OutsideWeaponRange`), the same answer as out of range, and the auto-cycle loop skips the target silently. NPC casters are not held to it: the NPC fight tick backs away instead. See [the ADR, decision 30](../architecture/abilities-and-effects-decisions-23-33.md#30-a-players-cast-honours-min_range-and-a-useweaponrange-ability-reaches-as-far-as-the-weapon-1016-1017) |
| Range units | DONE | `resources.abilities` ranges are UE3 units (100 per metre); the loader converts them to metres (#919). See [the ADR, decision 27](../architecture/abilities-and-effects-decisions-23-33.md#27-ability-ranges-are-ue3-units-in-the-data-and-metres-on-abilitydef-919) |
| Position/facing check | NOT IMPL (Rust) | Python validated the front/flank/rear mask. Rust `AbilityDef` has no `positions` field and `handle_use_ability` checks no facing |
| Weapon moniker requirement | DONE | `requiresWeapons()`, `itemMonikers` |
| AoE / cone targeting | DONE | `cell/abilities/cone_aoe/` — geometry, flag categories, and witness fan-out |
| Ground-target abilities | DONE | `useAbilityOnGroundTarget` in `cell/abilities/dispatch/mod.rs`. Note it charges cooldown and ammo even when no enemy is in radius or the nearest target is beyond `max_range` |
| Channeled abilities | DONE | Channel pulsing and cancellation in `cell/effects/pulsing/`, with the `AF_CHANNEL_ALLOWS_MOVEMENT` movement gate |
| Kismet sequences (begin, end) | DONE | `Ability_Begin` (1000) and `Ability_End` (1001) emitted from `use_ability/handle.rs` |
| Kismet sequence (interrupt) | DONE | `Ability_Interrupt` (1002) is sent when a warmup is interrupted (AT-10) |
| Kismet sequence (failed) | NOT IMPL | `Ability_Failed` (1003) is never emitted. Python never sent it either |
| Chain targeting | NOT IMPL | |
| Combo / response system | NOT IMPL | `Response` flag modifies cooldown only |
| Ability conditions | NOT IMPL | Pre-launch condition checks from ability data |
| Press of an ability with no mechanic | DONE | AB-12 (D-AB10). A player's press is refused before the cooldown with `onErrorCode` 167 and the feedback line "That ability has no effect yet." See [presses with no mechanic](#presses-with-no-mechanic-ab-12) |

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
| `debugAbilityList` | ARRAY\<INT32\> | CELL_PRIVATE | Debug: abilities being traced ([In-game combat debug](#in-game-combat-debug)) |
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

### Warmup in the Rust server

The Rust server keeps the same split (AT-10). `handle_use_ability`
(`cell/abilities/use_ability/handle.rs`) is the launch. It validates the cast,
starts the cooldown for `cooldown + warmup`, and sends the cooldown timer. Then:

- **Warmup = 0:** the cast fires in the same pass (`use_ability/fire.rs`). It
  spends the ammo, sends `Ability_End`, and applies the damage. The wire is the
  same as before AT-10.
- **Warmup > 0:** it sends `Ability_Begin` and an `AbilityWarmup` (type 1)
  `onTimerUpdate` to the player, then parks the cast on the caster. The cell's
  100 ms warmup tick fires it through the same fire path when the warmup
  expires. The ammo is spent then, not at launch.

A player or NPC has one cast in its warmup at a time. A second `useAbility`
during the warmup is refused and sends nothing, as python refused a launch
while `currentAbility` was set. An NPC holds still while it casts.

A warmup is interrupted, and the cast never fires, when:

- the caster dies;
- the caster changes its active bandolier slot;
- the caster moves 0.5 m or more from where it started, unless the ability
  has `AF_CHANNEL_ALLOWS_MOVEMENT` (the channel rule), or ends up in another
  space;
- at the moment it would fire, the target is gone, dead, in another space or
  no longer hostile; the target is out of range (`onErrorCode` 42); a player
  has no line of sight (`onErrorCode` 39); a player's active weapon is not
  the one the cast started with; or a player's weapon is reloading or short
  of ammo.

An interrupt refunds the cooldown. It sends the player a zeroed warmup timer
and a zeroed cooldown timer, then sends `Ability_Interrupt` to the caster and
its witnesses. If the interrupted ability was the auto-cycle ability, the
loop stops. Python interrupted on death and on a slot change only, and
re-checked nothing when the warmup ended. The other triggers are
server-authoritative additions. The design record is decision 21 of
[abilities-and-effects-system.md](../architecture/abilities-and-effects-decisions-16-22.md#21-warmup-is-a-pending-cast-per-caster-fired-by-the-100-ms-tick-at-10).

## Targeting Modes

| Mode | Constant | Status | Description |
|------|----------|--------|-------------|
| Self | `TargetSelf` | DONE | Lands on the caster, whatever target the client sent (AB-01, D-AB01). The client sends its current target for every non-ground ability, Self ones included. Before AB-01 the server took that id at its word, so this row's "DONE" was wrong (audit B-17): a Self heal with nothing selected did nothing, with yourself or an ally selected was refused by #444, and with a mob selected healed the mob. See [beneficial casts](#beneficial-casts-ab-01) |
| Single target | `TargetTarget` | DONE | Targets selected entity |
| Ground position | `TargetPosition` | NOT IMPL | AoE at position |
| Cone | unknown | NOT IMPL | Frontal cone AoE |
| Chain | unknown | NOT IMPL | Bounces between targets |

### Beneficial casts (AB-01)

A player's ability is **beneficial** when at least one of its effects does something and every effect that does something either carries `EF_Beneficial_Effect` (1) or, on an `ABILITY_TYPE_Heal` ability, runs a heal script (`HealHealth`, `HealFocus` or `HealPetHealth`). An ability with an effect that deals damage (`HealthDamage` or `FocusDamage` above 0) is never beneficial, whatever its type: the seed has a Heal-typed attack, 2228. The Heal type alone is not enough: about 200 seed abilities are typed Heal but are debuffs or crowd control (1874 Impose Weakness, 1988 InduceDaze, 2154 ShutDown and others), and they stay on the #444 path. The rule is `cimmeria_entity::abilities::ability_is_beneficial`. 597 Heal Focus, 1646 Health Heal and 1218 Recuperation are beneficial through their heal scripts; their effects carry no beneficial bit.

A beneficial cast lands here:

| The ability | The client's target | Lands on | `resolution` |
|-------------|---------------------|----------|--------------|
| `TargetSelf` (597) | anything, or nothing | the caster | `self_ability` |
| `TargetTarget` (1646, 1218) | the caster, or another player the caster may not attack, alive, in the same space | that player | `ally` |
| `TargetTarget` | a hostile, a neutral NPC, a dead or missing entity, or nothing | the caster | `fallback_to_caster` |

The last row is the proposed default of D-AB02, which the owner has not yet confirmed. The alternative refuses the cast with a feedback line ("That ability needs a friendly target.") before the cooldown is charged; it is one constant, `FALLBACK_TO_CASTER` in [`use_ability/beneficial.rs`](../../crates/cell-combat/src/cell/abilities/use_ability/beneficial.rs). Friendly NPCs and pets stay out, as for support darts.

The cast then runs each effect's script on that entity, with no QR roll, no `onEffectResults`, no threat, no in-combat state and no channel cancel. The stat change goes to that entity and its witnesses, a `StatBuff` timer goes with it, and a pulsing effect (Recuperation's 25 pulses) is registered on it with the caster as invoker. The warmup's fire-time re-check and the fire run the same resolver on the target the client sent, so an ally who dies or turns hostile during the warmup takes the cast to the fallback, and `Ability_End` names the entity the cast lands on. NPC casts and every non-beneficial cast keep the #444 gate and the damage pipeline unchanged. The heal shows as the bars moving; there is no floating heal number yet (AB-11).

Each resolution logs one `abilities` row, `event=beneficial_cast`: DEBUG at `stage=launch` (it runs before the launch's dead, known and cooldown checks) and INFO at `stage=fire`, with `ability_id`, `effect_ids`, `wire_target_id`, `resolved_target_id`, `target_player_id` and `resolution`; `event=beneficial_cast_applied` (DEBUG) carries the target's Health and Focus before and after. The design record is [decision 34](../architecture/abilities-and-effects-decisions-23-33.md#34-a-beneficial-cast-lands-on-the-caster-or-an-ally-never-on-a-hostile-ability-mechanics-ab-01).

### Presses with no mechanic (AB-12)

Most seeded abilities have nothing the server can resolve yet (ability-mechanics audit B-02). Before AB-12 a press of one charged the cooldown, sent the timer, maybe played an animation, and did nothing else. Now a player's press of an ability with no mechanic is refused at launch, right after the dead, warming-up, known-ability and cooldown checks, so a dead, friendly or out-of-range target never swallows the answer (`use_ability/no_mechanics.rs`):

- `onErrorCode` with `SystemID 0` (`ERRORCODE_SYSTEM_Ability`), `InstanceID` = the ability id and `ErrorCodeID 167` (`EntityDoesNotHaveAbility`, the code the pet-order gate already sends for an unimplemented pet ability; the client enum has no "no effect" value);
- then `onPlayerCommunication("SYSTEM", 0, CHAN_FEEDBACK, "That ability has no effect yet.")`, the line the player reads, since no client Lua renders `onErrorCode`;
- no cooldown, no `onTimerUpdate`, no animation, and the ability is not stashed for auto-cycle.

An ability **has a mechanic** (`ability_has_mechanics`) when any of these holds:

| Mechanic | Source |
|----------|--------|
| An effect deals damage (`HealthDamage` or `FocusDamage` above 0) or runs a registered script (a blank or unknown `script_name` does not count) | `cimmeria_entity::abilities::ability_effects_have_mechanics`. Heal and stat NVPs count through the script that reads them |
| It summons a pet, or acts on the owner's pet | `resources.pet_summons`; an owner-pet script (pets PT-08) |
| It places a deployable | `resources.deployables` |
| It is an ammo toggle | `resources.ammo_modifiers.toggle_ability_id`. The press behaves as before (D-AM07) |
| It is a weapon shot (`required_ammo` above 0) | It spends a round and carries the loaded ammo's modifier and on-hit effect |
| It is Cover Stance (1451) | Granted by the cover hold (NA22) |
| It is Reload (596) | The reload pipeline runs it; its effect 658 names the unregistered `Reload` |

An event set alone does not count: an animation is not a mechanic. The predicate reads the seed, so an ability lights up as soon as a generator packet gives one of its effects a number or a script (AB-03 damage, AB-04 stats, AB-08 stances and passives). On `main` after AB-10, 284 of 1,886 seeded abilities have a mechanic (276 after AB-08, 264 after AB-07, 262 after AB-04, 247 after AB-03); the live-DB test `seeded_has_mechanics_count_live_db` pins the number. Out-of-scope families (stealth, self-revive, Asgard energy, turrets; D-AB11) get the same refusal, because they have no mechanic either.

Never refused: NPC and pet casts, an ability the server has no definition for (silent, as before), and an ability the active weapon grants through `items_event_sets` (the basic attack). Each refusal logs one DEBUG `abilities` row, `event=no_mechanics_refused`, `reason=no_mechanics`, with `ability_id`, `ability_name`, `effect_count`, `animates`, `account_id` and `player_id`.

## In-game combat debug

A Game Master can watch the server resolve casts from inside the game (ability-mechanics AB-N1). The game has no combat-debug window and cannot receive `onSendCombatDebug`, so each debug line arrives as an ordinary chat line on the feedback channel (`onPlayerCommunication`, channel 9), the same sky-blue Info-tab line other GM feedback uses ([native-combat-debug.md](../reverse-engineering/findings/native-combat-debug.md)).

| Command | Index | What it toggles |
|---|---|---|
| `/gmdebugcombat` | 170 | Combat debug (`bCombatDebug`): one line per hostile hit you land or take |
| `/gmdebugcombatverbose` | 171 | Verbose combat debug (`bCombatVerboseDebug`): the hit lines, plus every effect plan, NVP damage entry, ledger entry and pulse |
| `/gmdebugheal` | 172 | Heal debug: one line per heal or buff you cast or receive |
| `/gmdebugability <abilityId>` | 169 | That ability in `debugAbilityList`: its casts by you or on you print with every other toggle off. `0` is `clearAbilityDebug` (empties the list and the mob list, lines back to you) |
| `/gmdebugabilityonmob <abilityId>` | 176 | Your selected mob's casts of that ability (`0`: all of them) print to you |

The game sends these only from a GM avatar, and the server's GM gate refuses them for anyone else. Every press answers with a feedback line (`gmDebugCombat: Combat debug on: ...`) and writes one `abilities` `gm_command` row. Cells 2 (`toggleCombatDebug`), 3 (`toggleCombatVerboseDebug`) and 6 (`toggleHealDebug`) do the same for a crafted caller; the stock game has no event bound to them. `setAbilityDebugTarget` (send your lines to another player in your space) and `clearAbilityDebug` are server-side helpers in `cell::combat_debug::commands`; no client event reaches the first. The toggles live in memory only, like god mode: a relog starts with them off, and a destroyed entity's toggles, and any mob debug on it, are forgotten so a recycled entity id starts clean.

**What a line says.** Every line starts with `[CD #<cast_id>]`, the cast's join key to its `abilities` rows. A hit: `[CD #7] Pistol Shot (592) Gm(1) -> Jaffa(3): hit, roll 0.620 qr 0.150; HP 100->77 (-23), FP 50->50 (+0)`. A heal or routed landing ends `landed;` and the recipient's pools; a pulse names its effect and the pulses left; a cast that reached no one prints `fired, nothing resolved`. Verbose adds `plan eff <id> -> <target>: <path> (<reason>)`, `nvp eff <id> -> ...: base H.. F.., dealt H.. F.., absorbed ..`, `ledger eff <id> -> ...: applied, 10.0 s` and `pulse eff <id> -> ...: path nvp`. The values are the ones the AB-T3 rows log (`qr_rolled`, `effect_planned`, `nvp_damage_resolved`, `stat_buff_applied`, `pulse_ticked`), taken at the same points.

**Who gets it.** You, when you cast the record or are touched by it and its toggle is on (combat or verbose for a hostile cast, heal for a beneficial one, any of the three for a later pulse), or when the ability is in your list; and you, for any cast of a mob you turned mob debug on for. The lines go out when the cast's scope closes (the zero-warmup launch, the warmup fire, each pulse, a ground cast's secondaries).

**Same text in SigNoz.** Each line, sent or not, also writes one `abilities.debug` DEBUG `combat_debug_line` row whose `text` is exactly the text sent; a sent line's row is written after the send, with `delivery = queued_to_base` (or `send_failed`) ([observability-target-catalog.md](../architecture/observability-target-catalog.md)).

**Limits.** A line longer than 255 UTF-16 units (the server's chat cap, D-SS12) is split, continuation lines starting `  ... `. Each recipient gets at most 20 lines a second; past that a line is held back (its row still written, `delivery = suppressed`) and the next line after the second ends is `[CD] +N lines suppressed (...)`. Ledger removals, absorb-shield settles and the after-hit scripts' own pool changes are not in the lines; their rows are.

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

## Ability Trees and Training

Each archetype's training tree is `resources.archetype_ability_tree`. It is read once per server process into `AbilityTreeCatalog` (`crates/cell-catalog/src/ability_tree/`), joined to `resources.abilities.training_cost`. The cell (trainer window and purchase gate) and the base (player load) share that one snapshot through `ability_tree::shared_catalog`. Beyond the original six columns, each node carries `required_branch_points` (default 0), `skill_point_cost` (default 1), `is_branch_root`, `is_capstone`, `branch_name` and `project_status`. The table is keyed on `(archetype, tree_index, ability_index)` and unique on `(archetype, ability_id)`; one ability can sit in several archetypes' trees.

The `onAbilityTreeInfo` message (client method 141) that world entry sends is built from the same catalog by `ability_tree::tree_info`: three branches by `tree_index`, each in catalog order (`tree_index, ability_index`), which is the order the trainer uses. There is no per-player query and no tree hard-coded in Rust. An archetype with no rows gets three empty branches and one `abilities event=tree_missing reason=archetype_has_no_tree` WARN. For the seeded archetypes the bytes are unchanged from the earlier per-player query.

The tree and trainer seeds are the owner's EMULATOR FINAL v2 level-50 workbook, generated by `tools/ability_trees/generate_seed.py` (see [its README](../../tools/ability_trees/README.md)); never edit the two seed files by hand. The seed holds 439 nodes (419 distinct abilities, 19 of them shared between archetypes) in 21 branches: Soldier 72, Commando 64, Scientist 59, Archaeologist 65, Asgard 66, Goa'uld 62 and Shol'va 51. The workbook's Free Jaffa / Shol'va tree maps to `ARCHETYPE_Sholva` only, so `ARCHETYPE_Jaffa` has no tree (decision D-AT04). Each branch has one root (node 1, no prerequisites) and one capstone that unlocks at level 50. `resources.trainer_abilities` debug list 1 (the Interaction Debug NPC, template 25) offers every node. The live-DB tests in `ability_tree::tests::seed_live_db` pin these counts and the trainer-to-tree match against a fresh `db/database.sql`.

Whether a player may train a node is decided in one place, `evaluate_train`. The trainer window's `trainable` byte (`onTrainerOpen`) and the `trainAbility` purchase gate both call it, so a node the window enables is always a node the server accepts. Its gates run in order: the ability exists, the player is a loaded character, the ability is not already known (a silent no-op), it is in the player's archetype tree, the player meets its level and prerequisites, the archetype-wide spend reaches its `required_branch_points` (`SpendGate`), and the player's training points cover its `skill_point_cost` (`NotEnoughPoints`). Gate families live one per file under `ability_tree/gates/`.

The spend gate counts trainer points spent **across the archetype**, not per branch (decision D-AT01 in [the ability-tree campaign](../analysis/ability-trees/README.md)): counted per branch, no branch can open past its root. Prerequisites, which always sit in the node's own branch, keep a node on its branch path. Only trainer purchases count as spend, so a starter ability satisfies a prerequisite but opens no spend gate.

A purchase the cell accepts is sent to the base as `CellToBaseMsg::TrainAbility` with the node's cost and branch. The base runs **one** `UPDATE` on `sgw_player` (`progression/train_ability.rs`): it appends the ability to `abilities` and `trained_abilities`, subtracts the cost from `training_points` and adds it to `tree_points_spent`, guarded by `training_points >= cost AND NOT (abilities @> ARRAY[id])`. A double-click or replayed packet matches no row, so it debits once. The four fields move together or not at all. `AbilityGranted` returns both counters, and the cell then sends, in order: `onKnownAbilitiesUpdate`, `onEntityProperty(GENERICPROPERTY_TrainingPoints, n)` (the point counter; the level-up bundle uses the same builder), and, while a trainer is pinned, the `onTrainerOpen` re-send.

The cell mirrors the trainer gates' inputs in `CellEntity::tree_progress` (`trained_abilities`, `tree_points_spent`, `training_points`) and `CellEntity::level`. All four are loaded at world entry by the `onClientReady` SELECT. `AbilityGranted` updates the purchase fields, `BaseToCellMsg::ProgressionChanged` (sent by `handle_grant_xp` after a level-up persists) updates the level and points, and `BaseToCellMsg::TrainingPointsGranted` (sent after a GM `gmGiveTrainingPoints` grant persists) updates the points and also sends the client counter and the pinned-trainer re-send. Before AT-03, nothing set a player's cell level, so every player trained as level 1.

A purchased node whose `resources.abilities.training_cost` is 0 logs `abilities event=train_raw_cost_zero` at WARN. The purchase still goes ahead at `skill_point_cost`; the source value is never rewritten.

### Trainer authority

A purchase must happen at a trainer (AT-04). Before AT-04, a forged `trainAbility` trained from anywhere. The trainer gates in `ability_tree/gates/trainer.rs` run after the node and spend gates. They read the player's pinned `last_interaction_target`, which the cell resolves into a `TrainerPin` (`cell/interactions/trainer_authority.rs`). The pin must:

- be set;
- resolve to an entity that still exists;
- have a template listed in `template_trainer_lists`;
- be a trainer whose list offers this ability to the player's archetype;
- still pass `interact_target_in_range`: the same space, and within `MAX_INTERACT_DISTANCE` (5).

The trainer window computes its `trainable` byte with the same pin. A player who walks out of range and then triggers a re-send therefore sees every node greyed out, which matches what a purchase would get.

`interact_target_in_range` now also rejects a target in another space. Positions are per-space coordinates, and `SpaceManager::get_entity` searches every space. Without the check, a trainer in another space at nearby coordinates counted as in range, and so did any `interact` target.

### Rejection feedback

A rejected purchase sends `onErrorCode` (121) with `SystemID 0` (`ERRORCODE_SYSTEM_Ability`), `InstanceID` set to the ability id, and the `ErrorCodeID` below (`cell/cell_methods/player/vendor/train_feedback.rs`). When the pin is a live trainer, `onTrainerOpen` is re-sent after it.

| Rejection | `ErrorCodeID` | Fit |
|---|---|---|
| Not in the archetype's tree | 6 `NotSpecifiedArchetype` | Exact |
| Level too low | 9 `LevelGreaterThanOrEqual` | Close |
| Missing prerequisite | 167 `EntityDoesNotHaveAbility` | Exact |
| No trainer pinned, trainer despawned, pin not a trainer, not offered here, out of range | 43 `OutsideDistanceCheck` | Close |
| Not enough training points, spend gate (AT-03) | 35 `StatValueLessThan` | Reused: the 2009 enum has no token for either |
| Already known | none: silent | The client already renders the node as known |
| Unknown ability id, no player id, no archetype | none: silent | A legitimate client cannot send these. They are logged at WARN |

The mapping comes from AT-E1 ([ability-trainer-ui.md](../reverse-engineering/findings/ability-trainer-ui.md) §2). Whether the client renders `onErrorCode` at all is **unresolved**: no client Lua consumes it. The trainer re-send is the feedback the player is known to see, so every coded rejection is followed by one. The re-send is skipped for the silent rows, so a forging client gets no free `onTrainerOpen` build per packet.

### Respec

The trainer window's Respec button calls `respecAbilities()`, which sends cell method 72 `resetMyAbilities` with no arguments (AT-E1 Q5). Since AT-08 the server implements it; before, it only logged `UNIMPLEMENTED`.

The cell checks two things (`cell/cell_methods/player/vendor/respec.rs`):

- The player's pin must be a live trainer that is still within `interact_target_in_range`. This is the same `trainer_pin` a purchase uses, but the trainer's offered list does not matter.
- Something must be trainer-bought (`tree_progress`).

It then sends `CellToBaseMsg::ResetAbilities` with the price, `RESPEC_COST_NAQUADAH` = 1000 (decision D-AT10). The same constant fills `onTrainerOpen`'s `CostToRespec` field, so the window shows what the respec charges.

The base runs **one** `UPDATE` on `sgw_player` (`progression/respec.rs`). It is guarded by `naquadah >= cost` and "`tree_points_spent > 0` or `trained_abilities` is not empty", and it does the following:

- removes every id in `trained_abilities` from `abilities`, keeping the order of the rest, so starter and quest grants survive;
- adds `tree_points_spent` back to `training_points`, which is the exact refund because only trainer purchases count as spend (D-AT03);
- sets `tree_points_spent = 0` and `trained_abilities = '{}'`;
- subtracts the price from `naquadah`.

A replayed respec finds nothing trainer-bought, so it matches no row and charges nothing. When the guard holds the row back, a read-only `SELECT` decides which refusal to report.

The base answers with `BaseToCellMsg::AbilitiesReset`, which carries a `RespecOutcome`. On `Reset`, the cell mirrors the row: it drops the refunded abilities, clears `tree_progress` and sets the points. A warmup in progress on a refunded ability is interrupted (reason `ability_unlearned`, AT-10), and its interrupt frames go out first. The cell then sends, in order:

1. `onKnownAbilitiesUpdate`;
2. `onEntityProperty(GENERICPROPERTY_TrainingPoints, n)`;
3. `onCashChanged(naquadah)`;
4. while a trainer is pinned, the `onTrainerOpen` re-send, which shows the branch roots as buyable again.

Every refused respec gets feedback on the first press: `onErrorCode` with `SystemID 0` and `InstanceID 0` (a respec names no ability), then the pinned trainer's re-send (`cell/interactions/respec_feedback.rs`).

| Refusal | Decided by | `ErrorCodeID` | Fit |
|---|---|---|---|
| No trainer pinned, trainer despawned, pin not a trainer, out of range | cell | 43 `OutsideDistanceCheck` | Close, as for purchases |
| Nothing trainer-bought (includes a replay) | cell, or base when the two race | 167 `EntityDoesNotHaveAbility` | Reused: the player has none of the abilities a respec removes |
| Too little naquadah | base | 35 `StatValueLessThan` | Reused: the enum has no currency token |
| Entity is not a loaded character | cell | none: silent | A legitimate client cannot send this. It is logged at WARN |

A first press with nothing trainer-bought gets feedback too: the button looks enabled, so the project's first-press rule applies. Because the trainer gate runs first, only a player standing at a trainer can trigger the re-send.

A press within 1 second of the last forwarded respec (`RESPEC_RETRY_WINDOW`) is dropped without an answer. The earlier press's answer is still on its way, so the dropped press is not a first press. The window stops a double-click from showing the success burst and then "nothing trained". It also limits the base to one row-locking `UPDATE` per player per second, which matters because the cell cannot see the naquadah balance. `AbilitiesReset` names the character that was reset, and the cell ignores the message if the entity id now belongs to another character.

**The hotbar.** The client keeps its action-bar bindings in a per-character Lua saved variable (`GActionProfiles`, declared as a `<CharacterVariable>` in `ActionButtons.toc` and written to `Documents/My Games/.../SGWGame/<account>/<character>/ActionButtons - Saved Vars.lua`). No server method, property or table carries it, so the server cannot strip refunded abilities from it. The only server-held list the bar draws from is `sgw_player.abilities`, which the respec `UPDATE` strips, and `onKnownAbilitiesUpdate` re-sends it. A button still bound to a refunded ability stays on the bar until the player clears it. Pressing it is refused with `onErrorCode(0, ability_id, 167 EntityDoesNotHaveAbility)` (`use_ability/handle.rs`, `send_not_known_feedback`). Before AT-08 that refusal was silent. An ability id with no server definition stays silent, because a legitimate client cannot send one, and so do NPC casters. The action bar has no server hook. A client Lua patch that clears it would be an owner decision. AT-E1 found no client-side cleanup either ([ability-trainer-ui.md](../reverse-engineering/findings/ability-trainer-ui.md) §5).

## Reading one cast

Every cast leaves rows on both sides, and one query sequence reads them in order. The design is the [ability-mechanics telemetry plan](../analysis/ability-mechanics/lab-uat-and-telemetry.md#the-correlation-model); the saved views and the dashboard are in [tools/signoz/abilities/](../../tools/signoz/abilities/README.md).

**1. Find the cast.** A cast's id is `cast_id`: the `effect_seq` its launch minted, which is also the effect id the client receives in `onEffectResults` and the `InstanceId` of its sequences. It counts per caster, so it is unique only together with the caster. Start from the launch row:

```text
service.name = 'cimmeria-server' AND scope_name = 'abilities'
  AND event = 'ability_launched' AND player_id = <P> AND ability_id = <A>
```

A press that never launched has no `cast_id`. Its refusal row (`use_ability_*`, `*_refused`; the **Abilities — Refusals by reason** view) names the `reason`, and `abilities_refused_total` counts it under the same value.

**2. Read the server rows in order.** Every server row of the cast carries its `cast_id`: receipt-to-launch gates, warmup, fire, the QR roll (`abilities.qr`), each effect's plan and NVP damage (`abilities.effect`), pulses (`abilities.pulse`), the timed-effect ledger, and every client-bound send (`abilities.wire`). Sort oldest first:

```text
service.name = 'cimmeria-server'
  AND (scope_name = 'abilities' OR scope_name LIKE 'abilities.%' OR scope_name = 'base.entity_method')
  AND cast_id = <C> AND (player_id = <P> OR entity_id = <caster entity>)
```

This is the **Abilities — One cast, in order** view. A wire row's `player_id` and `entity_id` name the entity the method is about, so a target's `onStatUpdate` carries the cast's `cast_id` but the target's ids: add `OR entity_id = <target>` to include it. A channel's cancel and an effect's expiry run outside the cast, and their rows name the cast that registered the effect (AB-T1, and the channel cancel since AB-T6).

**3. Join the press.** The receipt row, `scope_name = 'abilities' AND event = 'use_ability_recv'`, carries `mercury_seq`, the sequence number of the Mercury packet that delivered the `useAbility`. It has no `cast_id` (the launch mints it right after), so take the receipt just before the `ability_launched` row with the same `entity_id` and `ability_id`. The client logs the packets that carried its send as `client.ability.sent_seq` with `mercury_seq_first` and `mercury_seq_last`:

```text
service.name = 'cimmeria-client' AND client_target = 'client.ability.sent_seq' AND player_id = <P>
```

The receipt belongs to the send whose range holds its seq. The counter is 28 bits and wraps, so test membership modulo 2^28, `((seq - first) & 0x0fffffff) <= ((last - first) & 0x0fffffff)`, never `first <= seq <= last`. From that row, `send_id` leads to `client.ability.sent` (the arguments the client sent, and `client_target_id`, what the UI had targeted) and `press_id` to `client.ability.press` and any `client.ability.press_dropped`. A press the client dropped before sending has no seq: join it on `(player_id, ability_id)` within 2 s.

**4. Read the client's side.** Client rows are in the `cimmeria-client` service, named by `client_target`, with their fields in the `fields` JSON attribute. `player_id` and `account_id` are lifted out only when the event carries them; if a row has neither, filter on the session's `session_id` instead:

```text
service.name = 'cimmeria-client' AND player_id = <P>
  AND client_target LIKE 'client.ability.%' AND fields CONTAINS '"cast_id":<C>'
```

`client.ability.recv` rows decode what arrived (`onEffectResults`' `effect_id` is the `cast_id`; an `onSequence` gives its `instance_id`); `client.ability.applied` and `client.ability.shown` say what the client did with it. Timers join on `(player_id, ability_id)` for cooldowns and `(player_id, effect_id, secondary_id)` for effect durations.

**Counts, not stories.** The `abilities_*` metrics (`crates/cell-combat/src/cell/abilities/metrics/`) answer "how often" over many casts: outcomes, refusal reasons, effect paths, QR results, ledger removals, failed sends, press-to-fire time and damage and heal per pool, each with `world`. The **Cimmeria — Ability metrics** dashboard charts them. Their labels are the same strings as the rows' `reason`, `path` and `result` fields, so a spike on the dashboard leads straight to the rows.

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
