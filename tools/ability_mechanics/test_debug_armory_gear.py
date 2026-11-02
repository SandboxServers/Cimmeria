"""Tests for the Armory gear specs (DR-07): row shapes, the 30-row split, LF output and TOML validity."""

import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import debug_armory_gear as gear  # noqa: E402

REPO = Path(__file__).resolve().parents[2]
COMMITTED = REPO / "docs" / "guides" / "uat-specs"


def _parse(text: str) -> dict:
    return tomllib.loads(text)


class WeaponRowTest(unittest.TestCase):
    def test_bound_weapon_expects_its_ranged_send(self):
        row = gear.weapon_row(3753, "Cobra Staff", 584)
        self.assertIn('event = "client.ability.sent"', row)
        self.assertIn("match_fields = { ability_id = 584 }", row)
        self.assertIn('since = "press"', row)
        self.assertNotIn("weapon_unbound", row)

    def test_unbound_weapon_expects_the_signoz_gap_line(self):
        row = gear.weapon_row(3754, "Plain Club", None)
        self.assertIn('source = "signoz"', row)
        self.assertIn("event = 'weapon_unbound' AND reason = 'no_ranged_attack'", row)
        self.assertIn("item_type_id = 3754", row)
        self.assertNotIn("client.ability.sent", row)

    def test_equip_precedes_the_right_click(self):
        row = gear.weapon_row(3753, "Cobra Staff", 584)
        self.assertLess(row.index("@item_action"), row.index("@world_click"))
        self.assertLess(row.index("@dummy"), row.index("@world_click"))

    def test_row_targets_the_l1_dummy_template(self):
        row = gear.weapon_row(3753, "Cobra Staff", 584)
        self.assertIn(f"template_id = {gear.DUMMY_TEMPLATE}", row)
        self.assertEqual(gear.DUMMY_TEMPLATE, 1310)


class ThrottleTest(unittest.TestCase):
    def test_unbound_row_waits_out_the_log_throttle(self):
        self.assertIn("wait_ms = 10500", gear.weapon_row(1, "x", None))
        self.assertNotIn("wait_ms = 10500", gear.weapon_row(1, "x", 584))


class ArmorRowTest(unittest.TestCase):
    def test_armor_row_has_an_optional_move_persisted_clause(self):
        row = gear.armor_row(77, "Test Helm", 5)
        self.assertIn("Inventory move persisted", row)
        self.assertIn("required = false", row)
        self.assertIn("target_container_id = 5", row)

    def test_armor_row_reads_the_sheet_with_a_human_clause(self):
        row = gear.armor_row(77, "Test Helm", 5)
        self.assertIn('source = "human"', row)
        self.assertIn("@window_read", row)
        self.assertIn("@screenshot", row)
        self.assertIn("container 5", row)

    def test_armor_equips_before_reading_the_sheet(self):
        row = gear.armor_row(77, "Test Helm", 5)
        self.assertLess(row.index("@item_action"), row.index("@window_read"))


class BlocksTest(unittest.TestCase):
    def test_weapon_blocks_one_row_per_representative(self):
        weapons = {(0, None): (3753, False), (1, None): (3754, True)}
        items = {3753: {"name": "Cobra Staff"}, 3754: {"name": "Plain Club"}}
        ranged = {3753: 584}
        blocks = gear.weapon_blocks(weapons, items, ranged)
        self.assertEqual(len(blocks), 2)
        self.assertIn("ability_id = 584", blocks[0])
        self.assertIn("weapon_unbound", blocks[1])

    def test_armor_blocks_use_the_lowest_container_as_slot(self):
        equipment = {(5, None): (77, False)}
        items = {77: {"name": "Test Helm"}}
        blocks = gear.armor_blocks(equipment, items)
        self.assertEqual(len(blocks), 1)
        self.assertIn("container 5", blocks[0])


class SplitTest(unittest.TestCase):
    def test_twenty_rows_stay_in_one_file(self):
        parts = gear.split("armory-gear-armor", [str(i) for i in range(20)])
        self.assertEqual([name for name, _ in parts], ["armory-gear-armor"])
        self.assertEqual(len(parts[0][1]), 20)

    def test_splits_at_thirty_rows(self):
        parts = gear.split("armory-gear-weapons", [str(i) for i in range(73)])
        self.assertEqual(
            [(name, len(rows)) for name, rows in parts],
            [("armory-gear-weapons-1", 30), ("armory-gear-weapons-2", 30), ("armory-gear-weapons-3", 13)],
        )

    def test_exactly_thirty_rows_is_one_file(self):
        self.assertEqual(len(gear.split("s", [str(i) for i in range(30)])), 1)

    def test_thirty_one_rows_split_in_two(self):
        self.assertEqual(len(gear.split("s", [str(i) for i in range(31)])), 2)


class RenderTest(unittest.TestCase):
    def test_render_is_lf_only(self):
        blocks = [gear.weapon_row(3753, "Cobra Staff", 584), gear.armor_row(77, "Test Helm", 5)]
        text = gear.render("armory-gear-weapons-1", "weapons", blocks)
        self.assertNotIn("\r", text)

    def test_rendered_file_parses_as_toml_with_one_row_per_block(self):
        blocks = [
            gear.weapon_row(3753, "Cobra Staff", 584),
            gear.weapon_row(3754, "Plain Club", None),
            gear.armor_row(77, "Test Helm", 5),
        ]
        data = _parse(gear.render("armory-gear-test", "mixed", blocks))
        self.assertEqual(data["schema"], 1)
        self.assertEqual(data["section"]["id"], "armory-gear-test")
        self.assertEqual([r["id"] for r in data["row"]], ["GW-3753", "GW-3754", "GA-77"])
        self.assertEqual(data["row"][0]["expect"][0]["event"], "client.ability.sent")
        self.assertEqual(data["row"][2]["expect"][0]["source"], "human")


class CommittedFilesTest(unittest.TestCase):
    def test_committed_gear_files_parse_and_stay_lf(self):
        paths = sorted(COMMITTED.glob("armory-gear-*.toml"))
        self.assertTrue(paths, "run specs gear --out-dir docs/guides/uat-specs")
        for path in paths:
            raw = path.read_bytes()
            self.assertNotIn(b"\r", raw, path.name)
            data = tomllib.loads(raw.decode("utf-8"))
            self.assertLessEqual(len(data["row"]), gear.ROWS_PER_FILE, path.name)

    def test_each_generated_file_fits_the_main_bag(self):
        with tempfile.TemporaryDirectory() as tmp:
            gear.write_gear_specs(Path(tmp))
            for path in Path(tmp).glob("*.toml"):
                grants = path.read_text(encoding="utf-8").count("/gmgiveitem ")
                self.assertLessEqual(grants, 30, path.name)
                self.assertLess(grants, gear.MAIN_BAG_SLOTS, path.name)

    def test_committed_files_match_the_generator(self):
        with tempfile.TemporaryDirectory() as tmp:
            gear.write_gear_specs(Path(tmp))
            names = {p.name for p in Path(tmp).glob("*.toml")}
            self.assertEqual(names, {p.name for p in COMMITTED.glob("armory-gear-*.toml")})
            for name in names:
                self.assertEqual((Path(tmp) / name).read_bytes(), (COMMITTED / name).read_bytes(), name)

    def test_stale_split_file_is_removed(self):
        with tempfile.TemporaryDirectory() as tmp:
            stale = Path(tmp) / "armory-gear-weapons-9.toml"
            stale.write_text("stale\n", encoding="utf-8")
            gear.write_gear_specs(Path(tmp))
            self.assertFalse(stale.exists())

    def test_generator_writes_split_lf_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.assertEqual(gear.write_gear_specs(out), 0)
            names = sorted(p.name for p in out.glob("*.toml"))
            self.assertTrue(any(n.startswith("armory-gear-weapons") for n in names), names)
            self.assertTrue(any(n.startswith("armory-gear-armor") for n in names), names)
            for path in out.glob("*.toml"):
                self.assertNotIn(b"\r", path.read_bytes(), path.name)


if __name__ == "__main__":
    unittest.main()
