"""Whether a cast lands a helping effect on the caster or an ally (AB-10).

The cell routes a cast by ``ability_is_beneficial``
(``crates/entity/src/abilities/beneficial.rs``): it is beneficial when every
effect that does something carries ``EF_Beneficial_Effect`` or runs a script
that only helps (a heal on a Heal-typed ability, ``AbsorbShield``, a harmful
``RemoveEffects``). A beneficial cast lands on the caster or an ally; any
other cast lands on the client's target, possibly a hostile. So a shield or a
purge is only safe to bind when no other effect of its ability does
something on the hostile path.
"""

from __future__ import annotations

import re
from typing import Optional

from corpus import Corpus, Effect

EF_BENEFICIAL_EFFECT = 1

# Scripts that only ever help their target, as the Rust rule counts them.
HELPING_SCRIPTS = frozenset({"AbsorbShield", "RemoveEffects"})

DAMAGE_TEXT = re.compile(r"-\s?\d+ ?F\b|\bF ?-\d+|-\s?\d+ ?H\b", re.I)


def hostile_half(effect: Effect, corpus: Corpus) -> Optional[str]:
    """The reason the cast would take the hostile path, or None.

    Another effect of the same ability that is not beneficial and does
    something (a bound script other than a helping one, a damage NVP, or
    damage text a generator will bind) makes the ability non-beneficial.
    """
    for other in sorted(corpus.effects.values(), key=lambda e: e.effect_id):
        if other.ability_id != effect.ability_id or other.effect_id == effect.effect_id:
            continue
        if other.flags & EF_BENEFICIAL_EFFECT:
            continue
        if other.script_name in HELPING_SCRIPTS:
            continue
        if other.script_name is not None:
            return (
                f"effect {other.effect_id} of the same ability runs {other.script_name} and is not "
                "beneficial, so the cast takes the hostile path and would land this on the target"
            )
        if any(n in ("HealthDamage", "FocusDamage") for n, _ in corpus.hand_nvps.get(other.effect_id, [])):
            return f"effect {other.effect_id} of the same ability deals damage, so the cast takes the hostile path"
        if DAMAGE_TEXT.search(other.desc):
            return f"effect {other.effect_id} of the same ability states damage, so the cast takes the hostile path"
    return None
