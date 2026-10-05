"""Tests for the generator's ``cc`` family (ability mechanics AB-09).

Stock ``unittest``, discovered with the generator's other tests. The grammar
cases use the real designer strings from
``db/resources/Effects/Seed/effects.sql``; the committed-seed cases pin the
packet's effects.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import effect_nvps_from_desc as gen  # noqa: E402
from corpus import Ability, Corpus, Effect  # noqa: E402
from families.cc import CcFamily, parse_cc  # noqa: E402
from family import Generated, Rejected  # noqa: E402


def effect(desc, pd=5.0, pc=1, flags=68, tcm="TCM_Single", eid=1, aid=1):
    return Effect(eid, aid, "e", desc, flags, pc, pd, tcm, None, (0, 0))


def parsed(desc, pd=5.0):
    out = parse_cc(effect(desc, pd))
    assert isinstance(out, Generated), getattr(out, "reason", "")
    return out.script, out.nvps


def corpus(effects, abilities):
    return Corpus(
        {e.effect_id: e for e in effects},
        {a.ability_id: a for a in abilities},
        {a.ability_id for a in abilities},
        {},
        {},
        "",
        "",
        "\n",
        "\n",
    )


class CcGrammarOnRealStrings(unittest.TestCase):
    def test_stun_spellings(self):
        # 1599 Lethal Strike, 3525 Ashrak Dagger: Paralyze.
        self.assertEqual(parsed("Target\nStun: 5 seconds"), ("Stun", [("CcDuration", "5")]))
        self.assertEqual(parsed("4 second Stun", pd=4.0), ("Stun", [("CcDuration", "4")]))

    def test_knockdown_spellings(self):
        want = ("Knockdown", [("CcDuration", "5")])
        self.assertEqual(parsed("Knockdown: 5 seconds"), want)  # 2608 Takedown
        self.assertEqual(parsed("Knockdown:\n5 Seconds"), want)  # 2669 Whirlwind
        self.assertEqual(parsed("5 Second Knockdown"), want)  # 1390
        self.assertEqual(parsed("Single Target\nTarget Knockdown: 5 Seconds"), want)  # 1572
        self.assertEqual(parsed("Medium Radius AE\nKnockdown: 5 Seconds"), want)  # 910
        self.assertEqual(parsed("Knockdown 3 seconds", pd=3.0), ("Knockdown", [("CcDuration", "3")]))

    def test_a_bare_knockdown_takes_the_rows_duration(self):
        # 3203: "Knockdown", pulse_duration 2.
        out = parse_cc(effect("Knockdown", pd=2.0))
        self.assertEqual(out.nvps, [("CcDuration", "2")])
        self.assertTrue(any("text states no length" in n for n in out.notes))

    def test_a_bare_snare_gets_the_design_default(self):
        # 1462 Snare Shot.
        out = parse_cc(effect("Single Target\nSnare: 15 Seconds", pd=15.0))
        self.assertEqual((out.script, out.nvps), ("TimedStat", [("MovementSpeedMod", "-30")]))
        self.assertTrue(any(n.startswith("DESIGN default") for n in out.notes))

    def test_interrupting_shot(self):
        # 723: pulse_duration 0 is fine for an interrupt.
        self.assertEqual(
            parsed("Interrupts target", pd=0.0), ("Interrupt", [("InterruptChance", "100")])
        )


class CcRejects(unittest.TestCase):
    def rejected(self, e, needle, abilities=()):
        out = CcFamily().parse(e, corpus([e], list(abilities)))
        self.assertIsInstance(out, Rejected)
        self.assertIn(needle, out.reason)

    def test_a_duration_that_disagrees_with_the_row(self):
        # 2836: "Knockdown: / 10 Seconds" on a 6 s row.
        self.rejected(effect("Knockdown:\n10 Seconds", pd=6.0), "pulse shape")

    def test_no_duration_anywhere(self):
        self.rejected(effect("Knockdown", pd=0.0), "pulse shape")

    def test_a_secondary_line(self):
        self.rejected(effect("Single Target\nSecondary Knockdown: 5 Seconds"), "AB-07")

    def test_clear_on_damage(self):
        # 4089 Lord's Will, flags 78.
        self.rejected(effect("Snare: 15 seconds", pd=15.0, flags=78), "AB-11")

    def test_a_multi_pulse_row(self):
        self.rejected(effect("Stun: 5 seconds", pc=5, pd=1.0), "pulse_count 5")

    def test_a_deployable(self):
        a = Ability(1015, "Deployable: Gravity Well", "", "ABILITY_TYPE_Debuff")
        self.rejected(effect("Stun: 5 seconds", aid=1015), "deployable", [a])

    def test_unknown_prose(self):
        self.rejected(effect("Snare and Slow", pd=0.0), "grammar")


class CommittedCcSeed(unittest.TestCase):
    """The packet's effects, as the committed seed holds them."""

    @classmethod
    def setUpClass(cls):
        _, results, _, _ = gen.generate(["cc"])
        cls.result = results[0]
        cls.got = {g.effect.effect_id: (g.script, g.nvps) for g in cls.result.generated}

    def test_the_packet_effects_are_generated(self):
        want = {
            723: ("Interrupt", [("InterruptChance", "100")]),  # Interrupting Shot
            1462: ("TimedStat", [("MovementSpeedMod", "-30")]),  # Snare Shot
            1599: ("Stun", [("CcDuration", "5")]),  # Lethal Strike
            2608: ("Knockdown", [("CcDuration", "5")]),  # Takedown
            1466: ("Stun", [("CcDuration", "5")]),  # Flashbang Grenade
        }
        for eid, row in want.items():
            self.assertEqual(self.got.get(eid), row, eid)

    def test_the_counts(self):
        scripts = [s for s, _ in self.got.values()]
        self.assertEqual(
            (scripts.count("Stun"), scripts.count("Knockdown"), scripts.count("TimedStat"), scripts.count("Interrupt")),
            (7, 19, 3, 1),
        )


if __name__ == "__main__":
    unittest.main()
