"""The campaign registry (CM-01): the regime boundary, the rules check, tagging through ingest/campaigns.py, and the CLI."""

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

from . import cli, registry


class RegimeTest(unittest.TestCase):
    def test_haiku_starts_at_the_switch_second(self):
        config = registry.load_config()
        # Reverting the comparison to `<` makes the 20:02:54Z case come out "old".
        self.assertEqual(registry.regime_of("2026-10-10T20:02:53Z", config), "old")
        self.assertEqual(registry.regime_of("2026-10-10T20:02:54Z", config), "haiku")


class TaggingTest(unittest.TestCase):
    RULES = {"_doc": "x", "alpha": {"prefixes": ["lab/"]}, "beta": {"issues": [12]},
             "gamma": {"branches": ["content/gamma"]}}

    def test_campaign_of_pr_uses_the_ingest_rules(self):
        self.assertEqual(registry.campaign_of_pr({"head_branch": "lab/x", "title": "t", "base_branch": "main"},
                                                 self.RULES), "alpha")
        self.assertEqual(registry.campaign_of_pr({"head_branch": "fix/y", "title": "fix (#12)", "base_branch": "main"},
                                                 self.RULES), "beta")
        self.assertEqual(registry.campaign_of_pr({"head_branch": "p", "title": "p", "base_branch": "content/gamma"},
                                                 self.RULES), "gamma")
        self.assertIsNone(registry.campaign_of_pr({"head_branch": "fix/y", "title": "fix", "base_branch": "main"},
                                                  self.RULES))


class CheckTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name)
        for name in ("alpha", "beta", "gamma"):
            (self.repo / "docs" / "analysis" / name).mkdir(parents=True)
        self.clean = {"_doc": "x", "alpha": {"prefixes": ["lab/"]}, "beta": {"prefixes": ["lab-two/"]},
                      "gamma": {"prefixes": ["other/"]}}

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, rules):
        path = self.repo / "campaigns.json"
        path.write_text(json.dumps(rules), encoding="utf-8")
        return str(path)

    def test_overlapping_prefixes_are_reported(self):
        rules = dict(self.clean, beta={"prefixes": ["lab/x/"]})
        found = registry.problems(rules, self.repo)
        # Reverting the overlap test (startswith on both sides) leaves this list empty.
        self.assertEqual(len(found), 1)
        self.assertIn("overlaps", found[0])

    def test_campaign_without_a_folder_is_reported(self):
        found = registry.problems(dict(self.clean, delta={"prefixes": ["d/"]}), self.repo)
        self.assertEqual(found, ["no folder for delta"])

    def test_check_exits_1_on_overlap_and_0_when_clean(self):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(cli.main(["registry", "--repo", str(self.repo), "--check",
                                       "--campaigns", self.write(dict(self.clean, beta={"prefixes": ["lab/x/"]}))]), 1)
            self.assertEqual(cli.main(["registry", "--repo", str(self.repo), "--check",
                                       "--campaigns", self.write(self.clean)]), 0)

    def test_cli_prints_the_count_summary(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(cli.main(["registry", "--repo", str(self.repo), "--campaigns", self.write(self.clean)]), 0)
        got = json.loads(out.getvalue())
        self.assertEqual(got, {"campaigns": 3, "prefixes": 3, "regime_switch": "2026-10-10T20:02:54Z", "problems": 0})


if __name__ == "__main__":
    unittest.main()
