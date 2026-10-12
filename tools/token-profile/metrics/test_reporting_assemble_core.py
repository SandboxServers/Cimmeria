"""The regime labels and the core campaign merge (CM-08b): packet and campaign regime, then assemble_core."""

import copy
import unittest
from pathlib import Path

from reporting import assemble, inputs, regime

FIXTURES = Path(__file__).parent / "fixtures" / "inputs"


def packets(haiku, old):
    """A packets list with `haiku` haiku-implemented packets followed by `old` old ones."""
    return [{"id": f"H-{i}", "implementer_sessions": 1} for i in range(haiku)] + [
        {"id": f"O-{i}", "implementer_sessions": 0} for i in range(old)
    ]


class PacketRegimeTest(unittest.TestCase):
    def test_packet_regime(self):
        self.assertEqual(regime.packet_regime({"implementer_sessions": 0}), "old")
        self.assertEqual(regime.packet_regime({"implementer_sessions": 2}), "haiku")


class CampaignRegimeTest(unittest.TestCase):
    def test_campaign_regime_thresholds(self):
        # 7 of 10 is exactly the 70% line; a `>` threshold would call it mixed.
        self.assertEqual(regime.campaign_regime(packets(7, 3)), "haiku")
        self.assertEqual(regime.campaign_regime(packets(6, 4)), "mixed")
        self.assertEqual(regime.campaign_regime(packets(0, 10)), "old")
        self.assertEqual(regime.campaign_regime([]), "old")


class AssembleCoreTest(unittest.TestCase):
    def setUp(self):
        self.loaded = inputs.load_inputs(FIXTURES)

    def assemble(self, name, loaded=None):
        loaded = loaded or self.loaded
        return assemble.assemble_core(name, loaded["cost"], loaded["packets"], loaded["ledger"])

    def test_alpha_old_beta_haiku(self):
        self.assertEqual(self.assemble("alpha-old")["regime"], "old")
        self.assertEqual(self.assemble("beta-haiku")["regime"], "haiku")
        alpha_rows = self.assemble("alpha-old")["packet_rows"]
        self.assertEqual(len(alpha_rows), 4)
        self.assertTrue(all(row["regime"] == "old" for row in alpha_rows))

    def test_beta_numbers(self):
        core = self.assemble("beta-haiku")
        self.assertEqual(core["cost"]["total_usd"], 30.0)
        self.assertEqual(core["retries"]["total"], 2)
        self.assertEqual(core["escalation"]["rate"], 0.33)
        self.assertEqual(core["stop_context"]["by_stop"]["error"], 1)
        rows = core["packet_rows"]
        self.assertEqual([row["id"] for row in rows], ["B-1", "B-2", "B-3"])
        self.assertTrue(rows[2]["escalated"])
        self.assertEqual(rows[2]["regime"], "haiku")
        self.assertEqual(rows[2]["implementer"], "packet-coder")

    def test_missing_packets_record_is_gap(self):
        loaded = copy.deepcopy(self.loaded)
        del loaded["packets"]["campaigns"]["beta-haiku"]
        core = self.assemble("beta-haiku", loaded)
        self.assertEqual(core["first_pass"], {})
        self.assertIn("first_pass_gap", core)
        self.assertEqual(core["regime"], "old")
        self.assertIn("regime_gap", core)
        self.assertEqual(core["packet_rows"], [])

    def test_inputs_not_mutated(self):
        before = copy.deepcopy(self.loaded)
        for name in ("alpha-old", "beta-haiku"):
            core = self.assemble(name)
            # Edit every copied section; an aliased section would leak into the inputs.
            core["cost"]["total_usd"] = -1.0
            core["prs"]["per_pr"].append({"pr": 0})
            for section in assemble.SECTIONS:
                core[section]["edited"] = True
            core["packet_rows"][0]["retries"].append("edited")
        self.assertEqual(self.loaded, before)


if __name__ == "__main__":
    unittest.main()
