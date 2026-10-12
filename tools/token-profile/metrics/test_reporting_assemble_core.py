"""The regime labels and the core campaign merge (CM-08b): packet and campaign regime, then assemble_core."""

import unittest

from reporting import regime


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


if __name__ == "__main__":
    unittest.main()
