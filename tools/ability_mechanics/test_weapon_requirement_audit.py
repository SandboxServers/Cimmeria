"""Tests for weapon_requirement_audit.py (Class Start v6 CS-07).

They read the committed seeds, so a seed change that breaks a starter case
(592 + 55, 598 + 21, 598 + 3260 refused, 1984 + 2797, 1639 + 4565) fails
here, not only in the regenerated doc.
"""

import io
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

import weapon_requirement_audit as audit


def run_main(argv):
    out, err = io.StringIO(), io.StringIO()
    with redirect_stdout(out), redirect_stderr(err):
        code = audit.main(argv)
    return code, err.getvalue()


class WeaponRequirementAuditTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc = audit.build()

    def test_starter_cases_match_their_expectation(self):
        self.assertNotIn("**MISMATCH**", self.doc)
        self.assertIn(
            "| 598 Quick Burst | 3260 SK37 LMG | refused (WrongWeaponType) | refused (WrongWeaponType) |",
            self.doc,
        )

    def test_every_audited_row_has_a_result(self):
        section = self.doc.split("## Every audited ability", 1)[1]
        rows = [line for line in section.splitlines() if line.startswith("| ") and line[2].isdigit()]
        self.assertTrue(rows, "the audit found no weapon-requiring player ability")
        for line in rows:
            self.assertTrue(line.endswith("| PASS |") or line.endswith("| FAIL |"), line)

    def test_output_is_deterministic(self):
        self.assertEqual(self.doc, audit.build())

    def test_self_and_ground_abilities_are_listed_as_rust_only(self):
        """Python checked the weapon only for TargetTarget abilities (review
        finding 5): 1250 Escape (Self) and 1482 Ground Blast (Ground) are
        gated by the Rust server alone; 598 (Target) is not listed."""
        section = self.doc.split("## Gated by the Rust server only", 1)[1].split("\n## ", 1)[0]
        self.assertIn("| 1250 | Escape | Self |", section)
        self.assertIn("| 1482 | Ground Blast | Ground |", section)
        self.assertNotIn("| 598 |", section)

    def test_right_click_class_lists_weapons_with_no_ranged_binding(self):
        """Review finding 2: the bandolier weapons right-click cannot fire
        are listed; the 50 sniper rifles are not, since CS-07 bound 581."""
        section = self.doc.split("## Right-click with no RANGED binding", 1)[1].split("\n## ", 1)[0]
        self.assertIn("| ITEM_Blade | no | melee | 50 |", section)
        self.assertNotIn("ITEM_Rifle |", section)
        self.assertIn("51 rifles are bound to 581 Rifle Auto Attack", section)


class ContentGrantParsingTest(unittest.TestCase):
    """Review finding 3: a content grant is read from `params.ability_ids`,
    the only form the content loader accepts."""

    SEED = (
        "INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)\n"
        "VALUES\n"
        "  (1, 'add_item', 55, NULL, '{\"container\": 1}', 0, 0),\n"
        "  -- a comment between rows\n"
        "  (1, 'grant_ability', NULL, NULL,\n"
        "   '{\"ability_ids\": [592, 1218], \"source_kind\": \"tutorial\"}', 0, 1);\n"
    )

    def test_params_ability_ids_are_read(self):
        rows = audit.content_action_rows(self.SEED)
        self.assertEqual(len(rows), 2)
        self.assertEqual([audit.granted_ability_ids(r) for r in rows], [[], [592, 1218]])

    def test_the_committed_seeds_grant_through_params(self):
        """CS-04/CS-05 grant 598 (Soldier signature): it must show as a
        content grant in the audit."""
        doc = audit.build()
        row = next(line for line in doc.splitlines() if line.startswith("| 598 | Quick Burst |"))
        self.assertIn("content_grant", row)


class CheckLineEndingTest(unittest.TestCase):
    """Review finding 6: `--check` compares content, not line endings, and a
    write keeps the file's own line ending."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.saved_out = audit.OUT
        audit.OUT = Path(self.tmp.name) / "audit.md"

    def tearDown(self):
        audit.OUT = self.saved_out
        self.tmp.cleanup()

    def test_check_passes_for_an_lf_or_crlf_copy(self):
        doc = audit.build()
        for eol in ("\n", "\r\n"):
            audit.OUT.write_bytes(doc.replace("\n", eol).encode("utf-8"))
            code, err = run_main(["--check"])
            self.assertEqual(code, 0, f"eol {eol!r}: {err}")

    def test_check_fails_on_drift(self):
        audit.OUT.write_bytes(audit.build().replace("PASS", "PAS").encode("utf-8"))
        code, _ = run_main(["--check"])
        self.assertEqual(code, 1)

    def test_a_write_keeps_the_files_line_ending(self):
        audit.OUT.write_bytes(b"stale\r\n")
        code, _ = run_main([])
        self.assertEqual(code, 0)
        data = audit.OUT.read_bytes()
        self.assertIn(b"\r\n", data)
        self.assertNotIn(b"\n", data.replace(b"\r\n", b""))

    def test_the_committed_doc_is_current(self):
        audit.OUT = self.saved_out
        code, err = run_main(["--check"])
        self.assertEqual(code, 0, err)


if __name__ == "__main__":
    unittest.main()
