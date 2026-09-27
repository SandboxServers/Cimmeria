---
name: owner-pet-effects-and-passives
description: PT-08 traps - self casts apply no effects, pulse_count=1 buffs never register, passives need three seams, [0,0] stat bounds clamp buffs away
metadata:
  type: project
---

Learned while wiring pets PT-08 (owner abilities on pets), 2026-09-27.

- `fire_cast` returns before `damage_apply` when `target_id <= 0`, so a Self/no-target ability applies **none** of its effects. Anything that must act without a target needs its own fire branch (summon.rs, owner_pet/fire.rs).
- `register_active_effect` drops `pulse_count = 1` rows even with a long `pulse_duration` (timed buffs like "+400 for 60 s"). Pet buffs use the `PetState::buffs` ledger + `owner_pet_tick` instead.
- No passive-ability support existed. `effects::passives::apply_passives` runs `EF_AlwaysPersist` + passive-script effects; it must be called at every known-set change: `InitPlayerState`, `AbilityGranted`, `AbilitiesReset` (all in `crates/cell/.../base_messages/`). A new grant path (e.g. GM `.giveability`) must call it too.
- `DEFENSE`, `INTERRUPT_RES`, `SPEED_PET` default to `(0,0,0)`: `Stat::change` clamps a buff to nothing. Widen the bound on the entity (`pets::shift_stat`) and record the applied delta.
- Redirecting a cast off the client's target: zero `target_id` at the top of `handle_use_ability` (like summon) so the #444 gate never sees it; resolve the pet with `owner_pet_targets` (summoner-checked).

**Why:** each of these silently no-ops rather than erroring. **How to apply:** check them first when an ability "does nothing" in play. See [[crafting-induction-engine-seams]] for other seam lists.
