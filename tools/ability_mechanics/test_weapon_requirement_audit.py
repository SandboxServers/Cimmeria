"""Tests for weapon_requirement_audit.py (Class Start v6 CS-07).

They read the committed seeds, so a seed change that breaks a starter case
(592 + 55, 598 + 21, 598 + 3260 refused, 1984 + 2797, 1639 + 4565) fails
here, not only in the regenerated doc.
"""

import unittest

import weapon_requirement_audit as audit


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


if __name__ == "__main__":
    unittest.main()
