"""The ``stat`` family (AB-04): timed stat buffs and debuffs, read by ``TimedStat``.

The script (``crates/cell-effect-scripts/src/cell/effects/stat_buff/``)
reads one NVP per stat, named in ``STAT_BUFF_NVPS`` there, and puts one entry
on the timed effect ledger per ``(effect, invoker)`` for the effect's
``pulse_duration``. So this family writes stat-named rows for the
``pulse_count = 1``, ``pulse_duration > 0`` effects whose text is a stat
change ("+200 Accuracy: 15 Seconds", "Target -100 ACC", "Debuff -200 ACC /
DEF: 15 Seconds") and binds ``TimedStat``.

Units are decision D-AB09's, and every converted row says so in a note:

* a bare number is stat points ("+200 Accuracy" = 200 points, 2 QR per
  ``alias.xml``);
* a percentage on a resist or interrupt stat is 10 points per 1 % (effect
  2004 "+50 (5%) Mental Resist", 2005 "Subtlety -100 (10% increase to
  threat)");
* a percentage on run speed is ``movementSpeedMod`` percent (100 is
  unmodified, so "+50% Run Speed" is +50);
* a percentage on a pool max is a percentage of max: the ledger moves
  ``cur``, not ``max``, so those are reported (AB-05/AB-10), as are the
  regen stats (AB-05: ``regen.rs`` still reads them as points per second,
  D-AB04) and the armour factors (no D-AB09 rule).

Scope rejections, before the grammar:

* not a timed single pulse: a held effect (``pulse_duration = 0``, a stance)
  or a passive (``EF_AlwaysPersist``), or an ability with ``AF_TOGGLED``, is
  AB-08's; a multi-pulse one is not a ledger entry;
* ``EF_ClearOnDamage`` ("(1 hit)"): no hook takes it off on damage yet
  (AB-11), so it would last its whole duration;
* a cone, a group or aura effect (D-AB12), a hostile ``TCM_AERadius``
  effect of a non-ground ability or a beneficial one of a ground ability,
  or a "Secondary" line: the cast would land it on the one target or on
  hostiles (AB-07 routes user halves and beneficial radius halves only);
* a deployable's or a turret enhancement's effect: it acts on an object the
  server does not summon for it (D-AB11);
* routing (``routing.py``, the server's AB-07 rules): an effect the server
  lands on the user (``EF_ResolveOnAbilityUser``, or a single effect of a
  Self ability with no area effect, such as Combat Sprint's "-100 ACC") or on
  the caster's allies is bound. One that lands on the cast's target is
  refused when it is a "User" half, a single effect of a Self ability with
  an area effect, or a beneficial effect whose ability also has a
  non-beneficial effect that does something (the cast would take the
  hostile path and land the buff on the target, B-27).
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

import routing
from corpus import Ability, Corpus, Effect
from family import Family, Generated, Outcome, Rejected, desc_lines
from routing import is_ground

SCRIPT = "TimedStat"

EF_BENEFICIAL_EFFECT = 1
EF_CLEAR_ON_DAMAGE = 8
EF_ALWAYS_PERSIST = 524288
AF_TOGGLED = 8
TARGET_SELF = 1

# The NVP names this family writes. `stat_nvp_names_match_the_generator`
# (cell-effect-scripts, stat_buff/tests.rs) reads the quoted names between
# these markers and fails when the script would not read one.
# nvp-names begin
NVP_NAMES = (
    "Accuracy",
    "Defense",
    "CoverAccuracy",
    "CoverDefense",
    "CrouchingDefense",
    "Response",
    "InterruptResistance",
    "KineticResistance",
    "MentalResistance",
    "HealthResistance",
    "MovementSpeedMod",
)
# nvp-names end

# Designer spellings -> (NVP name, unit kind). Kinds: "points" (bare number
# only), "resist" (points, or 10 per 1 %), "speed" (percent only).
STATS: Dict[str, Tuple[str, str]] = {
    "accuracy": ("Accuracy", "points"),
    "acc": ("Accuracy", "points"),
    "defense": ("Defense", "points"),
    "def": ("Defense", "points"),
    "cover acc": ("CoverAccuracy", "points"),
    "cover accuracy": ("CoverAccuracy", "points"),
    "coveraccuracy": ("CoverAccuracy", "points"),
    "cover defense": ("CoverDefense", "points"),
    "coverdefense": ("CoverDefense", "points"),
    "crouching defense": ("CrouchingDefense", "points"),
    "response": ("Response", "points"),
    "interrupt resistance": ("InterruptResistance", "resist"),
    "kinetic resistance": ("KineticResistance", "resist"),
    "kinetic resist": ("KineticResistance", "resist"),
    "mental resistance": ("MentalResistance", "resist"),
    "mental resist": ("MentalResistance", "resist"),
    "health resistance": ("HealthResistance", "resist"),
    "health resist": ("HealthResistance", "resist"),
    "run speed": ("MovementSpeedMod", "speed"),
    "movement speed": ("MovementSpeedMod", "speed"),
    "move": ("MovementSpeedMod", "speed"),
}

# Stats the text names that this packet deliberately leaves alone, and why.
DEFERRED: Dict[str, str] = {
    "focus regen": "a regen stat: regen.rs reads it as points per second, D-AB04 makes it a percentage (AB-05)",
    "focus regeneration": "a regen stat: regen.rs reads it as points per second, D-AB04 makes it a percentage (AB-05)",
    "health regen": "a regen stat: regen.rs reads it as points per second, D-AB04 makes it a percentage (AB-05)",
    "maximum focus": "a pool-max change: the ledger moves cur, not max (D-AB09's percent of max needs a max delta)",
    "stealth rating": "stealth is out of scope (D-AB11)",
    "max stealth rating": "stealth is out of scope (D-AB11)",
    "phys af": "an armour factor: D-AB09 gives no unit for a percentage on it",
    "energy af": "an armour factor: D-AB09 gives no unit for a percentage on it",
    "contamination af": "an armour factor: D-AB09 gives no unit for a percentage on it",
    "energy mitigation": "mitigation is the enemy-combat campaign's (MITIGATION 0/0)",
}

ALL_NAMES = sorted(list(STATS) + list(DEFERRED), key=len, reverse=True)
STAT_RE = "(" + "|".join(re.escape(n).replace(r"\ ", r" ?") for n in ALL_NAMES) + ")"
NUM = r"(\d+(?:\.\d+)?)"
SIGN = r"([+-])"
DURATION = r"(?::? ?(?:for )?(\d+(?:\.\d+)?) ?(?:seconds?|sec)\.?)?"

# "+200 Accuracy", "-30% Movement Speed", "+200 Cover ACC for 15 Seconds"
SIGN_FIRST = re.compile(r"^" + SIGN + r" ?" + NUM + r"(%)? ?" + STAT_RE + DURATION + r"$", re.I)
# "Accuracy -100", "Run Speed +50%", "Movement Speed-30%", "Cover Defense Debuff: -100"
STAT_FIRST = re.compile(r"^" + STAT_RE + r"(?: debuff:)? ?" + SIGN + r" ?" + NUM + r"(%)?" + DURATION + r"$", re.I)
# "-200 ACC / DEF: 15 Seconds", "-200 ACC / -200 DEF", "-200ACC / -200DEF"
PAIR = re.compile(
    r"^" + SIGN + r" ?" + NUM + r" ?" + STAT_RE + r" ?/ ?(?:" + SIGN + r" ?" + NUM + r" ?)?" + STAT_RE + DURATION + r"$",
    re.I,
)

TARGETING_LINE = re.compile(
    r"^(single target|target|targeted|(short|small|medium|large|long|melee) radius( ae)?|(narrow|medium|wide) cone)$",
    re.I,
)
DURATION_LINE = re.compile(
    r"^(?:duration:? ?)?(\d+(?:\.\d+)?) ?(?:seconds?|sec)(?: duration)?\.?$", re.I
)
PREFIX = re.compile(r"^(target|user|debuff:?) ", re.I)

# Loose detector: a line that puts a sign or a number next to a stat name.
CANDIDATE = re.compile(
    r"(?:[+-] ?\d|\d ?%?) ?" + STAT_RE + r"\b|\b" + STAT_RE + r"(?: debuff:)? ?[+-] ?\d",
    re.I,
)
DAMAGE_TEXT = re.compile(r"-\d+ ?F\b|\bF ?-\d+|-\d+ ?H\b", re.I)


@dataclass
class StatClause:
    nvp: str
    kind: str
    sign: int
    value: float
    pct: bool
    duration: Optional[float]
    spelled: str
    line: str


def _key(spelled: str) -> str:
    return re.sub(r"\s+", " ", spelled.strip().lower())


def _lookup(spelled: str) -> Tuple[Optional[Tuple[str, str]], Optional[str]]:
    k = _key(spelled)
    for name, val in STATS.items():
        if k.replace(" ", "") == name.replace(" ", ""):
            return val, None
    for name, why in DEFERRED.items():
        if k.replace(" ", "") == name.replace(" ", ""):
            return None, why
    return None, f'unknown stat "{spelled}"'


def match_clauses(line: str) -> Optional[List[Tuple[str, int, float, bool, Optional[float]]]]:
    """The ``(spelled stat, sign, value, pct, duration)`` clauses of one
    line, or None when the line is not a stat clause."""
    m = PAIR.match(line)
    if m:
        s1, n1, st1, s2, n2, st2, dur = m.groups()
        d = float(dur) if dur else None
        sign1 = -1 if s1 == "-" else 1
        sign2 = sign1 if s2 is None else (-1 if s2 == "-" else 1)
        n2v = float(n1) if n2 is None else float(n2)
        return [(st1, sign1, float(n1), False, d), (st2, sign2, n2v, False, d)]
    m = SIGN_FIRST.match(line)
    if m:
        s, n, pct, st, dur = m.groups()
        return [(st, -1 if s == "-" else 1, float(n), bool(pct), float(dur) if dur else None)]
    m = STAT_FIRST.match(line)
    if m:
        st, s, n, pct, dur = m.groups()
        return [(st, -1 if s == "-" else 1, float(n), bool(pct), float(dur) if dur else None)]
    return None


def convert(c: StatClause) -> Tuple[Optional[int], Optional[str], Optional[str]]:
    """``(points, note, rejection)`` for one clause under D-AB09."""
    if abs(c.value - round(c.value)) > 1e-9:
        return None, None, f"{c.value:g} is not a whole number"
    v = int(round(c.value)) * c.sign
    if c.kind == "speed":
        if not c.pct:
            return None, None, f'"{c.spelled}" without a percentage has no D-AB09 unit'
        return v, f"D-AB09: {v:+d}% run speed is movementSpeedMod {v:+d} (100 = unmodified)", None
    if c.kind == "resist":
        if c.pct:
            return v * 10, f"D-AB09: {v:+d}% {c.spelled} is {v * 10:+d} points (10 per 1%)", None
        return v, f"D-AB09: a bare {v:+d} is {v:+d} points", None
    if c.pct:
        return None, None, f'a percentage on "{c.spelled}" has no D-AB09 unit'
    note = f"D-AB09: a bare {v:+d} is {v:+d} stat points"
    if c.nvp == "Accuracy" or c.nvp == "Defense":
        note += f" ({v / 100:+g} QR per alias.xml)"
    return v, note, None


def parse_stat(effect: Effect) -> Outcome:
    """The grammar proper, apart from the scope and routing rules."""
    clauses: List[StatClause] = []
    durations: List[float] = []  # every duration the text states
    user = False
    for raw in desc_lines(effect.desc):
        ln = raw
        if TARGETING_LINE.match(ln):
            continue
        m = DURATION_LINE.match(ln)
        if m:
            durations.append(float(m.group(1)))
            continue
        while True:
            p = PREFIX.match(ln)
            if not p:
                break
            if p.group(1).lower() == "user":
                user = True
            ln = ln[p.end():]
        found = match_clauses(ln)
        if found is None:
            return Rejected(effect, f'unrecognised line "{raw}"')
        for spelled, sign, value, pct, dur in found:
            stat, why = _lookup(spelled)
            if stat is None:
                return Rejected(effect, why or f'unknown stat "{spelled}"')
            clauses.append(StatClause(stat[0], stat[1], sign, value, pct, dur, spelled, raw))
            if dur is not None:
                durations.append(dur)
    if not clauses:
        return Rejected(effect, "no stat clause")
    pd = effect.pulse_duration
    # Every stated duration must match, not just the last.
    for duration in durations:
        if abs(duration - pd) > 1e-6:
            return Rejected(effect, f"text says {duration:g} s but pulse_duration is {pd:g}")
    nvps: List[Tuple[str, str]] = []
    notes: List[str] = []
    for c in clauses:
        points, note, why = convert(c)
        if why:
            return Rejected(effect, why)
        if points == 0:
            return Rejected(effect, f'"{c.line}" moves nothing')
        if any(n == c.nvp for n, _ in nvps):
            return Rejected(effect, f"{c.nvp} twice")
        nvps.append((c.nvp, str(points)))
        notes.append(note)
    notes.append(f"{pd:g} s, the effect's pulse_duration")
    out = Generated(effect, nvps, SCRIPT, " / ".join(dict.fromkeys(c.line for c in clauses)), notes)
    out.user = user  # type: ignore[attr-defined]
    return out


def tooltip_stats(ability: Optional[Ability]) -> set:
    """The NVP names the ability's tooltip talks about, for the cross-check.
    Lines the grammar cannot read are ignored: a tooltip is prose."""
    names = set()
    if ability is None:
        return names
    for ln in desc_lines(ability.description):
        ln = PREFIX.sub("", ln)
        for spelled, *_ in match_clauses(ln) or []:
            stat, _ = _lookup(spelled)
            if stat:
                names.add(stat[0])
    return names


def scope_rejection(effect: Effect, ability: Optional[Ability]) -> Optional[str]:
    """Why a stat effect must not be bound yet, whatever its text says."""
    if effect.pulse_count != 1:
        return f"pulse_count {effect.pulse_count}: not a single-pulse timed effect"
    if effect.flags & EF_ALWAYS_PERSIST:
        return "EF_AlwaysPersist: a passive, applied at login (AB-08)"
    if effect.pulse_duration <= 0:
        return "held (pulse_duration 0): a stance or toggle, removed by a second press (AB-08)"
    if ability and ability.flags & AF_TOGGLED:
        return "an AF_TOGGLED ability: a second press must remove it (AB-08)"
    if effect.flags & EF_CLEAR_ON_DAMAGE:
        return "EF_ClearOnDamage: no damage hook removes it yet (AB-11), so it would outlast its design"
    if effect.tcm == "TCM_AERadius":
        # AB-07: a beneficial radius effect of a non-ground cast fans out to
        # the caster's allies, and a ground cast's secondaries take its
        # hostile radius effects. A hostile radius effect of a non-ground
        # cast still lands on the one target, and a beneficial one of a
        # ground cast on hostiles.
        beneficial = bool(effect.flags & EF_BENEFICIAL_EFFECT)
        if beneficial and is_ground(ability):
            return "a beneficial TCM_AERadius effect of a ground ability: the ground collector takes hostiles"
        if not beneficial and not is_ground(ability):
            return (
                "a hostile TCM_AERadius effect of a non-ground ability: AB-07 fans out only beneficial "
                "radius effects there, so it would land on the one target"
            )
    elif effect.tcm in ("TCM_Group", "TCM_Aura"):
        return f"{effect.tcm}: group and aura routing waits for D-AB12; the pipeline would land it on the one target"
    elif effect.tcm != "TCM_Single":
        return f"{effect.tcm}: a stat cone is not routed (AB-07 routes user and radius halves only)"
    if re.search(r"\bsecondary\b", effect.desc, re.I):
        return (
            "a Secondary-target effect: AB-07 routes user and radius halves, not secondary targets; "
            "today it lands on the primary target"
        )
    if ability and ability.name.startswith("Deployable:"):
        return "deployable effect: needs a resources.deployables binding"
    if ability and re.search(r"\bturret\b", ability.name, re.I):
        return "a turret enhancement: it acts on the user's turret, and turret summons are out of scope (D-AB11)"
    return None


class StatFamily(Family):
    name = "stat"
    nvp_names = frozenset(NVP_NAMES)
    scripts = frozenset({SCRIPT})

    def is_candidate(self, effect: Effect, corpus: Corpus) -> bool:
        return any(CANDIDATE.search(ln) for ln in desc_lines(effect.desc))

    def parse(self, effect: Effect, corpus: Corpus) -> Outcome:
        ability = corpus.abilities.get(effect.ability_id)
        why = scope_rejection(effect, ability)
        if why:
            return Rejected(effect, why)
        out = parse_stat(effect)
        if isinstance(out, Rejected):
            return out
        why = self.routing_rejection(effect, ability, corpus, getattr(out, "user", False))
        if why:
            return Rejected(effect, why)
        tip = tooltip_stats(ability)
        if tip and not set(n for n, _ in out.nvps) & tip:
            out.notes.append(
                f"the ability tooltip names {sorted(tip)}; the effect row is what executes (as for heal 3211)"
            )
        return out

    def routing_rejection(
        self, effect: Effect, ability: Optional[Ability], corpus: Corpus, user: bool
    ) -> Optional[str]:
        """Whether the cast would land this effect where the text says."""
        landing = routing.route(effect, ability, corpus, SCRIPT)
        if landing != routing.TARGET:
            # AB-07 lands it on the user or the caster's allies whatever the
            # rest of the ability does (B-27).
            return None
        self_ability = ability is not None and ability.target_type_id == TARGET_SELF
        beneficial = bool(effect.flags & EF_BENEFICIAL_EFFECT)
        if user:
            return (
                "a User half with no EF_ResolveOnAbilityUser on an ability that is not a pure Self "
                "ability: AB-07 lands it on the cast's target (B-27)"
            )
        if not beneficial:
            if self_ability:
                return (
                    "a single effect of a Self ability that has an area effect: it is a follow-up of the "
                    "area hit and lands on the client's target (B-27)"
                )
            return None
        for other in corpus.effects.values():
            if other.ability_id != effect.ability_id or other.effect_id == effect.effect_id:
                continue
            if other.flags & EF_BENEFICIAL_EFFECT:
                continue
            if self.does_something(other, corpus):
                return (
                    f"effect {other.effect_id} of the same ability is not beneficial and does something, "
                    "so the cast takes the hostile path and would land this buff on the target (B-27, AB-07)"
                )
        return None

    def does_something(self, other: Effect, corpus: Corpus) -> bool:
        """Whether ``other`` does something today or once AB-03/AB-04 bind it."""
        if other.script_name is not None and other.script_name != SCRIPT:
            return True
        if any(n in ("HealthDamage", "FocusDamage") for n, _ in corpus.hand_nvps.get(other.effect_id, [])):
            return True
        if DAMAGE_TEXT.search(other.desc):
            return True
        # A non-beneficial stat effect this family binds (its acceptance never
        # depends on another effect, so there is no cycle).
        if self.is_candidate(other, corpus):
            ability = corpus.abilities.get(other.ability_id)
            if scope_rejection(other, ability) is None:
                out = parse_stat(other)
                if isinstance(out, Generated):
                    return self.routing_rejection(other, ability, corpus, getattr(out, "user", False)) is None
        return False
