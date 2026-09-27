#!/usr/bin/env python3
"""Tests for generate_item_effects.py: the mapping is checked against the
parsed ``items.sql`` row, never the CSV's own item name, and the committed
seed matches a fresh generation.

Run from the repo root with stock Python 3:

    python tools/crafting/test_generate_item_effects.py
"""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import generate_item_effects as gen  # noqa: E402

ITEMS = {
    6483: "Blueprint: Steel Plating (Materials Subcombine A)",
    7805: "Racial Paradigm Guide: Human",
}
BLUEPRINTS = {25}


def row(item_id: int, name: str, blueprint: str = "25", confidence: str = "high") -> dict[str, str]:
    return {
        "item_id": str(item_id),
        "item_name": name,
        "blueprint_id": blueprint,
        "product_id": "",
        "method": "name-exact",
        "confidence": confidence,
        "note": "",
    }


class ValidateBlueprintRows(unittest.TestCase):
    def test_a_real_blueprint_item_is_seeded_with_its_seed_name(self) -> None:
        rows = [row(6483, ITEMS[6483])]
        self.assertEqual(
            gen.validate_blueprint_rows(rows, ITEMS, BLUEPRINTS), [(6483, 25, ITEMS[6483])]
        )

    def test_a_csv_name_cannot_make_another_item_a_blueprint(self) -> None:
        # The CSV claims 7805 is a Blueprint item; the seed says it is a guide.
        rows = [row(7805, "Blueprint: Steel Plating (Materials Subcombine A)")]
        with self.assertRaisesRegex(gen.InputError, "not a Blueprint item"):
            gen.validate_blueprint_rows(rows, ITEMS, BLUEPRINTS)

    def test_a_reassigned_item_id_is_refused(self) -> None:
        rows = [row(6483, "Blueprint: Something Else")]
        with self.assertRaisesRegex(gen.InputError, "but the CSV names it"):
            gen.validate_blueprint_rows(rows, ITEMS, BLUEPRINTS)

    def test_unresolved_rows_are_skipped(self) -> None:
        rows = [row(7805, "Blueprint: X", blueprint="1;2", confidence="none")]
        self.assertEqual(gen.validate_blueprint_rows(rows, ITEMS, BLUEPRINTS), [])


class CommittedSeed(unittest.TestCase):
    def test_the_committed_seed_is_up_to_date(self) -> None:
        done = subprocess.run(
            [sys.executable, str(Path(gen.__file__)), "--check"],
            capture_output=True,
            text=True,
        )
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)


if __name__ == "__main__":
    unittest.main()
