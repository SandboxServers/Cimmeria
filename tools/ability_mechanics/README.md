# tools/ability_mechanics: effect NVP generator

`effect_nvps_from_desc.py` turns the designer text of an effect (`effect_desc`, such as "+10% Health" or "Heals 35% of target's Focus pool") into the effect NVP rows the server reads, and binds the script that reads them. It is decision D-AB06 of the [ability-mechanics campaign](../../docs/analysis/ability-mechanics/README.md), built in packet AB-02 ([work packets](../../docs/analysis/ability-mechanics/work-packets.md)).

Every row it writes is **RECONSTRUCTION**: the 2009 effect rows shipped no NVP for these effects, so each value is the number in the effect's own text, not recovered server data. Each generated row's comment quotes that text.

## Inputs and outputs

| | Path |
|---|---|
| Reads | `db/resources/Effects/Seed/effects.sql` (the text, pulse shape, target collection and current `script_name` of every effect) |
| Reads | `db/resources/Abilities/Seed/abilities.sql` (ability type and tooltip, for the scope rules and the tooltip cross-check) |
| Reads | `db/resources/Archetypes/Seed/archetype_ability_tree.sql` and `char_creation_abilities.sql`: the player-reachable abilities. Only their effects are considered |
| Writes | `db/resources/Effects/Seed/effect_nvps.sql`: one block per family between `-- ability-mechanics generated <family> begin` and `end` markers, with `nvp_id` in the family's reserved range |
| Writes | `db/resources/Effects/Seed/effects.sql`: the `script_name` of each generated effect (only that column of only those rows) |

`effects.sql` loads after `effect_nvps.sql` in `db/database.sql`, which is why the script binding is written into the effect row itself rather than as an `UPDATE` inside the block.

## Run it

From the repo root, with stock Python 3:

```bash
python tools/ability_mechanics/effect_nvps_from_desc.py            # regenerate
python tools/ability_mechanics/effect_nvps_from_desc.py --check    # exit 1 if the committed seed drifted
python tools/ability_mechanics/effect_nvps_from_desc.py --report   # generated, hand-authored and unparsed effects
python tools/ability_mechanics/effect_nvps_from_desc.py --family heal   # one family only
python tools/ability_mechanics/effect_nvps_from_desc.py --report --family damage > tools/ability_mechanics/reports/damage.txt
python -m unittest discover -s tools/ability_mechanics -p "test_*.py"   # test_stat_family.py is the stat family's
```

Exit codes: 0 success, 1 drift (`--check`), 2 input failure (an unparseable seed, an unmatched marker, a family out of `nvp_id`s). Output is deterministic, and a run keeps each file's CRLF line endings.

`reports/damage.txt` is the committed `damage` report: the generated effects with their notes, and every unparsed effect with its reason. A unit test fails when it no longer matches the parser, so a grammar change ships with the reviewed list. Regenerate it with the command above once the seed is current.

CI runs `--check` and the unit tests in the `test-live-db` job of [.github/workflows/test.yml](../../.github/workflows/test.yml), so a hand edit inside a block, or a parser change without a regenerated seed, fails the PR. The live-DB guards in `crates/cell-effect-scripts/src/cell/effects/heal_seed_live_db_tests.rs` (heal) and `stat_buff/seed_live_db_tests.rs` (stat) load the real seed and run the bound scripts on it; `crates/cell-combat/src/cell/abilities/damage_apply/damage_seed_live_db_tests.rs` checks the damage rows and fires Point Blank Shot on them.

## Ownership rules

- Rows outside the markers are hand-authored, and the tool never rewrites them.
- An effect that already has a hand-authored NVP of one of the family's names is left alone. The report says whether the parser agrees with it (it does for 659, 2008, 1383 and 3211).
- An effect whose `script_name` is set, and was not set by the family's committed block, is left alone and reported. A script the family bound earlier and no longer generates is cleared back to NULL on the next run.
- Hand ownership comes from the rows outside the markers alone. An effect the family generated, which then gains a hand row of the family's NVPs, keeps its `script_name`, even if its text no longer parses or its ability is no longer reachable. Removing that hand row later does not hand the effect back: its script now counts as hand-authored, and the report lists it.
- `nvp_id` ranges are fixed by the campaign ledger: heal 20000-20999, damage 21000-22999, stat 23000-23999, shield 24000-24499. Allocation skips every id used outside the family's block (a generated row moved out keeps its id), and any duplicate `nvp_id` in the file fails the run with exit 2, `--check` included.

## Families

| Family | Packet | NVPs | Script |
|---|---|---|---|
| `heal` | AB-02 | `HealPercentage` (percent of the pool's max) or `HealAmount` (flat points) | `HealHealth` / `HealFocus` |
| `damage` | AB-03 | `HealthDamage`, `FocusDamage` | none (NVP path) |
| `stat` | AB-04 | stat names (`Accuracy`, `Defense`, `CoverDefense`, `MovementSpeedMod`, ...) | `TimedStat` |
| `shield` | AB-10 | `ShieldAmount`, `ShieldType` | `AbsorbShield` |

A family is one module under `families/` that subclasses `family.Family` (`is_candidate`, `parse`) and one entry in `families/__init__.py`. The corpus loader (`corpus.py`), the seed reader (`seed_sql.py`), the ownership rules, the block writer, `--check` and `--report` are shared.

`routing.py` models where the server lands an effect: the AB-07 rules of `route_effect` in `crates/cell-combat/src/cell/abilities/effect_routing/mod.rs`, which is the authority ([decision 35](../../docs/architecture/abilities-and-effects-decisions-23-33.md#35-each-effect-of-a-cast-lands-where-its-routing-says-ability-mechanics-ab-07)). The families' scope rules read it, so a row is bound only where the cast lands it where its text says. Change both together.

### heal

The grammar accepts one heal clause per effect, as a whole line: "Heals N% of (the) player's/target's Health/Focus pool", "(Target) +N% Health/Focus", "N% Focus Heal", "Heals N health", and "Health Increase N%". Targeting lines ("Single Target") are skipped, and cost lines ("Energy -25") are noted but not modelled (B-04). Any other line rejects the effect.

Over time: `HealHealth` and `HealFocus` run once per pulse, and the pulsing layer fires exactly `pulse_count` pulses. So a total ("over 25 seconds") is divided by `pulse_count` after checking `pulse_count x pulse_duration` against it, and a rate ("per second", "Channeled: 1 Second interval") is written as is once its interval equals `pulse_duration`. A percentage share must be exact at two decimals.

A heal is reported instead of bound when the current pipeline would land it on the wrong entity:

- a `TCM_Group` or `TCM_Aura` effect (D-AB12; today it would heal the one target), a cone, or a `TCM_AERadius` effect of a ground ability (the ground collector takes hostiles). A radius heal of any other ability is bound: AB-07 fans it out to the caster's allies (Morale Boost's 1215);
- a "Deployable:" ability's pulse effect (it needs a `resources.deployables` binding);
- a revive (D-AB11);
- the heal half of an ability that is neither Heal nor Buff (it belongs on the user, B-27);
- a bare "+N% Focus" on a Buff ability, which may be a max-pool buff (the `stat` family's).

When the ability tooltip states a different number, the effect row wins and the comment says so, as for pet effect 3211.

### damage

The grammar reads one amount per pool: "-200F / -20H", "-800 F / -80 H", "F-200 H-20", "Target F-100 / H-10", "Focus Damage: -200", or one pool per line ("-200F" then "-20H"). A pool given twice, or an amount with no minus sign ("-800F / 80H"), rejects the effect. Targeting lines ("Single Target", "Medium Cone", "Short Radius AE", "Secondary Targets", "Frag Grenade Damage:") are read for the shape check below; ammo and energy costs are noted, not modelled (B-04); "Increased Threat +N" and "Energy Return: N%" are noted as not modelled.

No script is bound. Each row feeds the NVP damage path in `crates/cell-combat/src/cell/abilities/damage_apply/` ([combat-system.md, Damage sources](../../docs/gameplay/combat-system.md#damage-sources)), where each `TCM_Single` effect resolves on its own and a DoT tick re-reads the same row. So a DoT's row is its per-tick amount, which is how the text states it ("DOT: -150F -30H (8 Ticks)"), once the stated tick count equals `pulse_count` (and a stated interval, "x1 Second", equals `pulse_duration`). "Per tick" on a single-shot row writes one tick, with a note that channel ticks are not modelled.

An effect is reported, not written, when its rejection reason starts with one of these categories (the report counts them):

- `conditional`: the variant applies only against some targets or from some position: "Mechanical Target Damage", "Flank Position Damage", "Assassin Stance Bonus Damage", "while moving". No conditional NVP exists, and the pipeline would apply it on every hit.
- `sequenced`: a single-shot effect with `EF_SequenceOnFinish` (64), the follow-up of a check (Execution's and Red Mist's damage vs low Focus), a chain jump (Energy Cascade) or an extra shell (Grenade Barrage). A pulsing one (Lethal Shot's DoT) is still written.
- `targeting`: the text names a shape the row does not have, such as "Medium Cone" on a `TCM_Single` row, which would land a second hit on the primary.
- `pulse shape`: the text's tick count or interval disagrees with the row ("Channeled: 50 ticks" on a single-shot row), or a pulsing row states no count.
- `scope`: the damage half of a Buff ability, or an `EF_ResolveOnAbilityUser` effect: the server never routes damage onto the user (AB-07).
- `grammar`: anything else the parser refuses.

When the ability tooltip's numbers differ from the effect's, the effect row wins and the comment says so.

### stat

`families/stat.py` writes stat-named NVPs (`Accuracy`, `Defense`, `CoverAccuracy`, `CoverDefense`, `CrouchingDefense`, `Response`, `InterruptResistance`, the three resists, `MovementSpeedMod`) and binds `TimedStat`, which puts one entry per effect and caster on the timed effect ledger for the effect's `pulse_duration` (AB-04, D-AB08). The names it may write sit between `# nvp-names` markers, and a Rust test (`stat_nvp_names_match_the_generator`) fails if the script would ignore one.

The grammar accepts stat clauses as whole lines, after an optional `Target`, `User` or `Debuff` prefix: "+200 Accuracy: 15 Seconds", "+200 Cover ACC for 15 Seconds", "Accuracy -100", "Movement Speed-30%", "Cover Defense Debuff: -100", and pairs such as "-200 ACC / DEF" or "-200ACC / -200DEF". Targeting lines and duration lines ("10 Second Duration", "Duration: 15sec") are skipped; a stated duration must equal `pulse_duration`.

Units are D-AB09's, and every converted row carries a `note` saying how: a bare number is points (200 Accuracy is 2 QR); a percentage on a resist or interrupt stat is 10 points per 1 %; a percentage on run speed is `movementSpeedMod` percent. Any other percentage is rejected.

Reported instead of bound:

- anything that is not a timed single pulse: held effects (stances, toggles), `EF_AlwaysPersist` passives and `AF_TOGGLED` abilities are AB-08's;
- `EF_ClearOnDamage` effects ("(1 hit)"), until AB-11's damage hook exists;
- a cone, a group or aura effect (D-AB12), a hostile `TCM_AERadius` effect of a non-ground ability (it lands on the one target) or a beneficial one of a ground ability (the ground collector takes hostiles), and "Secondary" effects; deployables and turret enhancements (D-AB11);
- the regen stats (AB-05: `regen.rs` reads them as points per second until D-AB04's percentage model lands), pool maximums, armour factors, mitigation and stealth;
- an effect the cast would land on its target when its text puts it elsewhere: a "User" half with no `EF_ResolveOnAbilityUser` on an ability that is not a pure Self ability, a single effect of a Self ability that has an area effect (a follow-up of the area hit), and a beneficial effect whose ability has a non-beneficial effect that does something (the cast would take the hostile path, B-27).

An effect the server routes off the target is bound whatever the rest of its ability does: a single effect of a pure Self ability lands on the user (Combat Sprint's "-100 ACC" 2002, TimeShift's 4779), as does an `EF_ResolveOnAbilityUser` effect, and a beneficial radius effect of a non-ground ability lands on the caster's allies. A ground ability's hostile radius effect reaches every ground target (Forward Observer's 948).

When the ability tooltip names a different stat from the effect row, the row wins and the comment says so.

## Changing a family

Edit the parser, run the tool, run the unit tests, and commit the parser and both seed files together. Check `--report` for effects that moved between generated and unparsed, and update the live-DB guards when the generated set changes on purpose.
