"""Audit every player-reachable ability that requires a weapon moniker.

Class Start v6, packet CS-07 (decision OD-CS11). The server refuses a
player's cast of an ability whose ``abilities.item_monikers`` is non-empty
unless the active bandolier weapon carries at least one of those monikers.
This script reads the seeds (never a database) and writes
``docs/analysis/class-start-v6/weapon-requirement-audit.md``:

- one row per player-reachable ability with a requirement:
  ``ability_id | name | sources | required monikers | valid shipped weapons |
  PASS/FAIL``. FAIL means no shipped bandolier item carries a required
  moniker. A FAIL is a data-correction row, never a reason to weaken the rule.
- the weapon-granted abilities (``items_event_sets``) whose own requirement
  the granting weapon does not meet: those buttons would be refused by the
  weapon that grants them.

Player-reachable means: a char-creation grant, an archetype tree node, a
trainer list entry, a weapon/item event binding, or a content
``grant_ability`` action.

Run from the repo root with stock Python 3:

    python tools/ability_mechanics/weapon_requirement_audit.py          # rewrite the doc
    python tools/ability_mechanics/weapon_requirement_audit.py --check  # exit 1 if it drifted
    python tools/ability_mechanics/weapon_requirement_audit.py --stdout # print, write nothing
"""

from __future__ import annotations

import argparse
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Set

sys.path.insert(0, str(Path(__file__).resolve().parent))
from seed_sql import REPO_ROOT, read_text, sql_rows  # noqa: E402

OUT = Path("docs/analysis/class-start-v6/weapon-requirement-audit.md")

ABILITIES = Path("db/resources/Abilities/Seed/abilities.sql")
TRAINER = Path("db/resources/Abilities/Seed/trainer_abilities.sql")
ITEMS = Path("db/resources/Items/Seed/items.sql")
ITEM_EVENTS = Path("db/resources/Items/Seed/items_event_sets.sql")
MONIKERS = Path("db/resources/Entities/Seed/monikers.sql")
CHAR_CREATION = Path("db/resources/Archetypes/Seed/char_creation_abilities.sql")
CHAR_CREATION_ITEMS = Path("db/resources/Archetypes/Seed/char_creation_items.sql")
TREE = Path("db/resources/Archetypes/Seed/archetype_ability_tree.sql")
CONTENT_DIR = Path("db/resources/Content/Seed")

# `containers.container_id` of the bandolier: only an item that can sit
# there can be the active weapon.
BANDOLIER_CONTAINER = 3

# `items_event_sets.event_id` values (`spawner::EVENT_ITEM_*`).
EVENT_NAMES = {5: "use", 6: "melee", 7: "ranged"}

MAX_EXAMPLES = 4

# The Class Start v6 starter cases (CS-07 brief): (ability, item, expected).
STARTER_CASES = [
    (592, 55, True),
    (598, 21, True),
    (598, 3260, False),
    (1984, 2797, True),
    (1639, 4565, True),
]

_GRANT_ABILITY = re.compile(r"'grant_ability'\s*,\s*(\d+)")


def int_array(text: str | None) -> List[int]:
    if not text:
        return []
    body = text.strip().strip("{}")
    return [int(x) for x in body.split(",") if x.strip()]


def rows(path: Path, table: str):
    text, _ = read_text(path)
    return sql_rows(text, table)


def load():
    monikers = {int(r["moniker_id"].text): r["name"].text for r in rows(MONIKERS, "monikers")}

    abilities = {}
    for r in rows(ABILITIES, "abilities"):
        abilities[int(r["ability_id"].text)] = {
            "name": (r["name"].text or "").strip(),
            "item_monikers": int_array(r["item_monikers"].text),
        }

    items = {}
    for r in rows(ITEMS, "items"):
        items[int(r["item_id"].text)] = {
            "name": (r["name"].text or "").strip(),
            "monikers": set(int_array(r["moniker_ids"].text)),
            "bandolier": BANDOLIER_CONTAINER in int_array(r["container_sets"].text),
        }

    sources: Dict[int, Set[str]] = defaultdict(set)
    for r in rows(CHAR_CREATION, "char_creation_abilities"):
        sources[int(r["ability_id"].text)].add("char_creation")
    for r in rows(TREE, "archetype_ability_tree"):
        sources[int(r["ability_id"].text)].add("tree")
    for r in rows(TRAINER, "trainer_abilities"):
        sources[int(r["ability_id"].text)].add("trainer")
    item_events = []
    for r in rows(ITEM_EVENTS, "items_event_sets"):
        item_id = int(r["item_id"].text)
        ability_id = int(r["ability_id"].text)
        event_id = int(r["event_id"].text)
        item_events.append((item_id, ability_id, event_id))
        sources[ability_id].add("item_event")
    for path in sorted((REPO_ROOT / CONTENT_DIR).glob("*.sql")):
        text, _ = read_text(path.relative_to(REPO_ROOT))
        for m in _GRANT_ABILITY.finditer(text):
            sources[int(m.group(1))].add("content_grant")
    starter_items = {int(r["item_id"].text) for r in rows(CHAR_CREATION_ITEMS, "char_creation_items")}
    return monikers, abilities, items, sources, item_events, starter_items


def moniker_label(mid: int, monikers: Dict[int, str]) -> str:
    return f"{monikers.get(mid, 'unnamed moniker')} ({mid})"


def item_label(item_id: int, items) -> str:
    return f"{item_id} {items[item_id]['name']}"


def esc(s: str) -> str:
    return s.replace("|", "\\|")


def build() -> str:
    monikers, abilities, items, sources, item_events, starter_items = load()

    # moniker -> bandolier items carrying it
    by_moniker: Dict[int, List[int]] = defaultdict(list)
    for item_id in sorted(items):
        it = items[item_id]
        if it["bandolier"]:
            for m in it["monikers"]:
                by_moniker[m].append(item_id)

    audited = []
    for ability_id in sorted(sources):
        a = abilities.get(ability_id)
        if a is None or not a["item_monikers"]:
            continue
        valid = sorted({i for m in a["item_monikers"] for i in by_moniker.get(m, [])})
        audited.append((ability_id, a, sorted(sources[ability_id]), valid))

    passes = [x for x in audited if x[3]]
    fails = [x for x in audited if not x[3]]

    own_weapon_refused = []
    for item_id, ability_id, event_id in sorted(item_events):
        a = abilities.get(ability_id)
        it = items.get(item_id)
        if a is None or it is None or not a["item_monikers"]:
            continue
        if not (set(a["item_monikers"]) & it["monikers"]):
            own_weapon_refused.append((item_id, ability_id, event_id, a, it))

    reachable_total = sum(1 for i in sources if i in abilities)
    lines: List[str] = []
    w = lines.append
    w("# Class Start v6: weapon-requirement audit (CS-07)")
    w("")
    w("Generated by `tools/ability_mechanics/weapon_requirement_audit.py` from the")
    w("seeds under `db/resources/`. Do not edit by hand: change the seed and rerun")
    w("the script (`--check` fails when this file has drifted).")
    w("")
    w("The rule (OD-CS11, [ledger](README.md)): a **player** cast of an ability")
    w("whose `abilities.item_monikers` is non-empty is refused with")
    w("`CONDITION_FEEDBACK_WrongWeaponType` (63) unless the active bandolier weapon")
    w("carries at least one of those monikers. An ability with no requirement is")
    w("unchanged, and NPC casts are not checked.")
    w("")
    w("Player-reachable sources: `char_creation` (char_creation_abilities), `tree`")
    w("(archetype_ability_tree), `trainer` (trainer_abilities), `item_event`")
    w("(items_event_sets) and `content_grant` (a `grant_ability` content action).")
    w("A valid weapon is an item that can sit in the bandolier (container 3) and")
    w("carries a required moniker. A content `launch_ability` action applies its")
    w("effects directly (`content::effect_apply`), not through a player's launch,")
    w("so it is not checked and is not a source here.")
    w("")
    w("## Summary")
    w("")
    w(f"- Player-reachable abilities: {reachable_total}")
    w(f"- With a weapon requirement: {len(audited)}")
    w(f"- PASS: {len(passes)}")
    w(f"- FAIL (no shipped weapon satisfies it): {len(fails)}")
    w(f"- Weapon-granted abilities their own weapon does not satisfy: {len(own_weapon_refused)}")
    w("")
    w("## Starter cases")
    w("")
    w("| ability | item | expected | seed result |")
    w("|---|---|---|---|")
    for ability_id, item_id, expected in STARTER_CASES:
        a = abilities[ability_id]
        ok = bool(set(a["item_monikers"]) & items[item_id]["monikers"]) or not a["item_monikers"]
        verdict = "fires" if ok else "refused (WrongWeaponType)"
        want = "fires" if expected else "refused (WrongWeaponType)"
        flag = "" if ok == expected else " **MISMATCH**"
        w(f"| {ability_id} {esc(a['name'])} | {item_label(item_id, items)} | {want} | {verdict}{flag} |")
    w("")
    w("## Data-correction rows")
    w("")
    w("A FAIL is bad recovered data, not a reason to weaken the rule. Each row")
    w("needs a seed fix (the ability's `item_monikers` or a weapon's `moniker_ids`)")
    w("before the ability can be used by a player.")
    w("")
    if fails:
        w("| ability_id | name | sources | required monikers |")
        w("|---|---|---|---|")
        for ability_id, a, src, _ in fails:
            req = ", ".join(moniker_label(m, monikers) for m in a["item_monikers"])
            w(f"| {ability_id} | {esc(a['name'])} | {', '.join(src)} | {esc(req)} |")
    else:
        w("None.")
    w("")
    w("### Weapon-granted abilities refused by their own weapon")
    w("")
    w("An `items_event_sets` row binds the ability to the weapon, but the weapon")
    w("carries none of the ability's required monikers, so pressing the granted")
    w("button with that weapon active is refused.")
    w("")
    if own_weapon_refused:
        groups: Dict[tuple, List[int]] = defaultdict(list)
        for item_id, ability_id, event_id, a, it in own_weapon_refused:
            groups[(ability_id, event_id, tuple(sorted(it["monikers"])))].append(item_id)
        w("Grouped by ability and the weapons' monikers:")
        w("")
        w("| ability_id | ability | event | required monikers | weapons' monikers | weapons | starter weapon? |")
        w("|---|---|---|---|---|---|---|")
        for (ability_id, event_id, have), group in sorted(groups.items()):
            a = abilities[ability_id]
            req = ", ".join(moniker_label(m, monikers) for m in a["item_monikers"])
            have_s = ", ".join(moniker_label(m, monikers) for m in have) or "none"
            names = sorted({items[i]["name"] for i in group})
            span = f"{len(group)} ({', '.join(names)}; ids {group[0]}..{group[-1]})"
            starter = "yes" if any(i in starter_items for i in group) else "no"
            w(
                f"| {ability_id} | {esc(a['name'])} | {EVENT_NAMES.get(event_id, event_id)} | "
                f"{esc(req)} | {esc(have_s)} | {esc(span)} | {starter} |"
            )
        w("")
        w("Every row:")
        w("")
        w("| item | event | ability_id | ability | required monikers | item monikers |")
        w("|---|---|---|---|---|---|")
        for item_id, ability_id, event_id, a, it in own_weapon_refused:
            req = ", ".join(moniker_label(m, monikers) for m in a["item_monikers"])
            have = ", ".join(moniker_label(m, monikers) for m in sorted(it["monikers"])) or "none"
            w(
                f"| {item_id} {esc(it['name'])} | {EVENT_NAMES.get(event_id, event_id)} | "
                f"{ability_id} | {esc(a['name'])} | {esc(req)} | {esc(have)} |"
            )
    else:
        w("None.")
    w("")
    w("## Every audited ability")
    w("")
    w("| ability_id | name | sources | required monikers | valid shipped weapons | result |")
    w("|---|---|---|---|---|---|")
    for ability_id, a, src, valid in audited:
        req = ", ".join(moniker_label(m, monikers) for m in a["item_monikers"])
        if valid:
            shown = "; ".join(item_label(i, items) for i in valid[:MAX_EXAMPLES])
            more = f" (+{len(valid) - MAX_EXAMPLES} more)" if len(valid) > MAX_EXAMPLES else ""
            weapons = shown + more
        else:
            weapons = "none"
        result = "PASS" if valid else "FAIL"
        w(
            f"| {ability_id} | {esc(a['name'])} | {', '.join(src)} | {esc(req)} | "
            f"{esc(weapons)} | {result} |"
        )
    w("")
    return "\n".join(lines)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="exit 1 if the committed doc drifted")
    ap.add_argument("--stdout", action="store_true", help="print the doc, write nothing")
    args = ap.parse_args()
    doc = build()
    if args.stdout:
        sys.stdout.write(doc)
        return 0
    data = doc.replace("\n", "\r\n").encode("utf-8")
    path = REPO_ROOT / OUT
    if args.check:
        if not path.exists() or path.read_bytes() != data:
            print(f"{OUT} is stale: rerun {Path(__file__).name}", file=sys.stderr)
            return 1
        return 0
    path.write_bytes(data)
    print(f"wrote {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
