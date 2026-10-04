"""Tests for the generator's ``shield`` and ``cleanse`` families (ability
mechanics AB-10).

Stock ``unittest``, discovered with the generator's other tests. The grammar
cases use the real designer strings from ``db/resources/Effects/Seed/
effects.sql``; the routing and tagging cases a small synthetic corpus; the
committed-seed cases pin the packet's effects.
"""

from __future__ import annotations

import io
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import effect_nvps_from_desc as gen  # noqa: E402
from corpus import Ability, Corpus, Effect  # noqa: E402
from families.cleanse import CleanseFamily  # noqa: E402
from families.shield import ShieldFamily, parse_absorb  # noqa: E402
from family import Generated, Rejected  # noqa: E402


def effect(desc, pd=0.0, pc=1, flags=0, tcm="TCM_Single", eid=1, aid=1, script=None, seq=0, name="e"):
    return Effect(eid, aid, name, desc, flags, pc, pd, tcm, script, (0, 0), seq)


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


SELF = 1
AF_TOGGLED = 8


class ShieldGrammar(unittest.TestCase):
    def test_personal_shield_is_three_pools_of_500(self):
        # 4306, ability 1013 Personal Shield.
        out = parse_absorb(effect("Absorption:\n500 Physical\n500 Energy\n500 Contamination", pd=30.0, flags=342))
        self.assertIsInstance(out, Generated)
        self.assertEqual(out.script, "AbsorbShield")
        self.assertEqual(out.nvps, [("ShieldAmount", "500"), ("ShieldType", "Physical,Energy,Hazmat")])

    def test_a_shield_with_no_number_is_reported(self):
        # 4785 / 4852 Defensive Shield: Absorption.
        out = parse_absorb(effect("Total Absorption:\nEnergy:"))
        self.assertIsInstance(out, Rejected)
        self.assertTrue(out.reason.startswith("no number:"), out.reason)

    def test_different_capacities_are_refused(self):
        out = parse_absorb(effect("Absorption:\n500 Physical\n300 Energy"))
        self.assertIsInstance(out, Rejected)

    def test_a_toggled_mitigation_shield_binds_timed_stat(self):
        a = Ability(1016, "Shield: Physical", "User Target Shield\nToggle: +15% Physical Density",
                    "ABILITY_TYPE_Buff", flags=1816, target_type_id=SELF)
        buff = effect("Single Target\nTarget +15% Physical Mitigation", flags=341, eid=4270, aid=1016)
        off = effect("Single Target\nUser Remove Effect of EFFECT_Shield", eid=2253, aid=1016)
        c = corpus([buff, off], [a])
        out = ShieldFamily().parse(buff, c)
        self.assertIsInstance(out, Generated)
        self.assertEqual((out.script, out.nvps), ("TimedStat", [("Mitigation", "15")]))
        self.assertTrue(any("held toggle" in n for n in out.notes))
        removal = ShieldFamily().parse(off, c)
        self.assertTrue(removal.reason.startswith("moniker:"), removal.reason)

    def test_a_held_mitigation_without_a_toggle_is_refused(self):
        a = Ability(9, "Shield: Odd", "", "ABILITY_TYPE_Buff", flags=0, target_type_id=SELF)
        e = effect("Target +15% Physical Mitigation", flags=341, eid=90, aid=9)
        out = ShieldFamily().parse(e, corpus([e], [a]))
        self.assertIsInstance(out, Rejected)
        self.assertTrue(out.reason.startswith("toggle:"), out.reason)

    def test_a_turret_shield_is_out_of_scope(self):
        a = Ability(1212, "Enhance Turret: Shield", "", "ABILITY_TYPE_Buff", target_type_id=2)
        e = effect("+15% Phys AF", pd=30.0, flags=21, eid=3353, aid=1212)
        out = ShieldFamily().parse(e, corpus([e], [a]))
        self.assertTrue(out.reason.startswith("turret:"), out.reason)

    def test_a_shield_beside_a_hostile_effect_is_refused(self):
        a = Ability(5, "Shield Bash", "", "ABILITY_TYPE_DD", target_type_id=2)
        s = effect("Absorption:\n100 Physical", pd=10.0, eid=50, aid=5)
        hit = effect("Single Target\n-200F / -20H", eid=51, aid=5)
        out = ShieldFamily().parse(s, corpus([s, hit], [a]))
        self.assertIsInstance(out, Rejected)
        self.assertTrue(out.reason.startswith("routing:"), out.reason)


class CleanseGrammar(unittest.TestCase):
    def purge(self, desc, eid=4168, aid=2865):
        a = Ability(aid, "Absolution", "Self Target\nPurges 2 Mental and 2 Health effects",
                    "ABILITY_TYPE_Buff", flags=528, target_type_id=SELF)
        e = effect(desc, eid=eid, aid=aid)
        return CleanseFamily().parse(e, corpus([e], [a]))

    def test_the_real_purge_spellings(self):
        for desc, slots in {
            "Purge Mental Effects: X 2": "Mental:2",  # 4168 Absolution
            "Purge Health Effects: X 2": "Health:2",  # 4169 Absolution
            "Purge: Mental Effects x5": "Mental:5",  # 2672 Warrior's Will
            "Purges Mental States x5": "Mental:5",  # 2827 Clear: Mind
        }.items():
            out = self.purge(desc)
            self.assertIsInstance(out, Generated, desc)
            self.assertEqual(out.script, "RemoveEffects")
            self.assertEqual(out.nvps, [("RemoveCategories", slots), ("RemovePolarity", "Harmful")], desc)

    def test_a_purge_with_no_count_is_reported(self):
        # 2693 Warrior's Determination.
        out = self.purge("Purge: Kinetic Effects")
        self.assertTrue(out.reason.startswith("count:"), out.reason)

    def test_an_undefined_category_is_reported(self):
        # 1360 Clear: Disruptions, 824 Enhance: Humanoid: Focus.
        for desc in ("Purges Focus Degeneration state", "Removes 1 Focus Buff from User"):
            out = self.purge(desc)
            self.assertTrue(out.reason.startswith("category:"), out.reason)

    def test_a_moniker_removal_is_reported(self):
        for desc in ("Remove 1 Effect of Moniker EFFECT_Wound", "Remove Effect of moniker EFFECT_Stance",
                     "User Single Target\nRemove 1 Effect of EFFECT_Stance"):
            out = self.purge(desc)
            self.assertTrue(out.reason.startswith("moniker:"), (desc, out.reason))


class CleanseTags(unittest.TestCase):
    """Snare Shot's shape: the Kinetic Resist Roll and the snare share step 1;
    the damage at step 0 is instant."""

    def setUp(self):
        a = Ability(717, "Snare Shot", "", "ABILITY_TYPE_DD", target_type_id=2)
        self.hit = effect("Single Target\n-100F / -10H", eid=744, aid=717, seq=0)
        self.roll = effect("Single Target\nKinetic Resist Roll", eid=745, aid=717, seq=1, name="Kinetic Resist Roll")
        self.snare = effect("Single Target\nSnare: 15 Seconds", pd=15.0, flags=68, eid=1462, aid=717, seq=1)
        self.c = corpus([self.hit, self.roll, self.snare], [a])

    def test_the_gated_effect_is_tagged_with_the_roll_kind(self):
        fam = CleanseFamily()
        self.assertTrue(fam.is_candidate(self.snare, self.c))
        out = fam.parse(self.snare, self.c)
        self.assertEqual((out.script, out.nvps), (None, [("EffectCategory", "Kinetic")]))

    def test_the_roll_and_an_instant_hit_are_not_tagged(self):
        fam = CleanseFamily()
        self.assertFalse(fam.is_candidate(self.roll, self.c))
        self.assertFalse(fam.is_candidate(self.hit, self.c))


class CommittedShieldCleanseSeed(unittest.TestCase):
    """The packet's effects, as the committed seed holds them."""

    @classmethod
    def setUpClass(cls):
        _, results, _, _ = gen.generate(["shield", "cleanse"])
        cls.got = {g.effect.effect_id: (g.script, g.nvps) for r in results for g in r.generated}
        cls.rejected = {(r.family.name, x.effect.effect_id): x.reason for r in results for x in r.rejected}

    def test_the_packet_effects_are_generated(self):
        want = {
            4306: ("AbsorbShield", [("ShieldAmount", "500"), ("ShieldType", "Physical,Energy,Hazmat")]),
            4270: ("TimedStat", [("Mitigation", "15")]),  # Shield: Physical
            3148: ("TimedStat", [("Mitigation", "10")]),  # Shield: Universal
            4168: ("RemoveEffects", [("RemoveCategories", "Mental:2"), ("RemovePolarity", "Harmful")]),
            4169: ("RemoveEffects", [("RemoveCategories", "Health:2"), ("RemovePolarity", "Harmful")]),
            1462: (None, [("EffectCategory", "Kinetic")]),  # Snare Shot's snare
            1483: (None, [("EffectCategory", "Mental")]),  # Suppression Shot
            4237: (None, [("EffectCategory", "Health")]),  # Wounding Shot's DoT
        }
        for eid, row in want.items():
            self.assertEqual(self.got.get(eid), row, eid)

    def test_the_unresolvable_ones_are_reported(self):
        for key, prefix in {
            ("shield", 4785): "no number:",
            ("shield", 4852): "no number:",
            ("cleanse", 2693): "count:",
            ("cleanse", 1360): "category:",
        }.items():
            self.assertTrue(self.rejected.get(key, "").startswith(prefix), key)

    def test_the_committed_reports_are_current(self):
        # tools/ability_mechanics/reports/{shield,cleanse}.txt are the reviewed
        # lists; regenerate each with `--report --family <name>`.
        for family in ("shield", "cleanse"):
            _, results, _, _ = gen.generate([family])
            out = io.StringIO()
            gen.print_report(results, out)
            path = Path(__file__).resolve().parent / "reports" / f"{family}.txt"
            self.assertEqual(path.read_text(encoding="utf-8").replace("\r\n", "\n"), out.getvalue(), family)

    def test_the_families_stay_in_their_ranges(self):
        _, results, nvps, _ = gen.generate(["shield", "cleanse"])
        ids = gen.nvp_ids(nvps)
        self.assertEqual(len(ids), len(set(ids)))
        ours = {g.effect.effect_id for r in results for g in r.generated}
        self.assertTrue(ours)
        for lo, hi in (gen.NVP_RANGES["shield"], gen.NVP_RANGES["cleanse"]):
            self.assertLess(lo, hi)


if __name__ == "__main__":
    unittest.main()
