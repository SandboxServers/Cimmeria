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

    def test_stance_and_passive_spellings(self):
        # 2004, 2005, 2743, 2645, 4782, 1749, 2754, 1979.
        self.assertEqual(nvps("+50 (5%) Mental Resist buff", pd=0.0), [("MentalResistance", "50")])
        self.assertEqual(
            nvps("Increased Threat Rating Subtlety -100 (10% increase to threat)", pd=0.0), [("Subtlety", "-100")]
        )
        self.assertEqual(nvps("Engagement: +10", pd=0.0), [("Engagement", "10")])
        self.assertEqual(nvps("Kinetic Resists Increased: +15%", pd=0.0), [("KineticResistance", "150")])
        self.assertEqual(nvps("Defense: +100", pd=0.0), [("Defense", "100")])
        self.assertEqual(nvps("Single\n+100 Accuracy", pd=0.0), [("Accuracy", "100")])
        self.assertEqual(nvps("Self Defense +100", pd=0.0), [("Defense", "100")])
        self.assertEqual(nvps("Toggled: +100 Accuracy", pd=0.0), [("Accuracy", "100")])

    def test_points_and_percent_must_agree(self):
        out = parse_stat(effect("+50 (10%) Mental Resist buff", pd=0.0))
        self.assertIsInstance(out, Rejected)

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
    def test_a_held_effect_needs_something_to_take_it_off(self):
        # Neither a toggle nor a passive: nothing would ever remove it.
        self.assertIn("nothing would ever remove it", scope_rejection(effect("Cover Defense +100", pd=0.0), None))
        # A passive_yn ability's effect without EF_AlwaysPersist (1457 Steadfast).
        steadfast = Ability(1457, "Steadfast", "", "ABILITY_TYPE_Buff", target_type_id=1, passive=True)
        self.assertIn("B-36", scope_rejection(effect("Single\n+100 Mental Resistance", pd=0.0, flags=16), steadfast))

    def test_a_toggle_holds_on_its_own_caster_only(self):
        self_toggle = Ability(1642, "Stance: Soldier", "", "ABILITY_TYPE_Buff", flags=8, target_type_id=1)
        self.assertIsNone(scope_rejection(effect("Cover Defense +100", pd=0.0), self_toggle))
        target_toggle = Ability(1629, "Demand Accuracy", "", "ABILITY_TYPE_Buff", flags=520, target_type_id=2)
        self.assertIn("another entity", scope_rejection(effect("Toggled: +100 Accuracy", pd=0.0), target_toggle))
        # A toggle's timed effect (1250 Escape) does not fit the switch.
        self.assertIn("AB-08", scope_rejection(effect("Run Speed +50%", pd=5.0), self_toggle))
        shield = Ability(1232, "Shield: Reflective", "", "ABILITY_TYPE_Buff", flags=1560, target_type_id=1)
        self.assertIn("AB-10", scope_rejection(effect("Health Resist +15%", pd=0.0), shield))

    def test_a_passive_holds_only_on_a_passive_ability(self):
        persist = 524288 | 1
        passive = Ability(1731, "Warrior's Resilience", "", "ABILITY_TYPE_Buff", target_type_id=1, passive=True)
        self.assertIsNone(scope_rejection(effect("+15% Kinetic Resist", pd=0.0, flags=persist), passive))
        castable = Ability(2, "Castable", "", "ABILITY_TYPE_Buff", target_type_id=1)
        self.assertIn("castable", scope_rejection(effect("+15% Mental Resist", pd=0.0, flags=persist), castable))
        minigame = Ability(
            809, "Mental Fortitude", "", "ABILITY_TYPE_Undefined", target_type_id=1, passive=True,
            moniker_ids=(320218562, 1470900795),
        )
        self.assertIn("mini-game", scope_rejection(effect("+15% Mental Resist", pd=0.0, flags=persist), minigame))

    def test_clear_on_damage_waits_for_ab11(self):
        self.assertIn("AB-11", scope_rejection(effect("-200 Defense: 30 Seconds (1 hit)", flags=28, pd=30.0), None))

    def test_radius_effects_bind_where_ab07_routes_them(self):
        ground = Ability(877, "Forward Observer", "", "ABILITY_TYPE_Debuff", target_type_id=3)
        target = Ability(5, "T", "", "ABILITY_TYPE_Debuff", target_type_id=2)
        hostile = effect("-200 Defense", tcm="TCM_AERadius", flags=4)
        buff = effect("+200 Defense", tcm="TCM_AERadius", flags=21)
        # A ground cast's secondaries take a hostile radius effect (948).
        self.assertIsNone(scope_rejection(hostile, ground))
        # A beneficial radius effect of a non-ground cast fans out to allies.
        self.assertIsNone(scope_rejection(buff, target))
        # The other two would land on the one target or on hostiles.
        self.assertIn("AB-07", scope_rejection(hostile, target))
        self.assertIn("ground collector", scope_rejection(buff, ground))

    def test_group_cone_and_secondary_wait(self):
        self.assertIn("D-AB12", scope_rejection(effect("-200 Defense", tcm="TCM_Group"), None))
        self.assertIn("cone", scope_rejection(effect("-200 Defense", tcm="TCM_AECone"), None))
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

    def test_combat_sprint_binds_both_halves(self):
        # AB-07 rule 2: a Self ability with no area effect lands its single
        # effects on its user, so the "-100 ACC" penalty is the user's.
        sprint = Ability(1619, "Combat Sprint", "", "ABILITY_TYPE_Buff", flags=144, target_type_id=1)
        run = effect("Single Target\nUser +50% Run Speed\n10 Second Duration", pd=10.0, flags=23, eid=1962, aid=1619)
        acc = effect("Single Target\nTarget -100 ACC\n10 Second Duration", pd=10.0, flags=534, eid=2002, aid=1619)
        c = corpus([run, acc], [sprint])
        fam = StatFamily()
        self.assertIsInstance(fam.parse(run, c), Generated)
        self.assertEqual(fam.parse(acc, c).nvps, [("Accuracy", "-100")])

    def test_a_self_abilitys_single_beside_an_area_effect_is_refused(self):
        # Whirlwind's shape: the single effects follow the area hit.
        a = Ability(2025, "Whirlwind", "", "ABILITY_TYPE_DD", target_type_id=1)
        area = effect("AOE Damage\n-200 F -20 H", pd=0.0, flags=0, tcm="TCM_AERadius", eid=2667, aid=2025)
        debuff = effect("-100 Defense: 5 Seconds", pd=5.0, flags=64, eid=2669, aid=2025)
        out = StatFamily().parse(debuff, corpus([area, debuff], [a]))
        self.assertIsInstance(out, Rejected)
        self.assertIn("follow-up of the area hit", out.reason)

    def test_a_user_flagged_half_of_a_targeted_ability_binds(self):
        a = Ability(9, "T", "", "ABILITY_TYPE_DD", target_type_id=2)
        e = effect("User +50% Run Speed", pd=10.0, flags=131072 | 23, eid=90, aid=9)
        hit = effect("Single Target\n-200F / -20H", pd=0.0, flags=0, eid=91, aid=9)
        self.assertIsInstance(StatFamily().parse(e, corpus([e, hit], [a])), Generated)

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
            2002: ("TimedStat", [("Accuracy", "-100")]),  # Combat Sprint's penalty (AB-07)
            948: ("TimedStat", [("Defense", "-200")]),  # Forward Observer, ground radius (AB-07)
            1980: ("TimedStat", [("Accuracy", "-200"), ("Defense", "-200")]),  # Impose Weakness
        }
        for eid, row in want.items():
            self.assertEqual(self.got.get(eid), row, eid)

    def test_the_deferred_effects_are_reported(self):
        for eid, needle in {
            1211: "AB-05",  # Leadership
            921: "AB-05",  # Leadership's AE half: routed by AB-07, a regen stat
            1985: "AB-05",  # Demand Concentration
            1746: "AB-07",  # Hunker Down's secondary half
            923: "AB-11",  # Marked Prey "(1 hit)"
        }.items():
            self.assertIn(needle, self.rejected.get(eid, ""), eid)

    def test_every_timed_row_is_a_single_pulse_and_every_held_row_comes_off(self):
        corpus = gen.load_corpus()
        for g in self.result.generated:
            e = g.effect
            ability = corpus.abilities[e.ability_id]
            if e.pulse_duration > 0:
                self.assertEqual(e.pulse_count, 1, e.effect_id)
            elif g.script == "TimedStat":
                # Held: a Self toggle's press or a passive's respec removes it.
                toggle = bool(ability.flags & 8) and ability.target_type_id == 1
                passive = bool(e.flags & 524288) and ability.passive
                self.assertTrue(toggle or passive, e.effect_id)

    def test_the_stances_and_passives_are_generated(self):
        stance = ("EffectMoniker", "EFFECT_Stance")
        want = {
            2003: ("TimedStat", [("CoverDefense", "100"), stance]),  # Stance: Soldier
            2004: ("TimedStat", [("MentalResistance", "50"), stance]),
            2005: ("TimedStat", [("Subtlety", "-100"), stance]),
            1749: ("TimedStat", [("Accuracy", "100"), stance]),  # Stance: Ranged Specialist
            922: ("TimedStat", [("InterruptResistance", "250"), stance]),  # Concentration
            4294: ("RemoveByMoniker", [("RemoveMoniker", "EFFECT_Stance")]),  # its removal half
            1741: ("TimedStat", [("CoverAccuracy", "100")]),  # Cover Penetration (passive)
            2645: ("TimedStat", [("KineticResistance", "150")]),  # Warrior's Resilience (passive)
            4782: ("TimedStat", [("Defense", "100")]),  # Create Density: Basic (passive)
        }
        for eid, row in want.items():
            self.assertEqual(self.got.get(eid), row, eid)

    def test_no_bound_effect_carries_the_stance_moniker_outside_a_stance(self):
        corpus = gen.load_corpus()
        for g in self.result.generated:
            if ("EffectMoniker", "EFFECT_Stance") in g.nvps:
                ability = corpus.abilities[g.effect.ability_id]
                self.assertTrue(ability.flags & 8, g.effect.effect_id)

    def test_the_held_effects_left_alone_are_reported(self):
        for eid, needle in {
            854: "mini-game",  # 809 Mental Fortitude
            1748: "B-36",  # 1457 Steadfast: passive without EF_AlwaysPersist
            1979: "another entity",  # 1629 Demand Accuracy (Target toggle)
            713: "binds no stance effect",  # Reveal I's removal alone
            3146: "AB-10",  # a shield toggle
        }.items():
            self.assertIn(needle, self.rejected.get(eid, ""), eid)


if __name__ == "__main__":
    unittest.main()
