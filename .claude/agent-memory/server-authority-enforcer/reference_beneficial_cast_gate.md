---
name: reference-beneficial-cast-gate
description: AB-01 (PR 1155) beneficial-cast classifier + resolver; Heal type_id is unreliable in the Giza seed (debuff/CC abilities typed Heal) so the classifier is the trust boundary
metadata:
  type: reference
---

AB-01 (reviewed 2026-10-03, PR 1155): `cimmeria_entity::abilities::ability_is_beneficial` decides whether a player cast skips the #444 hostility gate and lands on caster/ally via `use_ability/beneficial.rs::fire_beneficial` (no QR, threat, combat state). Resolver `resolve_cast_target`: Self -> caster; Target -> wire target only if alive and `support_shot::classify == Ally` (self, or non-attackable player in same space); else fallback to caster (D-AB02 `FALLBACK_TO_CASTER`). Warmup tick re-resolves; Ally still takes range/LOS/space checks.

Hazard (FIXED in PR 1155 review, S1): the first version admitted `type_id == Heal` with NO effect check beyond a damage veto (`HealthDamage`/`FocusDamage` NVPs only). The seed types ~200 abilities Heal, including obvious debuffs/CC (1874 Impose Weakness, 1937 FocusDegen, 1988 InduceDaze, 2052 Diminish, 2090 TurnAndDie, 2154 ShutDown, 3253 ConvertEnergy:Enemy, deployables 3399-3402). They have no implemented effects today, so harmless; once someone authors a non-damage debuff/CC effect for one, it lands on the caster or any same-space player (ally grief). Now: at least one effect must do something, and each must carry `EF_Beneficial_Effect` or, on a Heal-typed ability, run a `HEAL_SCRIPTS` heal (HealHealth/HealFocus/HealPetHealth); empty, debuff or damaging Heal abilities are non-beneficial (live-DB guard pins 1874/1937/1988/2090/2154/3253). Re-check if `HEAL_SCRIPTS` grows. See [[reference-combat-exploit-classes]], [[reference-duel-harm-gate]].

Also (fixed, S2): `launch_target` runs before dead/known/cooldown checks, so its `beneficial_cast` row is DEBUG; the fire-stage row is INFO.
