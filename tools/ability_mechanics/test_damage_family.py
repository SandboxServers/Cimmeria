"""Tests for the generator's ``damage`` family (AB-03).

The grammar cases use the real designer strings from
``db/resources/Effects/Seed/effects.sql``, each with the row shape (pulse
count, duration, target collection, flags) it has in the seed.
"""

from __future__ import annotations

import io
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import effect_nvps_from_desc as gen  # noqa: E402
from corpus import Ability, Effect, corpus_from_texts  # noqa: E402
from families.damage import DamageFamily, conditional_reason, parse_damage, scope_reason  # noqa: E402
from family import Generated, Rejected  # noqa: E402

DD = "ABILITY_TYPE_DD"


def effect(desc, pc=1, pd=0.0, tcm="TCM_Single", flags=0, name="e"):
    return Effect(1, 1, name, desc, flags, pc, pd, tcm, None, (0, 0))


def ability(type_id=DD, description=""):
    return Ability(1, "a", description, type_id)


def damage(desc, **kw):
    return parse_damage(effect(desc, **kw))


def full(desc, type_id=DD, **kw):
    """Through the family's own rules (conditional, scope, grammar)."""
    e = effect(desc, **kw)
    a = ability(type_id)
    return conditional_reason(e, a) or scope_reason(e, a) or parse_damage(e)


class DamageGrammarOnRealStrings(unittest.TestCase):
    def assert_damage(self, outcome, focus, health):
        self.assertIsInstance(outcome, Generated, getattr(outcome, "reason", outcome))
        want = [(n, str(v)) for n, v in (("FocusDamage", focus), ("HealthDamage", health)) if v]
        self.assertEqual(outcome.nvps, want)
        self.assertIsNone(outcome.script, "the damage family binds no script")

    def test_slash_form(self):
        # 660 Quick Burst (598), 744 Snare Shot (717), 919 Takedown (856).
        self.assert_damage(damage("Single Target\n-200F / -20H"), 200, 20)
        self.assert_damage(damage("Single Target\n-100F / -10H"), 100, 10)
        self.assert_damage(damage("Single Target Melee Damage\n-100F / -10H"), 100, 10)

    def test_prefix_pool_form(self):
        self.assert_damage(damage("F-200 H-20"), 200, 20)
        self.assert_damage(damage("Single Target\nTarget F-100 / H-10"), 100, 10)
        self.assert_damage(damage("Wound: F -50 H -5", pc=1), 50, 5)

    def test_one_pool_per_line(self):
        self.assert_damage(damage("Single Target\n-200F\n-20H"), 200, 20)
        self.assert_damage(damage("Concussion Grenade Damage:\n- 500 F\n- 50 H", tcm="TCM_AERadius"), 500, 50)

    def test_spaced_amounts(self):
        self.assert_damage(damage("Short Radius AE\n- 800F / - 80H", tcm="TCM_AERadius"), 800, 80)
        self.assert_damage(damage("Medium Radius AE\n-1000 F / -100 H", tcm="TCM_AERadius"), 1000, 100)

    def test_focus_only(self):
        self.assert_damage(damage("Single Target\n-100F"), 100, 0)
        self.assert_damage(damage("Focus Damage: -200"), 200, 0)

    def test_dot_is_per_tick(self):
        # 2394 Point Blank Shot DOT (1879): 8 pulses of 1 s.
        self.assert_damage(damage("DOT: -150F -30H (8 Ticks)", pc=8, pd=1.0), 150, 30)
        self.assert_damage(damage("Single Target\n-50F / -5H DoT: 10 Ticks", pc=10, pd=1.0), 50, 5)
        self.assert_damage(damage("Target\n-100F / -10H\n8 Ticks x1 Second", pc=8, pd=1.0), 100, 10)
        self.assert_damage(damage("Focus DOT\n-50 F\n20 ticks", pc=20, pd=0.5), 50, 0)

    def test_channel_ticks_matching_pulse_count(self):
        # 866 Steady Aim: 20 pulses of 0.5 s.
        out = damage(
            "Single Target Channeled: 20 Ticks\n-150F / -10H per Tick\n5 Ammo per Tick", pc=20, pd=0.5
        )
        self.assert_damage(out, 150, 10)
        self.assertTrue(any("cost" in n for n in out.notes))

    def test_threat_line_is_noted(self):
        out = damage("Increased Threat +200\nFocus Damage -100")
        self.assert_damage(out, 100, 0)
        self.assertTrue(any("not modelled" in n for n in out.notes))

    def test_cone_and_radius_rows(self):
        self.assert_damage(damage("Medium Cone\nSecondary -100F / -10H", tcm="TCM_AECone"), 100, 10)
        self.assert_damage(damage("Secondary Targets\n-500F / -50H", tcm="TCM_AERadius"), 500, 50)


class DamageRejections(unittest.TestCase):
    def assert_rejected(self, outcome, category):
        if isinstance(outcome, str):
            reason = outcome
        else:
            self.assertIsInstance(outcome, Rejected, outcome)
            reason = outcome.reason
        self.assertTrue(reason.startswith(category + ":"), reason)

    def test_missing_minus_sign(self):
        self.assert_rejected(damage("Single Target\nTarget -800F / 80H"), "grammar")

    def test_one_pool_twice(self):
        self.assert_rejected(damage("Target\n-800F / -80F"), "grammar")

    def test_unrecognised_line(self):
        self.assert_rejected(damage("F-400\nH-40\nRemaining pulses"), "grammar")

    def test_vs_mechanical(self):
        self.assert_rejected(
            full("Single Target\nSecondary Target\n-0F / -225H", name="Mechanical Target Damage", flags=4194368),
            "conditional",
        )
        self.assert_rejected(
            full("Single Target\nSecondary Target\n-100F / -0H", name="Non-Mechanical Target Damage"),
            "conditional",
        )

    def test_positional_and_stance_variants(self):
        self.assert_rejected(full("Single Target\nTarget -500F / -50H", name="Rear Position Damage"), "conditional")
        self.assert_rejected(full("Bonus Damage:\nF-50\nH-5", name="Assassin Stance Bonus Damage"), "conditional")
        self.assert_rejected(
            full("Damage per second while moving: F-100 H-10\n10 seconds", pc=10, pd=1.0, name="Movement damage"),
            "conditional",
        )

    def test_non_positional_is_the_base(self):
        out = full("-200F / -20H", name="Back Slash Damae: Non-Positional")
        self.assertIsInstance(out, Generated, out)

    def test_vs_low_focus_is_a_sequenced_follow_up(self):
        # 1604 Execution: "-500F / -50H" fires after "Low Focus Check" 1602.
        self.assert_rejected(full("Target\n-500F / -50H", flags=64, name="Direct Damage"), "sequenced")

    def test_a_sequenced_dot_still_parses(self):
        # 1606 Lethal Shot DoT carries EF_SequenceOnFinish too, and pulses.
        out = full("Target\n-100F / -10H\n10 Ticks x1 Sec", pc=10, pd=1.0, flags=64)
        self.assertIsInstance(out, Generated, out)

    def test_buff_damage_belongs_on_the_user(self):
        self.assert_rejected(full("Damage: F-200 H-20", type_id="ABILITY_TYPE_Buff", flags=16), "scope")

    def test_resolve_on_user(self):
        self.assert_rejected(full("F-50\nH-5", tcm="TCM_AECone", flags=131072), "scope")

    def test_targeting_contradicts_the_row(self):
        self.assert_rejected(damage("Medium Cone\n-50F / -5H DoT: 10 Ticks", pc=10, pd=1.0), "targeting")
        self.assert_rejected(damage("Single Target\n-200F\n-20H", tcm="TCM_AECone"), "targeting")
        self.assert_rejected(damage("Secondary\n-100F / -10H"), "targeting")

    def test_channel_ticks_disagree_with_the_row(self):
        self.assert_rejected(
            damage("Single Target Channeled: 50 ticks\n-150F / -15H per tick\n-5 Ammo per tick"), "pulse shape"
        )
        self.assert_rejected(damage("Single Target\n-200F / -20H\n1 second pulse\n3 ammo"), "pulse shape")

    def test_pulsing_row_without_a_tick_count(self):
        self.assert_rejected(damage("Wound: F -50 H -5", pc=20, pd=1.0), "pulse shape")

    def test_pulses_with_no_duration(self):
        self.assert_rejected(damage("Single Target\n2 pulses:\n-250F\n-25H", pc=2, pd=0.0), "pulse shape")


# A synthetic seed for the shared machinery: ability 1 (DD, reachable).
EFFECT_COLS = (
    "effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, "
    "pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, "
    "is_channeled, name, target_collection_id, event_set_id, script_name"
)
ABILITIES = "INSERT INTO abilities (ability_id, name, description, type_id) VALUES (1, 'A1', '', 'ABILITY_TYPE_DD');\n"
STARTERS = "INSERT INTO char_creation_abilities (char_def_id, ability_id) VALUES (3, 1);\n"
NVP_TRAILER = "--\n-- TOC entry 1\n--\n\nSELECT pg_catalog.setval('s', 462, true);\n"


def effect_row(eid, desc):
    return (
        f"INSERT INTO effects ({EFFECT_COLS}) VALUES ({eid}, 1, 0, '{desc}', 0, 0, 'i', 1, 0, "
        f"NULL, NULL, 'TCM_Single', false, false, 'n', 0, NULL, NULL);\n\n"
    )


class DamageBlock(unittest.TestCase):
    def test_writes_rows_in_its_range_and_binds_no_script(self):
        effs = effect_row(10, "-200F / -20H") + effect_row(11, "F-50")
        c = corpus_from_texts(effs, NVP_TRAILER, ABILITIES, [STARTERS])
        _, results, nvps, effs2 = gen.generate(["damage"], c)
        self.assertEqual([g.effect.effect_id for g in results[0].generated], [10, 11])
        self.assertIn("(21000, 10, 'FocusDamage', '200')", nvps)
        self.assertIn("(21001, 10, 'HealthDamage', '20')", nvps)
        self.assertIn("(21002, 11, 'FocusDamage', '50')", nvps)
        self.assertIn("No script is bound", nvps)
        self.assertEqual(effs2, effs, "effects.sql is untouched")

    def test_a_hand_authored_damage_row_is_left_alone(self):
        nvps = "INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (7, 10, 'HealthDamage', '15');\n"
        c = corpus_from_texts(effect_row(10, "-200F / -20H"), nvps + NVP_TRAILER, ABILITIES, [STARTERS])
        _, results, out, _ = gen.generate(["damage"], c)
        self.assertEqual(results[0].generated, [])
        self.assertEqual([e.effect_id for e, _ in results[0].hand_authored], [10])
        self.assertNotIn("(21000,", out)


class CommittedDamageSeed(unittest.TestCase):
    def test_the_packet_effects_are_generated(self):
        _, results, _, _ = gen.generate(["damage"])
        got = {g.effect.effect_id: g.nvps for g in results[0].generated}
        want = {
            660: [("FocusDamage", "200"), ("HealthDamage", "20")],  # Quick Burst 598
            744: [("FocusDamage", "100"), ("HealthDamage", "10")],  # Snare Shot 717
            919: [("FocusDamage", "100"), ("HealthDamage", "10")],  # Takedown 856
            2393: [("FocusDamage", "200"), ("HealthDamage", "20")],  # Point Blank Shot 1879
            2394: [("FocusDamage", "150"), ("HealthDamage", "30")],  # its DoT, per tick
            3511: [("FocusDamage", "500"), ("HealthDamage", "50")],  # Frag Grenade 2419
        }
        for eid, rows in want.items():
            self.assertEqual(got.get(eid), rows, eid)
        unparsed = {x.effect.effect_id for x in results[0].rejected}
        for eid in (1604, 1609, 4200, 4202, 1559, 1560, 703):
            self.assertIn(eid, unparsed, eid)

    def test_the_committed_report_is_current(self):
        # tools/ability_mechanics/reports/damage.txt is the reviewed list of
        # unparsed effects; regenerate it with `--report --family damage`.
        _, results, _, _ = gen.generate(["damage"])
        out = io.StringIO()
        gen.print_report(results, out)
        committed = (Path(__file__).resolve().parent / "reports" / "damage.txt").read_text(encoding="utf-8")
        self.assertEqual(committed.replace("\r\n", "\n"), out.getvalue())

    def test_families_do_not_share_effects_or_ids(self):
        _, results, nvps, _ = gen.generate(["heal", "damage"])
        heal, dmg = ({g.effect.effect_id for g in r.generated} for r in results)
        self.assertFalse(heal & dmg)
        ids = gen.nvp_ids(nvps)
        self.assertEqual(len(ids), len(set(ids)))


class FamilyIsRegistered(unittest.TestCase):
    def test_damage_is_a_family(self):
        self.assertIsInstance(gen.FAMILIES["damage"], DamageFamily)


if __name__ == "__main__":
    unittest.main()
