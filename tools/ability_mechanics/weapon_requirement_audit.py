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
- the audited abilities python never checked: it applied the rule only to
  ``TargetTarget`` abilities, so the self- and ground-targeted ones are gated
  by the Rust server alone.
- the bandolier weapons with no RANGED binding (``items_event_sets`` event
  7): right-click on a hostile fires nothing with them and says so.

Player-reachable means: a char-creation grant (start profile or debug kit),
an archetype tree node, a trainer list entry, a weapon/item event binding, or
a content ``grant_ability`` action (its ``params.ability_ids``, the only form
the content loader accepts).

Run from the repo root with stock Python 3:

    python tools/ability_mechanics/weapon_requirement_audit.py          # rewrite the doc
    python tools/ability_mechanics/weapon_requirement_audit.py --check  # exit 1 if it drifted
    python tools/ability_mechanics/weapon_requirement_audit.py --stdout # print, write nothing
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Set

sys.path.insert(0, str(Path(__file__).resolve().parent))
from seed_sql import REPO_ROOT, InputError, Value, parse_values, read_text, sql_rows, write_text  # noqa: E402

OUT = Path("docs/analysis/class-start-v6/weapon-requirement-audit.md")

ABILITIES = Path("db/resources/Abilities/Seed/abilities.sql")
TRAINER = Path("db/resources/Abilities/Seed/trainer_abilities.sql")
ITEMS = Path("db/resources/Items/Seed/items.sql")
ITEM_EVENTS = Path("db/resources/Items/Seed/items_event_sets.sql")
MONIKERS = Path("db/resources/Entities/Seed/monikers.sql")
CHAR_CREATION = Path("db/resources/Archetypes/Seed/char_creation_abilities.sql")
CHAR_CREATION_ITEMS = Path("db/resources/Archetypes/Seed/char_creation_items.sql")
DEBUG_KIT = Path("db/resources/Archetypes/Seed/char_creation_debug_kit_abilities.sql")
DEBUG_KIT_ITEMS = Path("db/resources/Archetypes/Seed/char_creation_debug_kit_items.sql")
TREE = Path("db/resources/Archetypes/Seed/archetype_ability_tree.sql")
CONTENT_DIR = Path("db/resources/Content/Seed")

# `containers.container_id` of the bandolier: only an item that can sit
# there can be the active weapon.
BANDOLIER_CONTAINER = 3

# `items_event_sets.event_id` values (`spawner::EVENT_ITEM_*`).
EVENT_NAMES = {5: "use", 6: "melee", 7: "ranged"}
EVENT_RANGED = 7

# `abilities.target_type_id` (`enumerations.xml`): python's `canUse` checked
# the weapon only in the `TargetTarget` branch.
TARGET_TYPES = {1: "Self", 2: "Target", 3: "Ground"}
TARGET_TARGET = 2

# What each FAIL row looks like (decided 2026-10-10: all stay flagged, no seed
# change). Inference from the seeds, not from a CME source.
FAIL_NOTES = {
    997: "Likely a wrong tag: no item carries ITEM_Dart_Rifle, and the dart "
    "pistols carry ITEM_DartPistol.",
    1246: "No item carries ITEM_Melee; blades carry ITEM_Blade (inference).",
    1250: "Needs stealth armour monikers, which a bandolier weapon can never "
    "carry, and python never checked it (a Self ability). A scope question, "
    "not a seed fix.",
    1355: "No item carries ITEM_Melee; blades carry ITEM_Blade (inference).",
}

MAX_EXAMPLES = 4

# The right-click feedback line (`interaction/hostile_attack.rs`).
NO_RANGED_ATTACK_TEXT = "This weapon has no ranged attack."

# The Class Start v6 starter cases (CS-07 brief): (ability, item, expected).
STARTER_CASES = [
    (592, 55, True),
    (598, 21, True),
    (598, 3260, False),
    (1984, 2797, True),
    (1639, 4565, True),
]

_CONTENT_ACTIONS = re.compile(r"INSERT INTO content_actions \(([^)]*)\)\s*VALUES\s*")


def int_array(text: str | None) -> List[int]:
    if not text:
        return []
    body = text.strip().strip("{}")
    return [int(x) for x in body.split(",") if x.strip()]


def rows(path: Path, table: str):
    text, _ = read_text(path)
    return sql_rows(text, table)


def content_action_rows(text: str) -> List[Dict[str, Value]]:
    """Every row of every multi-row ``INSERT INTO content_actions (...) VALUES
    (...), (...);`` statement in a content seed (``sql_rows`` reads one row
    per statement). ``--`` comments between rows are skipped."""
    out = []
    for m in _CONTENT_ACTIONS.finditer(text):
        cols = [c.strip() for c in m.group(1).split(",")]
        i = m.end()
        while i < len(text):
            c = text[i]
            if c in " \t\n,":
                i += 1
            elif text.startswith("--", i):
                i = text.find("\n", i)
                i = len(text) if i < 0 else i
            elif c == "(":
                vals, i = parse_values(text, i)
                if len(vals) != len(cols):
                    raise InputError(f"content_actions: {len(cols)} columns but {len(vals)} values at offset {i}")
                out.append(dict(zip(cols, vals)))
            else:
                break  # the statement's ';'
    return out


def granted_ability_ids(row: Dict[str, Value]) -> List[int]:
    """The ``params.ability_ids`` of a ``grant_ability`` content action: the
    only form ``content-engine``'s loader accepts (``loader/action_ability.rs``
    refuses a ``target_id``)."""
    if row["action_type"].text != "grant_ability" or not row["params"].text:
        return []
    ids = json.loads(row["params"].text).get("ability_ids") or []
    return [int(i) for i in ids]


def load():
    monikers = {int(r["moniker_id"].text): r["name"].text for r in rows(MONIKERS, "monikers")}

    abilities = {}
    for r in rows(ABILITIES, "abilities"):
        abilities[int(r["ability_id"].text)] = {
            "name": (r["name"].text or "").strip(),
            "item_monikers": int_array(r["item_monikers"].text),
            "target_type": int(r["target_type_id"].text or 0),
        }

    items = {}
    for r in rows(ITEMS, "items"):
        items[int(r["item_id"].text)] = {
            "name": (r["name"].text or "").strip(),
            "monikers": set(int_array(r["moniker_ids"].text)),
            "bandolier": BANDOLIER_CONTAINER in int_array(r["container_sets"].text),
            "clip": int(r["clip_size"].text or 0),
        }

    sources: Dict[int, Set[str]] = defaultdict(set)
    for r in rows(CHAR_CREATION, "char_creation_abilities"):
        sources[int(r["ability_id"].text)].add("char_creation")
    for r in rows(DEBUG_KIT, "char_creation_debug_kit_abilities"):
        sources[int(r["ability_id"].text)].add("debug_kit")
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
        for row in content_action_rows(text):
            for ability_id in granted_ability_ids(row):
                sources[ability_id].add("content_grant")
    starter_items = {int(r["item_id"].text) for r in rows(CHAR_CREATION_ITEMS, "char_creation_items")}
    starter_items |= {int(r["item_id"].text) for r in rows(DEBUG_KIT_ITEMS, "char_creation_debug_kit_items")}
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

    # Python checked the weapon only for TargetTarget abilities.
    rust_only = [x for x in audited if x[1]["target_type"] != TARGET_TARGET]

    # Bandolier weapons with no RANGED binding: right-click fires nothing.
    ranged_bound = {item_id for item_id, _, event_id in item_events if event_id == EVENT_RANGED}
    events_of: Dict[int, Set[int]] = defaultdict(set)
    for item_id, _, event_id in item_events:
        events_of[item_id].add(event_id)
    no_ranged = sorted(i for i, it in items.items() if it["bandolier"] and i not in ranged_bound)
    rifle_581 = sorted(
        item_id for item_id, ability_id, event_id in item_events if ability_id == 581 and event_id == EVENT_RANGED
    )

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
    w("unchanged, and NPC casts are not checked. The match rule is python's")
    w("(`SGWPlayer.hasItemMoniker`, any-match), but python applied it only to")
    w("`TargetTarget` abilities and only after the cooldown check")
    w("(`AbilityManager.py:528-545`); the Rust server checks every target type")
    w("ahead of the cooldown.")
    w("")
    w("Player-reachable sources: `char_creation` (char_creation_abilities),")
    w("`debug_kit` (char_creation_debug_kit_abilities), `tree`")
    w("(archetype_ability_tree), `trainer` (trainer_abilities), `item_event`")
    w("(items_event_sets) and `content_grant` (the `params.ability_ids` of a")
    w("`grant_ability` content action, the only form the content loader accepts).")
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
    w(f"- Gated by the Rust server only (python checked `TargetTarget` alone): {len(rust_only)}")
    w(f"- Bandolier weapons with no RANGED binding (right-click fires nothing): {len(no_ranged)}")
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
    w("## FAIL rows")
    w("")
    w("No shipped bandolier weapon carries a required moniker, so no player can")
    w("use these. They stay flagged and the rule is not weakened for them; no seed")
    w("change was made (decided 2026-10-10). The notes are inferences from the")
    w("seeds, not CME sources.")
    w("")
    if fails:
        w("| ability_id | name | target | sources | required monikers | note |")
        w("|---|---|---|---|---|---|")
        for ability_id, a, src, _ in fails:
            req = ", ".join(moniker_label(m, monikers) for m in a["item_monikers"])
            target = TARGET_TYPES.get(a["target_type"], str(a["target_type"]))
            note = FAIL_NOTES.get(ability_id, "")
            w(f"| {ability_id} | {esc(a['name'])} | {target} | {', '.join(src)} | {esc(req)} | {esc(note)} |")
    else:
        w("None.")
    w("")
    w("## Gated by the Rust server only")
    w("")
    w("Python's `canUse` ran the weapon check only in its `TargetTarget` branch,")
    w("so it never refused these self- and ground-targeted abilities. OD-CS11")
    w("applies the rule to every target type, so the Rust server refuses them")
    w("with the wrong weapon.")
    w("")
    if rust_only:
        w("| ability_id | name | target | sources | required monikers | result |")
        w("|---|---|---|---|---|---|")
        for ability_id, a, src, valid in rust_only:
            req = ", ".join(moniker_label(m, monikers) for m in a["item_monikers"])
            target = TARGET_TYPES.get(a["target_type"], str(a["target_type"]))
            w(
                f"| {ability_id} | {esc(a['name'])} | {target} | {', '.join(src)} | {esc(req)} | "
                f"{'PASS' if valid else 'FAIL'} |"
            )
    else:
        w("None.")
    w("")
    w("## Right-click with no RANGED binding")
    w("")
    w("Right-click on a live hostile fires the active weapon's RANGED binding")
    w("(`items_event_sets` event 7), or 594 Strike with no weapon. Before CS-07 a")
    w("weapon with no RANGED binding fell back to 592 Pistol Shot, which the")
    w("weapon requirement now refuses for anything but a pistol. Since CS-07 such")
    w("a weapon fires nothing and charges nothing, and the player reads")
    w(f"\"{NO_RANGED_ATTACK_TEXT}\" (decided 2026-10-10).")
    w("")
    w(f"{len(rifle_581)} rifles are bound to 581 Rifle Auto Attack (RANGED). All but one are the")
    w("ITEM_Rifle sniper rifles that shipped with only a clip and a melee binding;")
    w("CS-07 seeded 581 onto them (decided 2026-10-10).")
    w("")
    if no_ranged:
        groups: Dict[tuple, List[int]] = defaultdict(list)
        for item_id in no_ranged:
            it = items[item_id]
            tags = tuple(
                sorted(monikers.get(m, str(m)) for m in it["monikers"] if monikers.get(m, "").upper().startswith("ITEM_"))
            )
            events = tuple(EVENT_NAMES.get(e, str(e)) for e in sorted(events_of[item_id]))
            groups[(tags, it["clip"] > 0, events)].append(item_id)
        w("| weapon monikers (ITEM_*) | magazine | other bindings | items | examples |")
        w("|---|---|---|---|---|")
        for (tags, has_clip, events), group in sorted(groups.items(), key=lambda kv: (-len(kv[1]), kv[0])):
            names = sorted({items[i]["name"] for i in group})
            more = f" (+{len(names) - MAX_EXAMPLES} more)" if len(names) > MAX_EXAMPLES else ""
            w(
                f"| {esc(', '.join(tags)) or 'none'} | {'yes' if has_clip else 'no'} | "
                f"{', '.join(events) or 'none'} | {len(group)} | {esc(', '.join(names[:MAX_EXAMPLES]) + more)} |"
            )
    else:
        w("None.")
    w("")
    w("## Weapon-granted abilities refused by their own weapon")
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


def main(argv: List[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="exit 1 if the committed doc drifted")
    ap.add_argument("--stdout", action="store_true", help="print the doc, write nothing")
    args = ap.parse_args(argv)
    doc = build()
    if args.stdout:
        sys.stdout.write(doc)
        return 0
    # Compare and write in LF, keeping the file's own line ending (the seeds'
    # `read_text` / `write_text` rule): a checkout's CRLF or LF is not drift,
    # and a run never rewrites the line endings.
    path = REPO_ROOT / OUT
    old, eol = read_text(OUT) if path.exists() else (None, "\n")
    if args.check:
        if old != doc:
            print(f"{OUT} is stale: rerun {Path(__file__).name}", file=sys.stderr)
            return 1
        print(f"ok: {OUT} matches the seeds")
        return 0
    if old == doc:
        print(f"{OUT} is up to date")
        return 0
    write_text(OUT, doc, eol)
    print(f"wrote {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
