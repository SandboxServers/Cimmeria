"""Held stat effects for the ``stat`` family (AB-08): toggles, stances and
passives, and the "Remove Effect of moniker EFFECT_Stance" halves.

A held effect has ``pulse_duration = 0``. ``TimedStat`` holds its entry with
no expiry, so the family binds one only when something takes it off again:

* a **toggle**: an ``AF_TOGGLED`` Self ability. The next press takes it off.
  A toggle on another entity (a Target ability) is reported: the off press
  would have to find the old target;
* a **passive**: an ``EF_AlwaysPersist`` effect of a ``passive_yn`` ability.
  ``apply_passives`` applies it at login and purchase and removes it on a
  respec.

A **stance** is a toggle whose ability is named a stance or authors a
"Remove ... EFFECT_Stance" effect. Its held effects get an ``EffectMoniker``
``EFFECT_Stance`` row (RECONSTRUCTION: the seed has no effect-moniker column,
audit B-74), which is what the removal keys on, so a removal never touches a
buff that only shares an ability moniker (1470900795 is on most combat
abilities). The removal effect itself gets ``RemoveMoniker EFFECT_Stance``
and ``RemoveByMoniker``, only when the same ability binds a stance effect:
alone it would make an otherwise empty ability (Reveal) look implemented.
"""

from __future__ import annotations

import re
from typing import Optional

from corpus import Ability, Corpus, Effect
from family import desc_lines

REMOVE_SCRIPT = "RemoveByMoniker"
STANCE_MONIKER = "EFFECT_Stance"
EFFECT_MONIKER_NVP = "EffectMoniker"
REMOVE_MONIKER_NVP = "RemoveMoniker"

AF_TOGGLED = 8
EF_ALWAYS_PERSIST = 524288
TARGET_SELF = 1
# The mini-game ability group: 809 Mental Fortitude's tooltip says its
# effect is "Active during Mini-game State", 815 checks for it, 813 removes
# mini-game abilities. No mini-game state exists on the server.
MINIGAME_MONIKER = 320218562

# "Remove Effect of moniker EFFECT_Stance", "Remove 1 EFFECT_Stance",
# "Remove 1 Effect of EFFECT_Stance", "Remove Moniker EFFECT_Stance".
REMOVE_STANCE = re.compile(r"^remove (?:1 )?(?:effect of )?(?:moniker )?effect_stance$", re.I)
REMOVE_TARGETING = re.compile(r"^(user )?single ?target$", re.I)


def is_stance_removal(effect: Effect) -> bool:
    """Whether the effect's text is only a "Remove ... EFFECT_Stance" line
    (with targeting lines)."""
    lines = desc_lines(effect.desc)
    removes = [ln for ln in lines if REMOVE_STANCE.match(ln)]
    rest = [ln for ln in lines if not REMOVE_STANCE.match(ln)]
    return len(removes) == 1 and all(REMOVE_TARGETING.match(ln) for ln in rest)


def is_toggle(ability: Optional[Ability]) -> bool:
    return ability is not None and bool(ability.flags & AF_TOGGLED)


def is_stance(ability: Optional[Ability], corpus: Corpus) -> bool:
    """A toggle named a stance, or one that authors its stance removal."""
    if not is_toggle(ability):
        return False
    if re.search(r"\bstance\b", ability.name, re.I):
        return True
    return any(
        e.ability_id == ability.ability_id and is_stance_removal(e) for e in corpus.effects.values()
    )


def held_kind(effect: Effect, ability: Optional[Ability]) -> Optional[str]:
    """``"passive"``, ``"toggle"`` or None (not a held effect)."""
    if effect.flags & EF_ALWAYS_PERSIST:
        return "passive"
    if effect.pulse_duration <= 0 and is_toggle(ability):
        return "toggle"
    return None


def held_scope_rejection(effect: Effect, ability: Optional[Ability]) -> Optional[str]:
    """Why a held effect must not be bound, or None. ``effect`` is held or
    persistent; timed effects never reach here."""
    if effect.flags & EF_ALWAYS_PERSIST:
        if effect.pulse_duration > 0:
            return "EF_AlwaysPersist with a duration: neither a passive nor a timed buff"
        if ability is None or not ability.passive:
            return "EF_AlwaysPersist on a castable ability: apply_passives would hold it for every caster (AB-08)"
        if effect.pulse_count not in (0, 1):
            return f"pulse_count {effect.pulse_count}: a passive is one held entry"
        if MINIGAME_MONIKER in ability.moniker_ids:
            return "a mini-game passive: its effect holds only in the mini-game state (809's tooltip), which the server lacks"
        return None
    if effect.pulse_duration > 0:
        if is_toggle(ability):
            return "an AF_TOGGLED ability's timed effect: a toggle holds, it does not count down (AB-08)"
        return None
    if ability is not None and ability.passive and not is_toggle(ability):
        return "a passive ability's effect without EF_AlwaysPersist: apply_passives runs only flagged effects (B-36)"
    if not is_toggle(ability):
        return "held (pulse_duration 0) on a neither toggled nor passive ability: nothing would ever remove it"
    if effect.pulse_count != 1:
        return f"pulse_count {effect.pulse_count}: a toggle is one held entry"
    if ability.target_type_id != TARGET_SELF:
        return "a toggle on another entity (a Target ability): the off press would have to find the old target (AB-08)"
    if re.match(r"shield\b", ability.name, re.I):
        return "a shield toggle: EFFECT_Shield exclusivity is AB-10's"
    return None
