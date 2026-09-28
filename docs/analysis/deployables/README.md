# Deployables campaign ledger

> **Last updated**: 2026-09-28
> **Status**: Phase 0 implemented (1012 Deployable: Microwave Emitter); owner in-client UAT pending. Phases 1-2 not started.
> **System doc**: [docs/gameplay/deployables.md](../../gameplay/deployables.md). **ADR**: decision 28 of [abilities-and-effects-system.md](../../architecture/abilities-and-effects-system.md).

This ledger tracks restoring the Scientist tree's "Deployable:" abilities: stationary objects a player places that pulse an effect for a fixed lifetime.

## Evidence

The cooked client data in the repo (`data/cache/CookedData*.pak`, zips of one SOAP-XML file per row) is the source. The seed transcribes it field for field.

| Finding | Evidence |
|---|---|
| The "Kit:" items are crafting components, not deployables | `CookedDataItems.pak` `_2054` Kit: Turret, `_2071` Kit: Medical Station, `_2076` Kit: Mobile Turret: `IsElementaryComponent="true"`, no ability link in the schema. Out of scope. |
| 1012 places an object that pulses and then goes | Effect 5065 "Pulser": "Single Target / 30 pulses x1 Second duration / Despawn Target on Finish" (`PulseCount=30`, `PulseDuration=1`). Effect 5066 "Damage": "Medium Radius AE / Secondary -100F" (`Target_Collection_Method=2`, `TCM_Param1="Medium"`, `Flags=144`). |
| 1236 Aggression Inducer has the same shape | Effect 3176 "Pulser", same wording and 30 x 1 s; 3175 "Threat Generator" rides on it. |
| `DeploymentBar` (ability flag 2) is a bar category, not a spawn marker | 105 abilities carry it, grenades and mines included (`abilities.sql`). |
| The deployable body exists in the client | `CookedPC/Packages/Character/WP-Human.upk` exports BodySet `BS_DeployableLow`, SkeletalMesh `DP-Base100`, BodyComponent `Dp_Standard100` and the `DP_IdleAnim` anim set (read with `tools/upk_parser.py`). Not rendered yet. |
| Units of `max_range` | UE3 units, 100 per metre (#919, decision 27 of the ADR): 1012's 500 is 5 m. |
| The AE radius of a tier | The client's own table (`AbilityInfo_AERadiusFromTier`, `0x00d29e90`, address map): Melee 250, Short 500, Medium 1000, Long 1500, Extreme 2000 UE3 units. 5066's "Medium" is 10 m. The server's cone table (`tcm_range_meters`, Medium = 8 m) is a server-side guess and is not used for AE radii here. |

## Phase 0 decisions

| Id | Decision | Why |
|---|---|---|
| D-DP01 | The ability-to-object binding is a seed table, `resources.deployables` (ability, template, lifetime effect, pulse effect, `max_active`). | No original data names the template or says which effect rides on which. Same pattern as `pet_summons`. |
| D-DP02 | The object is an `SGWBeing` with its owner's faction, never an `SGWPet`. | A being is in no AoE or cone candidate list and never fights, so nothing targets it and #1009 is not a dependency. A pet would bind into the owner's pet bar. |
| D-DP03 | One object per owner per ability (`max_active` 1); a re-cast replaces the oldest. | No evidence either way. With a 30 s cooldown equal to the 30 s lifetime, this only matters after a GM cooldown reset. |
| D-DP04 | The object goes when its owner dies, logs out, is destroyed on any path, or changes space. One per-tick verdict covers every path. | Same rule as pets (D-PT01, D-PT08). The verdict runs before the pulse, so a departed owner's object never pulses again. |
| D-DP05 | Pulse damage is the owner's: `apply_damage_to_target` with the owner as the attacker, over the owner's area-hostility rule (`may_hit_in_area`). | The owner gets threat, kill XP and mission credit as if they had hit; players, pets and friendly NPCs are never hit. An engaged duel partner is hit, as by any of the owner's area abilities (damage clamps at 1 HP, SS-D3). |
| D-DP06 | First pulse one interval after placement; 30 pulses; removed in the tick of the 30th. | The lifetime is exactly `pulse_count x pulse_duration`. The object exists on the client before it first hits. |
| D-DP07 | 5066 gets `FocusDamage` 100 (NVP 380) and the `RangedPhysicalDamage` script. | "-100F" is 100 Focus. The Focus-first script is what the other "-F -H" damage effects use (641); without a script the emitter would only drain Focus and never hurt anything. Open question Q-DP2. |
| D-DP08 | The ground point is validated before anything is charged: finite, within `max_range`, line of sight where an occluder exists, on the navmesh where the world enforces containment (snapped to the floor where a mesh covers it). | Server authority over a client-supplied point. An `Unknown` line-of-sight answer and an advisory or meshless world allow, like every other gate. |
| D-DP09 | A press during the cooldown or another warmup gets an answer (99 and a chat line). | The project's first-press rule. The ordinary launch refuses both silently. |
| D-DP10 | Templates 400-409 are the deployable block; the template sequence floor is raised to 409. | 350-399 are taken by pets, bank and social. |
| D-DP11 | The hit radius of a tier is the client's AE radius (`ae_radius_metres`: Medium = 10 m), not the server's cone tier (8 m). | The client converts AE radii with its own table (`0x00d29e90`); the cone tiers are a server-side guess. Ground AoE (`dispatch`) still uses a flat 5 m default when no `Radius` NVP exists: a separate follow-up. |

## Phase 0 status

| Piece | Status |
|---|---|
| `resources.deployables` table, 1012 row, template 400, NVP 380, 5066 script | Done |
| Ground-point launch, warmup, fire, one-per-owner replace | Done |
| Pulse tick, owner-attributed damage, kill credit | Done |
| Lifetime and owner-lifecycle cleanup | Done |
| Telemetry (`deployables.lifecycle`, `deployables.pulse`) | Done |
| In-client look and feel | UAT pending (DP-U1 to DP-U8) |

### Tests and revert proofs

Every guard below was run against a mutation of the code or seed it guards (a scratch driver applied one mutation, ran the named tests, and restored the file; the seed mutations ran through `tools/build-lane/live-db-test.sh`, which reloads the worktree database). Each of the 25 mutations failed its guard; the range, line-of-sight, navmesh and radius ones were re-run after the rebase onto #919's metre ranges.

| Guard | Test | Mutation that fails it |
|---|---|---|
| Hostile-only targeting | `pulse::a_pulse_targets_live_hostile_npcs_in_its_radius_only` | drop the `may_hit_in_area` filter |
| Radius edge (10 m inclusive) | same | `<=` to `<` |
| The client's AE radius (10 m), not the cone tier (8 m) | `deployables::tests::radius_and_schedule_come_from_the_effects`, `deployables::tests::spawn_places_a_stationary_being_owned_by_the_caster` | use `tcm_range_meters` for the radius |
| Damage is the owner's | `pulse::a_pulse_damages_as_the_owner`, `pulse::a_pulse_kill_is_the_owners_kill` | pass the object as the attacker |
| The lifetime effect never lands on a target | `pulse::a_pulse_registers_no_lifetime_effect_on_its_target` | hand the pipeline the whole ability |
| Owner death | `pulse::the_owners_death_removes_the_object_before_it_pulses` | drop the `OwnerDead` verdict |
| Owner logout, and an owner id reused by another player | `deployables::tests::verdict_ends_the_object_on_owner_death_logout_and_zone_change`, `deployables::tests::verdict_refuses_a_player_who_reused_the_owner_id` (`pulse::the_owners_logout_removes_the_object` is the end-to-end cover; alone it does not catch this mutation, because a departed owner also fails the space check) | drop the `OwnerGone` verdict |
| 30 pulses, then gone | `pulse::thirty_pulses_then_the_object_is_removed` | skip the despawn after the last pulse |
| Range | `launch::an_out_of_range_point_is_refused_with_feedback_and_charges_nothing` | drop the range check |
| Line of sight | `launch::a_point_behind_a_wall_is_refused_for_line_of_sight` | ignore a blocked ray |
| Navmesh containment | `launch::an_off_mesh_point_is_refused_and_an_on_mesh_point_is_grounded` | accept an off-mesh point |
| No ground point, no cast | `launch::a_plain_use_ability_on_a_deployable_is_refused` | skip the unstaged refusal |
| Re-cast replaces | `launch::a_recast_replaces_the_owners_object` | never remove the excess |
| An interrupt drops the point | `launch::an_interrupted_warmup_places_nothing_and_forgets_the_point` | keep the point on interrupt |
| Cooldown feedback | `launch::a_press_during_the_warmup_or_the_cooldown_gets_an_answer` | drop the cooldown refusal |
| Refusals log at DEBUG with `reason` | `logs::refusals_log_their_reason_at_debug` | log them at WARN |
| Observers see it arrive and leave | `deployables::tests::observers_meet_the_object_with_its_body_and_see_it_leave` | bare `destroy_entity` instead of `despawn_npc` |
| A departing owner gets no `LeftAoI` | `deployables::tests::a_departing_owner_is_spared_the_leave_other_witnesses_are_not` | drop the spare |
| A being, in the owner's faction | `deployables::tests::spawn_places_a_stationary_being_owned_by_the_caster` | spawn as `mob`; keep the template faction |
| SigNoz export | `logging::deployables_target_tests` | drop the `deployables=debug` row |
| Seed | `live_db_deployables` (four guards) | drop NVP 380; drop the 5066 script; seed template 400 as a `mob`; point the 1012 row's lifetime at 5066 |

## Remaining phases

| Phase | Ability | What it needs | Blocked on |
|---|---|---|---|
| 1 | 1236 Deployable: Aggression Inducer (3176 Pulser + 3175 Threat Generator) | A `deployables` row, a template, and a threat script that makes mobs in radius attack the object or the owner. If mobs must attack the object, it needs NPC-versus-NPC combat. | Design of the threat target; possibly #1009 |
| 1 | 1224 Deployable: Med Station (capstone, Medical) | No effects in the cooked data: the heal, radius and lifetime are greenfield. Needs an ally-targeting pulse (the owner's group), which no pulse does today. | Owner decision on the numbers |
| 1 | 1253 Pulse Sensor (Commando) | Description: "Pulses -25SR per 5 seconds / 10m radius / Duration 60sec", but no effects. Needs a stealth-reveal effect the server does not have. | Stealth system |
| - | 1014 Detection, 1015 Gravity Well, 1223 Focus Regen | Unresolved: 1014 and 1223 have no effects; 1015 reads as a direct AE debuff, not an object. Re-check each before building. | Evidence |
| 2 | Turrets (Summon Turret family, pet PT-12) | NPC-versus-NPC combat, and a turret body choice. The `Dp_Offensive` / `Dp_Defensive` bodies in `WP-Human.upk` are a lead for PT-12. | #1009 |

Known Phase 0 limits: `EF_DontUseQR` (5066) and `EF_SequenceOnPulse` are not honoured (no effect flag drives dispatch yet), there is no per-pulse visual, and a pulse does not check line of sight from the object to its targets (ground AoE does not either).

## Open owner questions

| Id | Question | Default adopted |
|---|---|---|
| Q-DP1 | Is the `WP-Human.BS_DeployableLow` + `Dp_Standard100` look right for the Microwave Emitter, or should it wear another `Dp_*` component (or `BS_DeployableHigh`)? | `Dp_Standard100`, pending the UAT look (DP-U2) |
| Q-DP2 | 5066's damage model: Focus-first with a Health bleed (`RangedPhysicalDamage`, adopted), parallel Focus and Health (`RangedEnergyDamage`), or Focus only (no script)? | `RangedPhysicalDamage` |
| Q-DP3 | One active per owner per ability, or several? | One (D-DP03) |
| Q-DP4 | Should the object outlive its owner's death (as a placed trap would)? | No (D-DP04) |
| Q-DP5 | Resolved by #919 (merged during this packet): `max_range` is UE3 units, so 1012 reaches 5 m. | 5 m |
| Q-DP6 | Should hostile mobs be able to attack the object (turret-like) in Phase 1? That makes #1009 a dependency. | Not attackable |
| Q-DP7 | The "Kit:" items are crafting components. Confirm they stay out of scope. | Out of scope |

## UAT (owner, colo after the release)

**Prerequisites:** a GM character. `.giveability 1012`, and a few hostile mobs (Castle Cellblock guards, or `.spawn`). A second player for DP-U7.

| # | Do | Expect |
|---|---|---|
| DP-U1 | Target the ground about 4 m away and cast 1012 | The cooldown and a 2 s warmup show at once. After 2 s an object appears at the point. |
| DP-U2 | Look at the object | A small deployable model (`DP-Base100` with `Dp_Standard100`) standing on the ground, not floating or sunk, named "Deployable: Microwave Emitter". Say what it looks like (Q-DP1). |
| DP-U3 | Let hostile mobs stand within 10 m of it | Every second their Focus drops, then their Health; they turn on you, even from 15 m away. A kill gives you XP and quest credit. |
| DP-U4 | Watch for 30 s | The object disappears 30 s after it appeared. |
| DP-U5 | Cast at a spot beyond range, behind a wall, and during the cooldown | Each press shows a chat line ("That spot is out of range.", "You cannot see that spot.", "That deployable is not ready yet.") and charges nothing. |
| DP-U6 | Place one, then die; place one, then log out; place one, then change zone | The object disappears each time. |
| DP-U7 | A second player stands inside the radius | They take no damage. They can see the object appear and disappear. |
| DP-U8 | Try to attack the object (click it, cast at it) | It cannot be attacked. |

**SigNoz:** `scope_name LIKE 'deployables.%' AND account_id = <N>`; `event = 'despawned'` rows carry the reason and the totals; `event = 'deploy_refused'` rows carry the reason.
