"""Tests for debug_armory_specs.py: a row per tree ability, gear before the press, the 60-row split, LF output."""

import tempfile
import tomllib
import unittest
from pathlib import Path

import debug_armory_specs as specs
from seed_sql import REPO_ROOT

TARGET_UNIT = 2


def ability(name="Shot", monikers=(), target=TARGET_UNIT):
    return {"name": name, "item_monikers": list(monikers), "target_type": target}


def parse_row(lines):
    doc = tomllib.loads("\n".join(lines) + "\n")
    return doc["row"][0]


class AbilityRowTest(unittest.TestCase):
    def test_the_row_gives_and_equips_its_weapon_before_it_places_the_ability(self):
        items = {42: {"name": "Pistol"}}
        monikers = {5: "ITEM_PISTOL"}
        row = parse_row(specs.ability_row("Soldier", 592, ability(monikers=[5]), 42, items, monikers))
        self.assertEqual(row["id"], "Soldier-592")
        setup = row["setup"]
        self.assertEqual(setup[0], {"chat": "/gmgiveitem 42 1", "tier": "G"})
        self.assertEqual(setup[1]["tool"], "@item_action")
        self.assertEqual(setup[1]["args"], {"action": "equip", "name": "Pistol"})
        self.assertEqual(setup[2]["tool"], "@use_ability")
        self.assertEqual(setup[2]["args"], {"ability_id": 592, "place": True})
        self.assertNotIn("blocked", row)

    def test_the_step_presses_the_key_and_is_not_in_setup(self):
        row = parse_row(specs.ability_row("Soldier", 592, ability(), None, {}, {}))
        self.assertEqual(row["step"][0]["args"], {"ability_id": 592, "press": "key", "fallback": "none"})
        self.assertFalse(any("press" in a.get("args", {}) for a in row["setup"]))

    def test_an_ability_with_no_shipped_weapon_is_blocked_and_gives_no_gear(self):
        row = parse_row(specs.ability_row("Soldier", 997, ability(monikers=[9]), None, {}, {9: "ITEM_RIFLE"}))
        self.assertIn("weapon-requirement audit FAIL", row["blocked"])
        self.assertIn("ITEM_RIFLE", row["blocked"])
        self.assertFalse(any("chat" in a for a in row["setup"]))

    def test_a_self_or_ground_ability_does_not_require_the_applied_event(self):
        for target in (1, 3):
            row = parse_row(specs.ability_row("Soldier", 1, ability(target=target), None, {}, {}))
            self.assertFalse(row["expect"][1]["required"])
        row = parse_row(specs.ability_row("Soldier", 1, ability(target=2), None, {}, {}))
        self.assertNotIn("required", row["expect"][1])

    def test_the_sent_clause_matches_the_ability_id(self):
        row = parse_row(specs.ability_row("Soldier", 592, ability(), None, {}, {}))
        self.assertEqual(row["expect"][0]["event"], "client.ability.sent")
        self.assertEqual(row["expect"][0]["match_fields"], {"ability_id": 592})


class SetupRowTest(unittest.TestCase):
    def test_the_section_setup_row_places_the_character_with_every_ability_and_a_dummy(self):
        row = parse_row(specs.setup_row("Soldier"))
        self.assertEqual(row["id"], "Soldier-00")
        self.assertEqual(row["setup"][0]["chat"], specs.SETUP_LOCATION)
        self.assertEqual(row["setup"][1]["chat"], "/gmgiveallabilities")
        self.assertEqual(row["setup"][2], {"tool": "@dummy", "args": {"disposition": "hostile"}})
        self.assertEqual(row["expect"][0]["pointer"], "/world_id")
        self.assertEqual(row["expect"][0]["value"], 1300)


class ArchetypeFilesTest(unittest.TestCase):
    def test_a_tree_of_sixty_rows_is_one_file(self):
        tree = [(1, i, 1000 + i) for i in range(60)]
        abilities = {aid: ability() for _, _, aid in tree}
        files = specs.archetype_files("sgu", "Soldier", tree, abilities, {}, {}, {})
        self.assertEqual(list(files), ["armory-abilities-soldier.toml"])
        self.assertEqual(files["armory-abilities-soldier.toml"].count("[[row]]"), 61)

    def test_a_tree_past_sixty_rows_splits_one_file_per_tree_index(self):
        tree = [(1, i, 2000 + i) for i in range(30)] + [(2, i, 3000 + i) for i in range(31)]
        abilities = {aid: ability() for _, _, aid in tree}
        files = specs.archetype_files("sgu", "Soldier", tree, abilities, {}, {}, {})
        self.assertEqual(sorted(files), ["armory-abilities-soldier-1.toml", "armory-abilities-soldier-2.toml"])
        self.assertEqual(tomllib.loads(files["armory-abilities-soldier-2.toml"])["section"]["id"], "armory-abilities-soldier-2")
        self.assertEqual(files["armory-abilities-soldier-1.toml"].count("[[row]]"), 31)
        self.assertEqual(files["armory-abilities-soldier-2.toml"].count("[[row]]"), 32)

    def test_a_tree_ability_missing_from_the_seed_stops_the_run(self):
        with self.assertRaises(specs.Stop):
            specs.archetype_files("sgu", "Soldier", [(1, 0, 777)], {}, {}, {}, {})


class UncoveredCheckTest(unittest.TestCase):
    def test_the_known_uncovered_abilities_pass(self):
        specs.check_uncovered({997: None, 5: 3}, [997, 5])

    def test_a_new_uncovered_ability_stops_the_run(self):
        with self.assertRaises(specs.Stop):
            specs.check_uncovered({5: None}, [5])


class WriteAbilitySpecsTest(unittest.TestCase):
    def test_every_committed_file_is_lf_parses_and_has_one_row_per_tree_ability(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.assertEqual(specs.write_ability_specs(out, REPO_ROOT), 0)
            files = sorted(out.glob("armory-abilities-*.toml"))
            self.assertTrue(files)
            rows = 0
            for path in files:
                raw = path.read_bytes()
                self.assertNotIn(b"\r", raw, path.name)
                doc = tomllib.loads(raw.decode("utf-8"))
                ids = [r["id"] for r in doc["row"]]
                self.assertEqual(len(ids), len(set(ids)), path.name)
                rows += len(ids)
            self.assertFalse(any("jaffa" in p.name for p in files))
            # Each file is its own section, so each carries its own -00 setup row.
            tree_rows = sum(len(t) for t in specs.tree_by_archetype().values())
            self.assertEqual(rows, tree_rows + len(files))


if __name__ == "__main__":
    unittest.main()
