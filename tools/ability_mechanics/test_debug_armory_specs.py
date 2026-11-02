"""Tests for debug_armory_specs.py: a row per tree ability, gear before the press, the 60-row split, LF output."""

import re
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
        self.assertEqual(setup[0], {"tool": "@item_action", "args": {"action": "unequip", "container": "Bandolier", "slot": 1}, "label": "reset-slot1", "optional": True})
        self.assertEqual(setup[1], {"tool": "@window_click", "args": {"target": specs.SHEET_CLOSE}, "label": "close-sheet", "optional": True})
        self.assertEqual(setup[2], {"chat": "/gmgiveitem 42 1", "tier": "G"})
        self.assertEqual(setup[3]["tool"], "@item_action")
        self.assertEqual(setup[3]["args"], {"action": "equip", "name": "Pistol"})
        self.assertEqual(setup[4], {"tool": "@window_click", "args": {"target": specs.BAG_CLOSE}, "label": "close-bag", "optional": True})
        self.assertEqual(setup[5], {"tool": "@key", "args": {"key": "F1"}, "label": "select-slot1"})
        self.assertEqual(setup[6], {"wait_ms": 1500})
        self.assertEqual(setup[7]["tool"], "@use_ability")
        self.assertEqual(setup[7]["args"], {"ability_id": 592, "place": True})
        self.assertNotIn("blocked", row)

    def test_a_gated_row_asserts_the_active_weapon(self):
        items = {42: {"name": "Pistol"}}
        monikers = {5: "ITEM_PISTOL"}
        row = parse_row(specs.ability_row("Soldier", 592, ability(monikers=[5]), 42, items, monikers))
        armed = row["expect"][-1]
        self.assertEqual(armed["id"], "armed")
        self.assertEqual(armed["tool"], "@player_state")
        self.assertEqual(armed["pointer"], "/ammo/weapon")
        self.assertEqual(armed["op"], "eq")
        self.assertEqual(armed["value"], "Pistol")
        self.assertNotIn("required", armed)

    def test_an_ungated_row_neither_resets_nor_selects(self):
        ungated = parse_row(specs.ability_row("Soldier", 592, ability(), None, {}, {}))
        blocked = parse_row(specs.ability_row("Soldier", 997, ability(monikers=[9]), None, {}, {9: "ITEM_RIFLE"}))
        for row in (ungated, blocked):
            self.assertFalse(any(a.get("tool") == "@key" for a in row["setup"] if isinstance(a, dict)))
            self.assertFalse(any(a.get("args", {}).get("action") == "unequip" for a in row["setup"] if isinstance(a, dict)))
            self.assertFalse(any(e["id"] == "armed" for e in row["expect"]))

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
        row = parse_row(specs.setup_row("Soldier", "sgu"))
        self.assertEqual(row["id"], "Soldier-00")
        self.assertEqual(row["setup"][0], {"wait_ms": 17000})
        self.assertEqual(row["setup"][1]["chat"], specs.SETUP_LOCATION)
        self.assertEqual(row["setup"][2]["tool"], "client_wait_for")
        self.assertEqual(row["setup"][2]["args"]["lua_condition"], "getCurrentWorldID() == 1300")
        self.assertEqual(row["setup"][4]["chat"], "/gmgiveallabilities")
        self.assertEqual(row["setup"][5], {"tool": "@dummy", "args": {"disposition": "hostile"}})
        self.assertEqual(row["expect"][0]["pointer"], "/world_id")
        self.assertEqual(row["expect"][0]["value"], 1300)

    def test_the_first_login_hold_is_sgu_only(self):
        row = parse_row(specs.setup_row("Goauld", "praxis"))
        self.assertEqual(row["setup"][0]["chat"], specs.SETUP_LOCATION)


class SectionHeaderTest(unittest.TestCase):
    def test_an_sgu_section_skips_the_intro_wait_and_a_praxis_section_keeps_it(self):
        sgu = tomllib.loads("\n".join(specs.section_header("sgu", "Soldier", None)))["section"]["fresh"]
        self.assertIs(sgu["finish_intro"], False)
        praxis = tomllib.loads("\n".join(specs.section_header("praxis", "Goauld", None)))["section"]["fresh"]
        self.assertNotIn("finish_intro", praxis)


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

    def test_known_uncovered_matches_the_live_db_guard(self):
        rust = REPO_ROOT / "crates" / "cell-catalog" / "src" / "cell" / "spawner" / "tests" / "live_db_debug_area_armory.rs"
        match = re.search(r"const KNOWN_UNCOVERED: \[i32; \d+\] = \[([^\]]*)\]", rust.read_text(encoding="utf-8"))
        self.assertIsNotNone(match)
        ids = sorted(int(part) for part in match.group(1).split(",") if part.strip())
        self.assertEqual(ids, sorted(specs.KNOWN_UNCOVERED))


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
                fresh = doc["section"]["fresh"]
                if fresh["alignment"] == "sgu":
                    self.assertIs(fresh.get("finish_intro"), False, path.name)
                else:
                    self.assertNotIn("finish_intro", fresh, path.name)
                ids = [r["id"] for r in doc["row"]]
                self.assertEqual(len(ids), len(set(ids)), path.name)
                rows += len(ids)
            self.assertFalse(any("jaffa" in p.name for p in files))
            # Each file is its own section, so each carries its own -00 setup row.
            tree_rows = sum(len(t) for t in specs.tree_by_archetype().values())
            self.assertEqual(rows, tree_rows + len(files))

    def test_committed_files_match_the_generator(self):
        committed = REPO_ROOT / "docs" / "guides" / "uat-specs"
        with tempfile.TemporaryDirectory() as tmp:
            specs.write_ability_specs(Path(tmp), REPO_ROOT)
            names = {p.name for p in Path(tmp).glob("armory-abilities-*.toml")}
            self.assertEqual(names, {p.name for p in committed.glob("armory-abilities-*.toml")})
            for name in names:
                self.assertEqual((Path(tmp) / name).read_bytes(), (committed / name).read_bytes(), name)


if __name__ == "__main__":
    unittest.main()
