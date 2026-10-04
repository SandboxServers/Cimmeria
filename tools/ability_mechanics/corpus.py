"""The seed data every family reads: effects, abilities, the player-reachable
ability set, the hand-authored NVP rows and the existing generated blocks.

"Reachable" is the audit's set (docs/analysis/ability-mechanics/audit.md §1):
every ability in ``archetype_ability_tree`` or ``char_creation_abilities``.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Set, Tuple

from seed_sql import InputError, read_text, sql_rows

EFFECTS_SQL = Path("db/resources/Effects/Seed/effects.sql")
NVPS_SQL = Path("db/resources/Effects/Seed/effect_nvps.sql")
ABILITIES_SQL = Path("db/resources/Abilities/Seed/abilities.sql")
TREE_SQL = Path("db/resources/Archetypes/Seed/archetype_ability_tree.sql")
STARTER_SQL = Path("db/resources/Archetypes/Seed/char_creation_abilities.sql")

# Not the docs-gen `gen:` syntax, which tools/docs-gen/regen.py owns.
BEGIN = "-- ability-mechanics generated {family} begin"
END = "-- ability-mechanics generated {family} end"


@dataclass
class Ability:
    ability_id: int
    name: str
    description: str
    type_id: str


@dataclass
class Effect:
    effect_id: int
    ability_id: int
    name: str
    desc: str
    flags: int
    pulse_count: int
    pulse_duration: float
    tcm: str
    script_name: Optional[str]
    script_span: Tuple[int, int]  # the script_name literal in effects.sql (LF text)


@dataclass
class Block:
    family: str
    start: int  # offset of the begin marker line
    end: int  # offset just past the end marker line's newline
    effect_ids: Set[int]


@dataclass
class Corpus:
    effects: Dict[int, Effect]
    abilities: Dict[int, Ability]
    reachable: Set[int]
    hand_nvps: Dict[int, List[Tuple[str, str]]]  # rows outside every generated block
    blocks: Dict[str, Block]
    effects_text: str
    nvps_text: str
    effects_eol: str
    nvps_eol: str

    def reachable_effects(self) -> Iterable[Effect]:
        for eid in sorted(self.effects):
            e = self.effects[eid]
            if e.ability_id in self.reachable:
                yield e

    def owned_before(self, family: str) -> Set[int]:
        """Effects the family's committed block holds rows for."""
        b = self.blocks.get(family)
        return set(b.effect_ids) if b else set()


def find_blocks(text: str) -> Dict[str, Block]:
    """The generated blocks of an ``effect_nvps.sql`` text, by family."""
    blocks: Dict[str, Block] = {}
    for m in re.finditer(r"^-- ability-mechanics generated (\w+) begin\n", text, re.M):
        family = m.group(1)
        end_marker = END.format(family=family) + "\n"
        e = text.find(end_marker, m.end())
        if e < 0:
            raise InputError(f"{NVPS_SQL}: '{family}' begin marker without an end marker")
        if family in blocks:
            raise InputError(f"{NVPS_SQL}: two '{family}' blocks")
        ids = {int(r["effect_id"].text) for r in sql_rows(text[m.end() : e], "effect_nvps")}
        blocks[family] = Block(family, m.start(), e + len(end_marker), ids)
    for m in re.finditer(r"^-- ability-mechanics generated (\w+) end\n", text, re.M):
        if m.group(1) not in blocks:
            raise InputError(f"{NVPS_SQL}: '{m.group(1)}' end marker without a begin marker")
    return blocks


def _effect(r) -> Effect:
    sn = r["script_name"]
    return Effect(
        effect_id=int(r["effect_id"].text),
        ability_id=int(r["ability_id"].text),
        name=r["name"].text or "",
        desc=r["effect_desc"].text or "",
        flags=int(r["flags"].text),
        pulse_count=int(r["pulse_count"].text),
        pulse_duration=float(r["pulse_duration"].text),
        tcm=r["target_collection_method"].text or "",
        script_name=sn.text,
        script_span=(sn.start, sn.end),
    )


def load_corpus() -> Corpus:
    """The corpus from the committed seed files."""
    effects_text, effects_eol = read_text(EFFECTS_SQL)
    nvps_text, nvps_eol = read_text(NVPS_SQL)
    abilities_text, _ = read_text(ABILITIES_SQL)
    reachable_texts = [read_text(TREE_SQL)[0], read_text(STARTER_SQL)[0]]
    return corpus_from_texts(effects_text, nvps_text, abilities_text, reachable_texts, effects_eol, nvps_eol)


def corpus_from_texts(
    effects_text: str,
    nvps_text: str,
    abilities_text: str,
    reachable_texts: List[str],
    effects_eol: str = "\n",
    nvps_eol: str = "\n",
) -> Corpus:
    """The corpus from seed texts (LF line ends). ``reachable_texts`` are the
    ``archetype_ability_tree`` and ``char_creation_abilities`` seeds; any row
    of either table in them makes its ability reachable."""
    effects: Dict[int, Effect] = {}
    for r in sql_rows(effects_text, "effects"):
        e = _effect(r)
        if e.effect_id in effects:
            raise InputError(f"{EFFECTS_SQL}: effect {e.effect_id} twice")
        effects[e.effect_id] = e
    if not effects:
        raise InputError(f"{EFFECTS_SQL}: no effect rows")

    abilities = {}
    for r in sql_rows(abilities_text, "abilities"):
        aid = int(r["ability_id"].text)
        abilities[aid] = Ability(aid, r["name"].text or "", r["description"].text or "", r["type_id"].text or "")

    reachable: Set[int] = set()
    for text in reachable_texts:
        for table in ("archetype_ability_tree", "char_creation_abilities"):
            reachable |= {int(r["ability_id"].text) for r in sql_rows(text, table)}
    if not reachable:
        raise InputError("no reachable abilities")

    blocks = find_blocks(nvps_text)
    outside = nvps_text
    for b in sorted(blocks.values(), key=lambda b: b.start, reverse=True):
        outside = outside[: b.start] + outside[b.end :]
    hand: Dict[int, List[Tuple[str, str]]] = {}
    for r in sql_rows(outside, "effect_nvps"):
        hand.setdefault(int(r["effect_id"].text), []).append((r["name"].text, r["value"].text))

    return Corpus(effects, abilities, reachable, hand, blocks, effects_text, nvps_text, effects_eol, nvps_eol)
