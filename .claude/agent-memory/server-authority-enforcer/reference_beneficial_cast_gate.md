---
name: reference-beneficial-cast-gate
description: AB-01 (PR 1155) beneficial-cast classifier + resolver; Heal type_id is unreliable in the Giza seed (debuff/CC abilities typed Heal) so the classifier is the trust boundary
metadata:
  type: reference
---

AB-01 (reviewed 2026-10-03, PR 1155): `cimmeria_entity::abilities::ability_is_beneficial` decides whether a player cast skips the #444 hostility gate and lands on caster/ally via `use_ability/beneficial.rs::fire_beneficial` (no QR, threat, combat state). Resolver `resolve_cast_target`: Self -> caster; Target -> wire target only if alive and `support_shot::classify == Ally` (self, or non-attackable player in same space); else fallback to caster (D-AB02 `FALLBACK_TO_CASTER`). Warmup tick re-resolves; Ally still takes range/LOS/space checks.

Latent hazard: the rule admits `type_id == Heal` with NO effect check beyond a damage veto (`HealthDamage`/`FocusDamage` NVPs only). The seed types ~200 abilities Heal, including obvious debuffs/CC (1874 Impose Weakness, 1937 FocusDegen, 1988 InduceDaze, 2052 Diminish, 2090 TurnAndDie, 2154 ShutDown, 3253 ConvertEnergy:Enemy, deployables 3399-3402). They have no implemented effects today, so harmless; once someone authors a non-damage debuff/CC effect for one, it lands on the caster or any same-space player (ally grief). Re-check this whenever effects are added to Heal-typed abilities. See [[reference-combat-exploit-classes]], [[reference-duel-harm-gate]].

Also: `launch_target` runs before dead/known/cooldown checks and logs INFO per packet (log amplification on a forged-packet path).
