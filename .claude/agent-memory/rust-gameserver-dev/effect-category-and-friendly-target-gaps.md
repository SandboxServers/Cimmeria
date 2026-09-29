---
name: effect-category-and-friendly-target-gaps
description: No effect-category data exists (EFFECT_* monikers unseeded, EffectDef has no name) so cleanses key on an EffectCategory NVP; players cannot target allies (#444), so beneficial ammo only lands on hostiles; unknown 91xx effect ids must not reach the client
metadata:
  type: project
---

Facts found while building AM-11c (support darts, 2026-09-28):

- **Effect categories.** 2009 cleanses say "Remove 1 Effect of Moniker EFFECT_Poison", but no
  `EFFECT_*` moniker is seeded and `EffectDef` carries no name. The convention is an
  `effect_nvps` row `EffectCategory` = Poison/Disease/Contagion/Wound/Burning on the removable
  effect. `RemoveEffects` (`cell/effects/ammo_dart_support/`) reads `RemoveCategories`.
- **No friendly single-target path.** `use_ability`'s #444 gate (`player_may_attack`) only
  admits hostile NPCs and duel opponents, and it refuses allies and self. `damage_apply`
  re-checks players at apply time. Beneficial on-hit effects therefore only land on hostiles.
- **`ammo_modifiers` CHECK forbids `damage_mult = 0`**. Support darts use 0.0001, which rounds
  any shot under 5000 pre-armour damage to 0.
- **Client risk:** a `StatBuff` or pulsing effect sends `onTimerUpdate` with its effect id.
  NPC targets fan the update out to witnesses. New ids (91xx) are not in cooked data, and an
  unknown cooked id has crashed the client before (#938). Prefer server-only on-hit scripts
  (heals, cleanses).
- **Tests outside owned files:** a new `crates/<crate>/tests/*.rs` integration test needs no
  `mod` line. Drive `handle_use_ability` there, but set `set_weapon_holstered(false)` first
  (holstered queues the attack and returns false) and call `clear_all_cooldowns()` between
  shots (a cooldown of 0 means 0.5 s).
