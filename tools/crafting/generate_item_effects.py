#!/usr/bin/env python3
"""Generate the crafting item-effect seed: which items teach a blueprint and
which raise a racial paradigm when used.

Reads:

* ``docs/analysis/crafting/source/blueprint-items.csv``: the Blueprint item
  to blueprint mapping recovered from the client's cooked data. Only rows
  whose ``confidence`` is ``high``, ``medium-high`` or ``medium`` are seeded
  (193 items). ``blueprint_id`` may list several ids separated by ``;``;
  for a seeded row every listed id is taught (item 8882 teaches two). The
  96 ``none`` rows name no product or only a group of candidates and are
  never seeded: an item mapped to the wrong blueprint would teach the
  player something the item does not say.
* ``db/resources/Items/Seed/items.sql``: the item ids, and the five
  "Racial Paradigm Guide: <paradigm>" items, matched to
  ``db/resources/Archetypes/Seed/racial_paradigm.sql`` by paradigm name.
* ``db/resources/Entities/Seed/blueprints.sql``: every mapped blueprint must
  exist.

Writes ``db/resources/Items/Seed/crafting_item_effects.sql``. The generated
SQL is committed, so CI never runs this script.

Usage (from the repo root, stock Python 3):

    python tools/crafting/generate_item_effects.py           # regenerate
    python tools/crafting/generate_item_effects.py --check   # exit 1 on drift

Exit codes: 0 ok, 1 drift (``--check``), 2 validation or input failure.
"""

from __future__ import annotations

import argparse
import csv
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

MAPPING_CSV = Path("docs/analysis/crafting/source/blueprint-items.csv")
ITEMS_SQL = Path("db/resources/Items/Seed/items.sql")
BLUEPRINTS_SQL = Path("db/resources/Entities/Seed/blueprints.sql")
PARADIGMS_SQL = Path("db/resources/Archetypes/Seed/racial_paradigm.sql")
OUT_SQL = Path("db/resources/Items/Seed/crafting_item_effects.sql")

CSV_HEADER = [
    "item_id",
    "item_name",
    "blueprint_id",
    "product_id",
    "method",
    "confidence",
    "note",
]
SEEDED_CONFIDENCE = ("high", "medium-high", "medium")

# Pinned counts: a changed mapping must be a deliberate edit here too.
EXPECTED_BLUEPRINT_ITEMS = 193
EXPECTED_BLUEPRINT_ROWS = 194
EXPECTED_GUIDES = 5
MAX_PARADIGM_ID = 5

GUIDE_PREFIX = "Racial Paradigm Guide: "

# Descriptions may span lines, so items are matched over the whole file.
ITEM_ROW = re.compile(
    r"^INSERT INTO items \(item_id, applied_science_id, description, icon_location, name,"
    r"[^)]*\) VALUES \((\d+), (?:NULL|\d+), '((?:[^']|'')*)', '((?:[^']|'')*)', '((?:[^']|'')*)',",
    re.MULTILINE,
)
BLUEPRINT_ROW = re.compile(r"^INSERT INTO blueprints \([^)]*\) VALUES \((\d+),")
PARADIGM_ROW = re.compile(
    r"^INSERT INTO racial_paradigm \(id, name\) VALUES \((\d+), '((?:[^']|'')*)'\);"
)


class InputError(Exception):
    pass


def read_lines(rel: Path) -> list[str]:
    path = REPO_ROOT / rel
    if not path.is_file():
        raise InputError(f"{rel}: not found")
    return path.read_text(encoding="utf-8").splitlines()


def sql_unquote(s: str) -> str:
    return s.replace("''", "'")


def load_items() -> dict[int, str]:
    """item_id -> seed name."""
    text = "\n".join(read_lines(ITEMS_SQL))
    items = {int(m.group(1)): sql_unquote(m.group(4)) for m in ITEM_ROW.finditer(text)}
    if not items:
        raise InputError(f"{ITEMS_SQL}: no item rows parsed")
    return items


def load_ids(rel: Path, pattern: re.Pattern[str]) -> set[int]:
    ids = {int(m.group(1)) for line in read_lines(rel) if (m := pattern.match(line))}
    if not ids:
        raise InputError(f"{rel}: no rows parsed")
    return ids


def load_paradigms() -> dict[str, int]:
    """paradigm name -> id."""
    paradigms = {
        sql_unquote(m.group(2)): int(m.group(1))
        for line in read_lines(PARADIGMS_SQL)
        if (m := PARADIGM_ROW.match(line))
    }
    if len(paradigms) != MAX_PARADIGM_ID or set(paradigms.values()) != set(
        range(1, MAX_PARADIGM_ID + 1)
    ):
        raise InputError(f"{PARADIGMS_SQL}: expected paradigm ids 1..{MAX_PARADIGM_ID}")
    return paradigms


def blueprint_rows(items: dict[int, str], blueprints: set[int]) -> list[tuple[int, int, str]]:
    """(item_id, blueprint_id, item_name) for every seeded mapping row."""
    path = REPO_ROOT / MAPPING_CSV
    if not path.is_file():
        raise InputError(f"{MAPPING_CSV}: not found")
    with path.open(encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)
        if reader.fieldnames != CSV_HEADER:
            raise InputError(f"{MAPPING_CSV}: header {reader.fieldnames} != {CSV_HEADER}")
        rows = list(reader)

    out: list[tuple[int, int, str]] = []
    seeded_items: set[int] = set()
    for r in rows:
        if r["confidence"] not in SEEDED_CONFIDENCE:
            continue
        item_id = int(r["item_id"])
        if item_id in seeded_items:
            raise InputError(f"{MAPPING_CSV}: item {item_id} listed twice")
        if item_id not in items:
            raise InputError(f"{MAPPING_CSV}: item {item_id} is not in {ITEMS_SQL}")
        if not r["item_name"].startswith("Blueprint: "):
            raise InputError(f"{MAPPING_CSV}: item {item_id} is not a Blueprint item")
        ids = [int(x) for x in r["blueprint_id"].split(";") if x]
        if not ids:
            raise InputError(f"{MAPPING_CSV}: item {item_id} names no blueprint")
        for blueprint_id in ids:
            if blueprint_id not in blueprints:
                raise InputError(
                    f"{MAPPING_CSV}: item {item_id} names blueprint {blueprint_id}, "
                    f"not in {BLUEPRINTS_SQL}"
                )
            out.append((item_id, blueprint_id, r["item_name"]))
        seeded_items.add(item_id)

    if len(seeded_items) != EXPECTED_BLUEPRINT_ITEMS or len(out) != EXPECTED_BLUEPRINT_ROWS:
        raise InputError(
            f"{MAPPING_CSV}: {len(seeded_items)} items / {len(out)} rows seeded, expected "
            f"{EXPECTED_BLUEPRINT_ITEMS} / {EXPECTED_BLUEPRINT_ROWS}"
        )
    return sorted(out)


def guide_rows(items: dict[int, str], paradigms: dict[str, int]) -> list[tuple[int, int, str]]:
    """(item_id, paradigm_id, item_name) for each Racial Paradigm Guide."""
    out = []
    for item_id, name in items.items():
        if not name.startswith(GUIDE_PREFIX):
            continue
        paradigm = name[len(GUIDE_PREFIX) :]
        if paradigm not in paradigms:
            raise InputError(f"{ITEMS_SQL}: item {item_id} names unknown paradigm {paradigm!r}")
        out.append((item_id, paradigms[paradigm], name))
    if len(out) != EXPECTED_GUIDES or len({p for _, p, _ in out}) != EXPECTED_GUIDES:
        raise InputError(f"{ITEMS_SQL}: expected one guide per paradigm, found {out}")
    return sorted(out)


def render(blueprint: list[tuple[int, int, str]], guides: list[tuple[int, int, str]]) -> str:
    lines = [
        "--",
        "-- Data for Name: crafting_item_effects; Type: TABLE DATA; Schema: resources; Owner: -",
        "--",
        "-- GENERATED by tools/crafting/generate_item_effects.py; do not edit by hand.",
        "-- Blueprint items come from docs/analysis/crafting/source/blueprint-items.csv",
        "-- (confidence high, medium-high or medium only); the Racial Paradigm Guides",
        "-- are matched to racial_paradigm by name.",
        "--",
        "",
        "-- Blueprint items: using one teaches its blueprint(s).",
    ]
    for item_id, blueprint_id, name in blueprint:
        lines.append(f"-- {name}")
        lines.append(
            "INSERT INTO crafting_item_effects (item_id, blueprint_id, racial_paradigm_id) "
            f"VALUES ({item_id}, {blueprint_id}, NULL);"
        )
    lines.append("")
    lines.append("-- Racial Paradigm Guides: using one raises its paradigm by 1, to at most 10.")
    for item_id, paradigm_id, name in guides:
        lines.append(f"-- {name}")
        lines.append(
            "INSERT INTO crafting_item_effects (item_id, blueprint_id, racial_paradigm_id) "
            f"VALUES ({item_id}, NULL, {paradigm_id});"
        )
    return "\r\n".join(lines) + "\r\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="exit 1 if the committed seed drifted")
    args = parser.parse_args()

    try:
        items = load_items()
        blueprints = load_ids(BLUEPRINTS_SQL, BLUEPRINT_ROW)
        paradigms = load_paradigms()
        text = render(blueprint_rows(items, blueprints), guide_rows(items, paradigms))
    except InputError as e:
        print(f"generate_item_effects: {e}", file=sys.stderr)
        return 2

    out = REPO_ROOT / OUT_SQL
    if args.check:
        current = out.read_bytes().decode("utf-8") if out.is_file() else ""
        if current.replace("\r\n", "\n") != text.replace("\r\n", "\n"):
            print(f"generate_item_effects: {OUT_SQL} is out of date; rerun without --check")
            return 1
        print(f"generate_item_effects: {OUT_SQL} is up to date")
        return 0
    out.write_bytes(text.encode("utf-8"))
    print(f"generate_item_effects: wrote {OUT_SQL}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
