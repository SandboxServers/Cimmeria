# Ability Mechanics

> Type: how-to. Audience: Claude Code coordinator and implementing engineers.
> Updated: 2026-10-03. Companions: [evidence audit](audit.md), [work packets](work-packets.md), [full telemetry and lab UAT plan](lab-uat-and-telemetry.md), [telemetry coverage matrix](telemetry-coverage.md), [ability trees campaign](../ability-trees/README.md), [combat formulas status](../../reverse-engineering/findings/combat-formulas-status.md), [abilities ADR](../../architecture/abilities-and-effects-system.md), [documentation index](../../readme.md).

## Purpose

The ability-trees campaign made 439 tree nodes trainable and left their mechanics out of scope. This campaign makes the abilities a player can actually hold do what their tooltips say: self heals and focus restores first, then focus regeneration, damage numbers, timed buffs and debuffs, toggles and stances, and crowd control.

A playtester asked for "self heals, focus regen, etc." The audit found the problem is wider than those two. Of the **423** abilities a player can own (the 5 starters plus the 419 distinct tree ids), about **24 do what they say today**. The rest charge a cooldown and do nothing, deal a 0-damage hit that aggroes the target, or play an animation and nothing else.

Out of scope:

- formula recovery: QR curves, armour, mitigation, resist maths. No shipped artefact contains them ([combat-formulas-status.md](../../reverse-engineering/findings/combat-formulas-status.md)); this campaign uses the designer numbers and leaves the curves alone;
- the enemy-combat blockers (`MITIGATION` 0/0, forced `DT_PHYSICAL`): their own campaign;
- stealth, disguise and reveal (20 abilities), resurrection (1), Asgard energy costs (27), turret summons and the remaining pet kits: each needs a subsystem that does not exist yet (D-AB11);
- NPC regeneration and NPC ability use;
- effect persistence across logout (B-38), after the timed-instance ledger exists.

## What was found

Against `main` @ `b40ef76d2`. Counts are from a keyword classifier over the seed (good to a few rows either way); every row in the [audit's spot-check table](audit.md#8-representative-abilities-by-family) was checked by hand.

| Family | Abilities | Working today | Why the rest fail (audit rows) | Packets |
|---|---:|---:|---|---|
| Heal self / other | 4 | 0 | Starter 1646 has its script and number but never reaches the caster or an ally: Self is not substituted, target 0 is a no-op, self/ally is refused by #444, a hostile target heals the hostile. Field Medic and Battlefield Heal have no script or number. (B-10 to B-15) | AB-01, AB-02 |
| Focus restore | 10 | 0 | Starter 597 Heal Focus: the same targeting failure. The rest have no script or number; Morale Boost's AE half never fans out. (B-12 to B-14, B-28) | AB-01, AB-02, AB-07 |
| HoT | 2 | 0 | Starter 1218 Recuperation is scripted and pulses, but is never targeted. (B-12 to B-14) | AB-01, AB-02 |
| Regen buff | 2 | 0 | Single-pulse timed effects never register, and carry no number. (B-30, B-54) | AB-04, AB-05 |
| Direct damage | 91 | 2 | 89 have damage only in text ("-200F / -20H") and hit for 0 while still pulling aggro. The two that work (592, 594) apply damage twice, and their script damages on a miss. (B-20 to B-23) | AB-03, AB-06 |
| DoT | 6 | 0 | Pulses register; each deals 0. (B-20) | AB-03 |
| Stat buff | 11 | 0 | No timed instance for single-pulse effects; the stat ledger only knows six attributes. (B-30, B-32) | AB-04 |
| Stat debuff | 12 | 0 | Same, delivered on a 0-damage hostile hit. (B-30, B-32) | AB-04 |
| Crowd control | 87 | 0 | Snare, knockdown, stun, suppression, interrupt: no timed instance, Stun leaks (#1049), NPCs ignore the movement lock, resist rolls unmodelled. (B-40 to B-43) | AB-09a-d |
| Shield / absorb | 15 | 0 | No numbers; `AbsorbShield`'s own authoring note uses the shape that never registers. (B-33) | AB-10 |
| Stance / toggle | 33 | 1 | `AF_TOGGLED` is honoured only for pet abilities; nothing removes by moniker. Cover Stance works through the cover hold. (B-35) | AB-08 |
| Passive | 46 | 0 | Only pet passive scripts run; 19 `EF_AlwaysPersist` effects never apply. (B-36) | AB-08 |
| Cleanse | 5 | 0 | `RemoveEffects` exists for darts only. | AB-10 |
| Movement | 2 | 0 | Combat Sprint: timed effect, plus a "User" half the server cannot route. (B-27, B-30) | AB-04, AB-07 |
| Summon / pet / deployable | 41 | 9 | Four pet summons, four owner-pet abilities and the Microwave Emitter work (pets and deployables campaigns). Turrets have no summon row. | out of scope |
| Ammo and dart toggles | 12 | 12 | Shipped by the [ammo campaign](../ammo/README.md). | none |
| Stealth / disguise / reveal | 20 | 0 | No stealth system. | out of scope |
| Resurrect | 1 | 0 | No self-revive path; a dead player cannot cast. | out of scope |
| No effect rows | 23 | 0 | Mines, Smoke Grenade, Incinerate Ground: the seed has no effects for them. | AB-12 (feedback only) |
| **Total** | **423** | **24** | | |

Cross-cutting findings:

| Area | State on `main` | Packets |
|---|---|---|
| Focus cost | **Not a mechanic.** No cost column, no client cost display, no python deduction; Focus is SGW's damage shield, not mana (B-04, B-05). Nothing to build; the decision records it. | D-AB05 |
| Regeneration | 1 point/s per pool, out of combat only: Focus refills in about 26 minutes. `alias.xml` defines the regen stats as percentage modifiers of a base rate, which no artefact records; the content implies Focus recharges in combat. (B-50 to B-53) | AB-05 |
| Feedback | Every no-op press is silent: target 0, the #444 refusal, and the 306 abilities with no mechanic. (B-60) | AB-01, AB-12 |
| Floating heal numbers | Heals send no `onEffectResults`. (B-61) | AB-E1, AB-11 |
| Telemetry | No ability heal has fired in 30 days of colo logs; no `useAbility` for 597/1646/1218 in 14 days. Either nobody pressed them or the client suppressed the press. (B-15) | AB-E1 |
| Effect flags | `EF_DONT_USE_QR` is 32 (should be 16) and unread; the other `EF_*` "categories" are not client bits; ClearOnDamage, ClearOnRez and RemoveOnBandolierSlotChange have no hook. (B-24, B-25, B-37) | AB-06, AB-11 |
| Category cooldowns | Type 8 timers never sent. (B-63) | AB-11 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-AB01 | PROPOSED | A `TargetSelf` (type 1) ability resolves on the caster, whatever target id the client sent. | The client sends its current target for every non-ground ability (B-10). Python did exactly this (`AbilityManager.py:527`). Server-authoritative, no client patch. Confidence high. |
| D-AB02 | **NEEDS OWNER** (proposed default given) | An ability is **beneficial** when every effect that does something carries `EF_Beneficial_Effect` (bit 1), or its `type_id` is `ABILITY_TYPE_Heal`. A beneficial `TargetTarget` cast lands on the target when it is the caster or an ally (the support-shot rule, `support_shot::classify`); on a hostile, a neutral NPC or no target it **falls back to the caster**. No QR roll, no threat, no in-combat state, no #444 refusal. | The fallback is the choice the owner should confirm. Alternative: refuse with `onErrorCode` and a feedback line. Fallback matches the tooltips ("Heals 10% of the player's Health pool" on 1646 and 742) and gives every press a visible result; refusal is stricter but turns the starter heal into a targeting puzzle. Heals on friendly NPCs and pets stay refused, as for support darts (ammo D-AM). |
| D-AB03 | **NEEDS OWNER** | Regeneration base rates. Proposed: **Focus** 3 % of max per second out of combat and 1 % per second in combat, starting 4 s after the player last took Focus damage; **Health** 1 % of max per second out of combat only. Each scaled by `1 + (regenStat + morale x 0.5) / 100` for Focus and `1 + healthRegen / 100` for Health. Ticked at 1 Hz, minimum 1 point. All constants in one module, labelled DESIGN, never "retail". | No retail value exists (B-52). The numbers make Focus a recharging shield (about 33 s to full out of combat, a slow trickle in a fight) and Health the resource that heals and consumables restore, which is what the content implies (B-53). Today's 26-minute Focus refill makes the shield one-use. |
| D-AB04 | PROPOSED | `healthRegen`, `focusRegen` and `energyRegen` are percentage modifiers of the base rate, and `morale` adds 0.5 % Focus regen per point, as `alias.xml` defines them. | ORIGINAL-DATA (B-51). `regen.rs` reads the stat as points per second, which no artefact supports. |
| D-AB05 | PROPOSED (owner to confirm the playtester's expectation) | No Focus cost on any ability. Asgard "Cost: X Gigajoules" stays unmodelled until an Asgard energy design exists (D-AB11). | No cost column, no client cost display, no python deduction (B-04). Inventing one would change every archetype's balance on no evidence. |
| D-AB06 | PROPOSED | Designer numbers become NVP rows through **one checked-in generator** that parses `effect_desc` and writes `HealthDamage`, `FocusDamage`, `HealPercentage`, `HealAmount` and stat-named rows into `db/resources/Effects/Seed/effect_nvps.sql`, each block commented RECONSTRUCTION with its source text. An effect whose text does not parse cleanly gets no row and is listed in the generator's report. | The repo already does this by hand (`HealAmount` from desc in `heal.rs`, the stimpack rows, nvp 18/19/100/200). 229 damage, 22 heal and 39 stat effects need it (B-03). A generator is reviewable and re-runnable; 290 hand edits are not. |
| D-AB07 | PROPOSED | When a damage effect has a damage script, **the script is the only damage path**: the NVP pipeline skips it. A QR-rolled effect whose roll misses runs no script. An effect with `EF_DontUseQR` (bit 16) never rolls. | Removes the double damage on Pistol Shot and Strike and the miss-still-bleeds bug (B-22, B-23, B-24). It changes starter damage numbers; the playtest numbers after it are the new baseline. |
| D-AB08 | PROPOSED | A single-pulse effect with a duration (`pulse_count = 1`, `pulse_duration > 0`) and a held effect on a toggle (`pulse_duration = 0` on a toggled ability) become **timed instances in a generalised stat-change ledger**: applied on land, reverted on expiry, toggle-off, removal by moniker, or death when `EF_ClearOnDeath`. The ledger extends the stimpack `stat_buffs` ledger (ADR decision 28), not the pulsing layer. | The pulsing layer would re-run `on_apply` at expiry (the reason decision 28 exists). Python reverted non-permanent changes on removal (B-31). One ledger keeps one timer and icon path (`onTimerUpdate` type 5, SecondaryId = effect id). |
| D-AB09 | PROPOSED | Designer stat units: a bare number is stat points ("+200 Accuracy" = 200 points, 2 QR per `alias.xml`); a percentage on a resist or interrupt stat is converted at **10 points per 1 %**; a percentage on run speed is `movementSpeedMod` percent; a percentage on a pool max is a percentage of max. | The 10:1 rule is stated twice in the data and nowhere contradicted (effect 2004 "+50 (5%) Mental Resist", 2005 "Subtlety -100 (10% increase to threat)"). Low n; the generator labels every converted row. |
| D-AB10 | PROPOSED | A press that cannot do anything gets feedback before any cost: `onErrorCode` with `ERRORCODE_SYSTEM_Ability` and a `CHAN_FEEDBACK` line ("That ability has no effect yet."), no cooldown charged. This covers an ability with no implemented effect and a beneficial Target cast with nothing to land on (only if D-AB02 chooses refusal). | Project rule: every press gets visible feedback. Precedent: the consumable refusal and the pet-order gate (`ability_is_unimplemented`). The predicate is tightened to ignore animation-only abilities, because an animation is not a mechanic. |
| D-AB11 | PROPOSED (owner may pull any of them in) | Stealth, disguise and reveal; self-revive (1190 Goldam's Gift); Asgard energy; turret summons; NPC regeneration; effect persistence across logout stay out of this campaign. Their presses get D-AB10 feedback meanwhile. | Each needs a subsystem that does not exist. Showcase priority is heals, regen, damage, buffs and CC. |
| D-AB12 | **NEEDS OWNER** | `TCM_Group` and `TCM_Aura` beneficial effects (13 reachable) apply to the caster and to the caster's squad members in the same space within the effect's radius tier (default Medium, 10 m). | Squads exist server-side since the Organizations campaign, so "group" can mean squad. The alternative, caster only, is simpler and loses nothing for solo play. |
| D-AB13 | **NEEDS OWNER** | Resist rolls: until a resist model is chosen, a resist-roll effect always passes (the CC lands). Proposed model when it is built: the target resists with probability `resistStat / 1000` (the D-AB09 10:1 rule), capped at 75 %, rolled once per gated `effect_sequence` step. | The shipped structure says resists are gating rolls (B-43) and the maths is UNKNOWN. Always-land is the honest placeholder; the probability model is DESIGN and should be the owner's call. |
| D-AB14 | PROPOSED | A positive heal sends `onEffectResults` with a positive Health or Focus delta only after AB-E1 confirms the client renders it without fault; until then heals show as bar movement only. | An effect result between two players outside a duel is unverified against the client (ammo known issue). |

PROPOSED rows are adopted at their defaults unless the owner objects. A change is recorded as a new row, never by editing an old one.

## Coordinator launch prompt

You are the Claude Code coordinator for the ability-mechanics campaign. Implement [work-packets.md](work-packets.md) as small reviewed PRs.

1. Record `git rev-parse origin/main` and re-check the audit's file and line references. Check `~/.claude/sessions/*.json` for a live peer on `abilities/*` branches; if one exists, message it and stand down.
2. Get the owner's answers to D-AB02, D-AB03, D-AB12 and D-AB13. Wave 0 needs only D-AB02; start it with the proposed default if the owner has not answered, because the fallback and the refusal share every line except one branch.
3. **Wave 0:** dispatch AB-E1, AB-01, AB-02 and AB-06 in parallel worktrees. AB-01 is the playtester's ask and merges first.
4. **Wave 1:** AB-03 (after AB-02's generator), AB-04, AB-05 (after D-AB03), AB-07, AB-12.
5. **Wave 2:** AB-08, AB-09a-d, AB-10, AB-11. Then AB-13 close-out.
6. Give each worker only its packet, the contract section and the audit rows it cites. Every worker prompt names the build lane, its `sgw_<worktree>` database, and `ship.sh pr -C <worktree>` to open the PR.
7. When blocked, leave `handoffs/<packet>.md` with the exact next action.

## UAT milestones

Add these to [guides/unified-uat.md](../../guides/unified-uat.md) when the first packet merges (the guide follows the ledgers; it is not edited before anything ships).

1. **After Wave 0 (heals):** on the colo, as a fresh character of any archetype: Heal Focus with no target, with yourself, with a mob, with another player targeted. Health Heal the same four ways. Recuperation on another player. Every press visibly heals someone or answers with a feedback line; the mob's bars never rise. SigNoz: `heal_health`, `heal_focus` with `ability_id` 597/1646/1218, and AB-01's `beneficial_cast` row (`stage`, `wire_target_id`, `resolved_target_id`, `resolution`). Steps AB-U1 to AB-U5 in the [unified UAT guide](../../guides/unified-uat.md#ability-mechanics).
2. **After AB-05 (regen):** take Focus damage, stand still, watch Focus return within about half a minute out of combat and slowly in combat. SigNoz: `regen_started` with the per-second amounts.
3. **After Wave 1 (damage and buffs):** each archetype's tree root deals its tooltip damage; Aim shows a 15 s buff icon that expires; Leadership speeds Focus regen for 20 s.
4. **After Wave 2 (toggles and CC):** stance on, stance off, switch stance; Snare Shot slows a mob for 15 s; Takedown knocks it down.

## Where confidence is low

- What the client sends, and whether it suppresses the press, for Heal Focus (B-15). AB-E1 answers it; until then AB-01's tests prove only the server side.
- The `onEffectResults` result-code numbering (B-62). Never change it without a capture.
- Every number the generator writes is RECONSTRUCTION from designer text, and roughly one row in ten will need a human look (text such as "-800F / 80H" with a missing sign, ranges, "vs Mechanical" variants).
- D-AB03 regen rates, D-AB09 unit conversions and D-AB13 resist odds are design, not recovery.
- The family counts come from a keyword classifier; a few abilities sit in the wrong family. The per-family fixes do not depend on the exact count.
