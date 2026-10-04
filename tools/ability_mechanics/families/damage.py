"""The ``damage`` family (AB-03): ``FocusDamage``/``HealthDamage`` on hostile effects.

The NVP damage path (``crates/cell-combat/src/cell/abilities/damage_apply/``)
reads ``HealthDamage`` and ``FocusDamage`` per effect: since AB-03 each
``TCM_Single`` effect resolves on its own, and a cone or radius effect is the
damage of its fan-out (and of the primary only when no single-target damage
effect lands). Each application is one pulse: the first in ``damage_apply``,
the rest from the pulsing layer (``cell-combat/src/cell/effects/pulsing/
tick.rs``), which re-reads the same NVPs. So a DoT's row is its per-tick
amount, which is how the designer text states it ("-150F -30H (8 Ticks)").

No script is bound: the rows feed the NVP path. Effects with a hand-authored
``FocusDamage``/``HealthDamage`` row (Pistol Shot 654, Strike 656, the
Microwave Emitter 5066) are left alone by the shared ownership rule.

What is reported instead of written (the reason starts with its category):

* ``conditional``: a variant that only applies against a kind of target or
  from a position ("Mechanical Target Damage", "Flank Position Damage",
  "Assassin Stance Bonus Damage", Execution's damage vs low Focus). No
  conditional NVP exists, and the pipeline would apply the variant on every
  hit, on top of the base damage.
* ``sequenced``: a single-shot effect carrying ``EF_SequenceOnFinish`` (64).
  It is the follow-up of a sequence (a check's outcome, a chain-lightning
  jump, a grenade barrage's extra shell), which the pipeline does not model;
  applying it on every hit would stack it on the base damage.
* ``targeting``: the text names a shape the row does not have ("Medium Cone"
  on a ``TCM_Single`` row would land a second hit on the primary).
* ``pulse shape``: the text's tick count or interval disagrees with
  ``pulse_count``/``pulse_duration``, or a pulsing row states no tick count.
* ``scope``: the damage half of a Buff ability (it belongs on the user,
  B-27/AB-07), an ``EF_ResolveOnAbilityUser`` effect, or a target
  collection the pipeline does not route.
* ``grammar``: a line the parser does not know, two amount clauses, a
  missing minus sign ("-800F / 80H"), two amounts for one pool.
"""

from __future__ import annotations

import re
from typing import List, Optional, Tuple

from corpus import Ability, Corpus, Effect
from family import Family, Generated, Outcome, Rejected, desc_lines

# EEffectFlag (entities/defs/enumerations.xml).
EF_SEQUENCE_ON_FINISH = 64
EF_RESOLVE_ON_ABILITY_USER = 131072

HOSTILE_TYPES = ("ABILITY_TYPE_DD", "ABILITY_TYPE_DOT", "ABILITY_TYPE_Debuff")
ROUTED_TCMS = ("TCM_Single", "TCM_AECone", "TCM_AERadius")

# One amount: "-200F", "- 800 F", "F-200", "F -50", "Focus Damage: -200".
AMOUNT = re.compile(
    r"-\s*(?P<n1>\d+)\s*(?P<p1>[FH])\b"
    r"|\b(?P<p2>[FH])\s*-\s*(?P<n2>\d+)\b"
    r"|\b(?P<p3>focus|health) damage:?\s*-\s*(?P<n3>\d+)\b",
    re.I,
)
# An amount with no minus sign ("80H"): the designer dropped the sign, or the
# text means something else. Either way a human reads it.
UNSIGNED = re.compile(r"(?<![-\w])\d+\s*[FH]\b|\b[FH]\s*\d+\b", re.I)

# A label before the amounts on the same line.
AMOUNT_LABEL = re.compile(
    r"^(?:target|secondary(?: damage)?|damage(?: over time)?|dot|focus dot|wound|"
    r"secondary damage|aoe damage|cone damage|grenade damage)\s*:?\s*",
    re.I,
)
# What may follow the amounts on the same line.
AMOUNT_SUFFIX = re.compile(
    r"^(?:per (?P<per>tick|pulse)"
    r"|\(?(?:ticks: ?)?(?P<n1>\d+) ?ticks?\)?"
    r"|\((?P<n2>\d+) ?ticks\)"
    r"|dot: (?P<n3>\d+) ticks)$",
    re.I,
)

# Whole lines that only say how the effect is targeted: (regex, shape).
SHAPE_LINES: List[Tuple[re.Pattern, Optional[str]]] = [
    (re.compile(r"^(?:single target|target(?:ed)?)(?: (?:melee )?damage)?:?$", re.I), "single"),
    (re.compile(r"^(?:melee )?damage:?$", re.I), None),
    (re.compile(r"^secondary(?: targets?| damage)?$", re.I), "secondary"),
    (re.compile(r"^focus dot$", re.I), None),
    (re.compile(r"^(?:short|small|medium|large|long|melee) radius ae$", re.I), "radius"),
    (re.compile(r"^aoe(?: damage| radius)?(?:: ?(?:short|medium|long))?:?$", re.I), "radius"),
    (re.compile(r"^(?:[a-z]+ )?grenade damage:$|^gamma strike damage:$", re.I), None),
    (
        re.compile(
            r"^(?:(?:wide|medium|narrow|melee|medium narrow|melee narrow) )?cone(?: damage)?"
            r"(?:: ?(?:beam|wide|narrow))?:?$|^cone narrow$",
            re.I,
        ),
        "cone",
    ),
]
# "Single Target Channeled: 10 ticks: 1.5 second intervals" and its kin.
CHANNEL_LINE = re.compile(
    r"^(?P<shape>single target|narrow cone) channeled(?: damage)?"
    r"(?:: (?P<n>\d+) ticks(?:: (?P<i>\d+(?:\.\d+)?) ?second (?:intervals?|duration))?)?$",
    re.I,
)
TICKS_LINE = re.compile(
    r"^(?P<n>\d+) (?:ticks|pulses):?(?: x ?(?P<i>\d+(?:\.\d+)?) ?(?:seconds?|sec))?$", re.I
)
INTERVAL_LINE = re.compile(r"^(?P<i>\d+(?:\.\d+)?) second pulse$", re.I)
COST_LINE = re.compile(
    r"^(?:-?\d+ ammo(?: per (?:tick|pulse))?|energy ?-?\d+|\d+ energy per (?:pulse|target))$", re.I
)
NOT_MODELLED_LINE = re.compile(r"^(?:increased threat \+\d+|energy return: \d+%)$", re.I)

# Variants that apply only against some targets or from some position.
CONDITIONAL = re.compile(
    r"\bmechanical\b|\bflank\b|\brear\b|\bpositional?\b|\bstance\b|\bbonus\b"
    r"|low focus|\bwhile moving\b",
    re.I,
)

TCM_SHAPE = {"TCM_Single": "single", "TCM_AECone": "cone", "TCM_AERadius": "radius"}

CANDIDATE = re.compile(
    r"-\s*\d+\s*[FH]\b|\b[FH]\s*-\s*\d|\b(?:focus|health) damage:?\s*-\s*\d", re.I
)


def conditional_reason(effect: Effect, ability: Optional[Ability]) -> Optional[str]:
    text = effect.name + "\n" + effect.desc
    # "Back Slash Damae: Non-Positional" is the unconditional half.
    text = re.sub(r"non-positional", "", text, flags=re.I)
    m = CONDITIONAL.search(text)
    if m:
        return f'conditional: "{m.group(0)}" variant; no conditional NVP exists, and every hit would apply it'
    return None


def scope_reason(effect: Effect, ability: Optional[Ability]) -> Optional[str]:
    atype = ability.type_id if ability else ""
    if atype not in HOSTILE_TYPES:
        return f"scope: damage half of an ability of type {atype}: it belongs on the user (B-27, AB-07)"
    if effect.tcm not in ROUTED_TCMS:
        return f"scope: {effect.tcm} damage is not routed by the pipeline (AB-07, D-AB12)"
    if effect.flags & EF_RESOLVE_ON_ABILITY_USER:
        return "scope: EF_ResolveOnAbilityUser resolves on the user; user routing is AB-07"
    if effect.flags & EF_SEQUENCE_ON_FINISH and effect.pulse_count == 1:
        return (
            "sequenced: single-shot EF_SequenceOnFinish follow-up (a check's outcome, a chain jump, "
            "an extra shell); the pipeline would apply it on every hit"
        )
    return None


class DamageFamily(Family):
    name = "damage"
    nvp_names = frozenset({"HealthDamage", "FocusDamage"})
    scripts = frozenset()
    reason_categories = ("conditional", "sequenced", "targeting", "pulse shape", "scope", "grammar")

    def is_candidate(self, effect: Effect, corpus: Corpus) -> bool:
        return bool(CANDIDATE.search(" ".join(desc_lines(effect.desc))))

    def parse(self, effect: Effect, corpus: Corpus) -> Outcome:
        ability = corpus.abilities.get(effect.ability_id)
        why = conditional_reason(effect, ability) or scope_reason(effect, ability)
        if why:
            return Rejected(effect, why)
        out = parse_damage(effect)
        tips = tooltip_clauses(ability)
        if isinstance(out, Generated) and tips and dict(out_pools(out)) not in [t for t, _ in tips]:
            out.notes.append(f'the ability tooltip says "{tips[0][1]}"; the effect row is what executes')
        return out


def out_pools(g: Generated) -> List[Tuple[str, int]]:
    return [({"FocusDamage": "F", "HealthDamage": "H"}[n], int(v)) for n, v in g.nvps]


def tooltip_clauses(ability: Optional[Ability]) -> List[Tuple[dict, str]]:
    """The ability tooltip's amount clauses (zeros dropped, one-pool lines
    merged with the next as the effect grammar does), for the cross-check
    note. Lines the grammar refuses are skipped."""
    out: List[Tuple[dict, str]] = []
    cur: dict = {}
    cur_text: List[str] = []
    for ln in desc_lines(ability.description) if ability else []:
        try:
            c = parse_amounts(ln)
        except ValueError:
            c = None
        if not c or any(p in cur for p in c[0]):
            if cur:
                out.append((cur, " / ".join(cur_text)))
            cur, cur_text = {}, []
        if c:
            cur.update({p: n for p, n in c[0].items() if n})
            cur_text.append(ln)
    if cur:
        out.append((cur, " / ".join(cur_text)))
    return out


def parse_amounts(line: str) -> Optional[Tuple[dict, Optional[str], str]]:
    """``({"F": n, "H": n}, suffix, line)`` when the line is an amount clause,
    else None. Raises ValueError for an amount clause it must refuse."""
    body = AMOUNT_LABEL.sub("", line, count=1)
    amounts = list(AMOUNT.finditer(body))
    if not amounts:
        return None
    pools: dict = {}
    for m in amounts:
        pool = (m.group("p1") or m.group("p2") or m.group("p3"))[0].upper()
        n = int(m.group("n1") or m.group("n2") or m.group("n3"))
        if pool in pools:
            raise ValueError(f'grammar: two {pool} amounts in "{line}"')
        pools[pool] = n
    rest = AMOUNT.sub(" ", body)
    if UNSIGNED.search(rest):
        raise ValueError(f'grammar: an amount with no minus sign in "{line}"')
    rest = re.sub(r"[/\s]+", " ", rest).strip()
    if rest and not AMOUNT_SUFFIX.match(rest):
        raise ValueError(f'grammar: unrecognised "{rest}" after the amounts in "{line}"')
    return pools, (rest or None), line


def parse_damage(effect: Effect) -> Outcome:
    """The grammar proper, apart from the conditional and scope rules."""
    clauses = []
    shapes: List[str] = []
    stated_ticks: Optional[int] = None
    interval: Optional[float] = None
    per = False
    notes: List[str] = []

    def ticks(n: Optional[str]) -> Optional[str]:
        nonlocal stated_ticks
        if n is None:
            return None
        if stated_ticks is not None and stated_ticks != int(n):
            return f"pulse shape: the text states {stated_ticks} and {n} ticks"
        stated_ticks = int(n)
        return None

    for ln in desc_lines(effect.desc):
        shape_hit = next(((rx, s) for rx, s in SHAPE_LINES if rx.match(ln)), None)
        if shape_hit:
            if shape_hit[1]:
                shapes.append(shape_hit[1])
            continue
        m = CHANNEL_LINE.match(ln)
        if m:
            shapes.append("single" if m.group("shape").lower() == "single target" else "cone")
            err = ticks(m.group("n"))
            if err:
                return Rejected(effect, err)
            if m.group("i"):
                interval = float(m.group("i"))
            per = True
            continue
        m = TICKS_LINE.match(ln)
        if m:
            err = ticks(m.group("n"))
            if err:
                return Rejected(effect, err)
            if m.group("i"):
                interval = float(m.group("i"))
            continue
        m = INTERVAL_LINE.match(ln)
        if m:
            interval = float(m.group("i"))
            continue
        if COST_LINE.match(ln):
            notes.append(f'"{ln}" is a cost; no ability cost is modelled (B-04)')
            continue
        if NOT_MODELLED_LINE.match(ln):
            notes.append(f'"{ln}" is not modelled')
            continue
        try:
            clause = parse_amounts(ln)
        except ValueError as exc:
            return Rejected(effect, str(exc))
        if clause is None:
            return Rejected(effect, f'grammar: unrecognised line "{ln}"')
        pools, suffix, _ = clause
        if suffix:
            s = AMOUNT_SUFFIX.match(suffix)
            if s.group("per"):
                per = True
            err = ticks(s.group("n1") or s.group("n2") or s.group("n3"))
            if err:
                return Rejected(effect, err)
        clauses.append(clause)

    # One clause, or one amount per line ("-100F" then "-10H"): never one
    # pool twice.
    if not clauses:
        return Rejected(effect, "grammar: no amount clause")
    pools: dict = {}
    for c_pools, _, ln in clauses:
        for p, n in c_pools.items():
            if p in pools:
                return Rejected(effect, f'grammar: two {p} amounts ("{ln}" and an earlier line)')
            pools[p] = n
    source = " / ".join(ln for _, _, ln in clauses)

    want = TCM_SHAPE[effect.tcm]
    for s in shapes:
        if s == "secondary" and want == "single":
            return Rejected(effect, "targeting: the text says secondary targets but the row is TCM_Single")
        if s != "secondary" and s != want:
            return Rejected(effect, f"targeting: the text says {s} but the row is {effect.tcm}")

    pc, pd = effect.pulse_count, effect.pulse_duration
    if stated_ticks is not None and stated_ticks != pc:
        return Rejected(effect, f"pulse shape: the text says {stated_ticks} ticks but pulse_count is {pc}")
    if interval is not None and abs(interval - pd) > 1e-6:
        return Rejected(effect, f"pulse shape: a tick every {interval:g} s but pulse_duration is {pd:g}")
    if pc > 1 and pd <= 0:
        return Rejected(effect, f"pulse shape: pulse_count {pc} with pulse_duration 0 never pulses")
    if pc > 1 and stated_ticks is None and not per:
        return Rejected(effect, f"pulse shape: pulse_count {pc} but the text states no tick count")
    if pc > 1:
        notes.append(f"per tick, {pc} ticks of {pd:g} s")
    elif pc == 0:
        notes.append(f"channelled (pulse_count 0): per pulse of {pd:g} s")
    elif per:
        notes.append("per tick on a single-shot row: one application deals one tick (channel ticks are not modelled)")

    nvps = [(name, str(pools[p])) for p, name in (("F", "FocusDamage"), ("H", "HealthDamage")) if pools.get(p)]
    if not nvps:
        return Rejected(effect, f'grammar: every amount in "{source}" is zero')
    if effect.script_name:
        notes.append(f"script_name {effect.script_name} is left as it is")
    return Generated(effect, nvps, None, source, notes)
