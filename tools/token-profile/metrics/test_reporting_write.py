"""The report files as text, then written behind the privacy gate: paths, line endings, refusal and rewrites."""

import re
import unittest
from pathlib import Path

from reporting.inputs import load_inputs
from reporting.write import build_outputs

FIXTURES = Path(__file__).resolve().parent / "fixtures" / "inputs"
UUID = re.compile(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
PATHS = [
    "data/alpha-old.json",
    "data/beta-haiku.json",
    "data/metrics.json",
    "data/summary.json",
    "reports/alpha-old.md",
    "reports/beta-haiku.md",
    "reports/summary.md",
]


class BuildOutputsTest(unittest.TestCase):
    def setUp(self):
        self.outputs = build_outputs(load_inputs(FIXTURES))

    def test_build_outputs_paths(self):
        self.assertEqual(sorted(self.outputs), PATHS)

    def test_no_branch_path_or_id_in_output(self):
        for path, text in self.outputs.items():
            with self.subTest(path=path):
                self.assertNotIn("campaign-metrics/cm", text)
                self.assertNotIn("C:\\", text)
                self.assertIsNone(UUID.search(text))


if __name__ == "__main__":
    unittest.main()
