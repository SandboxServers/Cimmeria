"""Tests for the ability-mechanics effect NVP generator.

Stock ``unittest`` (CI: ``python3 -m unittest discover -s tools/ability_mechanics
-p "test_*.py"``). The parser cases use the real designer strings from
``db/resources/Effects/Seed/effects.sql``; the ownership cases use a small
synthetic seed so each rule is tested on its own.
"""

from __future__ import annotations

import io
import sys
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import effect_nvps_from_desc as gen  # noqa: E402
from corpus import Ability, Effect, corpus_from_texts  # noqa: E402
from families.heal import HealFamily, parse_heal, scope_rejection  # noqa: E402
from family import Generated, Rejected  # noqa: E402


def effect(desc, pc=1, pd=0.0, tcm="TCM_Single", ability_id=1, effect_id=1, script=None):
    return Effect(effect_id, ability_id, "e", desc, 0, pc, pd, tcm, script, (0, 0))


def ability(type_id="ABILITY_TYPE_Heal", name="a", description=""):
    return Ability(1, name, description, type_id)


def heal(desc, pc=1, pd=0.0, type_id="ABILITY_TYPE_Heal", tooltip=""):
    return parse_heal(effect(desc, pc, pd), ability(type_id, description=tooltip))


class HealGrammarOnRealStrings(unittest.TestCase):
    """Every accepted shape, on the effect text it was written for."""

    def assert_heal(self, outcome, script, name, value):
        self.assertIsInstance(outcome, Generated, getattr(outcome, "reason", ""))
        self.assertEqual(outcome.script, script)
        self.assertEqual(outcome.nvps, [(name, value)])

    def test_the_hand_authored_starter_heals_reproduce(self):
        # 597 Heal Focus / 659, 1646 Health Heal / 2008, 1218 Recuperation /
        # 1383: the parser must agree with the rows PR #496/#497 wrote by hand.
        self.assert_heal(heal("Heals 35% of players Focus pool"), "HealFocus", "HealPercentage", "35.00")
        self.assert_heal(heal("+10% Health"), "HealHealth", "HealPercentage", "10.00")
        self.assert_heal(
            heal("Heals 75% of target's Health pool over 25 seconds.", pc=25, pd=1.0),
            "HealHealth",
            "HealPercentage",
            "3.00",
        )

    def test_target_bare_percent(self):
        self.assert_heal(heal("Single Target\nTarget +35% Focus"), "HealFocus", "HealPercentage", "35.00")
        self.assert_heal(heal("Single Target\nTarget +10% Health"), "HealHealth", "HealPercentage", "10.00")

    def test_percent_focus_heal(self):
        self.assert_heal(heal("Single Target\n35% Focus Heal"), "HealFocus", "HealPercentage", "35.00")

    def test_heals_of_targets_pool(self):
        self.assert_heal(heal("Heals 20% of target's Focus pool"), "HealFocus", "HealPercentage", "20.00")

    def test_flat_heal(self):
        self.assert_heal(heal("Heals 500 health."), "HealHealth", "HealAmount", "500")

    def test_channel_rate_is_per_pulse(self):
        # 4085 Lord's Vitae: 20 pulses of 1 s, "per second" in the tooltip.
        out = heal(
            "Heals 10% of target's Health pool\nChanneled: 1 Second interval\nEnergy -25",
            pc=20,
            pd=1.0,
            tooltip="Melee Range: Channeled\nHeals 10% of the target's Health pool per second",
        )
        self.assert_heal(out, "HealHealth", "HealPercentage", "10.00")
        self.assertTrue(any("cost" in n for n in out.notes))
        self.assertFalse(any("tooltip" in n for n in out.notes), "tooltip agrees")

    def test_tooltip_disagreement_is_noted_not_fatal(self):
        # 789 Field Medic II: effect "+20% Health", tooltip "10%".
        out = heal("+20% Health", tooltip="Heals 10% of the player's Health pool")
        self.assert_heal(out, "HealHealth", "HealPercentage", "20.00")
        self.assertTrue(any("tooltip" in n for n in out.notes))


class HealGrammarRejects(unittest.TestCase):
    def assert_rejected(self, outcome, needle):
        self.assertIsInstance(outcome, Rejected, outcome)
        self.assertIn(needle, outcome.reason)

    def test_bare_percent_on_a_buff_could_be_a_max_pool_buff(self):
        # 2134 Stance: Courage "+15% Focus" (tooltip: Maximum Focus +15%).
        self.assert_rejected(heal("+15% Focus", type_id="ABILITY_TYPE_Buff"), "max-pool buff")

    def test_stated_pulses_must_match_the_row(self):
        # 4781 Convert Energy: text 20 pulses, row pulse_count 1.
        self.assert_rejected(
            heal("Health Increase 5%\nPer Pulse\n20 pulses\n15 Energy per pulse"), "pulse_count is 1"
        )

    def test_unrecognised_line(self):
        # 4140 Submit to your Lord.
        self.assert_rejected(heal("Health Health + 5% health"), "unrecognised line")
        # 3350 (pet, hand-authored): "Turret Full Heal: +100% Health".
        self.assert_rejected(heal("Turret Full Heal: +100% Health", pc=10, pd=0.5), "unrecognised line")

    def test_over_time_text_on_a_single_pulse_row(self):
        self.assert_rejected(heal("Heals 75% of target's Health pool over 25 seconds."), "pulse_count is 1")

    def test_total_must_match_pulse_count_times_duration(self):
        self.assert_rejected(
            heal("Heals 75% of target's Health pool over 25 seconds.", pc=25, pd=2.0), "pulse_count x pulse_duration"
        )

    def test_pulsing_row_with_no_rate_or_total(self):
        self.assert_rejected(heal("+10% Health", pc=10, pd=0.5), "no rate or total")

    def test_share_must_be_exact_at_two_decimals(self):
        self.assert_rejected(heal("Heals 10% of target's Health pool over 3 seconds.", pc=3, pd=1.0), "two decimals")

    def test_two_clauses(self):
        self.assert_rejected(heal("+10% Health\n+10% Focus"), "2 heal clauses")

    def test_channel_interval_must_match_pulse_duration(self):
        self.assert_rejected(
            heal("Heals 10% of target's Health pool\nChanneled: 2 Second interval", pc=20, pd=1.0), "pulse_duration is 1"
        )


class HealScope(unittest.TestCase):
    """Heals the current pipeline would land on the wrong entity."""

    def test_ae_and_group_wait_for_routing(self):
        self.assertIn("AB-07", scope_rejection(effect("35% Focus Heal", tcm="TCM_AERadius"), ability()))
        self.assertIn("AB-07", scope_rejection(effect("+ 5% Every 3 seconds", tcm="TCM_Group"), ability()))

    def test_deployables_wait_for_a_binding(self):
        self.assertIn("deployable", scope_rejection(effect("x"), ability(name="Deployable: Stim Pack")))

    def test_revive_is_out_of_scope(self):
        a = ability(description="Defeated player revives self.")
        self.assertIn("D-AB11", scope_rejection(effect("+10% Health"), a))

    def test_heal_half_of_a_damaging_ability(self):
        self.assertIn("user", scope_rejection(effect("+5% Health"), ability("ABILITY_TYPE_DD")))

    def test_a_plain_single_target_heal_is_in_scope(self):
        self.assertIsNone(scope_rejection(effect("+10% Health"), ability()))


# A synthetic seed: abilities 1 (Heal, reachable), 2 (Heal, not reachable).
EFFECT_COLS = (
    "effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, "
    "pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, "
    "is_channeled, name, target_collection_id, event_set_id, script_name"
)


def effect_row(eid, aid, desc, script="NULL"):
    return (
        f"INSERT INTO effects ({EFFECT_COLS}) VALUES ({eid}, {aid}, 0, '{desc}', 0, 16, 'i', 1, 0, "
        f"NULL, NULL, 'TCM_Single', false, false, 'n', 0, NULL, {script});\n\n"
    )


ABILITIES = "".join(
    f"INSERT INTO abilities (ability_id, name, description, type_id) VALUES ({a}, 'A{a}', '', 'ABILITY_TYPE_Heal');\n"
    for a in (1, 2)
)
STARTERS = "INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (3, 1);\n"
NVP_TRAILER = "--\n-- TOC entry 1\n--\n\nSELECT pg_catalog.setval('s', 462, true);\n"


def nvp_row(nvp_id, eid, name, value):
    return f"INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES ({nvp_id}, {eid}, '{name}', '{value}');\n"


def corpus(effects_text, nvps_text):
    return corpus_from_texts(effects_text, nvps_text, ABILITIES, [STARTERS])


class Ownership(unittest.TestCase):
    def run_gen(self, effects_text, nvps_text):
        _, results, nvps, effs = gen.generate(["heal"], corpus(effects_text, nvps_text))
        return results[0], nvps, effs

    def test_generates_a_block_and_binds_the_script(self):
        hand = "-- hand\n" + nvp_row(7, 99, "HealthDamage", "10") + "\n"
        result, nvps, effs = self.run_gen(effect_row(10, 1, "+10% Health"), hand + NVP_TRAILER)
        self.assertEqual([g.effect.effect_id for g in result.generated], [10])
        self.assertIn("(20000, 10, 'HealPercentage', '10.00')", nvps)
        self.assertIn("RECONSTRUCTION 10", nvps)
        self.assertTrue(nvps.startswith(hand), "rows before the block are untouched")
        self.assertTrue(nvps.endswith("-- ability-mechanics generated heal end\n\n" + NVP_TRAILER))
        self.assertIn("NULL, 'HealHealth');", effs)

    def test_regenerating_is_idempotent(self):
        _, nvps, effs = self.run_gen(effect_row(10, 1, "+10% Health"), NVP_TRAILER)
        result, nvps2, effs2 = self.run_gen(effs, nvps)
        self.assertEqual((nvps2, effs2), (nvps, effs))
        self.assertEqual(len(result.generated), 1, "a script it bound itself is still its own")

    def test_unreachable_abilities_are_ignored(self):
        result, _, effs = self.run_gen(effect_row(10, 2, "+10% Health"), NVP_TRAILER)
        self.assertEqual(result.generated, [])
        self.assertNotIn("HealHealth", effs)

    def test_a_hand_authored_nvp_is_left_alone(self):
        nvps = nvp_row(100, 10, "HealPercentage", "10.00") + NVP_TRAILER
        result, out, effs = self.run_gen(effect_row(10, 1, "+10% Health"), nvps)
        self.assertEqual(result.generated, [])
        self.assertEqual(result.hand_authored[0][1], "parser agrees")
        self.assertNotIn("(20000,", out)
        self.assertNotIn("HealHealth", effs)

    def test_a_hand_authored_script_is_never_overwritten(self):
        result, _, effs = self.run_gen(effect_row(10, 1, "+10% Health", "'Reload'"), NVP_TRAILER)
        self.assertEqual(result.generated, [])
        self.assertIn("hand-authored", result.rejected[0].reason)
        self.assertIn("'Reload');", effs)

    def test_a_script_it_bound_is_cleared_when_the_effect_drops_out(self):
        _, nvps, effs = self.run_gen(effect_row(10, 1, "+10% Health"), NVP_TRAILER)
        # The designer text changes so the parser now refuses it.
        effs = effs.replace("+10% Health", "+10% Health, maybe")
        result, nvps2, effs2 = self.run_gen(effs, nvps)
        self.assertEqual(result.generated, [])
        self.assertNotIn("HealHealth", effs2)
        self.assertNotIn("INSERT", nvps2.split("heal begin")[1].split("heal end")[0])

    def test_a_hand_override_keeps_the_binding(self):
        # Generate, then a human adds a HealPercentage row outside the block
        # for the same effect: the effect leaves the block, but its script
        # must stay, or the hand row heals nothing.
        _, nvps, effs = self.run_gen(effect_row(10, 1, "+10% Health"), NVP_TRAILER)
        nvps = nvp_row(500, 10, "HealPercentage", "12.00") + nvps
        result, nvps2, effs2 = self.run_gen(effs, nvps)
        self.assertEqual(result.generated, [])
        self.assertEqual([e.effect_id for e, _ in result.hand_authored], [10])
        self.assertIn("NULL, 'HealHealth');", effs2)
        self.assertNotIn("(20000,", nvps2)
        # And it stays stable on the next run.
        _, nvps3, effs3 = self.run_gen(effs2, nvps2)
        self.assertEqual((nvps3, effs3), (nvps2, effs2))

    def test_a_hand_override_keeps_the_binding_when_the_text_stops_parsing(self):
        # Same hand-over, but the designer text also changes to something the
        # heal family no longer recognises: ownership must come from the hand
        # row, not from candidacy.
        _, nvps, effs = self.run_gen(effect_row(10, 1, "+10% Health"), NVP_TRAILER)
        nvps = nvp_row(500, 10, "HealPercentage", "12.00") + nvps
        effs = effs.replace("+10% Health", "Custom restoration")
        result, nvps2, effs2 = self.run_gen(effs, nvps)
        self.assertEqual(result.generated, [])
        self.assertEqual(result.hand_authored, [], "not a candidate any more")
        self.assertIn("NULL, 'HealHealth');", effs2)
        self.assertNotIn("(20000,", nvps2)

    def test_a_hand_override_on_an_unreachable_effect_keeps_the_binding(self):
        # Ability 1 drops out of the reachable set and gains a hand row.
        _, nvps, effs = self.run_gen(effect_row(10, 1, "+10% Health"), NVP_TRAILER)
        nvps = nvp_row(500, 10, "HealAmount", "40") + nvps
        c = corpus_from_texts(effs, nvps, ABILITIES, ["INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (3, 2);\n"])
        _, _, _, effs2 = gen.generate(["heal"], c)
        self.assertIn("NULL, 'HealHealth');", effs2)

    def test_a_generated_row_moved_out_of_the_block_keeps_its_id_unique(self):
        # Generate two effects (ids 20000, 20001), then move 10's row out of
        # the markers as a hand row with its id. Regenerating must not hand
        # 20000 to effect 11.
        effs = effect_row(10, 1, "+10% Health") + effect_row(11, 1, "+20% Health")
        _, nvps, effs = self.run_gen(effs, NVP_TRAILER)
        moved = nvp_row(20000, 10, "HealPercentage", "10.00")
        self.assertIn(moved, nvps)
        nvps = moved + nvps.replace(moved, "")
        result, nvps2, _ = self.run_gen(effs, nvps)
        self.assertEqual([g.effect.effect_id for g in result.generated], [11])
        ids = gen.nvp_ids(nvps2)
        self.assertEqual(len(ids), len(set(ids)), ids)
        self.assertIn("(20001, 11,", nvps2)
        # Stable on the next run.
        _, nvps3, _ = self.run_gen(effs, nvps2)
        self.assertEqual(nvps3, nvps2)

    def test_a_duplicate_nvp_id_is_rejected_before_writing(self):
        nvps = nvp_row(7, 99, "HealthDamage", "1") + nvp_row(7, 98, "HealthDamage", "2") + NVP_TRAILER
        with self.assertRaises(gen.InputError):
            self.run_gen(effect_row(10, 1, "+10% Health"), nvps)

    def test_a_repeated_family_runs_once(self):
        c = corpus(effect_row(10, 1, "+10% Health"), NVP_TRAILER)
        _, once, nvps1, effs1 = gen.generate(["heal"], c)
        _, twice, nvps2, effs2 = gen.generate(["heal", "heal"], c)
        self.assertEqual((nvps2, effs2), (nvps1, effs1))
        self.assertEqual(len(twice), 1)
        self.assertIn("(20000, 10,", nvps2)

    def test_an_unmatched_marker_is_an_input_error(self):
        broken = "-- ability-mechanics generated heal begin\n" + NVP_TRAILER
        with self.assertRaises(gen.InputError):
            corpus(effect_row(10, 1, "+10% Health"), broken)


class CommittedSeed(unittest.TestCase):
    """The committed seed is what the generator writes (the CI guard)."""

    def test_check_passes_on_the_committed_seed(self):
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = gen.main(["--check"])
        self.assertEqual(code, 0, err.getvalue())

    def test_the_packet_effects_are_generated(self):
        _, results, _, _ = gen.generate(["heal"])
        got = {g.effect.effect_id: (g.script, g.nvps) for g in results[0].generated}
        want = {
            788: ("HealHealth", [("HealPercentage", "10.00")]),
            834: ("HealFocus", [("HealPercentage", "10.00")]),
            835: ("HealFocus", [("HealPercentage", "20.00")]),
            939: ("HealFocus", [("HealPercentage", "35.00")]),
            1040: ("HealFocus", [("HealPercentage", "35.00")]),
            1044: ("HealHealth", [("HealPercentage", "10.00")]),
            2014: ("HealFocus", [("HealPercentage", "35.00")]),
        }
        for eid, row in want.items():
            self.assertEqual(got.get(eid), row, eid)
        hand = {e.effect_id: note for e, note in results[0].hand_authored}
        for eid in (659, 2008, 1383):
            self.assertEqual(hand.get(eid), "parser agrees", eid)


if __name__ == "__main__":
    unittest.main()
