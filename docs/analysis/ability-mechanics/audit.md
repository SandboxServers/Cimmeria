# Ability Mechanics: Evidence Audit Against `main`

> Type: reference. Audience: the coordinator and packet workers.
> Updated: 2026-10-03, against `main` @ `b40ef76d2`. Companions: [campaign README](README.md), [work packets](work-packets.md), [ability-trees audit](../ability-trees/audit.md), [combat formulas status](../../reverse-engineering/findings/combat-formulas-status.md), [abilities ADR](../../architecture/abilities-and-effects-system.md).

Every row was checked against the code, the seed, the client files or SigNoz on the date above. Row IDs (`B-nn`) are cited by the work packets. Line numbers drift; re-check them before editing.

Evidence tags follow [combat-formulas-status.md](../../reverse-engineering/findings/combat-formulas-status.md#evidence-classes-used-here): **ORIGINAL-DATA** (shipped declarations: `alias.xml`, `enumerations.xml`, cooked rows), **DESIGNER-TEXT** (an `effect_desc` or ability description), **FAN-GUESS** (`deprecated/python/`, never canonical), **CODE** (the Rust on `main`), **TELEMETRY** (colo SigNoz). The seed in `db/resources/` is Project Giza's client-derived approximation, not 2009 server data; anything this campaign adds to it is **RECONSTRUCTION** and must say so.

## 1. The player-reachable ability set

Method: the 423 distinct ability ids in `resources.archetype_ability_tree` (439 nodes, 419 ids) and `resources.char_creation_abilities` (5 starter ids), joined to `abilities`, `effects` and `effect_nvps` from the seed SQL, then classified by keyword over the effect and ability text plus `type_id`, `target_type_id`, `passive_yn` and `flags`. The classifier is a heuristic: family counts are good to a few rows either way, and every per-ability claim below was checked by hand.

| ID | Finding | Evidence |
|---|---|---|
| B-01 | **The five starter abilities** every character gets: 592 Pistol Shot, 594 Strike, 597 Heal Focus (Self), 1646 Health Heal (Target), 1218 Medical Attention: Recuperation (Target, 25 x 1 s HoT). Each is on all 23 `char_def_id`s. | `db/resources/Archetypes/Seed/char_creation_abilities.sql` |
| B-02 | **Only 15 of 423 reachable abilities carry anything the server can resolve** (a `HealthDamage`/`FocusDamage` NVP or a registered `script_name`): 592, 594, 597, 1646, 1218, 720, 1012, 1451, 1650, 2824, 2839, 2852, 967, 968, 1207. Another 102 play only an animation (`event_set_id` set, no mechanic); 306 have neither. | seed join; `crates/entity/src/abilities/implemented.rs:22-37` is the same predicate |
| B-03 | **`effect_nvps` holds 110 rows, almost all for consumables, ammo and pets.** No ability-tree node except the starters has a numeric NVP. 229 effects of reachable abilities state their damage in designer text (`-200F / -20H`) and have no NVP; 22 state a heal (`+10% Health`, `Heals 35% of target's Focus pool`); 39 state a stat change (`+200 Accuracy: 15 Seconds`); 110 name a CC (Snare, Knockdown, Suppression, Interrupt); 72 are resist rolls; 54 remove effects or stances; 45 are VFX-only. | `db/resources/Effects/Seed/effect_nvps.sql`; DESIGNER-TEXT counts over 809 effect rows |
| B-04 | **No ability costs Focus, and no Focus cost exists anywhere.** `resources.abilities` has no cost column; the client UI Lua prints a cost only for trainer purchases, vendors and minigames (`Trainer.lua:84`, `Vendor.lua`, `StartMinigame.lua`); python never deducted one. 0 reachable descriptions mention a Focus cost. 27 Asgard abilities say "Cost: X Gigajoules" or "5 Energy": a placeholder with no number, against an `ENERGY` pool that players do not have (`regen.rs:23-27`). | `db/resources/Abilities/Tables/abilities.sql`; client `SGWGame/Content/UI/**/*.lua`; `deprecated/python/cell/AbilityManager.py` |
| B-05 | **Focus is SGW's damage shield, not a mana pool.** Damage is authored as a Focus/Health pair at 10:1, and `RangedPhysicalDamage` drains Focus first and bleeds to Health only on overflow. "Focus regen" is therefore shield recharge, and its rate decides how long a player survives a fight. | ORIGINAL-DATA, [combat-formulas-status.md §5](../../reverse-engineering/findings/combat-formulas-status.md#5-focus); `crates/cell-effect-scripts/src/cell/effects/scripts.rs:423-520` |

## 2. Targeting: why the starter heals do nothing

| ID | Finding | Evidence |
|---|---|---|
| B-10 | **The client sends the current target for every non-ground ability, including Self ones.** `Ability.lua:181` calls `useAbility(id, Unit.Target)`; the action bar calls native `useAction`, whose emit (`0x00d2ae40`) branches only on Ground (type 3) and otherwise sends `AbilityID` + `TargetID` with no friend-or-foe or self substitution. So a Self ability arrives with target 0 (nothing selected), the caster, an ally, or a hostile. | client `UI/Core/Ability/Ability.lua:181`, `UI/Core/ActionButtons/ActionButtons.lua:163`; [ability-resolution-pipeline.md](../../reverse-engineering/findings/ability-resolution-pipeline.md) Phase 1; ADR decision 31 note (`abilities-and-effects-decisions-23-33.md:325`) |
| B-11 | **The server never substitutes the caster for a Self ability.** `handle_use_ability` only discards the target for summons, owner-pet and deployables (`use_ability/handle.rs:80-93`). Python did: `canUse` sets `self.target = self.entity` for `TargetSelf` (`AbilityManager.py:527-528`), and `playSequence` falls back to the caster (`:888-892`). | CODE; FAN-GUESS (behavioural reference) |
| B-12 | **Target 0 resolves nothing but charges the cooldown.** `fire_cast` returns at `target_id <= 0` with only the ammo flush (`use_ability/fire.rs:137-146`), after the launch already started the cooldown and sent `onTimerUpdate` (`handle.rs:513-596`). No feedback. | CODE |
| B-13 | **Self or ally as target is refused by #444.** `player_may_attack` is false for the caster and for any non-duel player (`cell-world/src/cell/combat/aggression.rs:155-187`), so `handle.rs:271-286` returns false with the WARN "player single-target ability against a non-hostile target". No client feedback. The code comment at `handle.rs:250-254` already names the gap: "supportive single-target abilities ... will need it too once an offensive/supportive ability flag exists". | CODE |
| B-14 | **A hostile as target heals the hostile.** The cast enters `apply_damage_to_target`: a QR roll, an empty `onEffectResults`, `generate_threat` with 0.0 (which still creates the threat key and puts the player in combat, `threat/aggro.rs:91`), and then `HealHealth`/`HealFocus` run with `target_id` = the NPC (`damage_apply/mod.rs:517-533`, `heal.rs:72-96`). Heal Focus with a mob selected restores the mob's Focus. | CODE |
| B-15 | **No ability heal has ever fired in play.** SigNoz, 30 days to 2026-10-03: `heal_health` 72 rows, all ability 648 (Slappack consumable); `heal_focus` 12, all 2206 (Focus consumable); `stat_buff_applied` 6, all stimpacks. 14 days of `useAbility: launched`: 559 (2,343), 579 (1,442), 221, 592, 594, 598, 584, 612 only; **zero** rows for 597, 1646 or 1218, and no #444 WARN for them either. Either nobody pressed them, or the client suppressed the press (the 2026-06-04 Heal Focus finding: an in-flight queue gate at `0x00d2b020`). AB-E1 settles which. | TELEMETRY; [client-wire-emit-suppression.md](../../reverse-engineering/findings/client-wire-emit-suppression.md) |
| B-16 | **A working beneficial path already exists twice.** Native consumables apply an ability's effects to the user through `content/effect_apply.rs::apply_ability_effects` (`consumable_use.rs:1-75`), and support darts classify a target as Ally / Hostile / Other and run only the beneficial effect (`use_ability/support_shot.rs:58-105`). Neither is reachable from a plain `useAbility`. | CODE |
| B-17 | `docs/gameplay/ability-system.md:23,148` marks `TargetSelf` "DONE: targets the caster". That is wrong on `main` (B-11, B-12). | doc vs CODE |

## 3. Resolution: what an effect does once it lands

| ID | Finding | Evidence |
|---|---|---|
| B-20 | **Damage is read only from NVPs; with none, a hit deals 0.** `apply_hit` reads `HealthDamage`/`FocusDamage` per effect (`damage_apply/mod.rs:227-248`), and `calculate_damage_penetrating` returns no results for a base of 0 (`combat/damage/pipeline.rs:97-101`). So 89 of 91 reachable direct-damage abilities (Quick Burst 598 "-200F / -20H", Snare Shot 717, Takedown 856, every grenade and mortar) land a 0-damage hit that still aggroes the target. | CODE + B-03 |
| B-21 | **Only the last positive NVP counts.** `h_dmg = hd` overwrites per effect (`damage_apply/mod.rs:234-239`), so an ability with a primary and a secondary damage effect resolves only one. | CODE |
| B-22 | **A scripted damage effect is applied twice.** The NVP path deducts `HealthDamage` and `FocusDamage` independently through the QR pipeline (`damage_apply/mod.rs:292-315`), then the same effect's `RangedPhysicalDamage`/`MeleePhysicalDamage` script drains Focus again and bleeds to Health (`damage_apply/mod.rs:517-533`, `scripts.rs:455-520`). The script docs say the NVP path is the fallback "without this script" (`scripts.rs:448-450`), but nothing skips it. Pistol Shot and Strike are affected. | CODE |
| B-23 | **Scripts ignore the QR result.** Script dispatch has no `RC_MISS` check (`damage_apply/mod.rs:517`), so a missed Pistol Shot still bleeds Focus and Health through its script. | CODE |
| B-24 | **`EF_DONT_USE_QR` is 32 and never read; the original bit is 16.** 184 effects of reachable abilities carry bit 16, including every heal and buff. 32 is `EF_HasInductionBar`. | `crates/entity/src/abilities/defs.rs:58`; `entities/defs/enumerations.xml:1094-1123` |
| B-25 | **The other `EF_*` "category" constants are not client bits.** `EF_STUN = 12` is ClearOnDeath + ClearOnDamage, `EF_SUPPRESSION = 76`, `EF_DOT = 516`, `EF_INTERRUPT_CHANCE = 16` (which is really DontUseQR). They feed only a DEBUG log (`cone_aoe/flag_categories.rs:17-58`), whose `categories` labels are therefore wrong. | `defs.rs:56-63` vs `enumerations.xml` |
| B-26 | **Damage type is forced to `DT_PHYSICAL`** unless special ammo overrides it (`damage_apply/mod.rs:193-199`); energy and hazmat abilities resolve as physical. `MITIGATION` is created 0/0/0 (`entity/src/stats/stat_list.rs:118`), so armour and penetration are inert. Both are the enemy-combat blockers, not this campaign's. | CODE |
| B-27 | **Every effect of an ability lands on the one target.** `apply_hit` runs all `effect_ids` against `target_eid`. Nothing reads `EF_ResolveOnAbilityUser` (131072; 10 reachable effects, on the three Disguise abilities 1081-1083 and Incinerate Area 1889). The "User" and "Secondary Target" halves are otherwise authored only in text (Combat Sprint 1962 "User +50% Run Speed", Hunker Down 1746 "Secondary Target"), so a self-cast that also hits a target needs a text-derived split. | CODE; `enumerations.xml` |
| B-28 | **A `TCM_AERadius` effect on a non-ground ability does not fan out**; radius fan-out runs only for ground casts (`abilities/dispatch/`). Morale Boost 869 ("Short Radius 35% Focus Heal") and Leadership 858 are Self casts with AE halves. `TCM_Group` (6) and `TCM_Aura` (7) effects exist on reachable abilities (the gap analysis says none are seeded) and have no code. | CODE; seed |

## 4. Duration, buffs and debuffs

| ID | Finding | Evidence |
|---|---|---|
| B-30 | **A timed single-pulse effect is never registered.** `is_pulsing` is `pulse_count == 0 \|\| > 1` (`defs.rs:239-241`); `register_active_effect` returns at `!effect.is_pulsing()` (`effects/pulsing/register.rs:50-52`). 135 reachable effects are `pulse_count = 1` with `pulse_duration > 0`: the authored shape of every timed buff and debuff ("+200 Accuracy: 15 Seconds", "Snare: 15 Seconds", "Knockdown: 5 seconds"). They get no instance, no `onTimerUpdate` icon and no expiry. | CODE; seed |
| B-31 | **Python treated every effect as an instance with a duration of `pulseDuration * pulseCount`, and reverted non-permanent stat changes on removal.** | FAN-GUESS, `AbilityManager.py:274-345, 406-443` |
| B-32 | **The stat-buff ledger is the working seam for timed stat changes**, but only for six primary attributes. `StatBuff` reads NVPs named after the stat (`STAT_BUFF_NVPS`: Coordination, Engagement, Fortitude, Intellect, Morale, Perception), the ledger expires them and sends `onTimerUpdate(TIMER_DURATION_EFFECT)` with the effect id as SecondaryId, and `clear_stat_buffs_on_death` honours `EF_ClearOnDeath`. Accuracy, Defense, CoverDefense, Tracking, resists, `movementSpeedMod` and the regen stats have no mapping. | `cell-effect-scripts/src/cell/effects/stat_buff/mod.rs:1-60`; `cell-combat/src/cell/effects/stat_buffs/mod.rs:1-80`; ADR decision 28 |
| B-33 | **`AbsorbShield`'s doc tells authors to use the shape that never registers**: "register the effect as pulsing with `pulse_count = 1` and `pulse_duration = <buff_seconds>`" (`scripts.rs:234-238`), which B-30 drops. No seed row binds `AbsorbShield` or carries `ShieldAmount`, so the 15 reachable shield abilities have no numbers either. | CODE; seed |
| B-34 | `docs/gameplay/effect-system.md:26` marks "Duration tracking DONE". True only for multi-pulse effects (B-30). | doc vs CODE |
| B-35 | **Toggles are honoured only for owner-pet abilities.** `AF_TOGGLED` (8) is read in `owner_pet/fire.rs:151` and `pet_scripts/mod.rs:123`; 52 reachable abilities carry it (stances, Reveal, Disarm, Stealth III/IV, disguises). A second press re-casts. Stance exclusivity is authored as an explicit "Remove Effect of moniker EFFECT_Stance" effect first (combat-formulas-status §6), and no code removes by moniker. | CODE; seed |
| B-36 | **Passives run only pet scripts.** `apply_passives` fires an `EF_AlwaysPersist` effect only when `is_passive_script` admits its script (`cell-world/src/cell/effects/passives.rs:36-80`). 19 reachable abilities carry `EF_AlwaysPersist` effects (809 Mental Fortitude "+15% Mental Resist", 1450 Cover Penetration "+100 CoverAccuracy", 2008 Warrior's Fortitude "Health regeneration Increased +10%", 1479 Throw Mastery, 3258 "10% Focus Regeneration"); only 2852 Heed Our Calling runs. Passives without the flag (1457 Steadfast, 867 Quickload) have no apply path at all. | CODE; seed |
| B-37 | **`EF_ClearOnDamage` (36 reachable effects), `EF_ClearOnRez` (1) and `EF_RemoveOnBandolierSlotChange` have no constant and no hook.** Python removed them in `onDamageReceived`, `onRevived` and `onBandolierSlotChange` (`AbilityManager.py:1059-1088`). | CODE; FAN-GUESS |
| B-38 | **Effects are not persisted across logout** (`EF_AlwaysPersist` has no save path; `EF_Offline_Time_Counts` is ignored). The stimpack ledger is in-memory too. | CODE; gap-analysis §10 |

## 5. Crowd control

| ID | Finding | Evidence |
|---|---|---|
| B-40 | **`Stun` sets `BSF_MOVEMENT_LOCK` and leaks it on refresh, and NPC AI never reads the flag** (#1049). A stun registered through the pulsing layer re-runs `on_apply` per pulse but `on_remove` once. | `scripts.rs:317-324`; issue #1049 |
| B-41 | **Snare, root, slow and knockdown have no mechanic.** 110 CC-named effects; the one scripted slow is `MovementSlow` (dart Tranquilizer, AM-11a), which writes `movementSpeedMod` directly. A snare is the same stat change with a duration, so it needs B-30's instance. | CODE; seed |
| B-42 | **Interrupt is not wired to the pending-cast table.** Effect scripts live in `cimmeria-cell-world` / `-effect-scripts` and cannot reach `pending_cast` in `cimmeria-cell-combat` (ammo known issue "EMP has no interrupt"). | ammo README known issues |
| B-43 | **Resist rolls (72 reachable effects) are modelled nowhere.** The shipped structure says a resist is a separate QR roll that gates the effect it shares an `effect_sequence` step with (kinetic gates Knockdown/Stun/Snare, mental gates Suppression/Disorient/Fear, health gates DoT/Wound/Slow). The roll's maths is UNKNOWN. | ORIGINAL-DATA, combat-formulas-status §4 |

## 6. Regeneration

| ID | Finding | Evidence |
|---|---|---|
| B-50 | **Regen today: 1 Hz, players only, out of combat only (`threatened_mobs` empty), `max(regenStat, 1)` points per second per pool.** Archetypes seed `healthRegen` and `focusRegen` at 0, so both pools regenerate 1 point/s: 760 Health in about 13 min, 1,570 Focus in about 26 min (more at higher level, where Focus grows 70 per level). | `crates/cell/src/cell/service/ticks/regen.rs:42-111`; `db/resources/Archetypes/Seed/archetypes.sql` |
| B-51 | **The regen stats are percentage modifiers, not points per second.** `alias.xml`: `healthRegen` "increases health regeneration rate by 1%", `focusRegen` "increases focus regeneration rate by 1%", `morale` "+100 focus points per point, +0.5% increase to focus regeneration rate per point", `energyRegen` likewise 1%. So a *base rate* exists that these scale, and `regen.rs:83` reads the stat as the rate itself. | ORIGINAL-DATA, `entities/defs/alias.xml:196,202-203,246` |
| B-52 | **No artefact gives the base rate.** Python declares the regen stats (`SGWBeing.py:181,282-283`) and never ticks them; the C++ reference has no regen; no client Lua reads them. | FAN-GUESS absence; grep of `deprecated/` and client `UI/` |
| B-53 | **The content implies Focus regenerates in combat.** Leadership 858 ("Focus Regeneration Increase: 50% / 20 Seconds", 300 s cooldown), Demand Concentration 1637 (+10% for 15 s), Stance: Rally 1891 and Motivation 2074 (group Focus regen stances), Deployable: Focus Regen 1223: short combat buffs to a rate that, if it only ran out of combat, they could never affect. The designers flag out-of-combat explicitly where they mean it (Stealth I: "+10% every 2 seconds (Outside of Combat)"). Health has no comparable in-combat regen buff except Warrior's Fortitude (passive +10%). | DESIGNER-TEXT, seed |
| B-54 | **Regen-buff abilities have nothing to change.** Leadership's effects 1211/921 and Demand Concentration's 1985 carry no NVP and are single-pulse timed effects (B-30). | seed; CODE |
| B-55 | **NPCs never regenerate** (`regen.rs:57-60` iterates players only); an NPC resets to full only on leash or respawn. Out of scope here; noted for the NPC AI ledger. | CODE |

## 7. Feedback and wire

| ID | Finding | Evidence |
|---|---|---|
| B-60 | **Every no-op press is silent.** Target-0 (B-12), the #444 refusal on self/ally (B-13), and the 306 abilities with no mechanic (B-02) charge or refuse without `onErrorCode` or a `CHAN_FEEDBACK` line, against the project rule. Precedents exist: `not_known::send_not_known_feedback`, the consumable "This item has no effect yet." refusal (`consumable_use.rs:207-240`), and `ability_is_unimplemented` for pet orders. | CODE; CLAUDE.md project rules |
| B-61 | **Heals send no `onEffectResults` and therefore no floating heal number.** Scripts mutate stats and the caller flushes `onStatUpdate` to the target only (`damage_apply/mod.rs:543-558`, via `send_entity_method`, not to witnesses). The ammo campaign left the same gap for support darts. | CODE; ammo README known issues |
| B-62 | **Open wire question: result-code numbering.** `enumerations.xml` `EResultCode` has `RC_Hit = 1 ... RC_Glancing = 5`, which the server sends. The client's 20-entry sequence-event table at `0x01e6ce00` is `ABILITY_INTERRUPT = 0, ABILITY_FAILED = 1, EFFECT_INIT = 2, ... EFFECT_HIT_NORMAL = 4, ... EFFECT_HIT_MISS = 8` ([combat-damage-analysis.md](../../reverse-engineering/findings/combat-damage-analysis.md#complete-qr-result-code-table-confirmed-from-0x01e6ce00-pointer-table)). Python mapped one to the other (`AbilityManager.py:256-263`). Whether `onEffectResults` carries the first or the second decides whether a hit plays its hit reaction (the open UAT-1 finding 10 "shoot animation does not play" may be this). Verify against a capture before touching it. | ORIGINAL-DATA vs ORIGINAL-RE; FAN-GUESS |
| B-63 | **Category cooldowns (timer type 8) are never sent**; `TIMER_CATEGORY_COOLDOWN` is defined and unused (`defs.rs:93`). | CODE |

## 8. Representative abilities by family

Spot checks, by hand, against the seed and the code path each takes today.

| Family | Ability | Effects (NVPs) | Today |
|---|---|---|---|
| Heal self (Focus) | 597 Heal Focus, Self, every archetype | 659 `HealFocus`, `HealPercentage=35.00` | Target 0: cooldown charged, nothing (B-12). Self: refused by #444 (B-13). Hostile: heals the hostile (B-14). |
| Heal (Health) | 1646 Health Heal, Target, starter + Goa'uld | 2008 `HealHealth`, `HealPercentage=10.00` (Cimmeria-authored, PR #496) | Same three outcomes; on an ally refused by #444. |
| HoT | 1218 Recuperation, Target, starter | 1383 `HealHealth`, `HealPercentage=3.00`, 25 x 1 s | Would pulse through the pulsing layer once targeted; never reached (B-15). |
| Heal (unscripted) | 742 Field Medic I (Archaeologist) | 788 "+10% Health", flags 17, no script, no NVP | Nothing even with a valid target. |
| Focus restore, AE | 869 Morale Boost (Soldier), Self | 939 "35% Focus Heal" (Single) + 1215 (AERadius Short), no script | Nothing (B-12, B-28). |
| Regen buff | 858 Leadership (Soldier), Self | 1211 + 921 "+50% Focus Regen: 20 Seconds", single-pulse timed | Nothing (B-30, B-54). |
| Direct damage | 598 Quick Burst (Soldier root) | 660 "-200F / -20H", no NVP | 0-damage hit; target aggroes (B-20). |
| Direct damage (scripted) | 592 Pistol Shot | 654 `RangedPhysicalDamage`, `HealthDamage=15`, `FocusDamage=150` | Works, applied twice; script damage lands on a miss (B-22, B-23). |
| DoT | 1879 Point Blank Shot (Soldier) | 2393 "-200F / -20H"; 2394 "DOT: -150F -30H (8 Ticks)", pulse 8 x 1 s | Pulses register, every pulse deals 0 (B-20). |
| Stat buff | 637 Aim (Soldier) | 700 "+200 Accuracy: 15 Seconds", flags 21 (Beneficial, ClearOnDeath, DontUseQR) | Nothing (B-30, B-32). |
| Stat debuff | 847 Call Target (Commando) | 903 "-100 Defense: 15 Seconds" | 0-damage hostile hit, nothing else. |
| CC | 717 Snare Shot (Soldier) | 744 damage "-100F / -10H"; 745 Kinetic Resist Roll; 1462 "Snare: 15 Seconds" (pulse 1 x 15) | 0 damage, no snare (B-20, B-30, B-41, B-43). |
| CC (knockdown) | 856 Takedown (Soldier) | 919 damage; 2607 resist roll; 2608 "Knockdown: 5 seconds" | Same. |
| Shield | 1579 Defensive Shield: Absorption (Asgard) | 4785, no NVP, no script | Nothing (B-33). |
| Stance / toggle | 1642 Stance: Soldier, Toggled | 2003 "Cover Defense +100", 2004 "+50 (5%) Mental Resist", 2005 "Subtlety -100" (pulse duration 0 = held) | Nothing; a second press re-casts (B-35). |
| Passive | 809 Mental Fortitude (Archaeologist), passive | 854 "+15% Mental Resist", `EF_AlwaysPersist` | Never applied (B-36). |
| Movement | 1619 Combat Sprint (Soldier) | 1962 "User +50% Run Speed / 10 s"; 2002 "-100 ACC / 10 s" | Nothing (B-27, B-30). |
| Cleanse | 2865 Absolution (Goa'uld) | 4168/4169 "Purges 2 Mental / 2 Health effects" | Nothing; `RemoveEffects` exists for darts only. |
| Stealth | 646 Stealth I | 710 "Max Stealth Rating +100" | Nothing; no stealth system. |
| Resurrect | 1190 Goldam's Gift | 1357/1358 "revives self, 25% Focus 10% Health" | Nothing; a dead player cannot cast (`handle.rs:146`). |
| Summon | 2826 Summon Straegis | none; `pet_summons` row | Works (pets PT-03). |
| Summon (turret) | 962-966 Summon Turret | no `pet_summons` row | Nothing; the turret repairs 967/968/1207 have scripts but nothing to repair. |

## 9. Docs that disagree with `main`

These are owed by the packet that touches each area, in the same PR:

- `docs/gameplay/ability-system.md:23,148` "TargetSelf DONE" (B-17): AB-01.
- `docs/gameplay/effect-system.md:26` "Duration tracking DONE" (B-34): AB-04.
- `docs/gap-analysis/core-gameplay.md` §9 "Group targeting / Aura targeting: no seeded effect uses it" (B-28: 6 and 7 do): campaign close-out.
- `crates/cell-effect-scripts/src/cell/effects/scripts.rs:234-238` `AbsorbShield` authoring note (B-33) and `:448-450` "Without this script, the legacy NVP fallback applies" (B-22): AB-06.

## 10. External archives checked (2026-10-03)

The owner's local archives were searched for original ability, effect, stat and regen material before this campaign starts: the external reconstruction effort's handoffs (`SGW_FINAL_*` bundles, historical CellBlocks, UI handoff, spreadsheets), four client builds (0.8293 of 2008-06-03, 0.8309 of 2008-09-26, 0.58674 of 2008-10-09, QA 4046), `r529-sgw.rar` and a third-party client copy. **None of them holds a regen rate, heal amount, cost, archetype growth curve or combat formula the repo lacks**, which confirms [combat-formulas-status.md](../../reverse-engineering/findings/combat-formulas-status.md). What they add:

| ID | Finding | Provenance | Used by |
|---|---|---|---|
| B-70 | `SGWCombatant` declares `regenTimerID` ("Timer ID for the Health and Regen Pulse"), `lastRegenTime` and `bHealDebug` in every build from 0.8293 to QA 4046. Regen was designed as one per-combatant timer pulse, not an effect; no build or script carries its rate. | ORIGINAL (entity defs, all builds) | AB-05, D-AB03 |
| B-71 | `r529-sgw.rar` is the 2014 community server checkout, identical to `deprecated/python` and the data scripts already in the repo. It ships 4 effect scripts (Debug, RangedEnergyDamage, RangedPhysicalDamage, Reload) and 17 `effect_nvps` rows. `HealHealth`, `HealFocus` and `StatBuff` are Cimmeria additions, not recovered scripts. | ORIGINAL archive, RECONSTRUCTION content | AB-02, AB-06 |
| B-72 | The 2014 server-build PAKs match QA 4046's ability and effect tables: 22 of 1,886 abilities differ only in description whitespace, typos or one name; 1 of 3,216 effects differs (test effect 347, `PulseDuration` 0 vs 30). There was no hidden balance pass; QA's tables are the data. | ORIGINAL (PAK diff) | AB-02, AB-03 |
| B-73 | Client `Effect.lua` caps the effect bar at `MAX_BENEFICIAL` = `MAX_HARMFUL` = 10 from 0.8309 on (14 in 0.8293). About 10 visible effects per side is the shipped UI budget. | ORIGINAL (client Lua) | AB-04 |
| B-74 | `effectMonikers` held (moniker CRC, effect count) before QA, which changed it to (entity id, moniker CRC); 0.58674 predates `clearEffectsByMoniker`, `removeEffectById` and `confirmationResponse`. Immunity and stance exclusivity were refcounted per moniker. | ORIGINAL (entity defs) | AB-04, AB-08, AB-10 |
| B-75 | The stat list grew from 48 (June 2008) to 54 (September) to 77 (October, equal to QA). `recovery`, `restoration`, the speed stats and all 15 `absorb*` stats arrived after September 2008, so healing modifiers and absorb shields are the youngest, least-settled part of the design. `EF_DontUseQR` is 16 already in 0.58674 (see AB-06). | ORIGINAL (entity defs) | AB-06, AB-10 |
| B-76 | Effect `Delay` was `unsignedShort` in October 2008 and is `float` in QA; `Taunt_Adjustment` is signed from September 2008, so negative taunt values are real data. | ORIGINAL (entity defs) | AB-04, AB-09 |
| B-77 | `adjustStat` took `(STRING, 6 x INT32)` through 55124; mitigation (FLOAT) and a PYTHON dict arrive in 58674, and QA adds a UINT8 `EDamageType` to `onAttacked`. The repo already has the final form. | ORIGINAL (entity defs) | AB-03 |
| B-78 | Two artefacts named by the reconstruction effort's README are not in any archive here: `SGW_FINAL_RE.sqlite` (said to hold ability and effect masters across builds) and the raw 63682-to-QA 4046 gameplay-cache delta (52,603 rows). Worth requesting from its authors; nothing waits on them. | missing | none |
| B-79 | Dead ends: `Stats.dll` is Epic's managed UE3 profiler-stats library, never loaded by `SGW.exe`; the third-party client's 21 cache PAKs are md5-identical to `data/cache`; `0.8307.0.55124.rar` is an HTML error page; the archetypes, progression and mission spreadsheets carry no stats. A marketing newsletter says Jaffa have "phenomenal healing abilities" from the larval Goa'uld, which supports, qualitatively only, a Jaffa regen bonus. | checked | D-AB03 (Jaffa note) |

The early-build `.def` files also keep developer comments that the repo copies lost (SGWMob, SGWPlayer, SGWInventoryManager among them). Read them first when a def's intent is unclear; they are in the owner's archives, not in the repo.
