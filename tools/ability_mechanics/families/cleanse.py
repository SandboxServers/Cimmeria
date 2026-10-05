"""The ``cleanse`` family (AB-10): purges, and the categories they remove.

Two kinds of row, one block:

* **A purge** ("Purge Mental Effects: X 2", "Purges Mental States x5",
  "Purge: Mental Effects x5") binds ``RemoveEffects``
  (``crates/cell-effect-scripts/src/cell/effects/cleanse/``) with
  ``RemoveCategories`` (``Mental:2``) and ``RemovePolarity`` ``Harmful``.
  The script takes exactly that many effects of the category off the caster
  or an ally, never a beneficial one.
* **A category tag**: ``EffectCategory`` on every persisting harmful effect
  a resist roll gates. The categories are the client data's own:
  ``alias.xml`` defines ``kineticRes`` / ``mentalRes`` / ``healthRes`` as
  "resistance to all harmful kinetic / mental / health effects", and every
  "<Kind> Resist Roll" effect shares an ``effect_sequence`` step with the
  effects it gates (combat-formulas-status.md §4). So Suppression gated by a
  Mental Resist Roll is tagged ``Mental``. Only effects that last
  (``pulse_duration > 0``) are tagged: an instant hit leaves nothing to
  remove.

Reported, by category:

* ``count``: a purge with no number ("Purge: Kinetic Effects"); the text
  does not say one or all;
* ``category``: a category the client data does not define ("Focus
  Degeneration", "Focus Buff");
* ``moniker``: removal by an ``EFFECT_*`` / ``DOT`` moniker (stance
  exclusivity is AB-08's; DoT refreshes remove their own earlier
  application; the monikers were never seeded);
* ``routing``: the cast would land the purge on a hostile;
* ``scope``: mini-game cleanup, not a combat effect;
* ``tag``: a gated effect two resist kinds claim.

Every row is RECONSTRUCTION: read from the effect table's own text and
structure.
"""

from __future__ import annotations

import re
from typing import Dict, List, Optional, Set

from corpus import Corpus, Effect
from family import Family, Generated, Outcome, Rejected, desc_lines
from families.cast_routing import EF_BENEFICIAL_EFFECT, hostile_half

SCRIPT = "RemoveEffects"

# The NVP names this family writes.
NVP_NAMES = ("RemoveCategories", "RemovePolarity", "EffectCategory")

KINDS = ("Mental", "Health", "Kinetic")
KIND_RE = "(" + "|".join(KINDS) + ")"

ROLL = re.compile(r"\b" + KIND_RE + r"\s+resist(?:ance)?\s+roll\b", re.I)
PURGE = re.compile(
    r"^purges?:? (?:target )?" + KIND_RE + r" (?:effects?|states?|debuffs?)(?::)?(?: ?x ?(\d+))?$",
    re.I,
)
REMOVAL = re.compile(r"\b(purges?|remove[sd]?|removal|cleanse)\b", re.I)
MONIKER = re.compile(
    r"\bmoniker\b|\bEFFECT_\w+|\bDOT\b|\bstance\b|\bDCHAIN|\bCHAIN_|\bTURRET_|\bcurrent (?:shield effect|buff)\b",
    re.I,
)
MINIGAME = re.compile(r"\bmini-?game\b", re.I)
TARGETING_LINE = re.compile(r"^(?:user )?(single ?target|target|user|self)$", re.I)


def rolls_of(corpus: Corpus) -> Dict[int, Dict[int, Set[str]]]:
    """ability id -> sequence step -> the resist kinds rolled at that step."""
    out: Dict[int, Dict[int, Set[str]]] = {}
    for e in corpus.effects.values():
        m = ROLL.search(e.desc + " " + e.name)
        if m:
            out.setdefault(e.ability_id, {}).setdefault(e.effect_sequence, set()).add(m.group(1).title())
    return out


class CleanseFamily(Family):
    name = "cleanse"
    nvp_names = frozenset(NVP_NAMES)
    scripts = frozenset({SCRIPT})
    reason_categories = ("count", "category", "moniker", "routing", "scope", "tag", "grammar")

    def __init__(self) -> None:
        self._rolls: Optional[Dict[int, Dict[int, Set[str]]]] = None
        self._corpus_id: Optional[int] = None

    def gate_kinds(self, effect: Effect, corpus: Corpus) -> Set[str]:
        """The resist kinds that gate ``effect``'s step, if it is not a roll."""
        if self._corpus_id != id(corpus):
            self._rolls, self._corpus_id = rolls_of(corpus), id(corpus)
        if ROLL.search(effect.desc + " " + effect.name):
            return set()
        return set((self._rolls or {}).get(effect.ability_id, {}).get(effect.effect_sequence, set()))

    def is_gated(self, effect: Effect, corpus: Corpus) -> bool:
        return (
            effect.pulse_duration > 0
            and not effect.flags & EF_BENEFICIAL_EFFECT
            and not REMOVAL.search(effect.desc)
            and bool(self.gate_kinds(effect, corpus))
        )

    def is_candidate(self, effect: Effect, corpus: Corpus) -> bool:
        return bool(REMOVAL.search(effect.desc)) or self.is_gated(effect, corpus)

    def parse(self, effect: Effect, corpus: Corpus) -> Outcome:
        if REMOVAL.search(effect.desc):
            return self.parse_purge(effect, corpus)
        kinds = self.gate_kinds(effect, corpus)
        if len(kinds) != 1:
            return Rejected(effect, f"tag: resist rolls of {sorted(kinds)} share its step")
        kind = kinds.pop()
        return Generated(
            effect,
            [("EffectCategory", kind)],
            None,
            f"{kind} Resist Roll at step {effect.effect_sequence}",
            [f"gated by a {kind} Resist Roll at effect_sequence {effect.effect_sequence}: a harmful {kind.lower()} effect (alias.xml {kind.lower()}Res)"],
        )

    def parse_purge(self, effect: Effect, corpus: Corpus) -> Outcome:
        lines = [ln for ln in desc_lines(effect.desc) if not TARGETING_LINE.match(ln)]
        text = " / ".join(lines)
        if MINIGAME.search(effect.desc):
            return Rejected(effect, "scope: mini-game cleanup, not a combat effect")
        slots: List[str] = []
        for ln in lines:
            m = PURGE.match(ln)
            if m:
                if not m.group(2):
                    return Rejected(effect, f'count: "{ln}" names no number (one, or all?)')
                n = int(m.group(2))
                if not 1 <= n <= 100:
                    return Rejected(effect, f'count: "{ln}" names {n}')
                slots.append(f"{m.group(1).title()}:{n}")
                continue
            if MONIKER.search(ln):
                return Rejected(
                    effect,
                    f'moniker: "{ln}" removes by an EFFECT_/DOT/stance moniker (AB-08 stances, DoT refresh; never seeded)',
                )
            if re.match(r"^purges?\b", ln, re.I) or re.search(r"\bremoves? \d+ ", ln, re.I):
                return Rejected(effect, f'category: "{ln}" names a category the client data does not define')
            return Rejected(effect, f'grammar: unrecognised line "{ln}"')
        if not slots:
            return Rejected(effect, f'grammar: no purge line in "{text}"')
        why = hostile_half(effect, corpus)
        if why:
            return Rejected(effect, "routing: " + why)
        return Generated(
            effect,
            [("RemoveCategories", ",".join(slots)), ("RemovePolarity", "Harmful")],
            SCRIPT,
            text,
            [
                "harmful effects only, from the caster or an ally: the resist kinds are 'harmful' effects in alias.xml",
            ],
        )
