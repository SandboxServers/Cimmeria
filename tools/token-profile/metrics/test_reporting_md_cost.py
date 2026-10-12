"""The campaign Markdown report, part one (CM-08f): cells, cost, PR, accepted, first-pass and retry tables."""

import unittest

from reporting import md_common


class CellTest(unittest.TestCase):
    def test_cell_gap_beats_dash(self):
        self.assertEqual(md_common.cell(None, gap="no ledger"), "n/a (no ledger)")
        self.assertNotEqual(md_common.cell(None, gap="no ledger"), "-")

    def test_cell_dash_without_gap(self):
        self.assertEqual(md_common.cell(None), "-")

    def test_cell_formats_present_values(self):
        self.assertEqual(md_common.cell(3, fmt=str), "3")
        self.assertEqual(md_common.cell(0, gap="no ledger", fmt=str), "0")

    def test_gap_of_reads_the_sibling_note(self):
        self.assertEqual(md_common.gap_of({"accepted_gap": "none merged"}, "accepted"), "none merged")
        self.assertIsNone(md_common.gap_of({}, "accepted"))

    def test_section_md_wraps_body(self):
        self.assertEqual(md_common.section_md("Cost", "body"), "### Cost\n\nbody\n")


if __name__ == "__main__":
    unittest.main()
