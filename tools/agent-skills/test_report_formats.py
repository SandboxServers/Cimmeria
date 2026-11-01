"""The return-line formats: the hook's LINE matches what the agents are told to write (C6, C1)."""
import importlib.util
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
AGENTS = ROOT / ".claude" / "agents"
HOOK = ROOT / ".claude" / "hooks" / "report_contract.py"
METRICS = ROOT / "tools" / "token-profile" / "metrics" / "transcripts.py"
TEMPLATE = re.compile(r"`(<packet[^`]*)`")
WAVE = ("<packet-or-scope> done <reviewed-head-sha> <report-path> verdict=<ship|fix|decision> "
        "verdicts=<packet>:<ship|fix|decision>,... not_assessed=<n>")
# Placeholder values; longer keys first so `<packet>:<ship|fix|decision>,...` wins over `<packet>`.
FILL = {
    "<packet>:<ship|fix|decision>,...": "HT-01:ship",
    "<packet-or-task>": "HT-01",
    "<packet-or-scope>": "HT-01",
    "<packet>": "HT-01",
    "<done|blocked|failed>": "done",
    "<commit-sha|->": "abc1234",
    "<reviewed-head-sha>": "abc1234",
    "<report-path>": "x.md",
    "<ship|fix|decision>": "ship",
    "<platform|host|tool>": "host",
    "<n>": "0",
}
SAMPLES_OK = (
    "HT-01 done abc1234 C:\\w\\.claude\\reports\\c\\HT-01-review-1.md",
    "HT-01 done abc1234 -",
    "HT-01 blocked - - reason=context",
    "HT-01 done abc1234 - not_run=3,4",
    "wave2 done abc1234 x.md verdict=fix verdicts=HT-06:ship,HT-07:fix not_assessed=2",
    "`HT-01 blocked - x.md reason=context`",
)
SAMPLES_BAD = ("HT-01 done", "done abc x.md")


def load_module(name, path):
    """Import one file by path, so the test reads the real code, not a copy."""
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def fill(template):
    """Replace each placeholder of a return-line template with a sample value."""
    for key in sorted(FILL, key=len, reverse=True):
        template = template.replace(key, FILL[key])
    return template


class ReportFormatTest(unittest.TestCase):
    """The hook's LINE accepts the return lines and rejects the malformed ones."""

    @classmethod
    def setUpClass(cls):
        """Load the hook once; its LINE is the format every agent is held to."""
        cls.line = load_module("report_contract", HOOK).LINE

    def test_sample_lines(self):
        for sample in SAMPLES_OK:
            with self.subTest(sample=sample):
                text = sample[1:-1] if sample.startswith("`") and sample.endswith("`") else sample
                self.assertIsNotNone(self.line.match(text))
        for sample in SAMPLES_BAD:
            with self.subTest(sample=sample):
                self.assertIsNone(self.line.match(sample))

    def test_reviewer_sections_and_return_words(self):
        text = (AGENTS / "packet-reviewer.md").read_text(encoding="utf-8")
        positions = [text.index(h) for h in ("## Verdict", "## Fix notes", "## Not fixed", "## Not checked")]
        self.assertEqual(positions, sorted(positions))
        for phrase in ("NOT ASSESSED —", "not_run=", "Rejected <note-id>:", "not_assessed=<n>"):
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, text)

    def test_orchestrator_waits_on_not_assessed(self):
        text = (AGENTS / "orchestrator.md").read_text(encoding="utf-8")
        self.assertIn("not_assessed=", text)

    def test_templates_match_the_line(self):
        for name in ("packet-coder", "packet-reviewer"):
            text = (AGENTS / f"{name}.md").read_text(encoding="utf-8")
            templates = [t for t in TEMPLATE.findall(text) if " blocked " in t or "<done|" in t]
            self.assertTrue(templates, f"{name} has no return-line template")
            for template in templates:
                with self.subTest(agent=name, template=template):
                    self.assertIsNotNone(self.line.match(fill(template)))

    def test_wave_template_matches_the_line(self):
        self.assertIsNotNone(self.line.match(fill(WAVE)))

    @unittest.skipUnless(METRICS.is_file(), "the metrics reader is not in this tree")
    def test_metrics_line_equals_the_hook_line(self):
        metrics = load_module("transcripts", METRICS)
        self.assertEqual(metrics.LINE.pattern, self.line.pattern)


if __name__ == "__main__":
    unittest.main()
