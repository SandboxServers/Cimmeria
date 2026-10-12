"""The self-session cost (CM-08i): pricing by tier and role, unpriced rows, and the database query's filters."""

import unittest

from ingest.prices import CURRENT
from metrics import cost
from reporting import self_cost

ZERO = {"input_tokens": 0, "output_tokens": 0, "cache_read": 0, "cache_write_5m": 0, "cache_write_1h": 0}


def _row(model, inp=1000, out=1000, **extra):
    """One SELF_SQL row as a dict (the zero token counts are the ones a row without them carries)."""
    return {**ZERO, "model": model, "input_tokens": inp, "output_tokens": out, **extra}


class PriceSessionsTest(unittest.TestCase):
    def test_prices_by_tier_and_role(self):
        rows = [_row("claude-opus-5-5"), _row("claude-haiku-5-5")]
        out = self_cost.price_sessions(rows)
        self.assertEqual(out["requests"], 2)
        self.assertEqual(out["unpriced"], 0)
        self.assertAlmostEqual(out["by_tier"]["opus"] + out["by_tier"]["haiku"], out["total_usd"], places=12)
        self.assertAlmostEqual(out["by_role"]["coordinator"], out["total_usd"], places=12)
        self.assertEqual(set(out["by_role"]), {"coordinator"})

    def test_unpriced_counted_not_summed(self):
        rows = [_row("claude-haiku-5-5"), _row("claude-no-such-model-9")]
        out = self_cost.price_sessions(rows)
        priced = cost.request_usd({"model": "claude-haiku-5-5", "weight": 1.0,
                                   "tokens": {"input": 1000, "output": 1000}}, CURRENT)
        self.assertEqual(out["requests"], 2)
        self.assertEqual(out["unpriced"], 1)
        self.assertAlmostEqual(out["total_usd"], priced, places=12)
        self.assertEqual(set(out["by_tier"]), {"haiku"})


if __name__ == "__main__":
    unittest.main()
