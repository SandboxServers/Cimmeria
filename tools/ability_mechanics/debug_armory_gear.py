"""The Armory gear specs (DR-07): one lab row per weapon and per equipment representative.

``debug_armory.py specs gear --out-dir docs/guides/uat-specs`` writes
``armory-gear-weapons.toml`` and ``armory-gear-armor.toml`` (LF, Soldier ``sgu``
fresh), split at 30 rows (one file, one fresh character, at most 30 grants into the 40-slot main bag) as ``-1``, ``-2``. The representatives are the ones
``representatives()`` stocks. A weapon with a RANGED binding expects its
ability's ``client.ability.sent``; an unbound one expects ``weapon_unbound``
(F8: the gap is measured, not failed). An armor row equips its piece and reads
the character sheet; the only server line is the DEBUG ``Inventory move persisted``, so that clause is optional.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Dict, List, Optional, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parent))
from debug_armory import (  # noqa: E402
    DEFAULT_CAP,
    MAX_VENDORS,
    drop_quality,
    equipment_keys,
    item_attributes,
    pick_equipment_reps,
    pick_weapon_reps,
    sort_key,
    vendor_count,
    weapon_keys,
)
from weapon_requirement_audit import EVENT_RANGED, load  # noqa: E402

# INV_Main capacity (docs/gameplay/inventory-system.md:118). Every row grants one item into it and nothing in a spec removes one; a split file runs on its own fresh character, so one file's grants must fit even if no equip succeeds. 30 leaves 10 slots of margin (DR-07 ruling 1).
MAIN_BAG_SLOTS = 40
ROWS_PER_FILE = 30
# The L1 hostile training dummy (db/resources/Entities/Seed/entity_templates_debug_area_plaza.sql, 1310-1313).
DUMMY_TEMPLATE = 1310
# The ability log's target and the event that names an unbound weapon (hostile_attack.rs, refuse_no_ranged_attack).
UNBOUND_SCOPE = "abilities"
SECTION_TAIL = """
schema = 1

[section]
id = "{section}"
system = "Debug Area Armory gear ({kind})"
guide = "docs/guides/unified-uat.md#debug-area"
ledger = "docs/analysis/debug-armory/README.md"
account = "gm"
character = "fresh"

[section.fresh]
alignment = "sgu"
archetype = "Soldier"
gender = "male"
first = "Uat"
"""
HEADER = """# Unified UAT, section "{section}" (Debug Area Armory gear, DR-07).
# Written by: python tools/ability_mechanics/debug_armory.py specs gear --out-dir docs/guides/uat-specs
# Generated: do not hand-edit; rerun the command. Campaign: docs/analysis/debug-armory/README.md.
# One file is one run on its own fresh character: at most 30 rows, so its grants fit the 40-slot main bag. Never merge split files into one run.
"""


def _q(text: str) -> str:
    """A TOML basic string (JSON escapes are valid TOML)."""
    return json.dumps(text, ensure_ascii=False)


def _inline(pairs: List[Tuple[str, str]]) -> str:
    """An inline table from (key, already-rendered TOML value) pairs."""
    return "{ " + ", ".join(f"{k} = {v}" for k, v in pairs) + " }"


def _tool(alias: str, args: Optional[List[Tuple[str, str]]] = None, label: Optional[str] = None) -> str:
    pairs = [("tool", _q(alias))]
    if args is not None:
        pairs.append(("args", _inline(args)))
    if label is not None:
        pairs.append(("label", _q(label)))
    return _inline(pairs)


def _chat(line: str) -> str:
    return _inline([("chat", _q(line)), ("tier", _q("G"))])


def gear_representatives(cap: int = DEFAULT_CAP):
    """(weapons, equipment, items, ranged): the representatives as ``representatives()`` stocks them.

    ``weapons`` and ``equipment`` map a key to (item_id, unbound); ``ranged`` maps a
    RANGED item to its ability id.
    """
    monikers, abilities, items, _sources, item_events, _starter = load()
    attrs = item_attributes()
    gated = {m for a in abilities.values() for m in a["item_monikers"]}
    ranged = {item_id: ability_id for item_id, ability_id, event_id in item_events if event_id == EVENT_RANGED}
    weapons = pick_weapon_reps(weapon_keys(items, attrs, gated, monikers), set(ranged))
    equipment = pick_equipment_reps(equipment_keys(attrs))
    if vendor_count(len(weapons), len(equipment), cap) > MAX_VENDORS:
        weapons = {k + (None,): v for k, v in drop_quality(weapons).items()}
        equipment = {k + (None,): v for k, v in drop_quality(equipment).items()}
    return weapons, equipment, items, ranged


def weapon_row(item_id: int, name: str, ability_id: Optional[int]) -> str:
    """One weapon row: give and equip it, right-click the L1 dummy, expect the ranged press or the unbound line."""
    if ability_id is None:
        title = f"{name} has no ranged binding (F8): the right-click is refused"
        expected = "The right-click gets the no-ranged-attack line and the server logs weapon_unbound."
        clause = [
            "[[row.expect]]",
            'id = "unbound"',
            f"text = {_q(f'SigNoz: weapon_unbound for item {item_id}, reason no_ranged_attack (a measured gap, PASS on this line)')}",
            'source = "signoz"',
            "filter = "
            + _q(
                f"scope_name = '{UNBOUND_SCOPE}' AND event = 'weapon_unbound' AND reason = 'no_ranged_attack' "
                f"AND item_type_id = {item_id} AND player_name CONTAINS '${{character}}'"
            ),
            "min_rows = 1",
        ]
    else:
        title = f"{name} fires its ranged ability on a right-click"
        expected = f"The right-click leaves the client as ability {ability_id}."
        clause = [
            "[[row.expect]]",
            'id = "sent"',
            f"text = {_q(f'the right-click sends ability {ability_id} (RANGED binding)')}",
            'source = "client_event"',
            'event = "client.ability.sent"',
            f"match_fields = {_inline([('ability_id', str(ability_id))])}",
            'since = "press"',
        ]
    # weapon_unbound is logged at most once per player per 10 s (NO_RANGED_ATTACK_LOG_INTERVAL), so wait it out.
    throttle = ["  { wait_ms = 10500 },"] if ability_id is None else []
    return "\n".join(
        [
            "[[row]]",
            f"id = {_q(f'GW-{item_id}')}",
            f"title = {_q(title)}",
            f"expected = {_q(expected)}",
            "setup = [",
            *throttle,
            f"  {_chat(f'/gmgiveitem {item_id} 1')},",
            f"  {_tool('@item_action', [('action', _q('equip')), ('name', _q(name))])},",
            f"  {_tool('@dummy', [('disposition', _q('hostile')), ('template_id', str(DUMMY_TEMPLATE))])},",
            "  { wait_ms = 2000 },",
            "]",
            "step = [",
            f"  {_tool('@target', [('entity_id', _q('${dummy_id}'))])},",
            f"  {_tool('@world_click', [('entity_id', _q('${dummy_id}')), ('button', _q('right'))], 'press')},",
            "  { wait_ms = 3000 },",
            "]",
            "teardown = [",
            f"  {_tool('@clear_effects', [('name', _q('${character}'))])},",
            "  { tool = \"@cooldowns_reset\" },",
            f"  {_tool('@dummy', [('disposition', _q('clear'))])},",
            "]",
            *clause,
            "",
        ]
    )


def armor_row(item_id: int, name: str, slot: int) -> str:
    """One armor row: give and equip the piece, read the character sheet, screenshot for a human."""
    return "\n".join(
        [
            "[[row]]",
            f"id = {_q(f'GA-{item_id}')}",
            f"title = {_q(f'{name} equips into its slot')}",
            f"expected = {_q(f'The piece sits in equipment container {slot} on the character sheet.')}",
            "setup = [",
            f"  {_chat(f'/gmgiveitem {item_id} 1')},",
            f"  {_tool('@item_action', [('action', _q('equip')), ('name', _q(name))])},",
            "  { wait_ms = 2000 },",
            "]",
            "step = [",
            f"  {_tool('@window_read', [('kind', _q('character'))], 'sheet')},",
            f"  {_tool('@screenshot')},",
            "]",
            "teardown = [",
            "  { tool = \"@cooldowns_reset\" },",
            "]",
            "[[row.expect]]",
            'id = "sheet"',
            f"text = {_q(f'the character sheet shows {name} in equipment container {slot}')}",
            'source = "human"',
            f"question = {_q(f'Does the character sheet show {name} equipped (see the screenshot)?')}",
            "[[row.expect]]",
            'id = "moved"',
            f"text = {_q(f'SigNoz: Inventory move persisted for {name} into container {slot}')}",
            'source = "signoz"',
            'required = false',
            "filter = "
            + _q(
                f"body CONTAINS 'Inventory move persisted' AND item_name = '{name}' "
                f"AND target_container_id = {slot} AND player_name CONTAINS '${{character}}'"
            ),
            "min_rows = 1",
            "",
        ]
    )


def weapon_blocks(weapons, items, ranged) -> List[str]:
    """One weapon row per weapon representative, in the Armory's vendor order."""
    out = []
    for _, (item_id, unbound) in sorted(weapons.items(), key=lambda kv: sort_key(kv[0])):
        out.append(weapon_row(item_id, items[item_id]["name"], None if unbound else ranged[item_id]))
    return out


def armor_blocks(equipment, items) -> List[str]:
    """One armor row per equipment representative, slot = its lowest equipment container."""
    return [
        armor_row(item_id, items[item_id]["name"], key[0])
        for key, (item_id, _) in sorted(equipment.items(), key=lambda kv: sort_key(kv[0]))
    ]


def split(stem: str, blocks: List[str]) -> List[Tuple[str, List[str]]]:
    """The file names and their rows: one file, or ``-1``, ``-2`` ... at ``ROWS_PER_FILE`` rows."""
    if len(blocks) <= ROWS_PER_FILE:
        return [(stem, blocks)]
    return [
        (f"{stem}-{n}", blocks[i : i + ROWS_PER_FILE])
        for n, i in enumerate(range(0, len(blocks), ROWS_PER_FILE), start=1)
    ]


def render(section: str, kind: str, blocks: List[str]) -> str:
    """A whole spec file in LF: header, section, rows."""
    note = (
        "# Armor rows have no server-side pass line except the optional DEBUG Inventory move persisted clause: the character sheet read and the human clause decide (DR-07).\n"
        if kind == "armor"
        else ""
    )
    return (
        HEADER.format(section=section)
        + note
        + SECTION_TAIL.format(section=section, kind=kind)
        + "\n"
        + "\n".join(blocks)
    )


def write_gear_specs(out_dir: Path, cap: int = DEFAULT_CAP) -> int:
    """Write the weapon and armor gear specs into ``out_dir``; returns 0."""
    weapons, equipment, items, ranged = gear_representatives(cap)
    groups = [
        ("armory-gear-weapons", "weapons", weapon_blocks(weapons, items, ranged)),
        ("armory-gear-armor", "armor", armor_blocks(equipment, items)),
    ]
    out_dir.mkdir(parents=True, exist_ok=True)
    for stem, kind, blocks in groups:
        for name, chunk in split(stem, blocks):
            path = out_dir / f"{name}.toml"
            with open(path, "w", encoding="utf-8", newline="\n") as fh:
                fh.write(render(name, kind, chunk))
            print(f"wrote {path} ({len(chunk)} rows)")
    return 0
