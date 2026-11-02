"""Debug Area armory lab specs: one ability spec per archetype tree (campaign debug-armory, DR-06).

``debug_armory.py specs abilities --out-dir docs/guides/uat-specs`` writes, per archetype
(README D-DR3; Jaffa has no tree, D-AT04), ``armory-abilities-<archetype>.toml`` (LF). A tree
with more than 60 rows splits into one file per ``tree_index``:
``armory-abilities-<archetype>-<n>.toml``.

Each row gives its ability (``/gmgiveallabilities`` in the section's ``-00`` row), equips the
ability's representative weapon from the catalog (``/gmgiveitem``, then ``@item_action`` equip),
places and resets the ability, then presses it from the bar and expects the press to leave the
client. An ability whose weapon the audit found no shipped item for (``KNOWN_UNCOVERED``, DR-04)
is a BLOCKED row with its reason. The generator reads the seeds only, through the same loaders
and representative rules as ``debug_armory.py survey``.
"""

from __future__ import annotations

import json
from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Set, Tuple

from debug_armory import Stop, _label_monikers, item_attributes, pick_weapon_reps, representative_for, weapon_keys
from weapon_requirement_audit import EVENT_RANGED, TREE, load, rows

# Alignment and archetype per tree (widgets.rs:177-196). Jaffa is not listed: it has no tree.
ARCHETYPES: Sequence[Tuple[str, str]] = (
    ("sgu", "Soldier"),
    ("sgu", "Commando"),
    ("sgu", "Scientist"),
    ("sgu", "Archeologist"),
    ("sgu", "Asgard"),
    ("sgu", "Sholva"),
    ("praxis", "Goauld"),
)
TREE_PREFIX = "ARCHETYPE_"  # archetype_ability_tree.archetype is e.g. ARCHETYPE_Soldier
SPLIT_ROWS = 60  # a tree with more rows than this writes one file per tree_index
SETUP_LOCATION = ".gotolocation DebugArea 246 6.6 -878"  # the Debug Area dummy plaza
TARGET_SELF, TARGET_GROUND = 1, 3  # target_type_id values that have no target to apply to
# Weapon-gated tree abilities with no shipped weapon (weapon-requirement audit FAIL, DR-04).
KNOWN_UNCOVERED = (997, 1246, 1250, 1355)
UNCOVERED_MISMATCH = 1


def tstr(text: str) -> str:
    """A TOML basic string. JSON escapes are valid TOML, and ``$`` needs none."""
    return json.dumps(text, ensure_ascii=False)


def toml_value(value: object) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    return tstr(str(value))


def table(pairs: Dict[str, object]) -> str:
    return "{ " + ", ".join(f"{k} = {toml_value(v)}" for k, v in pairs.items()) + " }"


def call(tool: str, args: Optional[Dict[str, object]] = None, label: Optional[str] = None, optional: bool = False) -> str:
    """A ``tool`` action, as abilities.toml writes it."""
    parts = [f"tool = {tstr(tool)}"]
    if args:
        parts.append(f"args = {table(args)}")
    if label:
        parts.append(f"label = {tstr(label)}")
    if optional:
        parts.append("optional = true")
    return "{ " + ", ".join(parts) + " }"


def chat(line: str, label: Optional[str] = None) -> str:
    """A ``chat`` action at tier G (a GM command the lab types)."""
    parts = [f"chat = {tstr(line)}", 'tier = "G"']
    if label:
        parts.append(f"label = {tstr(label)}")
    return "{ " + ", ".join(parts) + " }"


def list_block(name: str, actions: List[str]) -> List[str]:
    return [f"{name} = ["] + [f"  {a}," for a in actions] + ["]"]


def tree_by_archetype() -> Dict[str, List[Tuple[int, int, int]]]:
    """archetype -> (tree_index, ability_index, ability_id) rows, in tree order."""
    out: Dict[str, List[Tuple[int, int, int]]] = defaultdict(list)
    for r in rows(TREE, "archetype_ability_tree"):
        out[r["archetype"].text].append(
            (int(r["tree_index"].text), int(r["ability_index"].text), int(r["ability_id"].text))
        )
    return {arch: sorted(tree) for arch, tree in out.items()}


def gear_by_ability(abilities, items, item_events, monikers) -> Dict[int, Optional[int]]:
    """Each weapon-gated ability's representative weapon item id, or None when no weapon fits.

    Uses the same weapon representatives as the catalog (``debug_armory.build``): the lowest
    ranged-bound item per (family, tier, quality).
    """
    attrs = item_attributes()
    gated = {m for a in abilities.values() for m in a["item_monikers"]}
    ranged = {item_id for item_id, _, event_id in item_events if event_id == EVENT_RANGED}
    weapons = pick_weapon_reps(weapon_keys(items, attrs, gated, monikers), ranged)
    rep_ids = sorted(item_id for item_id, _ in weapons.values())
    return {
        aid: representative_for(set(a["item_monikers"]), rep_ids, items)
        for aid, a in abilities.items()
        if a["item_monikers"]
    }


def ability_row(arch: str, aid: int, ability: dict, rep: Optional[int], items, monikers) -> List[str]:
    """One ability's row: gear, place, reset, press, the two client clauses, teardown."""
    name = ability["name"] or str(aid)
    self_or_ground = ability["target_type"] in (TARGET_SELF, TARGET_GROUND)
    blocked = None
    setup: List[str] = []
    if ability["item_monikers"]:
        if rep is None:
            blocked = f"no shipped weapon carries {_label_monikers(ability['item_monikers'], monikers)} (weapon-requirement audit FAIL)"
        else:
            setup.append(chat(f"/gmgiveitem {rep} 1"))
            setup.append(call("@item_action", {"action": "equip", "name": items[rep]["name"]}, label="equip"))
    setup.append(call("@use_ability", {"ability_id": aid, "place": True}, label="place"))
    setup.append("{ wait_ms = 3000 }")
    setup.append(call("@cooldowns_reset", {"ability_id": aid}))
    gear = f" with {items[rep]['name']} equipped" if rep is not None and ability["item_monikers"] else ""
    lines = [
        "[[row]]",
        f"id = {tstr(f'{arch}-{aid}')}",
        f"title = {tstr(f'{name} ({aid}): press it from the bar{gear}')}",
        f"expected = {tstr('The press leaves the client' + ('.' if self_or_ground else ' and the client applies the ability.'))}",
    ]
    if blocked:
        lines.append(f"blocked = {tstr(blocked)}")
    lines += list_block("setup", setup)
    lines += list_block(
        "step",
        [call("@use_ability", {"ability_id": aid, "press": "key", "fallback": "none"}, label="press"), "{ wait_ms = 3000 }"],
    )
    lines += list_block(
        "teardown",
        [call("@clear_effects", {"name": "${character}"}, optional=True), call("@cooldowns_reset")],
    )
    lines += [
        "[[row.expect]]",
        'id = "sent"',
        'text = "the press leaves the client"',
        'source = "client_event"',
        'event = "client.ability.sent"',
        f"match_fields = {table({'ability_id': aid})}",
        'since = "press"',
        "[[row.expect]]",
        'id = "applied"',
        'text = "the client applies the ability after the press"',
        'source = "client_event"',
        'event = "client.ability.applied"',
        'since = "press"',
    ]
    if self_or_ground:
        lines.append("required = false")
    return lines


def setup_row(arch: str) -> List[str]:
    """The section's ``-00`` row: the Debug Area, every tree ability, one hostile dummy."""
    return [
        "[[row]]",
        f"id = {tstr(f'{arch}-00')}",
        f"title = {tstr(f'{arch}: the Debug Area, every tree ability, one hostile dummy')}",
        f"expected = {tstr('The character stands in the Debug Area with its tree and a hostile dummy.')}",
    ] + list_block(
        "setup",
        [
            chat(SETUP_LOCATION),
            chat("/gmgiveallabilities"),
            call("@dummy", {"disposition": "hostile"}),
        ],
    ) + ["step = [{ wait_ms = 1000 }]"]


def section_header(alignment: str, arch: str, part: Optional[int]) -> List[str]:
    stem = f"armory-abilities-{arch.lower()}" + (f"-{part}" if part is not None else "")
    title = f"{arch} abilities" + (f", tree {part}" if part is not None else "")
    return [
        f"# Debug Area armory: {title} (campaign debug-armory, DR-06).",
        "# Generated by tools/ability_mechanics/debug_armory.py specs abilities; do not edit it by hand.",
        "# Where it runs: the Debug Area (world 1300) with a fresh character per run. Every row",
        "# gives its gear (weapon-requirement audit, README F3), presses the ability from the bar and",
        "# grades the client events. Rows whose weapon the audit found none for are BLOCKED.",
        "",
        "schema = 1",
        "",
        "[section]",
        f"id = {tstr(stem)}",
        f"system = {tstr(f'Debug Area armory: {title}')}",
        'guide = "docs/guides/unified-uat.md#debug-area"',
        'ledger = "docs/analysis/debug-armory/README.md"',
        'account = "gm"',
        'character = "fresh"',
        "",
        "[section.fresh]",
        f"alignment = {tstr(alignment)}",
        f"archetype = {tstr(arch)}",
        'gender = "male"',
        'first = "Uat"',
        "",
    ]


def archetype_files(
    alignment: str, arch: str, tree: Sequence[Tuple[int, int, int]], abilities, items, gear, monikers
) -> Dict[str, str]:
    """File name -> TOML text for one archetype: one file, or one per tree_index past SPLIT_ROWS.

    ``tree`` is ``(tree_index, ability_index, ability_id)`` rows; ``gear`` maps each gated
    ability to its representative weapon (None when the audit found none).
    """
    if len(tree) <= SPLIT_ROWS:
        groups: List[Tuple[Optional[int], List[Tuple[int, int, int]]]] = [(None, list(tree))]
    else:
        by_index: Dict[int, List[Tuple[int, int, int]]] = defaultdict(list)
        for entry in tree:
            by_index[entry[0]].append(entry)
        groups = [(index, by_index[index]) for index in sorted(by_index)]
    files: Dict[str, str] = {}
    for part, entries in groups:
        stem = f"armory-abilities-{arch.lower()}" + (f"-{part}" if part is not None else "")
        lines = section_header(alignment, arch, part) + setup_row(arch) + [""]
        for _, _, aid in entries:
            if aid not in abilities:
                raise Stop(UNCOVERED_MISMATCH, f"{arch} tree ability {aid} is not in the abilities seed")
            lines += ability_row(arch, aid, abilities[aid], gear.get(aid), items, monikers) + [""]
        files[f"{stem}.toml"] = "\n".join(lines).rstrip("\n") + "\n"
    return files


def check_uncovered(gear: Dict[int, Optional[int]], trees: Sequence[int]) -> None:
    """Refuse when the audit's uncovered tree abilities drift from ``KNOWN_UNCOVERED`` (DR-04)."""
    uncovered: Set[int] = {aid for aid in trees if aid in gear and gear[aid] is None}
    expected = {aid for aid in KNOWN_UNCOVERED if aid in trees}
    if uncovered != expected:
        raise Stop(
            UNCOVERED_MISMATCH,
            f"uncovered tree abilities {sorted(uncovered)} differ from KNOWN_UNCOVERED {sorted(expected)}: "
            "rerun the audit and update DR-04's list",
        )


def write_ability_specs(out_dir: Path, repo_root: Path) -> int:
    """Write every armory-abilities spec under ``out_dir`` (relative to the repo root)."""
    monikers, abilities, items, _sources, item_events, _starter = load()
    gear = gear_by_ability(abilities, items, item_events, monikers)
    trees = tree_by_archetype()
    tree_ids = {aid for _, _, aid in (e for t in trees.values() for e in t)}
    check_uncovered(gear, sorted(tree_ids))

    target = repo_root / out_dir
    target.mkdir(parents=True, exist_ok=True)
    for stale in target.glob("armory-abilities-*.toml"):
        stale.unlink()
    written = 0
    for alignment, arch in ARCHETYPES:
        tree = trees.get(TREE_PREFIX + arch)
        if not tree:
            raise Stop(UNCOVERED_MISMATCH, f"no archetype_ability_tree rows for {TREE_PREFIX + arch}")
        for name, text in archetype_files(alignment, arch, tree, abilities, items, gear, monikers).items():
            with open(target / name, "w", encoding="utf-8", newline="\n") as fh:
                fh.write(text)
            written += 1
            print(f"wrote {out_dir / name} ({text.count('[[row]]')} rows)")
    print(f"wrote {written} spec files")
    return 0
