"""The ``heal`` family (AB-02): pool heals read by ``HealHealth``/``HealFocus``.

The scripts (``crates/cell-effect-scripts/src/cell/effects/heal.rs``) read
``HealAmount`` (flat points, wins when positive) or ``HealPercentage`` (a
percentage of the pool's max, parsed as a float, delta rounded once per
application). Each application is one pulse.

Over time: the pulsing layer fires exactly ``pulse_count`` pulses, the first
synchronously in ``damage_apply`` and ``pulse_count - 1`` from
``register_active_effect`` (``cell-combat/src/cell/effects/pulsing/
register.rs``), and every pulse re-runs the script (``pulsing/tick.rs``). So
a total ("over 25 seconds") is divided by ``pulse_count``, and a rate ("per
second", "Channeled: 1 Second interval") is written as is once its interval
matches ``pulse_duration``. The hand-authored Recuperation row (1383, 3.00 x
25 = 75 %) is the worked example, and the parser reproduces it.

Scope rejections come first: a heal the current pipeline would land on the
wrong entity is reported, not bound.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import List, Optional

from corpus import Ability, Corpus, Effect
from family import Family, Generated, Outcome, Rejected, desc_lines

NUM = r"(\d+(?:\.\d+)?)"
POOL = r"(health|focus)"

# Lines that only say how the effect is targeted.
TARGETING_LINE = re.compile(r"^(single target|(short|small|medium|large|long) radius ae)$", re.I)

# One heal clause per effect; each pattern is a whole line.
HEAL_POOL_PCT = re.compile(
    r"^heals? " + NUM + r"% of (?:the )?(?:player'?s|players|target'?s) " + POOL
    + r" pool(?: over (\d+) seconds| (per second))?\.?$",
    re.I,
)
HEAL_BARE_PCT = re.compile(r"^(?:target )?\+ ?" + NUM + r"% " + POOL + r"$", re.I)
HEAL_PCT_HEAL = re.compile(r"^" + NUM + r"% " + POOL + r" heal$", re.I)
HEAL_FLAT = re.compile(r"^heals? (\d+) " + POOL + r"\.?$", re.I)
HEAL_INCREASE = re.compile(r"^" + POOL + r" increase " + NUM + r"%$", re.I)

RATE_CHANNEL = re.compile(r"^channeled: " + NUM + r" second interval$", re.I)
RATE_PER_PULSE = re.compile(r"^per pulse$", re.I)
RATE_PULSES = re.compile(r"^(\d+) pulses$", re.I)
COST_LINE = re.compile(r"^(energy -?\d+|\d+ energy per pulse)$", re.I)

# Loose detector: text that talks about restoring Health or Focus.
HEAL_CANDIDATE = re.compile(
    r"\bheals?\b"
    r"|\+ ?\d+(?:\.\d+)?% ?(?:health|focus)\b(?! ?(?:regen|resist|damage))"
    r"|\d+(?:\.\d+)?% (?:health|focus) heal"
    r"|\b(?:health|focus) increase \d"
    r"|\+ ?\d+(?:\.\d+)?% every",
    re.I,
)

SCRIPT_FOR_POOL = {"health": "HealHealth", "focus": "HealFocus"}


@dataclass
class HealClause:
    pool: str  # "health" | "focus"
    value: float
    flat: bool  # points, not a percentage
    bare: bool  # "+10% Health": no heal verb, so it could be a max-pool buff
    total_over_secs: Optional[int]  # "over N seconds"
    per_second: bool  # "... pool per second"
    line: str


def match_heal_clause(line: str) -> Optional[HealClause]:
    m = HEAL_POOL_PCT.match(line)
    if m:
        over = int(m.group(3)) if m.group(3) else None
        return HealClause(m.group(2).lower(), float(m.group(1)), False, False, over, bool(m.group(4)), line)
    m = HEAL_BARE_PCT.match(line)
    if m:
        return HealClause(m.group(2).lower(), float(m.group(1)), False, True, None, False, line)
    m = HEAL_PCT_HEAL.match(line)
    if m:
        return HealClause(m.group(2).lower(), float(m.group(1)), False, False, None, False, line)
    m = HEAL_FLAT.match(line)
    if m:
        return HealClause(m.group(2).lower(), float(m.group(1)), True, False, None, False, line)
    m = HEAL_INCREASE.match(line)
    if m:
        return HealClause(m.group(1).lower(), float(m.group(2)), False, True, None, False, line)
    return None


def fmt_pct(v: float) -> Optional[str]:
    """``HealPercentage`` text with two decimals, as the hand rows write it,
    or None when the value is not exact at two decimals."""
    cents = round(v * 100)
    if abs(v * 100 - cents) > 1e-6:
        return None
    return f"{cents / 100:.2f}"


def tooltip_clause(ability: Optional[Ability]) -> Optional[HealClause]:
    """The first heal clause of the ability's own tooltip, for the cross-check."""
    if ability is None:
        return None
    for ln in desc_lines(ability.description):
        c = match_heal_clause(ln)
        if c:
            return c
    return None


def scope_rejection(effect: Effect, ability: Optional[Ability]) -> Optional[str]:
    """Why a heal must not be bound yet, whatever its text says."""
    aname = ability.name if ability else ""
    atype = ability.type_id if ability else ""
    if effect.tcm != "TCM_Single":
        return (
            f"{effect.tcm}: the pipeline lands every effect on the ability's one target; "
            "AE/group routing is AB-07 (D-AB12), and binding it now would heal that target twice"
        )
    if aname.startswith("Deployable:"):
        return (
            "deployable pulse effect: needs a resources.deployables binding; bound as a plain "
            "effect it would heal the caster for the deployable's whole lifetime"
        )
    if ability and re.search(r"\breviv", ability.description, re.I):
        return "revive heal: self-revive is out of scope (D-AB11)"
    if atype not in ("ABILITY_TYPE_Heal", "ABILITY_TYPE_Buff"):
        return f"heal half of an ability of type {atype}: it belongs on the user, not the target (B-27, AB-07)"
    return None


class HealFamily(Family):
    name = "heal"
    nvp_names = frozenset({"HealPercentage", "HealAmount"})
    scripts = frozenset(SCRIPT_FOR_POOL.values())

    def is_candidate(self, effect: Effect, corpus: Corpus) -> bool:
        return bool(HEAL_CANDIDATE.search(" ".join(desc_lines(effect.desc))))

    def parse(self, effect: Effect, corpus: Corpus) -> Outcome:
        ability = corpus.abilities.get(effect.ability_id)
        why = scope_rejection(effect, ability)
        if why:
            return Rejected(effect, why)
        return parse_heal(effect, ability)


def parse_heal(effect: Effect, ability: Optional[Ability]) -> Outcome:
    """The grammar proper, apart from the scope rules."""
    atype = ability.type_id if ability else ""
    clauses: List[HealClause] = []
    rate_interval: Optional[float] = None
    per_pulse = False
    stated_pulses: Optional[int] = None
    notes: List[str] = []
    for ln in desc_lines(effect.desc):
        if TARGETING_LINE.match(ln):
            continue
        c = match_heal_clause(ln)
        if c:
            clauses.append(c)
            continue
        m = RATE_CHANNEL.match(ln)
        if m:
            rate_interval = float(m.group(1))
            continue
        if RATE_PER_PULSE.match(ln):
            per_pulse = True
            continue
        m = RATE_PULSES.match(ln)
        if m:
            stated_pulses = int(m.group(1))
            continue
        if COST_LINE.match(ln):
            notes.append(f'"{ln}" is a cost; no ability cost is modelled (B-04)')
            continue
        return Rejected(effect, f'unrecognised line "{ln}"')
    if len(clauses) != 1:
        return Rejected(effect, f"{len(clauses)} heal clauses (want exactly one)")
    c = clauses[0]
    if c.bare and atype != "ABILITY_TYPE_Heal":
        return Rejected(effect, f'bare "{c.line}" on an ability of type {atype}: a max-pool buff or a heal? (stat family, AB-04)')

    pc, pd = effect.pulse_count, effect.pulse_duration
    unit = "" if c.flat else "%"
    if stated_pulses is not None and stated_pulses != pc:
        return Rejected(effect, f"text says {stated_pulses} pulses but pulse_count is {pc}")
    rate = per_pulse or c.per_second or rate_interval is not None
    if pc == 0:
        return Rejected(effect, "channelled (pulse_count 0): no finite pulse count to divide by")
    if pc == 1:
        if rate or c.total_over_secs is not None:
            return Rejected(effect, f"text heals over time but pulse_count is 1 (pulse_duration {pd:g})")
        share = c.value
    elif c.total_over_secs is not None:
        if rate:
            return Rejected(effect, "text gives both a total and a rate")
        if abs(pc * pd - c.total_over_secs) > 1e-6:
            return Rejected(effect, f"over {c.total_over_secs} seconds but pulse_count x pulse_duration is {pc * pd:g}")
        share = c.value / pc
        notes.append(f"{c.value:g}{unit} in total over {pc} pulses of {pd:g} s: {share:g}{unit} per pulse")
    elif rate:
        interval = rate_interval if rate_interval is not None else (1.0 if c.per_second else pd)
        if abs(interval - pd) > 1e-6:
            return Rejected(effect, f"a rate every {interval:g} s but pulse_duration is {pd:g}")
        share = c.value
        notes.append(f"{c.value:g}{unit} per pulse, {pc} pulses of {pd:g} s ({c.value * pc:g}{unit} in total)")
    else:
        return Rejected(effect, f"pulse_count {pc} but the text gives no rate or total")

    if c.flat:
        if share <= 0 or abs(share - round(share)) > 1e-9:
            return Rejected(effect, f"flat per-pulse share {share:g} is not a positive whole number")
        nvp = ("HealAmount", str(int(round(share))))
    else:
        v = fmt_pct(share)
        if v is None or share <= 0:
            return Rejected(effect, f"per-pulse share {share:g}% is not exact at two decimals")
        nvp = ("HealPercentage", v)

    tip = tooltip_clause(ability)
    if tip and (tip.pool != c.pool or abs(tip.value - c.value) > 1e-9):
        notes.append(f'the ability tooltip says "{tip.line}"; the effect row is what executes (as for 3211)')
    return Generated(effect, [nvp], SCRIPT_FOR_POOL[c.pool], c.line, notes)
