# Abilities + Effects System — decisions 23-33

> **Last updated**: 2026-10-03
> **Audience**: Engineers touching combat / abilities / effects on the cell
> **Type**: ADR + reference
> **Status**: Accepted
> **Part of**: [abilities-and-effects-system.md](abilities-and-effects-system.md), which holds the context, decisions 1-15 and the index of every decision. This file holds decisions 23-33 (pets, duels, range units, consumables, deployables, special ammo, NPC-vs-NPC targeting and the effect-script leaf crate), split out on 2026-10-03 with their numbers and text unchanged. Decision 34 (beneficial casts, AB-01) was added here after the split.

## Decisions

### 23. A pet summon is a player cast with a `pet_summons` row, diverted at launch and fire (pets PT-03)

**Decision:** A player ability with a `resources.pet_summons` row (`SpaceManager::pet_summons`)
summons a pet. It rides the ordinary cast of decision 21, with three diversions in
[`use_ability/summon.rs`](../../crates/cell-combat/src/cell/abilities/use_ability/summon.rs):

- **Launch.** Straight after the weapon redirect, the client's `target_id` is replaced by 0.
  The summon is a Self ability, so the client's target plays no part in it. With target 0
  the #444 target-validity gate never sees the cast. The gate itself is unchanged, so any
  other ability aimed at the caster still fails there. Two refusals run before the cooldown
  is charged: the summon must be in the trained set (a weapon grant does not count), and its
  template must be in the startup cache. Each sends `onErrorCode` plus a `CHAN_FEEDBACK`
  chat line.
- **Warmup.** The spawn timer is the ability's own warmup, scaled by the caster's
  `speedPet` stat (111) when the ability has `SpeedPet` (16384), like the other speed flags
  (D-PT10). Every decision-21 interrupt applies. An interrupted warmup never reaches the
  fire, so nothing spawns.
- **Fire.** `fire::fire_cast` diverts to `fire_summon` before any ammo, channel or damage
  step. `fire_summon` re-checks that the pet can be spawned before it touches the current
  pet (caster alive, in a space, template cached). A refusal plays `Ability_Interrupt` and
  sends the feedback pair, and the cooldown stays charged. Otherwise it calls
  `spawn_pet_from_template` first. A spawn that still fails answers exactly like a refusal
  (`Ability_Interrupt`, the feedback pair, the cooldown stays charged), and the owner keeps
  its current pet. A spawn that succeeds plays `Ability_End`, despawns the owner's oldest
  pets down to `max_active - 1` (D-PT04, counting every pet the owner had before the spawn),
  and queues the target VFX.

The summon's phase sequences carry TargetID = caster, as python's
`targetId or ent.entityId` did. The target VFX is event set 1122 `Effect_Init` (2000),
sequence 2293. It is an `onSequence` on the pet, with source = owner, target = pet and
`InstanceId` 0, which is how python played an effect sequence on its target. It waits on the
pet registry
([`pets/arrival.rs`](../../crates/cell-world/src/cell/pets/arrival.rs)) until the owner
witnesses the pet. The drain runs after the AoI tick and sends the VFX to the pet's
witnesses, so it can never reach a client ahead of the pet's CREATE_ENTITY. It is dropped
after 2 s, and `forget_pet` scrubs it on every teardown path. It is also dropped when the
owner's entity id now belongs to a player who is not the pet's summoner
(`PetRegistry::summoner_matches`, #870). It is counted and logged as sent only when at least
one witness send succeeds. A summon carries `Deactivate_AutoCycle` and
`DoNotActivate_AutoCycle`, so it is not stashed as the last-fired ability, and a later
`setAutoCycle(1)` press cannot re-fire it. Neither flag lets any ability arm the loop:
1024 clears it, and 512 leaves it as it was (python passed `autoCycle = False`,
`SGWPlayer.py:1177`).

**Why:** The 2009 data never linked a summon to a template. The summon abilities carry no
effects, and the editor's "Spawn Mob" effects name no template (pets audit A-26), so there
is no effect script to run. Keying on the ability id keeps the damage pipeline and the #444
gate untouched, which is what the packet asked for. Discarding the target is safer than
rejecting it: a Self ability legitimately arrives with 0 or with the caster's own id. The
VFX waits for the intro because a client cannot play a sequence on an entity it has not
created.

**Consequences:** `AF_CHANNEL_ALLOWS_MOVEMENT` used to be bit 14, which is the client's
`SpeedPet`, so every seeded summon warmed up immune to the move interrupt. It is now bit 20
(decision 7). NPC casters never summon, because `player_summon` answers only for players.

**Code:** [`use_ability/summon.rs`](../../crates/cell-combat/src/cell/abilities/use_ability/summon.rs),
the hooks in `handle.rs`, `fire.rs` and `sequence.rs`, and
[`pets/arrival.rs`](../../crates/cell-world/src/cell/pets/arrival.rs). Tests are in
`use_ability/tests/summon.rs` and `pets/tests/arrival.rs`; the evidence and the
regression proofs are in the [PT-03 worknote](../analysis/pets/worknotes/pt-03.md).

### 24. One hostility rule for every gate; a duel partner is the only player target (social systems SS-D2)

**Decision:** `combat::player_may_attack(attacker, target, &duels)` (`crates/cell-world/src/cell/combat/aggression.rs`) is the single rule for what a player may damage. An NPC target must be a hostile-faction non-pet, as before. A player target is admitted only when `DuelRegistry::can_harm` says the two are an engaged duel pair, in the same space. The four hostility gates all call it: the single-target launch (`use_ability/handle.rs`), the warmup re-check at fire (`use_ability/warmup/tick.rs`), and the ground-AoE and cone collectors (`dispatch/mod.rs`, `cone_aoe/geometry.rs`). The two collectors scan `combat::area_candidates` (every NPC, plus the caster's engaged partner) and filter with `combat::may_hit_in_area`, which is `player_may_attack` for a player caster and was the historical hostile-faction rule for an NPC caster (since #1009 an NPC caster follows the NPC target rule, decision 32).

**Why:** the gates had drifted into four inline copies of "hostile NPC only" (audit A-42), so a duel had to widen all four or leak through one. A candidate scan that adds only the partner means no filter mistake can reach a bystander. The client's PvP flag (`onEntityProperty(4, v)`, decision D-SS23) is never read back: a stuck flag cannot make anyone attackable.

**Consequences:** NPC-versus-player and pet targeting are unchanged; a pet never joins its owner's duel: `pet::fight_refusal` uses the no-duel form, `player_may_attack_pve`, which is also the NPC half of `player_may_attack`. Player-on-player damage creates no threat, so the duel supplies its own combat source (`cell::duel::combat`). The effect pulse does not re-run the rule, so the duel's single end (`duel::end_engaged`) strips every active effect the partner's engaged entity invoked on each duelist, with the normal `on_remove` and zero timer; an auto-cycle loop on a player the caster may no longer harm is cleared. Partner damage is non-lethal since SS-D3 (decision 26), with the clamp in the pulse seam as well.

**Code and tests:** `aggression.rs`, the four gates above, `crates/cell-world/src/cell/duel/`. `use_ability/tests/duel_gate.rs` (`duel_partner_damage_allowed_at_all_four_gates`, `bystander_untouchable_during_duel`) fails when any one gate is reverted; the proof is in the [SS-D2 worknote](../analysis/social-systems/worknotes/ss-d2.md).

### 25. Owner abilities that act on a pet are diverted to the owner's pet, and their state lives on the pet (pets PT-08)

**Decision:** An ability with an effect whose `script_name` is a pet script (`PetStatBuff`,
`PetDeathTimer`, `HealPetHealth`; `effects::pet_scripts::acts_on_owner_pet`) acts on the
caster's pet, never on the client's target. It rides the cast of decision 21 with the same
three diversions as decision 23, in
[`use_ability/owner_pet/`](../../crates/cell-combat/src/cell/abilities/use_ability/owner_pet/mod.rs):

- **Launch.** The client's `target_id` is replaced by 0, so the #444 gate never sees the cast
  and stays as strict for every other ability. The pet comes from
  `SpaceManager::owner_pet_targets`
  ([`pets/owner_target.rs`](../../crates/cell-world/src/cell/pets/owner_target.rs)): the
  registry's pets of the caster, each kept only when the summon-time identity says the caster
  summoned it (`PetRegistry::summoner_matches`), it is alive, and it is in the caster's
  space. A bare owner id is never enough. With no such pet the press is refused before the
  cooldown is charged, with `onErrorCode` plus a `CHAN_FEEDBACK` line: 190
  `EntityDoesNotHavePet` for no pet, a pet in another space or a reused owner id, 14
  `NotLiving` for a dead pet, and 133 `EffectMonikerOnEntity` for To The Death pressed while
  it already runs.
- **Warmup.** The ability's own, with every decision-21 interrupt.
- **Fire.** `fire::fire_cast` diverts to `fire_owner_pet` before any ammo or damage step. The
  pet is resolved again. A refusal plays `Ability_Interrupt` and sends the feedback pair, and
  the cooldown stays charged. Otherwise `Ability_End` plays with TargetID = the pet, and each
  pet script runs with source = owner and target = pet. A `TCM_Single` effect lands on one
  pet; any other collection method lands on every pet the owner has out (one today, D-PT04).
  A pulsing effect (Repair Turret: Regenerate) is registered on the pet, invoked by the owner.
  The pet's dirty stats go to its witnesses. Nothing enters the damage pipeline, threat or
  kill credit.

The state these abilities leave is on the pet, not in `active_effects`:

- **Buff ledger.** `register_active_effect` never registers a `pulse_count = 1` row, which is
  what the seed gives Holy Warrior (4220), To The Death (4121) and Lord's Concentration
  (350). `PetStatBuff` writes a `PetBuff` on `PetState::buffs` instead
  ([`pets/buffs.rs`](../../crates/cell-world/src/cell/pets/buffs.rs)). It records the delta
  each stat really moved, and removal takes back exactly that, as python's `statChanges` did
  (`AbilityManager.py:438-441`). Re-applying the same effect replaces it; it never stacks.
- **Bounds widen, a deliberate deviation.** `DEFENSE` and `INTERRUPT_RES` default to `[0, 0]`,
  so python's clamp would drop Holy Warrior's -100 Defense and Lord's Concentration's +50. The
  ledger widens that one pet's bound to admit the delta.
- **Toggle.** For an ability with `Toggled` (8, `AF_TOGGLED`), `PetStatBuff` takes the buff
  off when the pet has it and puts it on with no expiry when it has not. The owner gets a chat
  line with the new state ("Holy Warrior is on."), because the Ability window shows none.
- **Expiry and To The Death.** `owner_pet_tick` runs every AoI tick after the pet sweep. It
  takes expired buffs off, then kills each pet whose `PetState::doomed_at` has passed through
  `kill_npc_out_of_band(pet, pet, attacker_is_player = false, grant_xp = false)`, after
  zeroing its HEALTH. The kill pays nobody: no XP, no mission `EntityDeath` (only the
  kill-credit wrappers raise it), and a pet has no loot table. The corpse then follows the
  pet path of D-PT08. 4119 "Pet Death Timer" (`PetDeathTimer`) arms the doom; 4122 "Pet
  Death" has no script, because a script cannot await the death resolver. A re-cast while the
  pet is doomed is refused, another deliberate deviation: python's refresh would restart the
  60 s timer, and with a 30 s cooldown the +400 Accuracy would never end.
- **Passives.** An `EF_AlwaysPersist` (524288) effect whose script is a passive script
  (`pet_scripts::is_passive_script`, today only `PetSummonSpeed`) holds while its ability is
  known. [`effects/passives.rs`](../../crates/cell-world/src/cell/effects/passives.rs) runs it
  at `InitPlayerState`, `AbilityGranted` and `GmAbilityGranted` (the GM `.giveability` mirror),
  and runs its `on_remove` at `AbilitiesReset`.
  Heed Our Calling (2852 -> 4968) sets the owner's `speedPet` to its base plus 100, so a
  `SpeedPet` summon's warmup scales to 0 (D-PT10). The stat is server-side only: the passive
  leaves it clean, so no burst changes.

Holy Warrior's 4087 "Stance Removal" is "Remove Effect of moniker EFFECT_Stance", the
mutual-exclusion half every player stance carries. No player stance effect is active on this
server, and the seed links no effect to that moniker, so it has no script and removes nothing.

**Why:** The 2009 rows carry no `script_name` and no NVPs for these effects, so the scripts and
magnitudes are seed edits, each the number in the effect's own description
(`effect_nvps` 350-357). Keying the redirect on the scripts keeps it data-driven: a new
pet-acting ability is wired by naming the script on its effect. Keeping the state on the pet
means a despawn, a replacing summon or the owner's death (which despawns the pet) clears it,
and the owner carries no "buff on" flag. Lord's Concentration (1650) shipped with no effect at
all; effect 350 is server-only, and pets D-PT17 records its magnitude and duration as a
greenfield decision.

**Consequences:** Nothing reads `INTERRUPT_RES` yet. The server has no damage-driven warmup
interrupt (decision 21), so Lord's Concentration changes a stat that will matter only once
one lands. The Repair Turret heals redirect to whatever pet the owner has, which is a
Servant Lord pet until turrets exist (PT-12). Repair Turret: Restoration (1214, revive) is not
wired. The scripts are in their own module, `effects/pet_scripts/` in
`cimmeria-cell-effect-scripts`, because `scripts.rs` is over the file cap; the two name
predicates the layers below that crate ask (`acts_on_owner_pet`, `is_passive_script`) stay in
`cimmeria-cell-world`'s `effects/pet_scripts.rs` (decision 33).

**Code:** [`use_ability/owner_pet/`](../../crates/cell-combat/src/cell/abilities/use_ability/owner_pet/mod.rs)
(launch, fire, tick, feedback), the hooks in `handle.rs`, `fire.rs` and `sequence.rs`,
[`effects/pet_scripts/`](../../crates/cell-effect-scripts/src/cell/effects/pet_scripts/mod.rs)
(the scripts, with their tests), [`effects/pet_scripts.rs`](../../crates/cell-world/src/cell/effects/pet_scripts.rs)
(the name predicates), [`effects/passives.rs`](../../crates/cell-world/src/cell/effects/passives.rs),
[`pets/buffs.rs`](../../crates/cell-world/src/cell/pets/buffs.rs) and
[`pets/owner_target.rs`](../../crates/cell-world/src/cell/pets/owner_target.rs). Tests are in
`use_ability/owner_pet/tests/`, `pets/tests/owner_buffs.rs` and
`base_messages/tests/passive_abilities.rs`; the evidence and the regression proofs are in the
[PT-08 worknote](../analysis/pets/worknotes/pt-08.md).

### 26. Duel-partner damage is held at 1 HP in both damage seams (social systems SS-D3, D-SS20)

**Decision:** `cimmeria_cell_world::cell::duel::clamp_partner_lethal(mgr, attacker, target, source)` runs wherever a player's HEALTH is written by an attacker, before anything reads it for a death and before the stat flush:

- `apply_damage_to_target` (`damage_apply/mod.rs`), after the direct damage (so `target_died` sees 1) and again after the effect scripts (so the effect-driven death sweep sees 1);
- `fire_pulse` (`effects/pulsing/tick.rs`), after both the script and the NVP branch and after the surrender floor.

When the attacker and the target are an engaged duel's engaged entities and HEALTH is at or below 0, HEALTH becomes 1 and the hit is returned. The caller ends the duel with `duel::finish_clamped` (`EDUEL_DEFEAT_Health`, the clamped duelist losing) only after the rest of the resolution has run: in `apply_damage_to_target` at the very end, after the pulsing effects are registered, and in `fire_pulse` after the flush. `effect_pulse_tick` skips a due instance that an earlier pulse in the same tick removed.

**Why:** D-SS20 makes duels non-lethal, and a lethal duel would send duel kills down the loot and XP path. The pulse never re-checks hostility (decision 24), so a clamp only in `damage_apply` would let a partner's DoT kill. Ending the duel at once would strip the partner's effects and end `can_harm` before a script bleed or a newly registered DoT from the same hit had been clamped, and those would then kill after the duel. Ending last means the end's `strip_from` removes everything the hit registered.

**Consequences:** Damage from anyone else is untouched: a third party can still kill a duelist, and `resolve_death` reports that death to the duel (`duel::on_death`). A clamped duelist never reaches `resolve_death`: no corpse, loot, XP, Defeat Window or respawn. The client is told 1 HP, never 0. The pulse's `still_active` check also closes an older window, in which a channel cancel between awaits let a removed instance fire from the tick's snapshot. `apply_damage_to_target` re-runs the harm gate for player-on-player damage before anything else (PR #924 review): a multi-hit ability (two cones) collects its targets up front, so without the re-check the second cone would land on the ex-partner after the first had ended the duel. The gate (`player_may_attack`) also requires the exact engaged entities (`DuelRegistry::can_harm_entities`), the same pair the clamp keys on, so every hit the gate admits is one the clamp covers. A duelist already at 0 HP when a partner hit lands (a third-party DoT, which kills no player today) is raised to 1 by the clamp; accepted in review.

**Code and tests:** `crates/cell-world/src/cell/duel/paths.rs`, the two seams above, `death/mod.rs`. `use_ability/tests/duel_nonlethal.rs` (`lethal_partner_hit_clamps_to_one_hp`, `lethal_partner_bleed_clamps_to_one_hp`, `no_loot_xp_or_corpse_after_a_clamped_end`, `third_party_kill_is_normal_death`); the proof is in the [SS-D3 worknote](../analysis/social-systems/worknotes/ss-d3.md).

### 27. Ability ranges are UE3 units in the data and metres on `AbilityDef` (#919)

**Decision:** `load_ability_defs` divides `resources.abilities.min_range` / `max_range` by `ABILITY_RANGE_UNITS_PER_METRE` (100) once, and `AbilityDef::min_range` / `max_range` are `f32` metres. Every consumer resolves the reach through `AbilityDef::max_range_or_default` / `ability_max_range` (`crates/entity/src/abilities/range.rs`), which maps the `0` sentinel to `DEFAULT_ABILITY_MAX_RANGE` (30 m): the launch check in `handle_use_ability`, the warmup fire-time re-check, the ground-target primary check, the auto-cycle skip, the pet-order pre-check (CM 88) and the NPC/pet AI's `ability_ranges`.

**Why:** The data is the client's own. The 2009 `CookedDataAbilities.pak` ships the same numbers (1652 Jaffa: Double Blast `MaxRange="3000"`), and the client uses them in UE3 world space: the ground-target reticule clamps against them from the pawn's UE3 location, next to AoE radii it converts to UE3 units (Medium = 1000). Every non-zero seeded range is a multiple of 100. Compared raw with metre positions, every ranged ability with a real range reached 1 to 100 km. Evidence and addresses: [ability-resolution-pipeline.md § Range units](../reverse-engineering/findings/ability-resolution-pipeline.md#range-units-919-verified-2026-09-28).

**Consequences:** 1652 reaches 30 m, 1653 8 m, grenades 25 m, deployables 5 m; turret 1205's 300-unit minimum is 3 m. Weapon ranges (`resources.items.*_range`) are already metres and are not converted. Since decision 30, a player's cast is held to `min_range` (#1016) and a `UseWeaponRange` (flag 4) ability takes the weapon's reach (#1017). Tests: `spawner::abilities::range_unit_tests`, `use_ability/tests/range_units.rs`, and the live-DB `spawner/tests/live_db_ability_ranges.rs` and `use_ability/tests/range_units_live_db.rs`.

### 28. Native consumables: the base consumes before the cell applies, and timed stat buffs live in their own ledger

**Decision:** Two parts.

**(a) Items apply their own `items_event_sets` ability.** Using an item whose event-5 (`EVENT_ITEM_USE_ABILITY`) binding names an ability whose effects all run `HealHealth`, `HealFocus` or `StatBuff` applies that ability to the user, with no content chain, in [`cell::content::consumable_use`](../../crates/cell-content/src/cell/content/consumable_use.rs). Two bindings are excluded: ability 597 "Heal Focus", which the seed binds to 158 unrelated mission items as filler, and any item an `item_use` chain triggers on (the chain owns the item; the Ambernol vial keeps chain 1034). The use is a round trip:

1. `fire_item_use` offers the use to `try_native_use` before any chain runs. A use that would do nothing is refused with the owner-pet feedback pair (`onErrorCode` plus a `CHAN_FEEDBACK` line, decision 25): the user is dead (code 14 `NotLiving`), or every pool the item heals is full (code 32 `StatValueGreaterThanOrEqual`). A `StatBuff` is never refused for headroom.
2. Otherwise the cell sends `CellToBaseMsg::ConsumeItemForUse`. The base consumes one unit through `removeItem`'s locked transaction, held to the item's design id ([`consume_for_use.rs`](../../crates/base-methods/src/base/world_entry/methods/inventory/core/consume_for_use.rs)), and only after the commit answers `BaseToCellMsg::ItemUseConsumed`, straight on the channel (at most once, never through the outbox).
3. `apply_consumed_item` re-derives the ability from the consumed row's design id and applies it through decision 16's `effect_apply::apply_ability_effects`, target and source the user.

**(b) `StatBuff` and the stat-buff ledger.** The stimpacks' effects (3949-3990) run [`StatBuff`](../../crates/cell-effect-scripts/src/cell/effects/stat_buff/mod.rs) (the ledger it writes through is `cimmeria-cell-world`'s [`effects/stat_buff/`](../../crates/cell-world/src/cell/effects/stat_buff/mod.rs)), which reads one NVP per attribute (`Coordination`, `Engagement`, `Fortitude`, `Intellect`, `Morale`, `Perception`) and writes a `TimedEffect` entry to [`CellEntity::stat_buffs`](../../crates/entity/src/cell_entity/stat_buff.rs) that expires after the effect's `pulse_duration`. The ledger is **keyed by the stat**: a second buff on the same attribute takes the first off (restoring exactly what it moved) and applies itself, whatever its tier; buffs on different attributes never interact. A two-stat stim's two effects each land on their own stat. Bounds widen instead of clamping: a primary attribute sits at `cur == max`, so the buff raises `max` as far as the new value needs and records it, and removal takes back exactly that. The async half is combat's ([`effects/stat_buffs/`](../../crates/cell-combat/src/cell/effects/stat_buffs/mod.rs)): the 100 ms `stat_buff_tick` takes expired buffs off and sends the client the duration timers the synchronous ledger queued (`onTimerUpdate` type 5 with an absolute expiry, decision 22, and `0.0, 0.0` to clear, including the icon of a replaced buff), and `resolve_death` takes off every buff whose effect carries `EF_ClearOnDeath` (4).

**Extension (ability mechanics AB-04, D-AB08): the timed effect ledger.** The same ledger now holds every single-pulse timed effect with stat NVPs, not just the stimpacks. Its contract (the campaign's [work packets](../analysis/ability-mechanics/work-packets.md#contract-fixed-by-this-ledger)):

- **One entry per `(entity, effect_id, invoker)`** ([`TimedEffect`](../../crates/entity/src/cell_entity/stat_buff.rs)), holding every stat delta it applied (`AppliedStat`: requested and actually moved), its expiry or `None` while held, its effect flags and its ability's `moniker_ids`. An entry may move no stat (an icon and a duration only), for AB-09's crowd control.
- **Stacking is a parameter** (`TimedStacking`). Ability effects run the new [`TimedStat`](../../crates/cell-effect-scripts/src/cell/effects/stat_buff/mod.rs) script with `PerSource`: the same caster refreshes (the old entry comes off exactly, the new one gets a fresh expiry), another caster or another effect stacks, as the pulsing layer and Python's `addEffect` did. The stimpacks keep `StatBuff` and `ReplaceSameStat`, this decision's stat-keyed rule.
- **Any stat id.** Both scripts read one NVP per stat from `STAT_BUFF_NVPS` (the six attributes plus `Accuracy`, `Defense`, `CoverAccuracy`, `CoverDefense`, `CrouchingAccuracy`, `CrouchingDefense`, `Response`, `InterruptResistance`, the three resists, `MovementSpeedMod` and AB-05's two regen names). The rows come from the generator's `stat` family with D-AB09's units, every converted value commented on its row.
- **The API later packets call** (synchronous, so a script can): `SpaceManager::apply_timed_effect(target, TimedEffectSpec, now)`, `remove_timed_effects(target, StatBuffRemoval, pred)`, `remove_timed_effects_by_moniker` and `ability_moniker_ids`; `stat_buff::timed_spec(ctx, stacking)` builds a spec from an effect. The async seam is combat's `strip_timed_effects(entity, reason, pred, tx, mgr)`, which removes, logs and sends the clears and the restored stats; `clear_stat_buffs_on_death` is its `EF_ClearOnDeath` caller. `StatBuffRemoval` carries the contract's reasons: `Expired`, `Replaced`, `Removed`, `ToggledOff`, `RemovedByMoniker`, `Death` (logged `died`, as before), `Damage`, `Revive`, `BandolierSwap`, `Cleansed`.
- **Where entries come from.** The script runs where every landing effect's script runs: `fire_beneficial` (buffs, on the caster or an ally) and `damage_apply`'s after-hit scripts (debuffs, miss-gated by `plan_hit_effects`). Both send the entry's icon in the same resolution (`flush_stat_buff_timers`), `damage_apply` beside its pulsing registrations. Content's `apply_effect` does the same.
- **The wire is unchanged**: `onTimerUpdate(effect_id, TIMER_DURATION_EFFECT, invoker, effect_id, TotalTime, BigWorldTimeComplete)`, one per entry, so two casters' Aims are two timers with their own sources. A held entry sends no start timer (the client draws an icon only while `clock < BigWorldTimeComplete`; AB-08 owns the evidence), only the clear.
- **The effect bar.** The client shows at most ten icons per side (`Effect.lua`, audit B-73). The ledger never refuses an effect for it; an eleventh on one side logs `effect_bar_overflow` (INFO).
- **Held effects are refused for now.** A `pulse_duration = 0` effect would be a permanent stat until AB-08 wires `AF_TOGGLED`, so both scripts log `no_duration` and the generator does not bind stances, toggles or passives.

Monikers are the ability's (`resources.abilities.moniker_ids`): the seed has no effect-moniker column, and the ability lists share broad ids (1470900795 sits on most combat abilities), so AB-08 must pick which moniker means `EFFECT_Stance` before removing by it (audit B-74).

**Why:**

- **`pulse_count = 1` never registers.** `register_active_effect` computes `remaining = total_pulses - 1`, which is 0 for every stimpack row, and returns without registering, so the effect never gets an `active_effects` instance and never an `on_remove`. The same holds for the pet buffs of decision 25, which is why they have `PetState::buffs`. Raising `pulse_count` to 2 would make the pulse tick call `on_apply` a second time at expiry, reapplying the buff instead of removing it. The doc comments on `AbsorbShield` and `Stun` in `scripts.rs` still say a `pulse_count = 1` registration "ages out at expiry"; `register_active_effect` does not do that, and this decision does not rely on it.
- **Why a ledger apart from the pet one.** `PetBuff` is keyed by effect, carries toggles, lives on `PetState` and logs pet identities; a player's buff needs none of that and a different key. Unifying them would have changed pet behaviour for no gain, so this is a parallel, smaller type. It reuses the same idea (record the moved delta, take back exactly that) and the same log shape.
- **Why keyed by stat.** A Mark III Coordination stim (effect 3950, +5) and the Coordination half of a Mark V stim (effect 3956, +7) are different effects on the same stat. Keyed by effect they would stack to +12, which a tiered consumable line very likely did not intend. None of these rows had a script in 2009, so the data does not say; this is a server-side design decision.
- **Why the base consumes first.** A chain's `change_stat` then `remove_item` applies first and pays later: two clicks on the last slappack both pass the chain's gate before either removal commits, so the player got two heals for one unit, and an `ItemUsed` the outbox redelivered after a crash could heal again. With the row lock deciding, a unit that was not taken produces no effect. The answer is sent at most once for the same reason: an outbox replay could apply the effect twice for one unit, while a lost answer costs one unit's effect and logs an ERROR.
- **Why a native path at all.** 46 items carry a real heal or buff binding (22 heals, 24 stimpacks); a chain per item would duplicate the binding the seed already holds, and a chain condition that fails is silent, which breaks the rule that every press gets feedback.

**Security.** Decision 16's three properties hold for the second caller: `consumable_use` is a private module whose public face (`fire_item_use`, `apply_consumed_item`) takes an item design id, never an ability or effect id; the ability id comes from `items_event_sets` keyed by the design id the base read from the player's own inventory row (the client names only an inventory instance, and the base consumes that row under lock before the cell applies anything); the target is always the user.

**Consequences:**

- A stat buff is not persisted. Logout, gate travel, cross-world respawn and any other space change rebuild the `CellEntity`, whose stats come from the archetype again at `InitPlayerState` (which also clears any ledger it finds), so a buff is lost early but never leaks. The rows carry `EF_Offline_Time_Counts` (2): the 2009 design counted the hour down while offline, which would need persistence. A same-world death and respawn keep the entity and so keep the buff: no stimpack row carries `EF_ClearOnDeath`.
- Nothing derived needs recomputing. QR reads Coordination, Engagement and Perception live (`combat/damage/qr.rs`), the damage pipeline reads Fortitude and Intelligence live (`combat/damage/pipeline.rs`); Morale has no reader today. There is no cached derived-stat table.
- The heal scripts read a flat `HealAmount` before the older `HealPercentage` (`effects/heal.rs`), so one script per pool serves both the consumables and ability 597's 35%.
- Not wired: the Stealth, Energy and Disguise boosts (their stats have no server reader), the antidotes, and every 597-bound item. A use of one of those bag consumables (`container_sets` `{1,17}`, no chain) is refused with "This item has no effect yet." and nothing consumed; mission items and the 597 filler stay silent. See [consumable-via-onitemuse-pattern.md](../content/consumable-via-onitemuse-pattern.md#what-is-deliberately-not-wired).
- The same-world respawn's reanchor recreates the client's pawn; whether the client keeps a stim's duration icon through it is not verified. The server-side buff is unaffected.

**Reversibility:** High. The stacking rule is the `TimedStacking` the script passes to `CellEntity::apply_timed_effect`; persisting buffs would add a table and a replay at `InitPlayerState`. Moving a consumable back to a chain is authoring an `item_use` chain for it: the native path stands aside by rule.

**Code and tests:** [`consumable_use.rs`](../../crates/cell-content/src/cell/content/consumable_use.rs) with `consumable_use_tests.rs` (the gates, the refusal bytes, the restored-chain double-apply guard, the apply half) and `consumable_use_live_db_tests.rs` (the native set and every magnitude against the seed); [`consume_for_use.rs`](../../crates/base-methods/src/base/world_entry/methods/inventory/core/consume_for_use.rs) with its live-DB tests (one unit per request, one answer per unit, the design-id guard); `crates/services/src/consumable_round_trip_tests.rs` (the Health Slappack, a double-click on the last unit, a restored chain 4001, two stimpacks and the 597 gate end to end on real rows); `stat_buff_tests.rs` in `cimmeria-entity`, `effects/stat_buff/tests.rs` and `effects/heal.rs` in `cimmeria-cell-effect-scripts`, and `effects/stat_buffs/tests.rs` in `cimmeria-cell-combat` (the byte-exact duration timers, expiry, replacement and the death strip). AB-04: the per-source rules in `stat_buff_tests.rs` and `effects/stat_buff/tests.rs`, the ability timers byte for byte, two casters' timers, a held entry and the strip seam in `effects/stat_buffs/tests.rs`, the debuff hand-off and its miss gate in `damage_apply/timed_effect_tests.rs`, Aim and Combat Sprint through `fire_beneficial` in `use_ability/tests/timed_buffs.rs`, and the seeded rows in `effects/stat_buff/seed_live_db_tests.rs`.

### 29. A deployable is a player ground cast with a `deployables` row: the object is an owned `SGWBeing` that pulses as its owner (deployables Phase 0)

**Decision:** A player ability with a `resources.deployables` row (`SpaceManager::deployable_specs`) places a stationary object instead of hitting a target. It rides the cast of decision 21 with these diversions, in [`abilities/deployable/`](../../crates/cell-combat/src/cell/abilities/deployable/mod.rs):

- **Ground point.** `useAbilityOnGroundTarget` diverts to `handle_deploy_on_ground` before any AoE collection. The client's point must be finite, within the ability's `max_range` of the caster, in line of sight of the caster's eye where the world has an occluder (an `Unknown` answer allows, as decision 20 does), and on the navmesh where the world enforces containment. A point over the mesh is moved onto the floor. A refusal sends `onErrorCode` (42, 39 or 0) and a `CHAN_FEEDBACK` line before anything is charged, and so does a press during the cooldown or another warmup (99), which the ordinary launch refuses silently. The validated point is staged on `SpaceManager::deployables`, and the cast launches with target 0.
- **Launch.** `handle.rs` discards the client's target, as for a summon, so the #444 gate never sees the cast. A deployable launched with no staged point (a plain `useAbility`) is refused with feedback.
- **Warmup.** The ability's own. Every decision-21 interrupt applies, and `interrupt_pending_cast` drops the staged point.
- **Fire.** `fire::fire_cast` diverts to `fire_deploy` ahead of ammo and damage. It takes the staged point, re-checks the caster and the range, spawns the object (`SpaceManager::spawn_deployable`), plays `Ability_End` with TargetID = caster, and removes the owner's oldest object from that ability past `max_active`.
- **Pulse.** `deployable_tick` runs every AoI tick, after the pet sweep. The world-side verdict ([`cell-world/src/cell/deployables/teardown.rs`](../../crates/cell-world/src/cell/deployables/teardown.rs)) removes an object whose owner is gone, has another identity, is in another space or is dead, and one whose last pulse ran. Otherwise, each due pulse calls `apply_damage_to_target` with the **owner** as the attacker on every target of `combat::area_candidates(owner)` in the object's space and radius that `combat::may_hit_in_area` admits.

The object is an `SGWBeing` (class 0x01) with its owner's faction. Its lifetime is the lifetime effect's `pulse_count` x `pulse_duration`, and its radius is the pulse effect's `Radius` NVP, else the client's AE radius for its `tcm_param1` tier (`abilities::ae_radius_metres`: Medium = 1000 UE3 units = 10 m, `0x00d29e90`), not the server's cone tiers. The range check uses decision 27's metres (1012: 5 m).

**Why:**

- No 2009 row names the template or says which effect rides on which (1012: 5065 "Pulser, Despawn Target on Finish" and 5066 "Damage"), so the binding is seed data keyed on the ability, like `pet_summons` (decision 23).
- A being is in no AoE or cone candidate list and never gets a fight pass, so nothing can target or attack it without NPC-versus-NPC combat (#1009). A pet would bind into the owner's pet bar.
- Using the owner as the attacker gives the owner the threat, the kill XP (`resolve_death`) and the mission credit (`credit_ground_deaths`) with no new credit seam. It also applies the owner's own hostility rule, so players, pets and friendly NPCs are never hit.
- The pulse hands the damage pipeline the ability with **only** the pulse effect. `apply_damage_to_target` registers every pulsing effect of the ability it is given on its target, so the whole ability would put the 30-pulse lifetime effect on every mob hit.
- One per-tick verdict covers every owner path (logout, travel, respawn, space transfer, death), as `pet_owner_sweep` does for pets. It runs before the pulse, so a departed owner's object never pulses again.

**Consequences:** 5066 carries `EF_DontUseQR` and `EF_SequenceOnPulse`, and decision 10 still does not dispatch on flags: the pulse rolls QR and plays no per-pulse sequence. The pulse does not check line of sight from the object to its targets, as ground AoE does not. The engaged duel partner is hit, as by any of the owner's area abilities, and its damage clamps at 1 HP (decision 26).

**Reversibility:** High. The diversions are one call each in `dispatch/mod.rs`, `handle.rs`, `fire.rs`, `sequence.rs` and `warmup/interrupt.rs`, and one tick in the message loop.

**Code and tests:** [`abilities/deployable/`](../../crates/cell-combat/src/cell/abilities/deployable/mod.rs), [`cell-world/src/cell/deployables/`](../../crates/cell-world/src/cell/deployables/mod.rs), `spawner::load_deployables`. Tests are in `abilities/deployable/tests/` and `deployables/tests.rs`, and the seed guards in `spawner/tests/live_db_deployables.rs`; the revert proofs are in the [deployables ledger](../analysis/deployables/README.md#tests-and-revert-proofs). Gameplay: [deployables.md](../gameplay/deployables.md).

### 30. A player's cast honours `min_range`, and a `UseWeaponRange` ability reaches as far as the weapon (#1016, #1017)

**Decision:** Every targeted-cast range check resolves its bounds through [`crates/entity/src/abilities/range.rs`](../../crates/entity/src/abilities/range.rs): `ability_range_bounds` gives `RangeBounds { min, max }` in metres, and `RangeBounds::refusal(distance, enforce_min)` says whether a distance fails them (`TooFar` or `TooClose`). A player's cast closer than `min` is refused at launch (`handle_use_ability`) and at warmup fire (`warmup/tick.rs`); both go through [`use_ability/cast_range.rs`](../../crates/cell-combat/src/cell/abilities/use_ability/cast_range.rs), which answers the player with `onErrorCode(0, ability_id, 42)` (`CONDITION_FEEDBACK_OutsideWeaponRange`, the out-of-range answer) and logs one `abilities` DEBUG row `event=cast_refused` with `reason=target_too_close` (or `target_out_of_range`), `phase=launch` or `warmup_fire`, `account_id`, `player_id`, the distance and both bounds. The auto-cycle pre-gate skips such a target silently, as it does one out of range.

**Why:** The 2009 Python reference refuses `distance < minRange or distance > maxRange` with the one code (`AbilityManager.py:561`), and the client does not range-check targeted casts, so the server is the only gate. Before #1016 no player path read `min_range`: turret 1205 (3 m minimum) fired at point-blank range.

**Consequences:** NPC casters are not held to the minimum (`enforce_min` is the caster's `is_player`). The NPC fight tick owns that behaviour: `ability_ranges` reads `min_range`, a mobile NPC backs away, and a stationary one keeps firing. The out-of-range `onErrorCode` now goes to player casters only; for an NPC caster it had no client to reach. Tests: `use_ability/tests/min_range.rs` (refused at 1 m, allowed at 5 m, the log row), `warmup_interrupt.rs::target_inside_min_range_at_fire_interrupts_with_error_42`, `ticks/auto_cycle_range_tests.rs`, and `range.rs::min_range_refuses_a_player_inside_it`.

**`UseWeaponRange` (#1017).** `ability_range_bounds(def, weapon)` takes the equipped weapon's reach for an ability flagged `AF_USE_WEAPON_RANGE` (4): both bounds, from the ranged pair when the ability `is_ranged` and the melee pair otherwise. `caster_range_bounds(def, caster, &space_mgr.weapon_ranges)` finds the weapon (the design id in the caster's active bandolier slot) in `SpaceManager::weapon_ranges`, which `spawner::load_weapon_ranges` fills at startup from every `resources.items` row with a non-zero range. Those columns are metres and are not converted. Every targeted-cast range site goes through it: the launch, the warmup fire, the auto-cycle pre-gate, the ground-target primary check, the deployable ground point (launch and fire) and the pet-order pre-check (CM 88). A refusal's log row carries the bounds it used.

- **Evidence.** The client's range getters `FUN_00d29e00` / `FUN_00d29e30` return the weapon pair via `FUN_00d29da0` when `flags & 4`, chosen by `IsRanged` ([ability-resolution-pipeline.md § Range units](../reverse-engineering/findings/ability-resolution-pipeline.md#range-units-919-verified-2026-09-28)); python does the same (`AbilityManager.py:555`, `SGWPlayer.getWeaponRange`).
- **No weapon.** A flagged ability with no weapon equipped, or with a weapon that has no reach of the ability's kind (`max` 0), uses the ability's own range (its `max_range`, or 30 m for the `0` sentinel). Python has no deliberate answer: its player path raised on `getActiveItem().type`, and its mobs always had a template weapon. Here NPCs and pets carry no weapon item, and the flag is set on 579 seeded abilities, every NPC attack (592 included) and the pet attacks (1652) among them, so a refusal would disarm them all. A player's weapon abilities are already gated by the known-or-granted check before range.
- **NPC and pet AI.** Not changed. `ability_select::ability_ranges` / `effective_max_range` is the NPC's own reach policy (`NPC_MELEE_RANGE` for melee), not this choke point, and an NPC or pet has no weapon, so the flag would resolve to the fallback anyway. When an NPC can name its weapon (`entity_templates.weapon_item_id` has no consumer yet) both should read `caster_range_bounds`.
- **What players see.** The weapon's reach now decides: the SI 3 9mm Pistol (the starter sidearm) reaches 20 m instead of 30 m, rifles such as the SR1 .50-Cal and Nenz 24 reach 40 m, and a melee weapon attack (594 Strike, 595) reaches the weapon's 2 or 3 m. Most guns have a 2 m ranged minimum, so their ranged attack is refused inside 2 m.

Tests: `range.rs` (`use_weapon_range_takes_the_weapons_reach` and three more), `use_ability/tests/weapon_range.rs` (a 40 m weapon: in range at 35 m, refused at 45 m; the minimum; no weapon; the warmup fire), `ticks/auto_cycle_range_tests.rs::auto_cycle_tick_uses_the_weapons_reach_for_a_weapon_range_ability`, and the live-DB `spawner/tests/live_db_weapon_ranges.rs`.

### 31. Special ammo modifies the shot directly, from `resources.ammo_modifiers` (ammo campaign AM-04, D-AM07)

**Decision:** a player's weapon shot fired with a special ammo type loaded applies that type's `resources.ammo_modifiers` row on the server. There is no cast and no cooldown, and the toggle abilities (715 Hollow Point, 719 Armor Piercing, ...) are never launched; `toggle_ability_id` records only where the reconstructed numbers came from. The table loads at startup into `SpaceManager::ammo_catalog` (AM-F).

**Where it applies.** In the damage pipeline, not in an effect script: `damage_apply::apply_damage_to_target` asks `effects::ammo_damage::shot_ammo` for the shot's row once, after the QR roll. Effect scripts such as `RangedPhysicalDamage` run only for effects that set `script_name`, so a script wrapper would have modified a handful of abilities rather than every shot.

| Column | Effect on the shot |
|---|---|
| `damage_mult` | Multiplies the pre-armour damage, next to the cover scale (`cover_scale * damage_mult`), for both the HEALTH and the FOCUS component. |
| `penetration_mult` | Divides the armour mitigation `af * max(mitigation - penetration, 0) / 100` (`combat::calculate_damage_penetrating`). 2.0 lets half the armour stand, 0.5 twice as much. It scales the armour term, not the attacker's `PENETRATION` stat, because that stat is 0 on every player and a multiple of 0 would leave the column dead. |
| `damage_type` | Replaces the shot's damage type (today always `DT_PHYSICAL`) when set to a valid `EDamageType` ordinal. |
| `on_hit_effect_id` | On a hit (not `RC_MISS`), runs that effect on the target through the ordinary machinery: its `script_name` dispatches with the ability's scripts, after the damage, and a pulsing effect registers on the target like an ability effect. |
| `beneficial` | The ammo helps its target and never harms it (AM-11d, below). The shot targets allies and the shooter instead of hostiles, and skips the damage pipeline. Explicit in the seed, never inferred from the effect's script. |

**When a shot is modified.** All of: `ammo.finite_special` is on; the attacker is a player; the ability is a weapon shot (`required_ammo > 0`, the same test `use_ability` uses); the active bandolier slot's `cur_ammo_type` has a row. Otherwise the pipeline is unchanged. NPCs fire unmodified. The modifier and the reserve draw share one flag on purpose: with the flag off, special reloads are free and AM-F's widening offers Hollow Point on every Standard Pistol and SMG, so an ungated modifier would be free extra damage. The rows shipped dark behind the flag until AM-12 turned both halves on together; the flag is on by default since then (D-AM11, see the campaign summary below).

**Rows.** Hollow Point 1.25 damage / 0.5 penetration and Armor Piercing 0.9 / 2.0, both `DT_Physical`, in `db/resources/Abilities/Seed/ammo_modifiers_hp_ap.sql`. RECONSTRUCTION: the cooked text of 715, 719 and effect 747 gives only the directions ("Damage: Increased, Penetration: Decreased" and the reverse), no numbers.

**Known limits.** `MITIGATION` is capped at 0 in the default stat list, so the armour term, and with it `penetration_mult`, is 0 in live play until mitigation is populated; Armor Piercing is a plain 10% damage cut until then. The Focus-pierce bleed that `RangedPhysicalDamage` adds on top of the pipeline damage (effect 641, Pistol Auto Attack) is not scaled. Cover is not affected by penetration.

**Extending it (Wave-2 families).** A family adds rows in its own `ammo_modifiers_<family>.sql` (one `\ir` line after `Effects/Seed/effects.sql`), may seed its on-hit `effects` / `effect_nvps` rows in the same file from its reserved id block, and, only when no existing script fits, writes one `EffectScript` in its `cell/effects/ammo_<family>.rs` in `cimmeria-cell-effect-scripts` with one row in that crate's `EFFECT_SCRIPTS` table (decision 33). Nothing in the pipeline changes per family.

Telemetry: `ammo_damage_applied` (DEBUG, target `ammo`) per modified shot, and `ammo_on_hit_effect_missing` (WARN) when a row names an effect that is not loaded. Tests: `effects::ammo_damage::tests` (the resolution rules, and the live-DB seed guard `live_db_hp_ap_seed_rows`) and `damage_apply::ammo_tests` (the factors, default ammo unchanged, penetration against armour, the on-hit effect, the log row). Plan: [docs/analysis/ammo/work-packets.md](../analysis/ammo/work-packets.md).

**Family rows (Wave 2).** One row per family packet, appended as each lands.

| Family (packet) | Row | On-hit effect | Script | Stacking |
|---|---|---|---|---|
| Incendiary (AM-08, toggle 723) | 1.0 damage / 1.0 penetration, `DT_Energy` | 9110 Incendiary Burn: 4 pulses, 1 s apart, 15 Focus and 3 Health each; `EffectCategory` = `Burning` for AM-11c's cleanse | existing `RangedEnergyDamage`, no new script | decision 4: the same shooter refreshes, another shooter stacks |
| EMP dart (AM-11b, toggle 999) | 1.1 damage / 0.75 penetration, `DT_Physical` | 9150 Dart EMP Focus Drain: one shot, 50 Focus | existing `RangedEnergyDamage` (`FocusDamage` only), no new script | single shot, nothing registers |
| Radioactive dart (AM-11b; no toggle exists, 1227 Contagion cited) | 1.0 damage / 1.0 penetration, the ability's own type | 9151 Dart Radiation Dose: 5 pulses, 2 s apart, 3 Health each, the first on the hit | new `RadiationDamage` in `cell/effects/ammo_dart_tech.rs` | decision 4: the same shooter refreshes, another shooter stacks |
| Support darts (AM-11c: Stim 992, Antidote 1228/2874, Coagulant 3427, Adrenaline 1220; Nanites has no row, no evidence) | 0.0001 damage (the CHECK forbids 0; any shot under 5000 pre-armour damage rounds to 0) / 1.0 penetration, damage type kept | 9160 Stim +10% Focus; 9161 Antidote removes one each of Poison, Disease, Contagion, Wound, Burning; 9162 Coagulant removes one Wound; 9163 Adrenaline +10% Health (RECONSTRUCTION). All server-only: `onStatUpdate`, no per-effect client message, since 91xx ids are not in the client's cooked data | existing `HealFocus` / `HealHealth`; new `RemoveEffects` (`cell/effects/ammo_dart_support/`) | instant, nothing registers |
| EMP (AM-09, toggle 1445) | 1.1 damage / 0.75 penetration, `DT_Physical` | 9120 EMP Rounds Disruption: a living target loses 10 Focus; a mechanical one (its body set is in `MECHANICAL_BODY_SETS`: the Prisoner Retrieval Unit, the drones, the BattleWalker, deployables; never a player) loses 5 Health and no Focus, the split the EMP Grenade (2864, effects 4200/4202) makes. No disable. | new [`EmpDisrupt`](../../crates/cell-effect-scripts/src/cell/effects/ammo_emp.rs) | single-shot (`pulse_count` 1): nothing registers, nothing is sent to the client |
| Dart Poison (AM-11a, toggle 990) | 1.0 damage / 1.0 penetration, `DT_Physical` | 9140 Poison Dart Toxin: 4 Health on the hit and on each of 4 pulses 2 s apart; `EffectCategory` = `Poison` for AM-11c's cleanse | existing `Suppression`, no new script | decision 4: the same shooter refreshes, another shooter stacks |
| Dart Disease (AM-11a, toggle 991) | 1.0 / 1.0, `DT_Physical` | 9141 Disease Dart Infection: 2 Health on the hit and on each of 9 pulses 2 s apart; `EffectCategory` = `Disease` | existing `Suppression` | as Poison |
| Dart Tranquilizer (AM-11a, toggle 998 Disorient) | 1.0 / 1.0, `DT_Physical` | 9142 Tranquilizer Dart Sedation: `MOVEMENT_SPEED_MOD` -40 (60% speed) until the instance expires 6 s after the hit | new `MovementSlow` (`cell/effects/ammo_dart_cc.rs`), because `Stun` leaks its flag when it pulses and NPCs ignore `BSF_MOVEMENT_LOCK` (#1049) | one slow per effect per target: pulses and refreshes do not slow again, and a second shooter shares the slow until the last instance goes |
| Explosive (AM-10, toggle 1446) | 1.1 damage / 0.5 penetration, `DT_Physical` | 9130 Explosive Round Splash: `TCM_AERadius`, `tcm_param1` Short (5 m), `SplashDamageFraction` 0.5; every other hostile within 5 m of the target takes half the shot | none: the splash runs in the pipeline (`damage_apply::ammo_splash`), recognised by `ammo_explosive::splash_of` | not an active effect: one burst per hit, splash targets never splash |

**Effect categories for a cleanse (AM-11c).** The 2009 cleanses remove "1 Effect of Moniker EFFECT_Poison", but the `EFFECT_*` monikers were never seeded and `EffectDef` carries no name. An effect declares its category with one `effect_nvps` row named `EffectCategory` (`Poison`, `Disease`, `Contagion`, `Wound`, `Burning`; case-insensitive). `RemoveEffects` reads its own `RemoveCategories` NVP (comma-separated) and, per category, removes the oldest active instance on the target that carries it, running that effect's `on_remove`. An untagged effect is never cleansed, so content that wants a DoT cleansable tags it; AM-08 (`Burning`) and AM-11a (`Poison`, `Disease`) tag their on-hit DoTs. The removal sends no zero `onTimerUpdate` (the script is synchronous), so a removed effect's icon runs out its own countdown.

**Beneficial darts and targeting.** AM-11c shipped the support darts before any friendly-target path existed, so a Stim dart could only land on a hostile target and healed the enemy. AM-11d (decision below) replaces that.

**Support rounds target allies (AM-11d, owner decision 2026-09-28).** An `ammo_modifiers` row with `beneficial = true` (the four support darts) makes a player's weapon shot a *support shot* (`use_ability::support_shot`). Its target is classified once at launch, again at the warmup re-check, and again at fire:

- **Ally**: the shooter's own entity, or another player in the same space whom `combat::player_may_attack` does not admit. The launch admits it with the usual range, line-of-sight, ammo and cooldown checks. The fire (`fire_support`) runs only the ammo's on-hit effect on the ally and flushes the stat change to the ally and their witnesses. There is no QR roll, no damage, no `onEffectResults`, no threat, no `BSF_InCombat`, no duel or PvP state, the ability's own effects and the cone fan-out never run, and the shot never arms auto-cycle (an attack loop whose tick stops at any player it may not attack).
- **Hostile**: anything `player_may_attack` admits (a hostile NPC, an engaged duel opponent). The launch refuses it before the cooldown or the dart is charged, sends the `CHAN_FEEDBACK` line "Support rounds only affect allies.", and clears an armed auto-cycle loop. The fire refuses it again if a warmup or an ammo change let it through.
- **Anything else** (a vendor, a friendly NPC, an ally's pet): the #444 gate refuses it as before.

Ammo that is not beneficial is untouched: the #444 rule applies exactly as before, so a default or damaging dart at an ally is still refused. As a belt, `apply_damage_to_target` never runs a beneficial row's on-hit effect, so no other path (an AoE or cone secondary) can heal a hostile. The client emits `useAbility` with any target: the Lua (`Ability.lua`, `useAbility(id, Unit.Target)`) and the native emit chain (`0x00aa2910` -> `0x00ad78e0` -> `0x00d2afc0` -> `0x00d2ae40`, headless Ghidra 2026-09-28) carry no friend-or-foe check. Right-click on an ally sends `interact`, not a shot, so the ally path is the action-bar press; live confirmation is the first UAT step (AM-11d worknote). Telemetry on target `ammo`: `ammo_support_applied` (DEBUG, `decision_outcome = applied`, Health and Focus before and after) and `ammo_support_refused` (DEBUG, `decision_outcome = refused`, `stage` = `launch` \| `fire`, `reason` = `hostile_target` \| `target_gone` \| `not_an_ally`), both with the shooter's `account_id` / `player_id`, `target_entity_id`, `target_player_id`, `ammo_type` and `item_id`. Tests: `use_ability::tests::support_shot`, `damage_apply::ammo_support_tests`, the `cimmeria-cell-combat` integration test `ammo_dart_support`, and the live-DB seed guard `live_db_dart_support_seed_rows`.

**Explosive splash is a pipeline step, not a script (AM-10).** A script holds a synchronous `EffectContext` and cannot send a secondary's `onEffectResults` and `onStatUpdate`, resolve its death or give it threat, so an on-hit effect that is a splash (`TCM_AERadius` with a `SplashDamageFraction`) fans out after the target's own hit has resolved. The candidates come from the ground-AoE collector anchored at the target (decision 24's area rule: hostile non-pet NPCs and a duel partner, never the shooter, a friendly or another player), minus any the space's occluder says a wall hides from the blast (decision 20's occluder policy: no occluder or an off-grid ray never removes one). Each is applied through `apply_damage_to_target`'s body as `HitKind::Splash`: the damage is scaled by the fraction, and the target runs no on-hit effect (so no chaining), none of the ability's scripts and no pulsing effects. Splash kills join `last_aoe_deaths`, which the kill-credit wrapper now drains on every cast, not only when the primary died. This is the first fan-out of a radius effect on a single-target shot (the decision 8 caveat), scoped to ammo on-hit effects.

**Campaign summary (AM-12).** The campaign closed on 2026-09-28 ([ledger](../analysis/ammo/README.md)). Since AM-12, `ammo.finite_special` is **on by default** (D-AM11). The flag gates, together, the reserve draw and switch return (AM-02), every `ammo_modifiers` row on a shot (this decision and the family rows above), and the support-shot ally path (AM-11d). `CIMMERIA_AMMO_FINITE_SPECIAL=0` on the server is the rollback lever: shots fire unmodified and reloads refill for free. It does not withdraw the pushed ammo item definitions (9000-9014), the loot rows or the GM commands. The known limits, one line each:

- **Penetration is inert.** `MITIGATION` is capped at 0, so `penetration_mult` changes nothing in live play; Armor Piercing is a plain 10% damage cut until mitigation is populated.
- **EMP has no interrupt.** EMP rounds and EMP darts drain Focus (or a machine's Health) only; nothing cancels a cast, a warmup or a channel, and nothing disorients a machine.
- **Support darts reach players and the shooter only.** A friendly NPC or an ally's pet is still refused by #444.
- **No floating heal numbers.** A support shot sends no `onEffectResults`, so the heal shows only as the ally's bars moving.
- **Nanites has no effect.** There is no evidence for it and no `ammo_modifiers` row, so a Nanites dart fires as a plain dart.
- **Unknown effect ids reach clients.** The pulsing on-hit effects 9110 (Incendiary), 9140-9142 (Poison, Disease, Tranquilizer) and 9151 (Radioactive) register an active effect, so `onTimerUpdate` carries an id the client's cooked data does not have, to a player target (an NPC target's timer is no longer sent, decision 22). What the client does with it is unverified; an unknown cooked id has crashed it before (#938). The UAT's second risk check covers it.

### 32. NPC-vs-NPC: an NPC's area ability hits the NPCs it would target, and an NPC-only kill pays nobody (#1009)

**Decision:** An NPC may fight another NPC when its faction reaction toward it is HOSTILE, the 44 x 44 `FACTION_REACTION_TABLE` read with the NPC as the viewer (`combat::npc_may_target_npc` in [`aggression.rs`](../../crates/cell-world/src/cell/combat/aggression.rs)). Two effect-side consequences:

- **Area targeting.** `combat::may_hit_in_area` for a non-pet NPC caster is now `npc_may_target_npc(caster, candidate)`. Before, it was "any hostile-faction (10) NPC", so a NID guard's area ability landed on its own post and never on the friendlies it fought. A pet caster keeps its owner's rule, `player_may_attack_pve`. Players are still never candidates of an NPC's area ability (`area_candidates` never lists them); NPC-versus-player targeting is unchanged. The single-target gate needed no change: it only ever checked player casters.
- **Death credit.** Credit stays with the killing blow. A kill whose killer is a live NPC that is not a pet (`abilities::death::npc_only_kill`) rolls no loot (the corpse gets no loot cursor), pays no XP (`grant_kill_xp` finds no player to credit, as before) and fires no `EntityDeath` (every mission credit path resolves the killer through `credited_player`, which is `None` for a plain NPC, as before). The skipped roll logs `loot.drop event=skipped reason=npc_only_kill` at INFO with both entities and factions. A pet's kill stays its owner's (decision 23 and pets PT-06), and a killer that is no longer in the world keeps the old behaviour.

**Why:** A friendly NPC that kills a guard must not leave a lootable corpse for whoever walks by, or advance anyone's kill objective. Loot was the only credit path that did not already refuse a plain NPC.

**Consequences:** A player who wounds a guard that a friendly NPC then finishes gets no kill credit, the same as a guard another player finishes. The friendly standoff NPCs deal the default ability's damage, so this is a real, bounded cost; per-contributor credit is an owner decision recorded in the castle-population ledger (K20). The threat, AI and targeting side is in [npc-ai.md, NPC-vs-NPC combat](../gameplay/npc-ai.md#npc-vs-npc-combat-1009).

**Code and tests:** `aggression.rs` (unit tests `npc_viewer_reads_its_own_row_of_the_table`, `npc_area_ability_hits_hostile_npcs_not_its_own_side`), `abilities/death/npc_only_kill.rs` and `death/npc_only_kill_tests.rs` (no loot and no XP for an NPC kill, a player kill of the same mob still pays), and `service::tests::npc_ai::npc_vs_npc::an_npc_only_kill_through_the_tick_pays_nothing` (no `EntityDeath` through the real fight tick).

### 33. The scripts live in a leaf crate and register with the cell at startup (#962 step 4)

**Decision:** Every `EffectScript` implementation is in `cimmeria-cell-effect-scripts`, a leaf only the composition root (`cimmeria-services`) and test code depend on. `cimmeria-cell-world` keeps the trait, `EffectContext`, `dispatch_by_name` / `dispatch_on_remove` and the registry type, `EffectScripts`. At startup the root builds an `EffectScripts` from the leaf's `EFFECT_SCRIPTS` table (`plugins::effect_scripts`), the orchestrator hands it to the `CellService`, and `CellService::start` installs it on its `SpaceManager` beside the plugin table, before anything spawns (the spawn-time cover hold runs Cover Stance). Dispatch reads the registry off `ctx.space_mgr`.

- **Lookup is by name.** A `HashMap` keyed by the exact, case-sensitive `script_name`; table order reaches only `EffectScripts::names` (logs, tests).
- **A duplicate or empty name fails the build** (`EffectScriptError::DuplicateScript`, `EmptyName`); the orchestrator then keeps an empty registry and refuses to start the cell (`effect_scripts_empty`), as it refuses an incomplete plugin table.
- **A missing registration warns.** Once the effect definitions load, the cell logs one `effect_script_unregistered` row (WARN, `reason = no_registered_script`, `count`, and each unanswered `script_name` with its effect ids); dispatch still logs `effect_script_unknown` per call and falls back to the legacy NVP path, as before. The shipped seed has two such rows, effect 658 (`Reload`, the Reload ability, which the reload pipeline handles) and effect 2907 (a `test` row with an empty name); no script ever answered either, and the live-DB guard `every_seeded_script_name_is_registered` pins exactly those two. A bare `CellService` with no registry logs `effect_scripts_empty` at start.
- **What stayed in `cimmeria-cell-world`, and why:** the pieces the layers below the leaf call directly. The passive pass (`effects/passives.rs`, called by `cimmeria-cell`'s base-message handlers), the pet-script name predicates `acts_on_owner_pet` / `is_passive_script` (the owner-pet cast redirect in combat and the passive pass), the timed effect ledger (`SpaceManager::apply_timed_effect` / `remove_timed_effects`, `StatBuffRemoval`: combat's stat-buff tick calls it, and an inherent `impl SpaceManager` must stay in the crate that defines `SpaceManager`), and the special-ammo shot helpers the damage path reads (`effects/ammo_damage.rs`, `effects/ammo_explosive.rs`). No script needed a value-returning combat hook to move: every one is a synchronous `SpaceManager` mutator.

**Why:** Adding or editing a script used to rebuild the cell track from `cimmeria-cell-world` up (14 crates in the server build); now it rebuilds the leaf, the facade and the binaries ([plugin-architecture.md §4.4](plugin-architecture.md#44-step-4-effect-scripts)). The seam needs no hook: dispatch already had `&mut SpaceManager` in hand, so the registry sits on it like the plugin table.

**Consequences:** A test that builds its own `SpaceManager` and dispatches a script installs the registry first (`cimmeria_cell_effect_scripts::cell::effects::registry::install`, re-exported as `test_support::install_effect_scripts` in combat, content and `cimmeria-cell`); a test in `cimmeria-cell-world` cannot (the leaf depends on it), so the script tests live in the leaf. A bare manager has no scripts, and every dispatch through it misses with the `effect_script_unknown` WARN.

**Adding a script:** an `impl EffectScript` in the module for its family in `cimmeria-cell-effect-scripts`, one `("Name", &Type)` row in `EFFECT_SCRIPTS`, and the effect row's `script_name` in the seed. The live-DB guard `every_seeded_script_name_is_registered` fails when a seeded name has no row.

**Code:** [`crates/cell-world/src/cell/effects/registry.rs`](../../crates/cell-world/src/cell/effects/registry.rs) (the type, the checks), [`crates/cell-effect-scripts/src/cell/effects/registry.rs`](../../crates/cell-effect-scripts/src/cell/effects/registry.rs) (the table), [`crates/services/src/plugins.rs`](../../crates/services/src/plugins.rs) (the build), `CellService::start` in [`crates/cell/src/cell/service/startup.rs`](../../crates/cell/src/cell/service/startup.rs) (install and the startup logs).

### 34. A beneficial cast lands on the caster or an ally, never on a hostile (ability mechanics AB-01)

**Decision:** A player's ability is beneficial when `cimmeria_entity::abilities::ability_is_beneficial` says so: at least one effect does something, and every effect that does something carries `EF_Beneficial_Effect` (`EF_BENEFICIAL_EFFECT` = 1) or, on an `ABILITY_TYPE_Heal` ability (`AbilityDef::type_id`, now loaded from `resources.abilities.type_id`), runs a heal script (`HEAL_SCRIPTS`: `HealHealth`, `HealFocus`, `HealPetHealth`). An effect with positive `HealthDamage` or `FocusDamage` vetoes it. The Heal type alone is not enough: about 200 seed abilities typed Heal are debuffs or crowd control (1874, 1937, 1988, 2090, 2154, 3253). `use_ability/beneficial.rs::resolve_cast_target` is the one place the target rule lives, and the launch (`handle.rs`), the fire (`fire.rs`) and the warmup's fire-time re-check (`warmup/tick.rs`) all call it:

- a `TargetSelf` ability resolves to `CastTarget::Caster`, whatever target id the client sent (D-AB01);
- a `TargetTarget` ability resolves to `CastTarget::Ally(id)` when the client named the caster or a live player in the same space that the caster may not attack (`support_shot::classify`, the support-dart rule);
- anything else takes the D-AB02 branch, `no_ally`: `CastTarget::Caster` under the proposed default, or `CastTarget::None` and a feedback line under the refusal. The const `FALLBACK_TO_CASTER` picks between them, and nothing else differs.

The launch rewrites the cast's target to the resolved entity, so range, line of sight, the warmup anchor and `Ability_Begin` name where it lands, and keeps the client's target (`PendingCast::wire_target_id`). The warmup re-check and the fire re-resolve from that original target, so a `fallback_to_caster` keeps its reason, and the fire resolves before `Ability_End`, so the animation and the effects name the same entity. The launch's `beneficial_cast` row is DEBUG, because it runs before the dead, known and cooldown checks; the fire's is INFO. `fire_beneficial` owns every effect of a beneficial cast, so `damage_apply` and its QR and miss gates never see one. It runs each effect script on that entity (an `EF_ResolveOnAbilityUser` effect, `EF_RESOLVE_ON_ABILITY_USER` = 131072, on the caster instead) and flushes the stats to it and its witnesses, sends `StatBuff` timers and registers pulsing effects with the caster as invoker. It does no QR roll, sends no `onEffectResults`, generates no threat, sets no in-combat state and cancels no channel. The #444 gate (`player_may_attack`) moved, unchanged, into `beneficial::target_gate` beside its two reversals; a beneficial cast aimed at a non-ally after resolution is refused there with a WARN (`reason = beneficial_non_ally_target`), which no client input can reach. Only player casts are resolved: an NPC's cast and every non-beneficial cast keep the wire target, the #444 gate and the damage pipeline.

**Why:** The client sends its current target for every non-ground ability, Self ones included ([audit B-10](../analysis/ability-mechanics/audit.md)). Taking that id at its word meant 597 Heal Focus did nothing with no target, was refused by #444 on the caster or an ally, and healed the mob when one was selected (B-12 to B-14). Python resolved Self abilities on the caster (`AbilityManager.py:527`). The fallback matches the tooltips ("Heals 10% of the player's Health pool") and gives every press a visible result. The damage veto exists because the seed has a Heal-typed attack (2228, effect 3091 `RangedPhysicalDamage`); without it that attack would skip #444 and land on an ally.

**Consequences:** A beneficial cast never arms auto-cycle. A Heal-typed ability whose effects do nothing yet (742 Field Medic I, 869 Morale Boost) is not beneficial and keeps the #444 path until a heal script is bound to its effect (AB-02); AB-12 owes such presses feedback. Healing a friendly NPC or a pet stays refused, as for support darts. There is no floating heal number (AB-11). D-AB02 is still open with the owner; flipping it changes `FALLBACK_TO_CASTER` and the tests that pin the fallback.

**Code and tests:** [`crates/entity/src/abilities/beneficial.rs`](../../crates/entity/src/abilities/beneficial.rs) (the classification and its unit tests), [`crates/cell-combat/src/cell/abilities/use_ability/beneficial.rs`](../../crates/cell-combat/src/cell/abilities/use_ability/beneficial.rs) (the resolver, the gate, the fire). `use_ability/tests/beneficial.rs`: each resolver arm; 597 at no target, the caster, an ally and a hostile restores only the caster's Focus (on revert the mob's Focus rises); 1646 heals an ally and falls back at a hostile; a damaging ability and the Heal-typed attack at an ally are still refused; the warmed-up heal lands at fire; Recuperation registers its pulses; the caster's `onStatUpdate` byte for byte; no #444 WARN for a self heal. `beneficial_live_db.rs` loads the real seed: 597, 1646 and 1218 are beneficial, 592, 559 and 2228 are not.
