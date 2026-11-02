"""Tests for debug_armory.py: representative choice, keying, capacity, the id audit and line endings."""

import io
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

import debug_armory as armory

NORMAL = "ITEM_QUALITY_Normal"
GOOD = "ITEM_QUALITY_Good"
FANTASTIC = "ITEM_QUALITY_Fantastic"


def run_main(argv):
    out, err = io.StringIO(), io.StringIO()
    with redirect_stdout(out), redirect_stderr(err):
        code = armory.main(argv)
    return code, err.getvalue()


class PickWeaponRepsTest(unittest.TestCase):
    def test_a_bound_item_beats_an_unbound_one_with_a_lower_id(self):
        key = (("ITEM_SMG",), 1, NORMAL)
        reps = armory.pick_weapon_reps({1: key, 2: key}, ranged={2})
        self.assertEqual(reps[key], (2, False))

    def test_the_lowest_unbound_id_wins_when_nothing_is_bound(self):
        key = (("ITEM_SMG",), 1, NORMAL)
        reps = armory.pick_weapon_reps({5: key, 3: key}, ranged=set())
        self.assertEqual(reps[key], (3, True))

    def test_the_lowest_bound_id_wins_among_bound_items(self):
        key = (("ITEM_SMG",), 1, NORMAL)
        reps = armory.pick_weapon_reps({4: key, 7: key}, ranged={4, 7})
        self.assertEqual(reps[key], (4, False))

    def test_quality_keeps_representatives_apart(self):
        normal = (("ITEM_SMG",), 1, NORMAL)
        good = (("ITEM_SMG",), 1, GOOD)
        reps = armory.pick_weapon_reps({1: normal, 2: good}, ranged=set())
        self.assertEqual(reps, {normal: (1, True), good: (2, True)})


class DropQualityTest(unittest.TestCase):
    def test_the_highest_quality_is_kept_and_null_loses(self):
        reps = {
            (("ITEM_SMG",), 1, NORMAL): (10, False),
            (("ITEM_SMG",), 1, FANTASTIC): (20, False),
            (("ITEM_SMG",), 1, None): (30, True),
        }
        self.assertEqual(armory.drop_quality(reps), {(("ITEM_SMG",), 1): (20, False)})


class SortKeyTest(unittest.TestCase):
    def test_quality_sorts_worst_to_best_and_null_last(self):
        names = [None, FANTASTIC, "ITEM_QUALITY_Poor", GOOD, NORMAL, "ITEM_QUALITY_Great"]
        ordered = sorted(names, key=lambda q: armory.sort_key((0, 1, q)))
        self.assertEqual(
            ordered,
            ["ITEM_QUALITY_Poor", NORMAL, GOOD, "ITEM_QUALITY_Great", FANTASTIC, None],
        )

    def test_an_unknown_quality_is_an_error(self):
        with self.assertRaises(KeyError):
            armory.sort_key((0, 1, "ITEM_QUALITY_Mythic"))


class FamilyKeyTest(unittest.TestCase):
    def test_only_gated_monikers_form_the_family(self):
        names = {10: "ITEM_A", 11: "ITEM_B", 12: "ITEM_C"}
        self.assertEqual(armory.family_key({10, 11, 12}, {10, 12}, names), ("ITEM_A", "ITEM_C"))

    def test_a_moniker_without_a_name_is_keyed_by_its_id(self):
        self.assertEqual(armory.family_key({99}, {99}, {}), ("99",))

    def test_no_gated_moniker_gives_the_empty_family(self):
        self.assertEqual(armory.family_key({10}, set(), {10: "ITEM_A"}), ())


class EquipmentKeysTest(unittest.TestCase):
    def test_the_slot_is_the_lowest_equipment_container(self):
        attrs = {
            1: {"containers": [3, 5, 9], "tier": 1, "quality": NORMAL},
            2: {"containers": [3], "tier": 1, "quality": NORMAL},
            3: {"containers": [2], "tier": 1, "quality": NORMAL},
        }
        self.assertEqual(armory.equipment_keys(attrs), {1: (5, 1, NORMAL)})

    def test_the_lowest_item_id_represents_a_slot_tier_and_quality(self):
        key = (4, 1, NORMAL)
        self.assertEqual(armory.pick_equipment_reps({8: key, 6: key}), {key: (6, False)})


class CapacityTest(unittest.TestCase):
    def test_rows_split_into_vendors_of_at_most_the_cap(self):
        self.assertEqual(armory.vendor_count(19, 0, 19), 1)
        self.assertEqual(armory.vendor_count(20, 0, 19), 2)
        self.assertEqual(armory.vendor_count(73, 41, 19), 7)

    def test_the_default_cap_is_the_proven_buy_list_size(self):
        self.assertEqual(armory.DEFAULT_CAP, 19)
        self.assertEqual(armory.MAX_VENDORS, 39)


class AuditTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def _seed(self, name, text):
        path = self.dir / name
        path.write_bytes(text.encode("utf-8"))
        return path

    def test_a_free_block_exits_0(self):
        path = self._seed("a.sql", "INSERT INTO entity_templates (template_id, name) VALUES (1700, 'x');\n")
        lines, code = armory.audit([path])
        self.assertEqual(code, 0)
        self.assertIn("templates 1610-1659: free (50 ids)", lines)

    def test_an_injected_collision_exits_1(self):
        path = self._seed("b.sql", "INSERT INTO entity_templates (template_id, name) VALUES (1620, 'x');\n")
        lines, code = armory.audit([path])
        self.assertEqual(code, 1)
        self.assertIn("  entity_templates.template_id = 1620: b.sql", lines)

    def test_the_item_list_block_reports_a_collision(self):
        path = self._seed("c.sql", "INSERT INTO item_lists (item_list_id, name) VALUES (1311, 'x');\n")
        _, code = armory.audit([path])
        self.assertEqual(code, 1)


class LineEndingTest(unittest.TestCase):
    """A new catalog is CRLF, and a write keeps the file's own line ending."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.saved_out = armory.OUT
        armory.OUT = Path(self.tmp.name) / "catalog.md"

    def tearDown(self):
        armory.OUT = self.saved_out
        self.tmp.cleanup()

    def test_a_new_catalog_is_crlf(self):
        code, _ = run_main(["survey"])
        self.assertEqual(code, 0)
        data = armory.OUT.read_bytes()
        self.assertIn(b"\r\n", data)
        self.assertNotIn(b"\n", data.replace(b"\r\n", b""))

    def test_a_write_keeps_the_files_line_ending(self):
        armory.OUT.write_bytes(b"stale\n")
        code, _ = run_main(["survey"])
        self.assertEqual(code, 0)
        self.assertNotIn(b"\r\n", armory.OUT.read_bytes())

    def test_check_passes_for_an_lf_or_crlf_copy(self):
        doc = armory.build()
        for eol in ("\n", "\r\n"):
            armory.OUT.write_bytes(doc.replace("\n", eol).encode("utf-8"))
            code, err = run_main(["survey", "--check"])
            self.assertEqual(code, 0, f"eol {eol!r}: {err}")

    def test_check_fails_on_drift(self):
        armory.OUT.write_bytes(armory.build().replace("Summary", "Sumary").encode("utf-8"))
        code, _ = run_main(["survey", "--check"])
        self.assertEqual(code, 1)


if __name__ == "__main__":
    unittest.main()
