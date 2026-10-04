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
python -m unittest discover -s tools/ability_mechanics -p "test_*.py"
```

Exit codes: 0 success, 1 drift (`--check`), 2 input failure (an unparseable seed, an unmatched marker, a family out of `nvp_id`s). Output is deterministic, and a run keeps each file's CRLF line endings.

CI runs `--check` and the unit tests in the `test-live-db` job of [.github/workflows/test.yml](../../.github/workflows/test.yml), so a hand edit inside a block, or a parser change without a regenerated seed, fails the PR. The live-DB guards in `crates/cell-effect-scripts/src/cell/effects/heal_seed_live_db_tests.rs` load the real seed and run the bound scripts on it.

## Ownership rules

- Rows outside the markers are hand-authored, and the tool never rewrites them.
- An effect that already has a hand-authored NVP of one of the family's names is left alone. The report says whether the parser agrees with it (it does for 659, 2008, 1383 and 3211).
- An effect whose `script_name` is set, and was not set by the family's committed block, is left alone and reported. A script the family bound earlier and no longer generates is cleared back to NULL on the next run.
- `nvp_id` ranges are fixed by the campaign ledger: heal 20000-20999, damage 21000-22999, stat 23000-23999, shield 24000-24499.

## Families

| Family | Packet | NVPs | Script |
|---|---|---|---|
| `heal` | AB-02 | `HealPercentage` (percent of the pool's max) or `HealAmount` (flat points) | `HealHealth` / `HealFocus` |
| `damage` | AB-03 | `HealthDamage`, `FocusDamage` | none (NVP path) |
| `stat` | AB-04 | the stat names `StatBuff` reads | the ledger script |
| `shield` | AB-10 | `ShieldAmount`, `ShieldType` | `AbsorbShield` |

A family is one module under `families/` that subclasses `family.Family` (`is_candidate`, `parse`) and one entry in `families/__init__.py`. The corpus loader (`corpus.py`), the seed reader (`seed_sql.py`), the ownership rules, the block writer, `--check` and `--report` are shared.

### heal

The grammar accepts one heal clause per effect, as a whole line: "Heals N% of (the) player's/target's Health/Focus pool", "(Target) +N% Health/Focus", "N% Focus Heal", "Heals N health", and "Health Increase N%". Targeting lines ("Single Target") are skipped, and cost lines ("Energy -25") are noted but not modelled (B-04). Any other line rejects the effect.

Over time: `HealHealth` and `HealFocus` run once per pulse, and the pulsing layer fires exactly `pulse_count` pulses. So a total ("over 25 seconds") is divided by `pulse_count` after checking `pulse_count x pulse_duration` against it, and a rate ("per second", "Channeled: 1 Second interval") is written as is once its interval equals `pulse_duration`. A percentage share must be exact at two decimals.

A heal is reported instead of bound when the current pipeline would land it on the wrong entity:

- a `TCM_AERadius`, `TCM_Group` or `TCM_Aura` effect (routing is AB-07 and D-AB12; today it would heal the one target twice);
- a "Deployable:" ability's pulse effect (it needs a `resources.deployables` binding);
- a revive (D-AB11);
- the heal half of an ability that is neither Heal nor Buff (it belongs on the user, B-27);
- a bare "+N% Focus" on a Buff ability, which may be a max-pool buff (the `stat` family's).

When the ability tooltip states a different number, the effect row wins and the comment says so, as for pet effect 3211.

## Changing a family

Edit the parser, run the tool, run the unit tests, and commit the parser and both seed files together. Check `--report` for effects that moved between generated and unparsed, and update the live-DB guards when the generated set changes on purpose.
