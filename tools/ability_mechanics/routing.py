"""Where the server lands an effect (AB-07), for the families' scope rules.

This mirrors ``route_effect`` in
``crates/cell-combat/src/cell/abilities/effect_routing/mod.rs``, which is
the authority; a family binds an effect only where the cast will land it
where its text says. The rules, in order:

1. ``EF_ResolveOnAbilityUser`` and no damage: the user.
2. A ``TCM_Single`` effect of a Self ability that has no area effect, and no
   damage: the user (a Self ability targets its user).
3. A beneficial ``TCM_AERadius`` effect of a non-ground ability: the caster's
   allies in its radius ("beneficial" here is the effect's own
   ``EF_Beneficial_Effect`` bit or a heal script; the server also counts a
   beneficial cast, which this model leaves out to stay conservative).
4. Anything else: the cast's target (``TCM_Group``/``TCM_Aura`` included,
   until D-AB12).
"""

from __future__ import annotations

import re
from typing import Optional

from corpus import Ability, Corpus, Effect

USER = "user"
ALLY_AREA = "ally_area"
TARGET = "target"

EF_BENEFICIAL_EFFECT = 1
EF_RESOLVE_ON_ABILITY_USER = 131072
TARGET_SELF = 1
TARGET_GROUND = 3
AREA_TCMS = ("TCM_AERadius", "TCM_AECone")
HEAL_SCRIPTS = ("HealHealth", "HealFocus", "HealPetHealth")
DAMAGE_SCRIPTS = ("RangedPhysicalDamage", "MeleePhysicalDamage", "RangedEnergyDamage", "MeleeDamage")
DAMAGE_TEXT = re.compile(r"-\d+ ?F\b|\bF ?-\d+|-\d+ ?H\b", re.I)


def is_ground(ability: Optional[Ability]) -> bool:
    return ability is not None and ability.target_type_id == TARGET_GROUND


def has_area_effect(ability_id: int, corpus: Corpus) -> bool:
    return any(o.ability_id == ability_id and o.tcm in AREA_TCMS for o in corpus.effects.values())


def deals_damage(effect: Effect, corpus: Corpus) -> bool:
    """A damage NVP outside the generated blocks, a damage script, or damage
    text (which the ``damage`` family turns into NVPs)."""
    if effect.script_name in DAMAGE_SCRIPTS:
        return True
    if any(n in ("HealthDamage", "FocusDamage") for n, _ in corpus.hand_nvps.get(effect.effect_id, [])):
        return True
    return bool(DAMAGE_TEXT.search(effect.desc))


def route(effect: Effect, ability: Optional[Ability], corpus: Corpus, script: Optional[str] = None) -> str:
    """Where the server lands ``effect`` once it runs ``script`` (the family's
    script, or the effect's own when None)."""
    script = script if script is not None else effect.script_name
    self_ability = ability is not None and ability.target_type_id == TARGET_SELF
    user_flag = bool(effect.flags & EF_RESOLVE_ON_ABILITY_USER)
    pure_self_single = (
        self_ability and effect.tcm == "TCM_Single" and not has_area_effect(effect.ability_id, corpus)
    )
    if (user_flag or pure_self_single) and not deals_damage(effect, corpus):
        return USER
    beneficial = bool(effect.flags & EF_BENEFICIAL_EFFECT) or script in HEAL_SCRIPTS
    if effect.tcm == "TCM_AERadius" and not is_ground(ability) and beneficial:
        return ALLY_AREA
    return TARGET
