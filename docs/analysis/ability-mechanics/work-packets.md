# Ability Mechanics Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-10-03. Companions: [launch prompt and decisions](README.md), [evidence audit](audit.md), [testing playbook](../../../TESTING.md), [ability trees ledger](../ability-trees/work-packets.md) and [ammo ledger](../ammo/README.md) (same dispatch rules). `AM-` is the ammo campaign's prefix; this campaign uses `AB-`.

## Dispatch rules

- One worktree per worker under `.claude/worktrees/` (Agent `isolation: "worktree"`). Branches are `abilities/<packet>-<slug>`.
- Every compiling `cargo` call goes through `bash tools/build-lane/lane.sh`, per crate with `-p` (`cimmeria-cell-combat`, `cimmeria-cell-effect-scripts`, `cimmeria-cell-world`, `cimmeria-entity`, `cimmeria-cell`). Live-DB tests use the worktree's own `sgw_<worktree>` database.
- Open the PR with `bash tools/build-lane/ship.sh pr -C <worktree> -m <msg>`; merge with `ship.sh merge <PR> --retire <worktree>` after the minimum build-proving CI is green.
- Seed changes go in `db/resources/` only. No `db/scripts/*.sql` migrations.
- Every row the generator writes is labelled RECONSTRUCTION with its source text. Never call a number "retail".
- Initial state: documentation only, against `main` @ `b40ef76d2`. No packet has started.

Status vocabulary: **Ready**, **BlockedDependency**, **BlockedDecision**, **Writing**, **Review**, **Integrated**, **UATPending**, **Done**.

`rust-gameserver-dev` writes the Rust. `combat-systems-advisor` advises every packet and reviews AB-01, AB-03, AB-06 and AB-09. `server-authority-enforcer` reviews AB-01 and AB-08 (both change who a client-named target can affect). `database-persistence` reviews the generator packets (AB-02, AB-03). `testing-validation-engineer` reviews the AB-01 and AB-06 regression strategy. `documentation-writer` reviews the docs each packet owes ([doc-update map](../../agents/doc-update-map.md)).

## Contract fixed by this ledger

Parallel packets build against these names. A worker who needs to change one raises it with the coordinator instead of renaming locally.

**Beneficial classification** (AB-01), in `cimmeria-entity` beside `implemented.rs`:

- `pub fn ability_is_beneficial(def: &AbilityDef, effects: &HashMap<i32, EffectDef>) -> bool`: `type_id` Heal, or every effect that does something carries `EF_BENEFICIAL_EFFECT` (1). `AbilityDef` gains `type_id: AbilityType` (loaded from `resources.abilities.type_id`).
- `pub const EF_BENEFICIAL_EFFECT: u32 = 1;` and `pub const EF_DONT_USE_QR: u32 = 16;` (AB-06 corrects the latter; AB-01 must not depend on the old value).

**Cast target resolution** (AB-01), in `cell-combat/src/cell/abilities/use_ability/beneficial.rs`:

- `enum CastTarget { Caster, Ally(u32), Hostile(u32), None }` and `fn resolve_cast_target(...) -> CastTarget`, the one place the D-AB01/D-AB02 rules live. `handle.rs` and `fire.rs` call it; nothing else re-derives the rule.
- A beneficial cast resolves through `fire_beneficial(...)`: the ability's effects run on the resolved entity with no QR roll, no threat, no in-combat state and no #444 gate; the stat flush goes to the target and its witnesses.

**Effect NVP generator** (AB-02), `tools/ability_mechanics/effect_nvps_from_desc.py`:

- Reads `db/resources/Effects/Seed/effects.sql`, writes one generated block per family into `db/resources/Effects/Seed/effect_nvps.sql` between `-- ability-mechanics generated <family> begin` / `end` comment markers (not the docs-gen `gen:` syntax, which `tools/docs-gen/regen.py` owns), with `nvp_id`s in a reserved range per family (heal 20000-20999, damage 21000-22999, stat 23000-23999, shield 24000-24499). Hand-authored rows outside the markers are never touched.
- `--report` lists every effect it could not parse. `--check` exits non-zero when the generated blocks differ from what it would write (a CI-friendly guard the live-DB test also runs).
- NVP names: `HealthDamage`, `FocusDamage`, `HealPercentage`, `HealAmount`, and the stat names `StatBuff` reads (AB-04 extends the list).

**Timed effect ledger** (AB-04): `StatBuffLedger` generalises from six attributes to any `cimmeria_entity::stats` id. One entry per `(entity, effect_id, invoker)`, holding the stat deltas it applied, its expiry (or `None` for held toggle effects), its effect flags and its moniker ids. Removal reasons: `Expired`, `ToggledOff`, `RemovedByMoniker`, `Death`, `Damage`, `Revive`, `BandolierSwap`, `Cleansed`. The client sees the existing `onTimerUpdate(TIMER_DURATION_EFFECT)` with SecondaryId = effect id.

## Dependency graph and waves

```text
Wave 0 (parallel)                Wave 1                               Wave 2                          Close
AB-E1 client evidence ─────────► AB-11 onEffectResults heal + flags ─┐
AB-01 beneficial targeting ──┬─► AB-12 no-op press feedback ─────────┤
                             └─► AB-07 effect routing (user/AE/group)┤
AB-02 NVP generator + heals ───► AB-03 damage numbers ───────────────┤
AB-06 damage-script + QR fixes ─┘                                     ├──► AB-13 close-out ─► owner UAT
                                 AB-04 timed effect ledger ─┬─► AB-08 toggles, stances, passives
                                                            ├─► AB-09a stun/knockdown (#1049)
                                 AB-05 regen (needs D-AB03) ┤   AB-09b snare/slow
                                                            ├─► AB-09c interrupt
                                                            ├─► AB-09d resist rolls (D-AB13)
                                                            └─► AB-10 shields + cleanse
```

Priority inside each wave: anything that changes what Heal Focus, Health Heal, Recuperation or Focus regeneration do merges first. AB-01 is the playtester's ask.

**Contended files.** The coordinator merges these one packet at a time:

- `cell-combat/src/cell/abilities/use_ability/handle.rs` (687 lines, hard cap 700): AB-01, then AB-12. **Both must move code out, not add it**; AB-01 lifts the #444 block into `beneficial.rs` first.
- `cell-combat/src/cell/abilities/use_ability/fire.rs`: AB-01, then AB-07.
- `cell-combat/src/cell/abilities/damage_apply/mod.rs` (682 lines): AB-06 first (it splits the script dispatch out into `damage_apply/effect_scripts.rs`), then AB-03 (B-21), then AB-07.
- `db/resources/Effects/Seed/effect_nvps.sql` and `effects.sql` (`script_name`): every generator run (AB-02, AB-03, AB-04, AB-10). Each packet regenerates only its own marker block; rebase and re-run `--check` before merge.
- `cell-effect-scripts/src/cell/effects/registry.rs` (`EFFECT_SCRIPTS`): AB-04, AB-09a-c, AB-10 add one row each.
- `cell-world/src/cell/effects/stat_buff/`: AB-04 owns the generalisation; AB-05, AB-08, AB-09b and AB-10 only call it.
- `entity/src/abilities/defs.rs`: AB-01 (`EF_BENEFICIAL_EFFECT`, `type_id`), AB-06 (`EF_DONT_USE_QR`, the category constants), AB-11 (the clear flags).

## Common acceptance

- Every behaviour change ships a regression guard that **fails when the fix is reverted** ([TESTING.md § Regression-guard shape](../../../TESTING.md#regression-guard-shape)). State in the PR which assertion fails on revert.
- Seed changes ship a live-DB guard (`require_db_or_skip!`, `live_db` in the name) that loads the real seed and asserts the specific rows: for example "effect 660 has `HealthDamage` 20 and `FocusDamage` 200".
- New wire output gets a byte-exact test. New WARNs and feedback rows get `LogCapture` (negative-log) guards. New log targets join `OTEL_FILTER` with its pinning assertion.
- Telemetry is first class: every resolution logs `ability_id`, `effect_id`, the resolved target and why, so a playtest is diagnosable from SigNoz alone.
- Docs owed per packet are named in the packet; the status docs (`docs/gap-analysis/`, `docs/project-status.md`) change once, in AB-13.

## Wave 0

### AB-E1. Client evidence for beneficial casts (lab, no server code)

- **Goal.** Settle B-15 and B-62 with the live research lab before AB-01's UAT relies on them.
- **Questions.** (1) With the action bar, what `TargetID` does the client send for 597 (Self), 1646 and 1218 (Target) with no target, self, an ally and a hostile selected? (2) Does a 597 press reach the wire at all, or does the in-flight queue gate (`0x00d2b020`) or the arg check (`0x00aa2910`) drop it? (3) Does `onEffectResults` with a positive Health or Focus delta from one player to another, outside a duel, render a heal number without fault? (4) Which `ResultCode` byte makes the client play a hit reaction: `RC_Hit` (1) or the sequence table's `EFFECT_HIT_NORMAL` (4)?
- **Method.** `lab_play_character`, `client_use_ability` and `server_packet_tap_*`; a hand-built `onEffectResults` through `server_console_exec` only on the lab server.
- **Output.** `docs/reverse-engineering/findings/beneficial-cast-client-evidence.md`, and the answers copied into D-AB14 and AB-11.
- **Advisor.** `game-archaeology-specialist`. **Tests.** None (finding). **Depends.** Nothing.

### AB-01. Beneficial targeting: Self casts land on the caster, heals on allies (D-AB01, D-AB02)

- **Goal.** Heal Focus, Health Heal and Recuperation work on the first press, from the starter bar, for every archetype.
- **Audit rows.** B-10 to B-16, B-60.
- **Change.**
  - Add `AbilityDef::type_id`, `EF_BENEFICIAL_EFFECT`, `ability_is_beneficial` (contract).
  - `use_ability/beneficial.rs`: `resolve_cast_target`. `TargetSelf` → `Caster` whatever the wire said. A beneficial `TargetTarget` → `Ally(id)` for the caster or a same-space player the caster may not attack (reuse `support_shot::classify`), else the D-AB02 branch (fallback to `Caster`, or refusal with feedback). A non-beneficial cast keeps today's #444 path untouched.
  - `fire_beneficial`: runs every effect's script on the resolved entity, registers pulsing effects (Recuperation's 25 pulses) with the caster as invoker, flushes the target's dirty stats to the target and its witnesses. No QR, no `generate_threat`, no `enter_player_combat`, no channel cancel of other abilities.
  - The warmup tick's fire-time re-check uses the same resolver (`warmup/tick.rs`).
  - Telemetry: `abilities` target, `event = "beneficial_cast"`, with `ability_id`, `wire_target_id`, `resolved_target_id`, `resolution` (`self_ability`, `ally`, `fallback_to_caster`, `refused`).
- **Must not.** Let a beneficial effect reach a hostile (the B-14 bug), or let a damaging ability skip #444. Grow `handle.rs` past 700 lines.
- **Tests.** Unit: each `resolve_cast_target` arm. Pipeline (`use_ability/tests/beneficial.rs`): 597 with target 0, self, ally and hostile restores the caster's Focus and never the hostile's (the hostile assertion fails on revert); 1646 on an ally heals the ally; a damaging ability at an ally is still refused (#444 guard). Wire-format: the `onStatUpdate` the caster receives. Negative-log: no #444 WARN for a beneficial self cast. Live-DB: 597/1646/1218 load as beneficial from the real seed.
- **Docs.** `docs/gameplay/ability-system.md` (TargetSelf row, B-17), `docs/architecture/abilities-and-effects-decisions-23-33.md` (new decision 34: beneficial resolution), `docs/gameplay/combat-system.md` targeting section.
- **Advisor.** `combat-systems-advisor`; review `server-authority-enforcer`. **Depends.** Nothing (D-AB02 default if unanswered).
- **Status.** Review (2026-10-03, branch `abilities/ab-01-beneficial-targeting`). Built with the D-AB02 default; the const `FALLBACK_TO_CASTER` in `use_ability/beneficial.rs` flips it. Contract refinements: `ability_is_beneficial` requires at least one effect that does something, and every such effect must carry the beneficial bit or, on a Heal-typed ability, run a heal script (the Heal type alone admitted ~200 seeded debuffs; server-authority review S1); any effect with positive `HealthDamage`/`FocusDamage` vetoes beneficial (2228 is a Heal-typed attack). `PendingCast` gains `wire_target_id`, so the fire re-resolves from the client's target. Only player casts are resolved; NPC casts keep the wire target. `AbilityType`'s DD variant is `DirectDamage`.

### AB-02. Effect NVP generator, and the heal rows (D-AB06)

- **Goal.** One reviewable tool turns designer text into NVPs, and every reachable heal effect gets its number and its script.
- **Audit rows.** B-03, B-16.
- **Change.** `tools/ability_mechanics/effect_nvps_from_desc.py` (contract) with a `heal` family: parses "+10% Health", "Heals 35% of target's Focus pool", "Target +35% Focus" into `HealPercentage`; flat amounts into `HealAmount`; sets `script_name` to `HealHealth`/`HealFocus` on those 22 effects. Leaves the rows PR #496/#497 authored by hand alone. Over-time heals ("over 25 seconds", "per second") take the per-pulse share.
- **Tests.** Unit (pytest or a Python doctest run by the live-DB job): the parser on the real strings, including the ones it must reject. Live-DB: effects 788, 834, 835, 939, 1040, 1044, 2014 carry the expected script and NVP; `every_seeded_script_name_is_registered` still passes. `--check` in CI.
- **Docs.** `tools/ability_mechanics/README.md` (pattern: `tools/ability_trees/`); `db/resources/Effects/Seed/` provenance note.
- **Advisor.** `database-persistence`, `combat-systems-advisor`. **Depends.** Nothing; the heals are visible in play after AB-01.

### AB-06. Damage scripts are the only damage path; misses run no script; `EF_DontUseQR` (D-AB07)

- **Goal.** Pistol Shot and Strike deal their damage once, and a miss deals none.
- **Audit rows.** B-22 to B-25.
- **Change.** Split script dispatch out of `damage_apply/mod.rs` into `damage_apply/effect_scripts.rs`. Skip the NVP pipeline for an effect whose script is a damage script (`RangedPhysicalDamage`, `MeleePhysicalDamage`, `RangedEnergyDamage`, `MeleeDamage`); skip every QR-rolled script on `RC_MISS`. `EF_DONT_USE_QR = 16`, read in the roll (no roll, result `RC_Hit`). Rename or delete the non-client `EF_*` category constants and fix `flag_categories.rs` to log real bits. Correct the two stale doc comments in `scripts.rs` (B-22, B-33).
- **Tests.** Unit regression guards: Pistol Shot at full Focus changes Focus by the script's amount once (fails on revert with the doubled amount); a forced miss leaves both pools untouched; a `DontUseQR` effect never misses. Update the existing bleed-death tests that assumed the double path, with the reason in the PR.
- **Docs.** ADR decision 10 addendum; `docs/reverse-engineering/findings/combat-formulas-status.md` divergence 1 marked fixed.
- **Advisor.** `combat-systems-advisor`; review `testing-validation-engineer`. **Depends.** Nothing. Announce the damage change in the PR: starter damage drops.

## Wave 1

### AB-03. Damage numbers for every reachable damage effect

- **Goal.** Each tree damage ability deals its tooltip numbers instead of 0.
- **Audit rows.** B-03, B-20, B-21.
- **Change.** Generator `damage` family: "-200F / -20H", "F-200 H-20", "-150F -30H per tick (8 Ticks)" into `FocusDamage`/`HealthDamage` (per pulse for DoTs). "vs Mechanical" and "vs Low Focus" variants are left out and reported (no conditional NVP exists). In `damage_apply`, resolve each `TCM_Single` damage effect on its own instead of keeping the last positive value (B-21); cone and radius effects stay with their fan-outs.
- **Tests.** Live-DB: 598, 717, 856, 1879 and one grenade carry their numbers. Pipeline: an ability with two single-target damage effects applies both. Unit: the parser on the corpus. Generator report committed, with the unparsed list.
- **Docs.** `docs/gameplay/combat-system.md` damage-source section.
- **Advisor.** `combat-systems-advisor`, `database-persistence`. **Depends.** AB-02 (generator), AB-06 (so scripted and unscripted effects do not both apply).
- **Status.** Review (2026-10-03, branch `abilities/ab-03-damage-numbers`). The `damage` family writes 178 effects (nvp_id 21000-21349) and reports 54 by category: conditional 15, sequenced 14, pulse shape 15, targeting 5, scope 2, grammar 3 (`tools/ability_mechanics/reports/damage.txt`). Single-shot `EF_SequenceOnFinish` (64) effects are reported as sequenced follow-ups: that is how Execution's and Red Mist's "vs low Focus" rows, the Energy Cascade jumps and the Grenade Barrage's extra shells are told apart from the base hit. Pipeline rule: a cone or radius effect lands on a hit only when no direct (non-pulsing) `TCM_Single` damage effect does. AB-06 carry-over closed: `EF_DontUseQR` is resolved per effect, so a flagged effect in a mixed ability deals its base on any roll, a miss included.

### AB-04. Timed effect ledger: buffs, debuffs and their icons (D-AB08, D-AB09)

- **Goal.** Aim, Hunker Down, Call Target, Combat Sprint, Leadership and every other `pulse_count = 1` timed effect applies, shows its icon and expires.
- **Audit rows.** B-30 to B-32, B-34.
- **Change.** Generalise the stat-buff ledger (contract). A `TimedStat` script (or `StatBuff` extended) reads stat-named NVPs for any stat id. Generator `stat` family writes them for the 39 stat effects using D-AB09's units, and binds the script. `damage_apply` and `fire_beneficial` hand single-pulse timed effects to the ledger.
- **Tests.** Unit: apply, refresh (same source), stack (different source), expiry reverts exactly the applied delta, death removes `EF_ClearOnDeath` entries. Wire-format: `onTimerUpdate` start and clear with SecondaryId. Live-DB: effects 700, 903, 1747, 1962 carry their NVPs.
- **Docs.** ADR decision 28 extended; `docs/gameplay/effect-system.md` duration row (B-34).
- **Advisor.** `combat-systems-advisor`. **Depends.** AB-02 (generator). Merges before AB-05, AB-08, AB-09b, AB-10.
- **Status.** Review (2026-10-03, branch `abilities/ab-04-timed-effect-ledger`). API for the later packets: `SpaceManager::apply_timed_effect(target, TimedEffectSpec, now)`, `remove_timed_effects(target, StatBuffRemoval, pred)`, `remove_timed_effects_by_moniker`, `stat_buff::timed_spec(ctx, stacking)`, and combat's async `strip_timed_effects(entity, reason, pred, tx, mgr)`; ADR decision 28's extension has the details. Contract refinements: `TimedStacking` is a spec field (`PerSource` for `TimedStat`, `ReplaceSameStat` keeps the stimpacks' decision 28 rule); `StatBuffRemoval` keeps `Replaced` and `Removed` beside the contract's reasons, and `Death` still logs `died`; a held entry sends no start timer; monikers are the ability's `moniker_ids` (the seed has no effect monikers, B-74). The `stat` family binds 21 effects: 8 buffs (Aim, Sight In, Brace, Impeccable Aim, Duck and Cover, the primary halves of Heroism and Hunker Down, Combat Sprint's run speed) and 13 debuffs (Call Target, Impose Weakness, Distraction, Focused Fire, Time Distortion, and the debuff halves of Flashbang, Blinding Vision, Blinding Strike, Aimed Shot: Leg and Arm, Disabling Shot x2, Pinning Fire); Leadership and Demand Concentration wait for AB-05 (regen stats), the Secondary and AE halves for AB-07, stances and passives for AB-08, "(1 hit)" debuffs for AB-11. `generator --report --family stat` lists the 56 unparsed.

### AB-05. Regeneration: percentage model, Focus recharges in combat (D-AB03, D-AB04)

- **Goal.** Focus behaves like a recharging shield and Health like a slow pool, at the owner's rates.
- **Audit rows.** B-50 to B-54.
- **Change.** `ticks/regen.rs` reads base rates from one constants module labelled DESIGN; scales by `focusRegen + morale x 0.5` and `healthRegen` as percentages; runs Focus in combat at the in-combat rate after the no-damage delay (a `last_focus_damage_at` stamp set in the damage seams). Leadership, Demand Concentration and the regen stances move `FOCUS_REGEN` through AB-04's ledger; the generator writes their NVPs. Keep the `regen_started`/`regen_stopped` rows, add `in_combat` and the per-pool amounts.
- **Tests.** Unit: the rate formula, the in-combat delay, the regen-buff multiplier, the floor of 1. Existing regen tests updated with the reason. Negative-log guard on the transition rows.
- **Docs.** `docs/gameplay/stat-system.md` regen section; README D-AB03 answer recorded.
- **Advisor.** `combat-systems-advisor`. **Depends.** D-AB03 (BlockedDecision until answered), AB-04 for the buffs (the base-rate half can merge first).

### AB-07. Effect routing: user halves, AE halves, groups

- **Goal.** Morale Boost heals the caster and nearby allies; Combat Sprint's run-speed half lands on the caster while its debuff half lands where authored; Disguise's `EF_ResolveOnAbilityUser` effects land on the user.
- **Audit rows.** B-27, B-28.
- **Change.** A per-effect target resolver: `EF_ResolveOnAbilityUser` → caster; a beneficial `TCM_AERadius` effect on a non-ground cast fans out around the caster to allies (hostile AE keeps today's collectors); `TCM_Group`/`TCM_Aura` per D-AB12. "User"/"Secondary Target" text halves get an explicit per-effect override column only if the generator cannot express them; prefer the flag.
- **Tests.** Pipeline: Morale Boost with an ally in range and one out of range; Combat Sprint's two halves. Unit: the resolver.
- **Advisor.** `combat-systems-advisor`. **Depends.** AB-01; D-AB12 for the group half.

### AB-12. Feedback for presses that cannot do anything (D-AB10)

- **Goal.** No silent presses: an ability with no implemented effect, or (if D-AB02 chose refusal) a beneficial cast with nothing to land on, answers with `onErrorCode` and a feedback line and charges no cooldown.
- **Audit rows.** B-02, B-60.
- **Change.** `ability_has_mechanics` (stricter than `ability_is_unimplemented`: an animation alone does not count), checked at launch before the cooldown. One `abilities` row per refusal with `reason`.
- **Tests.** Unit on the predicate. Pipeline: an unimplemented ability is refused with no cooldown timer sent (fails on revert because the timer is sent). Negative-log guard.
- **Advisor.** `combat-systems-advisor`. **Depends.** AB-01 (shares `handle.rs`). Re-run the predicate's live-DB count after AB-03/AB-04 so the refusal list shrinks as packets land.
- **Status.** Review (2026-10-03, branch `abilities/ab-12-noop-press-feedback`). `ability_has_mechanics` in `use_ability/no_mechanics.rs`: an effect with a damage NVP or a registered, non-blank script (`ability_effects_have_mechanics` in `cimmeria-entity`), a pet summon, an owner-pet script, a deployable, an ammo toggle, a weapon shot (`required_ammo > 0`), Cover Stance or Reload (596). Refusal: `onErrorCode(0, id, 167)` plus "That ability has no effect yet.", players only; abilities the active weapon grants are never refused. Count on `main` after AB-03: **247 of 1,886** (221 through effects, 7 only as weapon shots; 108 before AB-03); update `HAS_MECHANICS_TODAY` in `use_ability/tests/no_mechanics_live_db.rs` after AB-04 merges. Stat NVPs count through the script that reads them, not on their own (no code reads a stat NVP without a script). `handle.rs` moved its weapon-attack gates to `use_ability/weapon_gate.rs` (667 to 612 lines). Test fixtures that stand for "some attack" now carry the shared no-op mechanic effect (`test_fixtures::seed_mechanic_effect`).

## Wave 2

### AB-08. Toggles, stances and passives

- **Goal.** A second press turns a toggle off; a new stance replaces the old; passives apply when known.
- **Audit rows.** B-35, B-36.
- **Change.** `AF_TOGGLED` for player casts: on → held ledger entries (no expiry), off → `ToggledOff`. "Remove Effect of moniker EFFECT_Stance" effects remove ledger entries by moniker before the new stance applies. `apply_passives` runs any `EF_AlwaysPersist` effect whose script is a ledger script (not heals), at login, purchase and respec.
- **Tests.** Pipeline: toggle on/off restores stats exactly; switching stance removes the old one. Live-DB: a passive's stat applied at login from the real seed.
- **Advisor.** `combat-systems-advisor`; review `server-authority-enforcer` (a forged toggle must not stack). **Depends.** AB-04.

### AB-09a. Stun and knockdown (#1049)

- **Change.** Stun as a ledger entry that sets `BSF_MOVEMENT_LOCK` once and clears once; NPC movement and attack ticks honour it; knockdown uses the same lock with its own duration. Close #1049.
- **Tests.** Unit: refresh does not leak the refcount (the #1049 shape). NPC AI: a stunned NPC neither moves nor fires. Wire: the state-field broadcast.
- **Advisor.** `combat-systems-advisor`, `npc-ai-spawn-advisor`. **Depends.** AB-04.

### AB-09b. Snare and slow

- **Change.** Snare and slow effects become `movementSpeedMod` ledger entries (D-AB09); `MovementSlow` migrates onto the ledger so its overshoot (ammo known issue) goes away. NPC movement reads the stat.
- **Tests.** Unit: stacked slows revert exactly. NPC AI: a snared NPC's chase speed.
- **Advisor.** `combat-systems-advisor`, `npc-ai-spawn-advisor`. **Depends.** AB-04.

### AB-09c. Interrupt

- **Change.** An interrupt effect cancels the target's `pending_cast` and channels through a combat-side hook the effect layer can call (resolves the ammo "EMP has no interrupt" limit); `interruptRes` reduces the chance per `alias.xml`.
- **Tests.** Pipeline: Interrupting Shot during an NPC warmup sends `Ability_Interrupt` and refunds as AT-10 does.
- **Advisor.** `combat-systems-advisor`. **Depends.** AB-04.

### AB-09d. Resist rolls (D-AB13)

- **Change.** Resist-roll effects gate the effects that share their `effect_sequence` step, using D-AB13's model.
- **Tests.** Unit with a seeded RNG: the gate, the cap, the stat scaling.
- **Advisor.** `combat-systems-advisor`. **Depends.** D-AB13 (BlockedDecision), AB-09a/b.

### AB-10. Shields and cleanses

- **Change.** Shield effects get `ShieldAmount`/`ShieldType` rows from text where a number exists, and density/mitigation shields become ledger stat entries; `AbsorbShield`'s pool is removed on expiry. Cleanse effects ("Purges 2 Mental and 2 Health effects") map to `RemoveEffects` categories over ledger and pulsing entries.
- **Tests.** Unit: a shield drains then expires to 0; a cleanse removes exactly the named count.
- **Advisor.** `combat-systems-advisor`. **Depends.** AB-04.

### AB-11. Heal numbers, clear flags and category cooldowns

- **Change.** Per AB-E1 (D-AB14): `onEffectResults` with a positive delta for beneficial results. `EF_ClearOnDamage`, `EF_ClearOnRez` and `EF_RemoveOnBandolierSlotChange` constants and hooks (damage seams, respawn/revive, slot swap) over the ledger. Category (type 8) cooldown timers sent where monikers share a cooldown.
- **Tests.** Wire-format for the heal result and the type-8 timer. Unit per clear hook.
- **Advisor.** `combat-systems-advisor`. **Depends.** AB-E1, AB-04.

## Close-out

### AB-13. Close-out and owner UAT

- Update `docs/gap-analysis/core-gameplay.md` §9-§11 (including the TCM_Group/Aura correction), `docs/project-status.md`, and add the campaign's UAT steps to `docs/guides/unified-uat.md`.
- Re-run the family classifier against the seed and the code and publish the new working count in the README.
- `/release`, then the owner runs the README's four UAT milestones on the colo and the coordinator reads SigNoz for the `abilities` and `vitals` targets.
