"""Tests for the generator's ``stat`` family (ability mechanics AB-04).

Stock ``unittest``, discovered with the generator's other tests. The
grammar cases use the real designer strings from
``db/resources/Effects/Seed/effects.sql``; the routing cases use a small
synthetic corpus; the committed-seed cases pin the packet's effects.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import effect_nvps_from_desc as gen  # noqa: E402
from corpus import Ability, Corpus, Effect  # noqa: E402
from families.stat import StatFamily, parse_stat, scope_rejection  # noqa: E402
from family import Generated, Rejected  # noqa: E402


def effect(desc, pd=15.0, pc=1, flags=21, tcm="TCM_Single", eid=1, aid=1, script=None):
    return Effect(eid, aid, "e", desc, flags, pc, pd, tcm, script, (0, 0))


def nvps(desc, pd=15.0):
    out = parse_stat(effect(desc, pd))
    assert isinstance(out, Generated), getattr(out, "reason", "")
    return out.nvps


class StatGrammarOnRealStrings(unittest.TestCase):
    def test_aim(self):
        # 700, ability 637 Aim.
        self.assertEqual(nvps("Single Target\n+200 Accuracy: 15 Seconds"), [("Accuracy", "200")])

    def test_call_target(self):
        # 903, ability 847.
        self.assertEqual(nvps("-100 Defense: 15 Seconds"), [("Defense", "-100")])

    def test_hunker_down_spellings(self):
        self.assertEqual(nvps("Single Target\n+100 Cover Defense: 15 seconds"), [("CoverDefense", "100")])
        self.assertEqual(nvps("Single Target\n+200 CoverDefense: 15 seconds"), [("CoverDefense", "200")])
        self.assertEqual(nvps("+200 Cover ACC for 15 Seconds"), [("CoverAccuracy", "200")])

    def test_combat_sprint_run_speed_is_movement_speed_mod_percent(self):
        # 1962: D-AB09, a run-speed percentage is movementSpeedMod percent.
        out = parse_stat(effect("Single Target\nUser +50% Run Speed\n10 Second Duration", pd=10.0))
        self.assertEqual(out.nvps, [("MovementSpeedMod", "50")])
        self.assertTrue(any("movementSpeedMod +50" in n for n in out.notes))
        self.assertTrue(out.user)

    def test_stat_first_and_glued_signs(self):
        self.assertEqual(nvps("Accuracy -100", pd=25.0), [("Accuracy", "-100")])
        self.assertEqual(nvps("Single Target\nTarget Movement Speed-30%"), [("MovementSpeedMod", "-30")])

    def test_pairs(self):
        want = [("Accuracy", "-200"), ("Defense", "-200")]
        self.assertEqual(nvps("Small Radius AE\nDebuff -200 ACC / DEF: 15 Seconds"), want)
        self.assertEqual(nvps("Single Target\nTarget -200ACC / -200DEF"), want)
        self.assertEqual(nvps("Single Target\nTarget -200 ACC / -200 DEF\nDuration: 15sec"), want)
        self.assertEqual(nvps("Single Target\n-200 Accuracy: 15 Seconds\n-200 Defense: 15 Seconds"), want)

    def test_resist_percent_is_ten_points_per_percent(self):
        # D-AB09's 10:1 rule (2004 "+50 (5%) Mental Resist").
        out = parse_stat(effect("+10% Interrupt Resistance: 30 Seconds", pd=30.0))
        self.assertEqual(out.nvps, [("InterruptResistance", "100")])
        self.assertTrue(any("10 per 1%" in n for n in out.notes))


class StatGrammarRejects(unittest.TestCase):
    def rejected(self, desc, needle, pd=15.0):
        out = parse_stat(effect(desc, pd))
        self.assertIsInstance(out, Rejected)
        self.assertIn(needle, out.reason)

    def test_a_conditional_clause(self):
        self.rejected("+10% Interrupt Resistance: 30 Seconds with missiles", "unrecognised", pd=30.0)

    def test_a_regen_stat_waits_for_ab05(self):
        self.rejected("Single Target\n+50% Focus Regen: 20 Seconds", "AB-05", pd=20.0)

    def test_a_pool_max(self):
        self.rejected("Maximum Focus +10%\nDuration: 600sec", "pool-max", pd=600.0)

    def test_an_armour_factor(self):
        self.rejected("+15% Phys AF", "armour factor", pd=30.0)

    def test_a_duration_that_disagrees_with_the_row(self):
        self.rejected("Cover Defense Debuff: -100\n1 second duration", "pulse_duration is 1.5", pd=1.5)

    def test_every_stated_duration_must_match_the_row(self):
        # A mismatch on any line rejects, not only on the last one.
        self.rejected("-200 Accuracy: 10 Seconds\n-200 Defense: 15 Seconds", "text says 10 s")
        self.rejected("-200 Accuracy: 15 Seconds\n-200 Defense: 10 Seconds", "text says 10 s")
        self.rejected("+200 Accuracy: 10 Seconds\nDuration: 15 Seconds", "text says 10 s")

    def test_a_percentage_on_points_stat(self):
        self.rejected("+10% Accuracy", "no D-AB09 unit")


class StatScope(unittest.TestCase):
    def test_held_and_passive_are_ab08s(self):
        self.assertIn("AB-08", scope_rejection(effect("Cover Defense +100", pd=0.0), None))
        self.assertIn("AB-08", scope_rejection(effect("+15% Mental Resist", flags=524288 | 1), None))
        toggled = Ability(1, "a", "", "ABILITY_TYPE_Buff", flags=8)
        self.assertIn("AB-08", scope_rejection(effect("Run Speed +50%", pd=5.0), toggled))

    def test_clear_on_damage_waits_for_ab11(self):
        self.assertIn("AB-11", scope_rejection(effect("-200 Defense: 30 Seconds (1 hit)", flags=28, pd=30.0), None))

    def test_ae_and_secondary_wait_for_ab07(self):
        self.assertIn("AB-07", scope_rejection(effect("-200 Defense", tcm="TCM_AERadius"), None))
        self.assertIn("AB-07", scope_rejection(effect("Secondary Target\n+50 Response: 15 seconds"), None))


def corpus(effects, abilities):
    return Corpus(
        effects={e.effect_id: e for e in effects},
        abilities={a.ability_id: a for a in abilities},
        reachable={a.ability_id for a in abilities},
        hand_nvps={},
        blocks={},
        effects_text="",
        nvps_text="",
        effects_eol="\n",
        nvps_eol="\n",
    )


class StatRouting(unittest.TestCase):
    """A buff must reach the beneficial path, which lands on the caster or an
    ally; anything that makes the ability non-beneficial would land it on the
    client's target instead (B-27)."""

    def test_combat_sprint_binds_the_run_speed_and_reports_the_penalty(self):
        sprint = Ability(1619, "Combat Sprint", "", "ABILITY_TYPE_Buff", flags=144, target_type_id=1)
        run = effect("Single Target\nUser +50% Run Speed\n10 Second Duration", pd=10.0, flags=23, eid=1962, aid=1619)
        acc = effect("Single Target\nTarget -100 ACC\n10 Second Duration", pd=10.0, flags=534, eid=2002, aid=1619)
        c = corpus([run, acc], [sprint])
        fam = StatFamily()
        self.assertIsInstance(fam.parse(run, c), Generated)
        out = fam.parse(acc, c)
        self.assertIsInstance(out, Rejected)
        self.assertIn("Self ability's non-beneficial half", out.reason)

    def test_a_buff_beside_a_hostile_effect_is_refused(self):
        a = Ability(5, "Mixed", "", "ABILITY_TYPE_DD", target_type_id=2)
        buff = effect("+200 Accuracy: 15 Seconds", flags=21, eid=50, aid=5)
        hit = effect("Single Target\n-200F / -20H", pd=0.0, flags=0, eid=51, aid=5)
        out = StatFamily().parse(buff, corpus([buff, hit], [a]))
        self.assertIsInstance(out, Rejected)
        self.assertIn("effect 51", out.reason)

    def test_a_debuff_on_a_hostile_ability_binds(self):
        a = Ability(847, "Call Target", "", "ABILITY_TYPE_Debuff", target_type_id=2)
        d = effect("-100 Defense: 15 Seconds", flags=20, eid=903, aid=847)
        self.assertIsInstance(StatFamily().parse(d, corpus([d], [a])), Generated)

    def test_a_user_half_of_a_targeted_ability_is_refused(self):
        a = Ability(9, "T", "", "ABILITY_TYPE_Buff", target_type_id=2)
        e = effect("User +50% Run Speed", pd=10.0, flags=23, eid=90, aid=9)
        out = StatFamily().parse(e, corpus([e], [a]))
        self.assertIsInstance(out, Rejected)
        self.assertIn("User half", out.reason)


class CommittedStatSeed(unittest.TestCase):
    """The packet's effects, as the committed seed holds them."""

    @classmethod
    def setUpClass(cls):
        _, results, _, _ = gen.generate(["stat"])
        cls.result = results[0]
        cls.got = {g.effect.effect_id: (g.script, g.nvps) for g in cls.result.generated}
        cls.rejected = {r.effect.effect_id: r.reason for r in cls.result.rejected}

    def test_the_packet_effects_are_generated(self):
        want = {
            700: ("TimedStat", [("Accuracy", "200")]),  # Aim
            903: ("TimedStat", [("Defense", "-100")]),  # Call Target
            1747: ("TimedStat", [("CoverDefense", "100")]),  # Hunker Down
            1962: ("TimedStat", [("MovementSpeedMod", "50")]),  # Combat Sprint
            1980: ("TimedStat", [("Accuracy", "-200"), ("Defense", "-200")]),  # Impose Weakness
        }
        for eid, row in want.items():
            self.assertEqual(self.got.get(eid), row, eid)

    def test_the_deferred_effects_are_reported(self):
        for eid, needle in {
            1211: "AB-05",  # Leadership
            921: "AB-07",  # Leadership's AE half
            1985: "AB-05",  # Demand Concentration
            1746: "AB-07",  # Hunker Down's secondary half
            2002: "non-beneficial half",  # Combat Sprint's penalty
            923: "AB-11",  # Marked Prey "(1 hit)"
        }.items():
            self.assertIn(needle, self.rejected.get(eid, ""), eid)

    def test_every_generated_row_is_a_timed_single_pulse(self):
        for g in self.result.generated:
            self.assertEqual(g.effect.pulse_count, 1, g.effect.effect_id)
            self.assertGreater(g.effect.pulse_duration, 0, g.effect.effect_id)


if __name__ == "__main__":
    unittest.main()
