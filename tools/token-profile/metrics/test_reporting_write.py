"""The report files as text, then written behind the privacy gate: paths, line endings, refusal and rewrites."""

import json
import re
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from report.scrub import PrivacyError
from reporting.inputs import load_inputs
from reporting.write import build_outputs, write_outputs

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

    def test_zero_own_cost_keeps_placeholder(self):
        zeros = {"sessions": 0, "requests": 0, "total_usd": 0, "by_role": {}}
        outputs = build_outputs(load_inputs(FIXTURES), self_cost=zeros)
        self.assertIn("Filled at close-out (CM-10).", outputs["reports/summary.md"])


class WriteOutputsTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.outputs = build_outputs(load_inputs(FIXTURES))

    def tearDown(self):
        self.tmp.cleanup()

    def test_md_is_crlf_json_is_lf(self):
        out = self.root / "out"
        write_outputs(self.outputs, out)
        for path in PATHS:
            data = (out / path).read_bytes()
            with self.subTest(path=path):
                if path.endswith(".md"):
                    self.assertNotIn(b"\n", data.replace(b"\r\n", b""))
                    self.assertNotIn(b"\r\r", data)
                    self.assertEqual(data.count(b"\r\n"), data.count(b"\n"))
                else:
                    self.assertNotIn(b"\r", data)
                    self.assertTrue(data.endswith(b"\n"))

    def test_unsafe_campaign_names_are_refused_and_nothing_is_written(self):
        out = self.root / "out"
        out.mkdir()
        for name in ["../x", "a/b", "summary", "nul"]:
            with self.subTest(name=name):
                fake = {"campaigns": {name: {}}, "meta": {"stamp": {}}}
                with mock.patch("reporting.write.assemble", return_value=fake):
                    with self.assertRaises(ValueError):
                        build_outputs(load_inputs(FIXTURES))
                self.assertEqual(list(out.iterdir()), [])

    def test_write_refuses_path_outside_out_folder(self):
        out = self.root / "out"
        out.mkdir()
        with self.assertRaises(ValueError):
            write_outputs({"../escaped.json": "{}\n"}, out)
        self.assertEqual(list(out.iterdir()), [])

    def test_privacy_gate_refuses_injected_path_and_writes_nothing(self):
        fixture = self.root / "fixture"
        shutil.copytree(FIXTURES, fixture)
        cost_path = fixture / "cost.json"
        cost = json.loads(cost_path.read_text(encoding="utf-8"))
        cost["stamp"]["price_source"] = "C:\\Users\\x\\secret"
        cost_path.write_text(json.dumps(cost), encoding="utf-8")
        outputs = build_outputs(load_inputs(fixture))
        out = self.root / "out"
        out.mkdir()
        with self.assertRaises(PrivacyError):
            write_outputs(outputs, out)
        self.assertEqual(list(out.iterdir()), [])

    def test_rewrite_is_idempotent(self):
        out = self.root / "out"
        write_outputs(self.outputs, out)
        first = {path: (out / path).read_bytes() for path in PATHS}
        write_outputs(self.outputs, out)
        second = {path: (out / path).read_bytes() for path in PATHS}
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
