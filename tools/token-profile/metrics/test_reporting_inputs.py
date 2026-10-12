"""The report inputs loader (CM-08a): the five files, missing files, bad JSON, and the two-campaign fixtures."""

import shutil
import tempfile
import unittest
from pathlib import Path

from reporting import inputs

FIXTURES = Path(__file__).parent / "fixtures" / "inputs"


class LoadInputsTest(unittest.TestCase):
    def test_loads_all_five(self):
        loaded = inputs.load_inputs(FIXTURES)
        # Dropping a name from inputs.FILES makes its key go missing here.
        for name in ("cost", "ledger", "github", "packets", "overstep"):
            self.assertIn(name, loaded)
        self.assertEqual(loaded["missing"], [])
        self.assertIn("alpha-old", loaded["cost"]["campaigns"])
        self.assertIn("beta-haiku", loaded["cost"]["campaigns"])

    def test_missing_file_is_listed(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp) / "inputs"
            shutil.copytree(FIXTURES, folder)
            (folder / "overstep.json").unlink()
            loaded = inputs.load_inputs(folder)
        # Hiding the gap (no entry in `missing`) or raising would fail here.
        self.assertEqual(loaded["missing"], ["overstep"])
        self.assertEqual(loaded["overstep"], {})
        self.assertIn("cost", loaded)

    def test_utf8_bom_still_loads(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp) / "inputs"
            shutil.copytree(FIXTURES, folder)
            # Excel and some editors prepend a UTF-8 BOM; the loader must accept it.
            original = (folder / "cost.json").read_bytes()
            (folder / "cost.json").write_bytes(b"\xef\xbb\xbf" + original)
            loaded = inputs.load_inputs(folder)
        self.assertIn("alpha-old", loaded["cost"]["campaigns"])

    def test_bad_json_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            (folder / "cost.json").write_text("{", encoding="utf-8")
            with self.assertRaises(ValueError) as caught:
                inputs.load_inputs(folder)
        self.assertEqual(str(caught.exception), "cost.json is not valid JSON")

    def test_fixture_pr_map_covers_all_prs(self):
        loaded = inputs.load_inputs(FIXTURES)
        pr_map = loaded["ledger"]["pr_map"]
        for name, campaign in loaded["cost"]["campaigns"].items():
            for row in campaign["prs"]["per_pr"]:
                # A fixture edit that drops a PR from pr_map fails here.
                self.assertEqual(pr_map.get(str(row["pr"])), name)


if __name__ == "__main__":
    unittest.main()
