"""The report from inputs in the shapes the real collectors write: a dict unattributed share, rows with gaps, and an O3b rule."""

import copy
import unittest
from pathlib import Path

from reporting.inputs import load_inputs
from reporting.write import build_outputs

FIXTURES = Path(__file__).resolve().parent / "fixtures" / "inputs"


def real_shape_inputs():
    """Return the fixture inputs with the real-data shapes mutated in; the fixture files are not touched."""
    loaded = copy.deepcopy(load_inputs(FIXTURES))

    loaded["github"]["prs"].append(
        {"number": 999, "campaign": None, "ci": None, "queue": None, "commits": None, "code_pr": None, "mergedAt": None}
    )

    packet = loaded["packets"]["campaigns"]["alpha-old"]["packets"][0]
    packet["first_pass"] = None
    packet["fix_commits"] = None
    packet["cycle_hours"] = None
    first_pass = loaded["packets"]["campaigns"]["alpha-old"]["first_pass"]["packets"][0]
    first_pass["first_pass"] = None
    first_pass["fix_commits"] = None

    accepted = loaded["cost"]["campaigns"]["alpha-old"]["accepted"]
    accepted["cost_per_accepted_usd"] = None
    accepted["accepted_gap"] = "no accepted packets"

    loaded["overstep"]["by_campaign"]["beta-haiku"]["rules"]["O3b"] = {"count": 1, "usd": 2.0, "wasted_usd": 1.0}
    return loaded


class RealShapesTest(unittest.TestCase):
    def setUp(self):
        self.outputs = build_outputs(real_shape_inputs())

    def test_build_outputs_does_not_raise(self):
        self.assertIn("reports/summary.md", self.outputs)

    def test_summary_shows_unattributed_dollar_share(self):
        self.assertIn("16.0%", self.outputs["reports/summary.md"])

    def test_by_family_never_reaches_output(self):
        for path, text in self.outputs.items():
            with self.subTest(path=path):
                self.assertNotIn("by_family", text)
                self.assertNotIn('"lx"', text)


if __name__ == "__main__":
    unittest.main()
