"""Debug Area armory survey and id-block audit (campaign debug-armory, DR-01).

The Armory (README D-DR1, D-DR2) stocks one representative per weapon family,
tier and quality, and one per armor slot, tier and quality. This script reads
the seeds (never a database) and:

- ``survey`` writes ``docs/analysis/debug-armory/catalog.md``: the weapon and
  equipment representatives, the archetype ability coverage, and the vendor
  count at a row cap (``--cap``, default 19, the largest buy list proven to
  open). When the count needs more than the 39 buy lists, it also prints the
  counts with ``quality_id`` dropped, keeping the highest quality.
- ``audit`` prints each free-id block the Armory needs (README F4) and exits 1
  if an id in one is already used under ``db/resources/``.

It reuses ``weapon_requirement_audit.load()`` for the parsed seeds.

Run from the repo root with stock Python 3:

    python tools/ability_mechanics/debug_armory.py survey           # rewrite the catalog
    python tools/ability_mechanics/debug_armory.py survey --check   # exit 1 if it drifted
    python tools/ability_mechanics/debug_armory.py survey --stdout  # print, write nothing
    python tools/ability_mechanics/debug_armory.py audit            # exit 1 on a collision
"""

from __future__ import annotations

import argparse
import math
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Set, Tuple

sys.path.insert(0, str(Path(__file__).resolve().parent))
from debug_armory_sql import (  # noqa: E402
    ARMORY_LISTS,
    ARMORY_WORLD,
    ITEM_ROW_BASE,
    SEEDS,
    SPAWN_BASE,
    TEMPLATE_BASE,
    item_sql,
    spawn_sql,
    template_sql,
)
from seed_sql import REPO_ROOT, read_text, sql_rows, write_text  # noqa: E402
from weapon_requirement_audit import (  # noqa: E402
    EVENT_RANGED,
    ITEMS,
    TREE,
    int_array,
    load,
    rows,
)

OUT = Path("docs/analysis/debug-armory/catalog.md")

# `items.container_sets` of the equipment slots: 4-14, `INV_Head` ... `INV_Artifact2`
# (inventory-system.md:104). The bandolier (3) is the weapon container.
EQUIP_CONTAINERS = range(4, 15)

DEFAULT_CAP = 19

# Buy lists 1310-1348; 1349 is the shared sell list (README F4).
BUY_LISTS = range(1310, 1349)
MAX_VENDORS = len(BUY_LISTS)

# README F4 id blocks: (label, table, id column, first id, last id).
ID_BLOCKS = [
    ("templates", "entity_templates", "template_id", 1610, 1659),
    ("spawns", "spawnlist", "spawn_id", 14110, 14299),
    ("item lists", "item_lists", "item_list_id", 1310, 1349),
    ("item_list_items", "item_list_items", "item_id", 131000, 134999),
]


# Seed generator (DR-03a): `seed probes` and `seed emit`. Vendor k stands at PROBE_X, PROBE_Z + k * PROBE_STEP.
PROBE_X, PROBE_Y, PROBE_Z, PROBE_STEP = 266.0, 10.0, -915.0, -3.0
CLEAR_M = 2.5  # a probe within this many metres of a world 1300 spawn is refused
REGROUND_M = 2.0  # an ungrounded probe moves this far in x and is probed again
MAX_RETRIES = 3
# The sell path takes a 0 price: vendor/sell/mod.rs reads the sell row's naquadah with no
# price filter, and a 0 gain skips the balance update. So the sell list pays 0 (D-DR5).
SELL_PRICE = 0
HEIGHT_LINE = re.compile(r"^\s*(DR\d+)\s.*\bdy=([+-]?\d+\.\d+) m\s+ok\s*$", re.M)


class Stop(Exception):
    """A refusal: ``code`` is the exit status and the message goes to stderr."""

    def __init__(self, code: int, message: str):
        super().__init__(message)
        self.code = code


def opt_int(text: Optional[str]) -> Optional[int]:
    return None if text is None or text == "" else int(text)


def opt_text(text: Optional[str]) -> Optional[str]:
    return None if text is None or text == "" else text


# ``EItemQuality`` (db/resources/Items/Types/EItemQuality.sql), worst to best: the
# crafting docs rank Fantastic highest (docs/gameplay/crafting-system.md:289).
QUALITY_RANK = {
    "ITEM_QUALITY_Poor": 0,
    "ITEM_QUALITY_Normal": 1,
    "ITEM_QUALITY_Good": 2,
    "ITEM_QUALITY_Great": 3,
    "ITEM_QUALITY_Fantastic": 4,
}


def sortable(value: Optional[int]) -> Tuple[bool, int]:
    """NULL sorts after every number, so a tier or quality of NULL lists last."""
    return (value is None, value or 0)


def quality_rank(name: Optional[str]) -> int:
    """The rank of a quality enum name (Poor 0 ... Fantastic 4); NULL is -1, the worst. Unknown is an error."""
    return -1 if name is None else QUALITY_RANK[name]


def sort_key(key: tuple) -> tuple:
    """Sort a (family or slot, tier, quality) key: tier numerically, quality by rank, NULL last."""
    *head, tier, quality = key
    return (tuple(head), sortable(tier), (quality is None, quality_rank(quality)))


def item_attributes() -> Dict[int, dict]:
    """``tier``, ``quality_id`` (enum name), ``container_sets`` and ``flags`` per item: ``load()`` keeps none of them."""
    return {
        int(r["item_id"].text): {
            "tier": opt_int(r["tier"].text),
            "quality": opt_text(r["quality_id"].text),
            "containers": int_array(r["container_sets"].text),
            "flags": int(r["flags"].text or 0),
        }
        for r in rows(ITEMS, "items")
    }


def family_key(item_monikers: Set[int], gated: Set[int], names: Dict[int, str]) -> Tuple[str, ...]:
    """The sorted moniker names an item shares with any ability's requirement (D-DR2)."""
    return tuple(sorted(names.get(m, str(m)) for m in item_monikers & gated))


def weapon_keys(items, attrs, gated: Set[int], names: Dict[int, str]) -> Dict[int, tuple]:
    """(family, tier, quality) of every item that can sit in the bandolier."""
    return {
        item_id: (
            family_key(items[item_id]["monikers"], gated, names),
            attrs[item_id]["tier"],
            attrs[item_id]["quality"],
        )
        for item_id in items
        if items[item_id]["bandolier"]
    }


def equipment_keys(attrs: Dict[int, dict]) -> Dict[int, tuple]:
    """(slot, tier, quality) of every item in an equipment container; slot = the lowest such container."""
    out = {}
    for item_id, a in attrs.items():
        slots = [c for c in a["containers"] if c in EQUIP_CONTAINERS]
        if slots:
            out[item_id] = (min(slots), a["tier"], a["quality"])
    return out


def _groups(keyed: Dict[int, tuple]) -> Dict[tuple, List[int]]:
    groups: Dict[tuple, List[int]] = defaultdict(list)
    for item_id in sorted(keyed):
        groups[keyed[item_id]].append(item_id)
    return groups


def pick_weapon_reps(keyed: Dict[int, tuple], ranged: Set[int]) -> Dict[tuple, Tuple[int, bool]]:
    """key -> (item_id, unbound): the lowest id with a RANGED binding, else the lowest id (F8)."""
    reps = {}
    for key, ids in _groups(keyed).items():
        bound = [i for i in ids if i in ranged]
        reps[key] = (bound[0], False) if bound else (ids[0], True)
    return reps


def pick_equipment_reps(keyed: Dict[int, tuple]) -> Dict[tuple, Tuple[int, bool]]:
    """key -> (item_id, False): the lowest item id per (slot, tier, quality)."""
    return {key: (ids[0], False) for key, ids in _groups(keyed).items()}


def drop_quality(reps: Dict[tuple, Tuple[int, bool]]) -> Dict[tuple, Tuple[int, bool]]:
    """The key without its last field (quality_id), keeping the representative of the highest quality (NULL loses)."""
    best: Dict[tuple, tuple] = {}
    for key, value in reps.items():
        short, rank = key[:-1], quality_rank(key[-1])
        if short not in best or rank > best[short][0]:
            best[short] = (rank, value)
    return {short: value for short, (_, value) in best.items()}


def vendor_count(weapons: int, equipment: int, cap: int) -> int:
    """Weapon and equipment rows are each split into vendors of at most ``cap`` rows."""
    return -(-weapons // cap) + -(-equipment // cap)


def representative_for(required: Set[int], rep_ids: List[int], items) -> Optional[int]:
    """The first representative, in item-id order, carrying one of the required monikers."""
    for item_id in rep_ids:
        if required & items[item_id]["monikers"]:
            return item_id
    return None


def _label_monikers(ids: Iterable[int], names: Dict[int, str]) -> str:
    return ", ".join(sorted(names.get(m, str(m)) for m in ids))


def _esc(s: str) -> str:
    return s.replace("|", "\\|")


def _num(v: Optional[int]) -> str:
    return "-" if v is None else str(v)


def _quality(name: Optional[str]) -> str:
    return "-" if name is None else name.removeprefix("ITEM_QUALITY_")


def build(cap: int = DEFAULT_CAP) -> str:
    monikers, abilities, items, _sources, item_events, _starter = load()
    attrs = item_attributes()
    gated = {m for a in abilities.values() for m in a["item_monikers"]}
    ranged = {item_id for item_id, _, event_id in item_events if event_id == EVENT_RANGED}
    ranged_ability: Dict[int, int] = {}
    for item_id, ability_id, event_id in item_events:
        if event_id == EVENT_RANGED:
            ranged_ability[item_id] = min(ability_id, ranged_ability.get(item_id, ability_id))

    weapons = pick_weapon_reps(weapon_keys(items, attrs, gated, monikers), ranged)
    equipment = pick_equipment_reps(equipment_keys(attrs))
    need = vendor_count(len(weapons), len(equipment), cap)
    dropped = None
    if need > MAX_VENDORS:
        short_w, short_e = drop_quality(weapons), drop_quality(equipment)
        dropped = (len(short_w), len(short_e), vendor_count(len(short_w), len(short_e), cap))

    # Ability coverage: each archetype's tree rows, and the representative for each weapon-gated row.
    rep_ids = sorted(item_id for item_id, _ in weapons.values())
    tree_by_arch: Dict[str, List[int]] = defaultdict(list)
    for r in rows(TREE, "archetype_ability_tree"):
        tree_by_arch[r["archetype"].text].append(int(r["ability_id"].text))
    bandolier_monikers: Set[int] = set()
    for item in items.values():
        if item["bandolier"]:
            bandolier_monikers |= item["monikers"]
    coverage = []  # (archetype, tree rows, [(ability_id, required, representative)])
    for arch in sorted(tree_by_arch):
        gated_rows = []
        for ability_id in tree_by_arch[arch]:
            a = abilities.get(ability_id)
            if a and a["item_monikers"]:
                req = set(a["item_monikers"])
                gated_rows.append((ability_id, req, representative_for(req, rep_ids, items)))
        coverage.append((arch, len(tree_by_arch[arch]), gated_rows))
    no_rep = {aid for _, _, g in coverage for aid, _, rep in g if rep is None}
    fail_tree = {
        aid for _, _, g in coverage for aid, req, _ in g if not (req & bandolier_monikers)
    }

    lines: List[str] = []
    w = lines.append
    w("# Debug Area armory: representative catalog (DR-01)")
    w("")
    w("Generated by `tools/ability_mechanics/debug_armory.py survey` from the seeds under")
    w("`db/resources/`. Do not edit by hand: change the seed and rerun the script")
    w("(`survey --check` fails when this file has drifted).")
    w("")
    if dropped:
        w(f"**Quality dropped.** At {cap} rows per vendor the catalog needs {need} vendors, more than")
        w(f"the {MAX_VENDORS} buy lists (1310-1348). Keyed without `quality_id` (the highest quality")
        w(f"kept), it needs {dropped[2]} vendors: {dropped[0]} weapon and {dropped[1]} equipment")
        w("representatives. DR-03a drops quality on its own when the generator needs it.")
        w("")
    w("The rules (README D-DR2): a weapon is an item in the bandolier (container 3). Its family")
    w("is the sorted moniker names it shares with any `abilities.item_monikers`. Its representative")
    w("per (family, tier, quality) is the lowest item id with a RANGED binding (`items_event_sets`")
    w("event 7), else the lowest item id, marked unbound. Equipment: an item in container 4-14 has")
    w("the lowest such container as its slot; the representative per (slot, tier, quality) is the")
    w("lowest item id. Vendors: weapon and equipment rows are each split into vendors of at most")
    w("the row cap, in key order.")
    w("")
    w("## Summary")
    w("")
    w(f"- Weapon families: {len({k[0] for k in weapons})}")
    w(f"- Weapon representatives: {len(weapons)} (unbound: {sum(1 for _, u in weapons.values() if u)})")
    w(f"- Equipment slots: {len({k[0] for k in equipment})}")
    w(f"- Equipment representatives: {len(equipment)}")
    w(f"- Row cap per vendor: {cap}")
    w(f"- Vendors needed: {need} (buy lists available: {MAX_VENDORS})")
    if dropped:
        w(f"- Vendors needed with quality dropped: {dropped[2]}")
    w(f"- Gated tree abilities with no representative: {len(no_rep)}")
    w(f"- Those match the audit's FAIL tree rows: {'yes' if no_rep == fail_tree else 'no'}")
    w("")
    w("## Weapon representatives")
    w("")
    w("| Family | Tier | Quality | Item | Name | Ranged ability | Unbound |")
    w("|---|---|---|---|---|---|---|")
    for key, (item_id, unbound) in sorted(weapons.items(), key=lambda kv: sort_key(kv[0])):
        family, tier, quality = key
        ranged_id = ranged_ability.get(item_id)
        ranged_s = f"{ranged_id} {abilities[ranged_id]['name']}" if ranged_id is not None else "-"
        fam = ", ".join(family) or "(no gated moniker)"
        w(
            f"| {_esc(fam)} | {_num(tier)} | {_quality(quality)} | {item_id} | "
            f"{_esc(items[item_id]['name'])} | {_esc(ranged_s)} | {'yes' if unbound else 'no'} |"
        )
    w("")
    w("## Equipment representatives")
    w("")
    w("| Slot | Tier | Quality | Item | Name |")
    w("|---|---|---|---|---|")
    for key, (item_id, _) in sorted(equipment.items(), key=lambda kv: sort_key(kv[0])):
        slot, tier, quality = key
        w(f"| container {slot} | {_num(tier)} | {_quality(quality)} | {item_id} | {_esc(items[item_id]['name'])} |")
    w("")
    w("## Ability coverage (archetype trees)")
    w("")
    w("Only the seven archetype trees (README F6, D-DR3). An ability with no requirement is not listed.")
    w("")
    w("| Archetype | Tree rows | Weapon-gated rows | No representative |")
    w("|---|---|---|---|")
    for arch, count, gated_rows in coverage:
        missing = sum(1 for _, _, rep in gated_rows if rep is None)
        w(f"| {arch} | {count} | {len(gated_rows)} | {missing} |")
    w("")
    w("| Archetype | ability_id | Name | Required monikers | Representative |")
    w("|---|---|---|---|---|")
    for arch, _, gated_rows in coverage:
        for ability_id, req, rep in gated_rows:
            rep_s = f"{rep} {items[rep]['name']}" if rep is not None else "none"
            w(
                f"| {arch} | {ability_id} | {_esc(abilities[ability_id]['name'])} | "
                f"{_esc(_label_monikers(req, monikers))} | {_esc(rep_s)} |"
            )
    w("")
    return "\n".join(lines)


def used_ids(files: Iterable[Path]) -> Dict[str, Dict[int, List[str]]]:
    """table -> id -> the file names that insert it, for each table of ``ID_BLOCKS``."""
    used: Dict[str, Dict[int, List[str]]] = defaultdict(lambda: defaultdict(list))
    for path in files:
        if path.name.endswith("_debug_area_armory.sql"):
            continue  # the Armory's own seed rows are not a collision
        text, _ = read_text(path)
        for _, table, col, _, _ in ID_BLOCKS:
            if f"INSERT INTO {table} (" not in text:
                continue
            for r in sql_rows(text, table):
                used[table][int(r[col].text)].append(path.name)
    return used


def audit(files: Iterable[Path]) -> Tuple[List[str], int]:
    """The README F4 blocks, each free or with its used ids; exit code 1 on any used id."""
    used = used_ids(files)
    lines: List[str] = []
    code = 0
    for label, table, col, lo, hi in ID_BLOCKS:
        hits = sorted(i for i in used[table] if lo <= i <= hi)
        span = hi - lo + 1
        if not hits:
            lines.append(f"{label} {lo}-{hi}: free ({span} ids)")
            continue
        code = 1
        lines.append(f"{label} {lo}-{hi}: {len(hits)} of {span} ids used")
        for i in hits:
            lines.append(f"  {table}.{col} = {i}: {', '.join(sorted(set(used[table][i])))}")
    return lines, code


def read_probes(path: Path) -> Dict[str, List[str]]:
    """name -> [x, y, z] from a probe file (``NAME X Y Z`` per line, nav_inspect's format)."""
    lines = path.read_text(encoding="utf-8").splitlines()
    return {p[0]: p[1:4] for p in (line.split() for line in lines) if p}


def write_probes(path: Path, probes: Dict[str, List[str]]) -> str:
    text = "".join(f"{name} {x} {y} {z}\n" for name, (x, y, z) in probes.items())
    path.write_text(text, encoding="utf-8")
    return text


def retries_path(probes: Path) -> Path:
    return probes.with_name(probes.name + ".retries")


def read_retries(probes: Path) -> Dict[str, int]:
    path = retries_path(probes)
    if not path.exists():
        return {}
    return {n: int(c) for n, c in (line.split() for line in path.read_text(encoding="utf-8").splitlines() if line.strip())}


def write_retries(probes: Path, retries: Dict[str, int]) -> None:
    retries_path(probes).write_text("".join(f"{n} {c}\n" for n, c in retries.items()), encoding="utf-8")


def world_spawns() -> List[Tuple[float, float]]:
    """The (x, z) of every world ``ARMORY_WORLD`` spawn in any spawnlist seed."""
    out: List[Tuple[float, float]] = []
    for path in sorted((REPO_ROOT / SEEDS).rglob("spawnlist*.sql")):
        if path.name == "spawnlist_debug_area_armory.sql":
            continue  # the Armory's own spawns must not block its probes
        text, _ = read_text(path)
        for r in sql_rows(text, "spawnlist"):
            if r["world_id"].text == str(ARMORY_WORLD):
                out.append((float(r["x"].text), float(r["z"].text)))
    return out


def cmd_probes(cap: int, out: Path) -> int:
    """Write one probe per vendor; refuse a probe within CLEAR_M of a world 1300 spawn."""
    vendors, _items = representatives(cap)
    spawns = world_spawns()
    probes: Dict[str, List[str]] = {}
    for k in range(len(vendors)):
        x, z = PROBE_X, PROBE_Z + PROBE_STEP * k
        near = min((math.hypot(x - sx, z - sz) for sx, sz in spawns), default=math.inf)
        if near <= CLEAR_M:
            raise Stop(1, f"DR{k} at {x}, {z} is {near:.2f} m from a world {ARMORY_WORLD} spawn")
        probes[f"DR{k}"] = [f"{x:.1f}", f"{PROBE_Y:.1f}", f"{z:.1f}"]
    retries_path(out).unlink(missing_ok=True)
    sys.stdout.write(write_probes(out, probes))
    print(f"wrote {out}: {len(vendors)} vendors at {cap} rows")
    return 0


def check_heights(probes_path: Path, heights: str) -> Dict[str, float]:
    """The grounded dy per probe. An ungrounded probe moves +REGROUND_M in x and exits 3; one out of retries exits 4."""
    probes = read_probes(probes_path)
    grounded = {m.group(1): float(m.group(2)) for m in HEIGHT_LINE.finditer(heights)}
    retries = read_retries(probes_path)
    missing = [n for n in probes if n not in grounded]
    stuck = [f"ungroundable: {n} at {probes[n][0]}, {probes[n][2]}" for n in missing if retries.get(n, 0) >= MAX_RETRIES]
    if stuck:
        raise Stop(4, "\n".join(stuck))
    if missing:
        for n in missing:
            probes[n][0] = f"{float(probes[n][0]) + REGROUND_M:.1f}"
            retries[n] = retries.get(n, 0) + 1
        write_probes(probes_path, probes)
        write_retries(probes_path, retries)
        raise Stop(3, f"re-probed {', '.join(missing)}: run nav_inspect on the new probe file, then emit again")
    return grounded


def cmd_emit(cap: int, heights_path: Path, probes_path: Path, out_dir: Path) -> int:
    """Write the three Armory seed files once every probe is grounded; exit 3 or 4 before that."""
    vendors, items = representatives(cap)
    probes = read_probes(probes_path)
    expected = {f"DR{k}" for k in range(len(vendors))}
    if set(probes) != expected:
        raise Stop(1, f"{probes_path} has {len(probes)} probes, expected {len(expected)} (DR0-DR{len(vendors) - 1})")
    try:
        grounded = check_heights(probes_path, heights_path.read_text(encoding="utf-8"))
    except Stop as stop:
        if stop.code == 3:
            sys.stdout.write(probes_path.read_text(encoding="utf-8"))
        raise
    buy, sell, skipped = stock(vendors, items, SELL_PRICE)
    texts = {
        "item_lists_debug_area_armory.sql": item_sql(buy, sell, SELL_PRICE),
        "entity_templates_debug_area_armory.sql": template_sql(vendors, items),
        "spawnlist_debug_area_armory.sql": spawn_sql(vendors, probes, grounded),
    }
    item_rows = sum(len(rows_) for rows_ in buy) + len(sell)
    last = {
        "entity_templates": TEMPLATE_BASE + len(vendors) - 1,
        "spawnlist": SPAWN_BASE + len(vendors) - 1,
        "item_list_items": ITEM_ROW_BASE + item_rows - 1,
    }
    for table, top in last.items():
        if top > next(hi for _, t, _, _, hi in ID_BLOCKS if t == table):
            raise Stop(1, f"{table} ids run to {top}, past the README F4 block")
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, text in texts.items():
        write_text(out_dir / name, text, "\r\n")
    for line in skipped:
        print(line)
    print(f"wrote {len(vendors)} vendors to {out_dir}")
    return 0


def price_maps() -> Tuple[Dict[int, int], Dict[int, List[Tuple[int, int]]]]:
    """From every seed: the highest sell price per design (any template's sell list), and each design's (list, price) on the other buy lists."""
    buy_ids: Set[int] = set()
    sell_ids: Set[int] = set()
    prices: Dict[int, List[Tuple[int, int]]] = defaultdict(list)  # list id -> (design, naquadah)
    for path in sorted((REPO_ROOT / SEEDS).rglob("*.sql")):
        text, _ = read_text(path)
        if "INSERT INTO entity_templates (" in text:
            for r in sql_rows(text, "entity_templates"):
                # Some seeds write a narrower column list: a missing column is no list.
                for col, ids in (("buy_item_list", buy_ids), ("sell_item_list", sell_ids)):
                    if r.get(col) is not None and r[col].text is not None:
                        ids.add(int(r[col].text))
        if "INSERT INTO item_list_items (" in text:
            for r in sql_rows(text, "item_list_items"):
                price = int(r["naquadah"].text or 0)
                prices[int(r["item_list_id"].text)].append((int(r["design_id"].text), price))
    sell: Dict[int, int] = {}
    for list_id in sell_ids:
        for design, price in prices[list_id]:
            sell[design] = max(sell.get(design, 0), price)
    buy: Dict[int, List[Tuple[int, int]]] = defaultdict(list)
    for list_id in buy_ids:
        for design, price in prices[list_id]:
            buy[design].append((list_id, price))
    return sell, buy


def representatives(cap: int) -> Tuple[List[List[int]], Dict[int, dict]]:
    """The vendors (lists of representative item ids: weapons first, then equipment, ``cap`` rows at most) and the item table.

    Quality is dropped (the highest kept) only when the representatives need more than 39 vendors at the cap.
    """
    monikers, abilities, items, _sources, item_events, _starter = load()
    attrs = item_attributes()
    for item_id, a in attrs.items():
        if item_id in items:
            items[item_id]["sellable"] = bool(a["flags"] & 1024) and not (a["flags"] & 4)
    gated = {m for a in abilities.values() for m in a["item_monikers"]}
    ranged = {item_id for item_id, _, event_id in item_events if event_id == EVENT_RANGED}
    weapons = pick_weapon_reps(weapon_keys(items, attrs, gated, monikers), ranged)
    equipment = pick_equipment_reps(equipment_keys(attrs))
    if vendor_count(len(weapons), len(equipment), cap) > MAX_VENDORS:
        weapons = {k + (None,): v for k, v in drop_quality(weapons).items()}
        equipment = {k + (None,): v for k, v in drop_quality(equipment).items()}
    vendors: List[List[int]] = []
    for group in (weapons, equipment):
        ids = [v[0] for _, v in sorted(group.items(), key=lambda kv: sort_key(kv[0]))]
        vendors += [ids[i : i + cap] for i in range(0, len(ids), cap)]
    if len(vendors) > MAX_VENDORS:
        raise Stop(
            1,
            f"{len(vendors)} vendors needed at {cap} rows, more than the {MAX_VENDORS} buy lists, "
            "even with quality_id dropped",
        )
    return vendors, items


def stock(vendors: List[List[int]], items: Dict[int, dict], sell_price: int):
    """The buy rows per vendor as (design, price), the shared sell list's designs, and the designs it skips (D-DR5)."""
    sell_of, buy_of = price_maps()
    buy = [[(d, max(1, sell_of.get(d, 0))) for d in reps] for reps in vendors]
    sell: List[int] = []
    skipped: List[str] = []
    for d in sorted(d for reps in vendors for d in reps):
        if not items[d].get("sellable", True):
            skipped.append(f"skipped from sell list: {d} {items[d]['name']}, not sellable (items.flags lacks CanBeSold 1024 or has BindOnAcquire 4)")
            continue
        cheaper = [(lid, p) for lid, p in buy_of.get(d, []) if p < sell_price]
        cheaper += [(ARMORY_LISTS[k], p) for k, rows_ in enumerate(buy) for dd, p in rows_ if dd == d and p < sell_price]
        if cheaper:
            lid, p = min(cheaper, key=lambda t: t[1])
            skipped.append(f"skipped from sell list: {d} {items[d]['name']}, cheaper on list {lid} at {p}")
        else:
            sell.append(d)
    return buy, sell, skipped


def main(argv: List[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="command", required=True)
    sv = sub.add_parser("survey", help="write the representative catalog")
    sv.add_argument("--check", action="store_true", help="exit 1 if the committed catalog drifted")
    sv.add_argument("--stdout", action="store_true", help="print the catalog, write nothing")
    sv.add_argument("--cap", type=int, default=DEFAULT_CAP, help="rows per vendor (default %(default)s)")
    sub.add_parser("audit", help="exit 1 if a README F4 id block is already used")
    sd = sub.add_parser("seed", help="the Armory's probe file and seed generator (DR-03a)")
    ss = sd.add_subparsers(dest="step", required=True)
    sp = ss.add_parser("probes", help="lay out one probe per vendor")
    sp.add_argument("--out", required=True, type=Path, help="the probe file to write")
    sp.add_argument("--cap", type=int, default=DEFAULT_CAP, help="rows per vendor (default %(default)s)")
    se = ss.add_parser("emit", help="write the three Armory seed files from grounded probes")
    se.add_argument("--heights", required=True, type=Path, help="nav_inspect output for the probe file")
    se.add_argument("--probes", required=True, type=Path, help="the probe file that nav_inspect read")
    se.add_argument("--out-dir", required=True, type=Path, help="where the seed files go")
    se.add_argument("--cap", type=int, default=DEFAULT_CAP, help="rows per vendor (default %(default)s)")
    args = ap.parse_args(argv)

    if args.command == "audit":
        lines, code = audit(sorted((REPO_ROOT / SEEDS).rglob("*.sql")))
        print("\n".join(lines))
        return code

    if args.cap < 1:
        ap.error("--cap must be at least 1")
    if args.command == "seed":
        try:
            if args.step == "probes":
                return cmd_probes(args.cap, args.out)
            return cmd_emit(args.cap, args.heights, args.probes, args.out_dir)
        except Stop as stop:
            print(stop, file=sys.stderr)
            return stop.code
    doc = build(args.cap)
    if args.stdout:
        sys.stdout.write(doc)
        return 0
    # Compare in LF and keep the file's own line ending on a write (the seeds' rule);
    # a new catalog is CRLF, as every file under docs/ is.
    path = REPO_ROOT / OUT
    old, eol = read_text(OUT) if path.exists() else (None, "\r\n")
    if args.check:
        if old != doc:
            print(f"{OUT} is stale: rerun {Path(__file__).name} survey", file=sys.stderr)
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
