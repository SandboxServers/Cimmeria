"""The ``cc`` family (AB-09): stuns, knockdowns, snares and interrupts.

The scripts are in ``crates/cell-effect-scripts/src/cell/effects/``:

* ``Stun`` and ``Knockdown`` (``crowd_control.rs``) put one entry per
  ``(effect, caster)`` on the timed effect ledger holding
  ``BSF_MovementLock`` for ``CcDuration`` seconds. This family writes
  ``CcDuration`` from the text ("Stun: 5 seconds", "Knockdown: / 5 Seconds",
  "4 second Stun").
* A snare is a ``movementSpeedMod`` ledger entry: this family binds
  ``TimedStat`` with ``MovementSpeedMod``. Snares whose text gives a number
  ("Movement Speed-30%") are the ``stat`` family's; the bare "Snare: 15
  Seconds" rows state none, so they get -30, the reduction every snare that
  states one uses (effects 1460, 1765, 3114, 4411). That number is a DESIGN
  default and every row says so.
* ``Interrupt`` breaks the target's warmup and channels: "Interrupts target"
  gets ``InterruptChance`` 100.

A stated duration must equal the row's ``pulse_duration``; with no stated
duration the row's ``pulse_duration`` is the length, and the note says so.

Resist rolls ("Kinetic Resist Roll") are not modelled (D-AB13 waits on the
owner); the CC lands on every hit that is not a miss.

Scope rejections, before the grammar:

* not a single pulse (a disorient over 20 pulses, "High Interrupt Chance"
  over 5): the scripts take one entry per hit;
* ``EF_ClearOnDamage`` ("Lord's Will"): no damage hook removes it (AB-11);
* ``EF_ResolveOnAbilityUser``, or a "Secondary" line: per-effect routing is
  AB-07, and the pipeline would land it on the primary target;
* a beneficial effect, a passive (``EF_AlwaysPersist``) or a toggled
  ability: not crowd control on a hostile;
* a deployable's effect: it needs a ``resources.deployables`` binding;
* a target collection other than ``TCM_Single``, ``TCM_AERadius`` or
  ``TCM_AECone``. The two AE methods are accepted: the pipeline runs every
  effect of the ability on every target the cast's fan-out hits, so an AE
  knockdown lands on each of them.
"""

from __future__ import annotations

import re
from typing import List, Optional, Tuple

from corpus import Corpus, Effect
from family import Family, Generated, Outcome, Rejected, desc_lines

STUN = "Stun"
KNOCKDOWN = "Knockdown"
SNARE = "TimedStat"
INTERRUPT = "Interrupt"

CC_DURATION = "CcDuration"
INTERRUPT_CHANCE = "InterruptChance"
SPEED = "MovementSpeedMod"

# DESIGN: the reduction of every snare whose text states one (D-AB09's
# run-speed unit: movementSpeedMod percent).
DEFAULT_SNARE = -30
SNARE_SOURCES = "1460, 1765, 3114, 4411"

EF_BENEFICIAL_EFFECT = 1
EF_CLEAR_ON_DAMAGE = 8
EF_RESOLVE_ON_ABILITY_USER = 131072
EF_ALWAYS_PERSIST = 524288
AF_TOGGLED = 8

ACCEPTED_TCM = ("TCM_Single", "TCM_AERadius", "TCM_AECone")

KINDS = {"stun": STUN, "knockdown": KNOCKDOWN, "snare": SNARE}
KIND = r"(stun|knockdown|snare)"
SECS = r"(\d+(?:\.\d+)?) ?(?:seconds?|secs?)"

# "Stun: 5 seconds", "Knockdown 3 seconds", "Target Knockdown: 5 Seconds"
KIND_FIRST = re.compile(r"^" + KIND + r":? ?" + SECS + r"\.?$", re.I)
# "4 second Stun", "5 Second Knockdown", "10 second Snare"
SECS_FIRST = re.compile(r"^" + SECS + r" " + KIND + r"$", re.I)
# "Knockdown:" / "Knockdown" alone (its length on the next line, or none)
KIND_ALONE = re.compile(r"^" + KIND + r":?$", re.I)
DURATION_LINE = re.compile(r"^(?:duration:? ?)?" + SECS + r"\.?$", re.I)
INTERRUPT_LINE = re.compile(r"^interrupts target$", re.I)
TARGETING_LINE = re.compile(
    r"^(single target|target|targeted|(short|small|medium|large|long|melee) radius( ae)?|"
    r"(narrow|medium|wide) cone)$",
    re.I,
)
PREFIX = re.compile(r"^target ", re.I)

# Loose detector: anything that names a CC this family knows.
CANDIDATE = re.compile(r"\b(stun|knockdown|snare|interrupts target)\b", re.I)


def parse_cc(effect: Effect) -> Outcome:
    """The grammar proper: one CC clause, optional targeting and duration
    lines."""
    kinds: List[Tuple[str, str]] = []  # (script, the line it came from)
    durations: List[float] = []
    for raw in desc_lines(effect.desc):
        if re.search(r"\bsecondary\b", raw, re.I):
            return Rejected(effect, "scope: a Secondary-target line; secondary routing is AB-07")
        if TARGETING_LINE.match(raw):
            continue
        ln = PREFIX.sub("", raw)
        m = DURATION_LINE.match(ln)
        if m:
            durations.append(float(m.group(1)))
            continue
        if INTERRUPT_LINE.match(ln):
            kinds.append((INTERRUPT, raw))
            continue
        m = KIND_FIRST.match(ln)
        if m:
            kinds.append((KINDS[m.group(1).lower()], raw))
            durations.append(float(m.group(2)))
            continue
        m = SECS_FIRST.match(ln)
        if m:
            kinds.append((KINDS[m.group(2).lower()], raw))
            durations.append(float(m.group(1)))
            continue
        m = KIND_ALONE.match(ln)
        if m:
            kinds.append((KINDS[m.group(1).lower()], raw))
            continue
        return Rejected(effect, f'grammar: unrecognised line "{raw}"')
    if not kinds:
        return Rejected(effect, "grammar: no crowd-control clause")
    if len({k for k, _ in kinds}) > 1 or len(kinds) > 1:
        return Rejected(effect, "grammar: more than one crowd-control clause")
    script, line = kinds[0]
    pd = effect.pulse_duration
    notes: List[str] = []

    if script == INTERRUPT:
        if durations:
            return Rejected(effect, "grammar: an interrupt with a duration")
        notes.append("InterruptChance 100: the text interrupts outright; combat rolls it against interruptRes")
        return Generated(effect, [(INTERRUPT_CHANCE, "100")], INTERRUPT, line, notes)

    for d in durations:
        if pd > 0 and abs(d - pd) > 1e-6:
            return Rejected(effect, f"pulse shape: text says {d:g} s but pulse_duration is {pd:g}")
    if durations:
        secs = durations[0]
        notes.append(f"{secs:g} s from the text, equal to the row's pulse_duration")
    elif pd > 0:
        secs = pd
        notes.append(f"{secs:g} s from the row's pulse_duration: the text states no length")
    else:
        return Rejected(effect, "pulse shape: no duration in the text and pulse_duration 0")
    if pd <= 0:
        return Rejected(effect, "pulse shape: pulse_duration 0, so the ledger entry would have no length")

    if script == SNARE:
        notes.insert(
            0,
            f"DESIGN default: the text states no amount; {DEFAULT_SNARE}% run speed is the reduction of "
            f"every snare that states one ({SNARE_SOURCES}), movementSpeedMod {DEFAULT_SNARE} (D-AB09)",
        )
        return Generated(effect, [(SPEED, str(DEFAULT_SNARE))], SNARE, line, notes)
    secs_text = f"{secs:g}"
    return Generated(effect, [(CC_DURATION, secs_text)], script, line, notes)


def scope_rejection(effect: Effect, corpus: Corpus) -> Optional[str]:
    ability = corpus.abilities.get(effect.ability_id)
    if effect.pulse_count != 1:
        return f"pulse shape: pulse_count {effect.pulse_count}, not one entry per hit"
    if effect.flags & EF_CLEAR_ON_DAMAGE:
        return "scope: EF_ClearOnDamage; no damage hook removes it yet (AB-11)"
    if effect.flags & EF_RESOLVE_ON_ABILITY_USER:
        return "scope: EF_ResolveOnAbilityUser; user routing is AB-07"
    if effect.flags & EF_BENEFICIAL_EFFECT:
        return "scope: a beneficial effect is not crowd control on a hostile"
    if effect.flags & EF_ALWAYS_PERSIST:
        return "scope: EF_AlwaysPersist, a passive (AB-08)"
    if ability and ability.flags & AF_TOGGLED:
        return "scope: an AF_TOGGLED ability (AB-08)"
    if ability and ability.name.startswith("Deployable:"):
        return "scope: deployable effect, needs a resources.deployables binding"
    if effect.tcm not in ACCEPTED_TCM:
        return f"targeting: {effect.tcm}; group and aura routing is D-AB12"
    return None


class CcFamily(Family):
    name = "cc"
    nvp_names = frozenset({CC_DURATION, INTERRUPT_CHANCE, SPEED})
    scripts = frozenset({STUN, KNOCKDOWN, SNARE, INTERRUPT})
    reason_categories = ("scope", "pulse shape", "targeting", "grammar")

    def is_candidate(self, effect: Effect, corpus: Corpus) -> bool:
        return bool(CANDIDATE.search(effect.desc))

    def parse(self, effect: Effect, corpus: Corpus) -> Outcome:
        why = scope_rejection(effect, corpus)
        if why:
            return Rejected(effect, why)
        out = parse_cc(effect)
        if isinstance(out, Generated) and effect.tcm != "TCM_Single":
            out.notes.append(f"{effect.tcm}: lands on every target the cast's fan-out hits")
        return out
