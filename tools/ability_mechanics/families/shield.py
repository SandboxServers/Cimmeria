"""The ``shield`` family (AB-10): absorb shields and mitigation shields.

Two shapes, two scripts (``crates/cell-effect-scripts``):

* **An absorb pool with a number** ("Absorption:\\n500 Physical\\n500
  Energy\\n500 Contamination") binds ``AbsorbShield`` with ``ShieldAmount``
  (the capacity per type) and ``ShieldType`` (the types, by name:
  ``Physical``, ``Energy``, ``Hazmat`` for Contamination, ``Psionic``). The
  script puts one ledger entry with one pool per type on the target; damage
  drains it, and it comes off when empty or at its ``pulse_duration``.
* **A mitigation shield with no pool** ("Target +15% Physical Mitigation")
  binds ``TimedStat`` with a ``Mitigation`` stat row: ``alias.xml`` defines
  ``mitigation`` as "armor mitigation percent (0-100%)", so +15% is 15
  points, no D-AB09 conversion. The stat list has one untyped mitigation
  stat, so the type the text names is dropped (and noted). These rows are
  held toggles (``pulse_duration`` 0 on an ``AF_TOGGLED`` ability): the
  entry is held until the second press (AB-08's held entries).

Everything else that talks about shields is reported, by category:

* ``no number``: "Total Absorption:\\nEnergy:" states no capacity;
* ``moniker``: "Remove Effect of EFFECT_Shield" is a toggle's off half or a
  stance's exclusivity (AB-08), not a shield;
* ``unit``: an armour-factor percentage ("+10% Contamination AF") has no
  D-AB09 unit;
* ``turret`` / ``deployable``: acts on an object the server does not summon
  (D-AB11);
* ``stat``: a resist change under a shield's name is the ``stat`` family's;
* ``routing``: the cast would land it on a hostile (``cast_routing.py``);
* ``grammar``: anything else.

Every row is RECONSTRUCTION: the number is the effect's own text.
"""

from __future__ import annotations

import re
from typing import List, Optional, Tuple

from corpus import Ability, Corpus, Effect
from family import Family, Generated, Outcome, Rejected, desc_lines
from families.cast_routing import hostile_half

ABSORB_SCRIPT = "AbsorbShield"
STAT_SCRIPT = "TimedStat"

AF_TOGGLED = 8

# The NVP names this family writes. `shield_nvp_names_match_the_generator`
# (cell-effect-scripts, shield tests) reads the quoted names between these
# markers and fails when a script would not read one.
# nvp-names begin
NVP_NAMES = (
    "ShieldAmount",
    "ShieldType",
    "Mitigation",
)
# nvp-names end

# Designer spelling -> the ShieldType name the script reads.
DAMAGE_TYPES = {
    "physical": "Physical",
    "energy": "Energy",
    "contamination": "Hazmat",
    "hazmat": "Hazmat",
    "psionic": "Psionic",
}
TYPE_RE = "(" + "|".join(DAMAGE_TYPES) + ")"

CANDIDATE_TEXT = re.compile(r"\babsor|\bmitigation\b|\bdensity\b|\bshield\b", re.I)
SHIELD_NAME = re.compile(r"\bshield\b", re.I)

TARGETING_LINE = re.compile(r"^(single ?target|target|targeted|user)$", re.I)
ABSORB_HEAD = re.compile(r"^(?:total )?absorption:?$", re.I)
ABSORB_POOL = re.compile(r"^(\d+) " + TYPE_RE + r"$", re.I)
ABSORB_EMPTY = re.compile(r"^" + TYPE_RE + r":?$", re.I)
MITIGATION = re.compile(r"^(?:target )?\+(\d+)% (?:" + TYPE_RE + r" )?mitigation$", re.I)
MONIKER = re.compile(r"\bremove(?:s)? (?:effect of EFFECT_Shield|current (?:shield effect|buff))\b|EFFECT_Shield", re.I)
AF_PERCENT = re.compile(r"\b(?:phys|physical|energy|contamination) AF\b", re.I)
RESIST = re.compile(r"\b(?:mental|health|kinetic|interrupt) resist", re.I)


def _lines(effect: Effect) -> List[str]:
    return [ln for ln in desc_lines(effect.desc) if not TARGETING_LINE.match(ln)]


def parse_absorb(effect: Effect) -> Optional[Outcome]:
    """The absorb-pool grammar, or None when the text is not one."""
    lines = _lines(effect)
    if not lines or not ABSORB_HEAD.match(lines[0]):
        return None
    pools: List[Tuple[int, str]] = []
    for ln in lines[1:]:
        m = ABSORB_POOL.match(ln)
        if m:
            pools.append((int(m.group(1)), DAMAGE_TYPES[m.group(2).lower()]))
            continue
        if ABSORB_EMPTY.match(ln):
            return Rejected(effect, f'no number: "{ln}" names a type but no capacity')
        return Rejected(effect, f'grammar: unrecognised line "{ln}"')
    if not pools:
        return Rejected(effect, "no number: the text names no capacity")
    amounts = {a for a, _ in pools}
    if len(amounts) != 1:
        return Rejected(effect, f"grammar: different capacities per type {pools}; ShieldAmount holds one")
    amount = amounts.pop()
    if amount <= 0:
        return Rejected(effect, "no number: a zero capacity")
    types = list(dict.fromkeys(t for _, t in pools))
    notes = [f"{amount} per type, one pool each: {', '.join(types)}"]
    if effect.pulse_duration > 0:
        notes.append(f"{effect.pulse_duration:g} s, the effect's pulse_duration; drained sooner by damage")
    else:
        notes.append("no pulse_duration: held until drained or removed")
    if effect.tcm != "TCM_Single":
        notes.append(f"{effect.tcm}: AB-07 effect routing fans it out to the caster and the allies in its radius")
    return Generated(
        effect,
        [("ShieldAmount", str(amount)), ("ShieldType", ",".join(types))],
        ABSORB_SCRIPT,
        " / ".join(lines),
        notes,
    )


def parse_mitigation(effect: Effect, ability: Optional[Ability]) -> Optional[Outcome]:
    """The mitigation-shield grammar, or None when the text is not one."""
    lines = _lines(effect)
    if len(lines) != 1:
        return None
    m = MITIGATION.match(lines[0])
    if not m:
        return None
    points = int(m.group(1))
    notes = [f"alias.xml: mitigation is a 0-100% stat, so +{points}% is {points} points (no D-AB09 conversion)"]
    if m.group(2):
        notes.append(
            f"the text names {m.group(2)} mitigation; the stat list has one untyped mitigation stat, so the type is dropped"
        )
    if ability is not None and ability.description:
        tip = " ".join(desc_lines(ability.description))
        if m.group(2) and m.group(2).lower() not in tip.lower():
            notes.append(f'the ability tooltip says "{tip}"; the effect row is what executes')
    if effect.pulse_duration <= 0:
        if not (ability and ability.flags & AF_TOGGLED):
            return Rejected(effect, "toggle: held with no AF_TOGGLED, so nothing would take it off")
        notes.append("held toggle (pulse_duration 0, AF_TOGGLED): a held entry until the second press")
    else:
        notes.append(f"{effect.pulse_duration:g} s, the effect's pulse_duration")
    return Generated(effect, [("Mitigation", str(points))], STAT_SCRIPT, lines[0], notes)


class ShieldFamily(Family):
    name = "shield"
    nvp_names = frozenset(NVP_NAMES)
    scripts = frozenset({ABSORB_SCRIPT, STAT_SCRIPT})
    reason_categories = ("no number", "moniker", "unit", "turret", "deployable", "stat", "routing", "toggle", "grammar")

    def is_candidate(self, effect: Effect, corpus: Corpus) -> bool:
        if CANDIDATE_TEXT.search(effect.desc):
            return True
        ability = corpus.abilities.get(effect.ability_id)
        return bool(ability and SHIELD_NAME.search(ability.name))

    def parse(self, effect: Effect, corpus: Corpus) -> Outcome:
        ability = corpus.abilities.get(effect.ability_id)
        if ability and re.search(r"\bturret\b", ability.name, re.I):
            return Rejected(effect, "turret: it shields the user's turret, and turret summons are out of scope (D-AB11)")
        if ability and ability.name.startswith("Deployable:"):
            return Rejected(effect, "deployable: needs a resources.deployables binding")
        if effect.pulse_count != 1:
            return Rejected(effect, f"grammar: pulse_count {effect.pulse_count}, not a single shield")
        out = parse_absorb(effect) or parse_mitigation(effect, ability)
        if out is None:
            return self.classify_other(effect)
        if isinstance(out, Rejected):
            return out
        why = hostile_half(effect, corpus)
        if why:
            return Rejected(effect, "routing: " + why)
        return out

    def classify_other(self, effect: Effect) -> Rejected:
        text = " / ".join(desc_lines(effect.desc))
        if MONIKER.search(effect.desc):
            return Rejected(effect, "moniker: removes a shield or buff by moniker, the toggle-off or exclusivity half (AB-08)")
        if AF_PERCENT.search(effect.desc):
            return Rejected(effect, "unit: an armour-factor percentage has no D-AB09 unit")
        if RESIST.search(effect.desc):
            return Rejected(effect, "stat: a resist change under a shield's name is the stat family's (AB-08 for held toggles)")
        if re.search(r"\bdamage taken\b", effect.desc, re.I):
            return Rejected(effect, "grammar: a damage counter (\"absorbs damage and releases it slowly\"), no shield shape")
        return Rejected(effect, f'grammar: unrecognised text "{text}"')
