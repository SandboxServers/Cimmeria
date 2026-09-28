//! Owner abilities that act on pets (pets PT-08, issue #570).
//!
//! | Ability | Effect | Script | What it does |
//! |---|---|---|---|
//! | 2824 Holy Warrior (Toggled) | 4220 | [`PetStatBuff`] | pet +100 Accuracy, -100 Defense until toggled off |
//! | 2824 Holy Warrior | 4087 "Stance Removal" | none | removes the owner's other `EFFECT_Stance` effect; no player stance effect exists on this server, so nothing to remove |
//! | 2839 To The Death | 4121 | [`PetStatBuff`] | pet +400 Accuracy for 60 s |
//! | 2839 To The Death | 4119 "Pet Death Timer" | [`PetDeathTimer`] | the pet dies 60 s later |
//! | 2839 To The Death | 4122 "Pet Death" | none | the death itself, carried out by the owner-pet tick when the timer runs out |
//! | 1650 Lord's Concentration | 350 (new, server-only) | [`PetStatBuff`] | pets +50 Interrupt Resistance for 30 s (D-PT17) |
//! | 967 / 968 / 1207 Repair Turret | 3211 / 3230 / 3350 | [`HealPetHealth`] | heals the owner's pet |
//! | 2852 Heed Our Calling (passive) | 4968 | [`PetSummonSpeed`] | owner `speedPet` +100: the next summon is instant (D-PT10) |
//!
//! Every script but [`PetSummonSpeed`] runs on the owner's pet: the cast is
//! redirected in `cimmeria-cell-combat` (`use_ability::owner_pet`), which
//! resolves the pet through the registry with the summon-time identity
//! (`SpaceManager::owner_pet_targets`) and never from the client's target.
//! [`acts_on_owner_pet`] is how that redirect recognises such an ability:
//! by the scripts on its effects, which the seed names.
//!
//! The magnitudes are `effect_nvps` rows (`db/resources/Effects/Seed/
//! effect_nvps.sql`, ids 350-359): the 2009 data shipped none, so each is
//! the number in the effect's own description.
//!
//! Log target `pets.buff`.

use cimmeria_entity::cell_entity::PetState;
use std::time::{Duration, Instant};

use cimmeria_entity::abilities::AF_TOGGLED;
use cimmeria_entity::stats::{ACCURACY, DEFENSE, INTERRUPT_RES, SPEED_PET};

use super::{EffectContext, EffectScript};
use crate::cell::pets::BuffRemoval;

/// `effect_nvps` names [`PetStatBuff`] reads, and the stat each moves.
pub const PET_STAT_NVPS: [(&str, i32); 3] = [
    ("Accuracy", ACCURACY),
    ("Defense", DEFENSE),
    ("InterruptResistance", INTERRUPT_RES),
];

/// Whether an effect with `script` acts on the caster's pet, so its
/// ability's cast must be redirected to the owner's pet.
pub fn acts_on_owner_pet(script: &str) -> bool {
    matches!(script, "PetStatBuff" | "PetDeathTimer" | "HealPetHealth")
}

/// Whether `script` is the effect of a passive ability, applied while the
/// ability is known (`EF_AlwaysPersist`, see `super::passives`).
pub fn is_passive_script(script: &str) -> bool {
    script == "PetSummonSpeed"
}

/// The `(stat, delta)` pairs an effect's NVPs ask for.
pub fn pet_stat_mods(ctx: &EffectContext) -> Vec<(i32, i32)> {
    PET_STAT_NVPS
        .iter()
        .filter_map(|&(name, stat)| match ctx.effect.param_i32(name) {
            0 => None,
            delta => Some((stat, delta)),
        })
        .collect()
}

/// Whether the target is a pet, logging the seed defect when it is not
/// (the redirect only ever hands these scripts a pet).
fn target_is_pet(ctx: &EffectContext, script: &'static str) -> bool {
    if ctx
        .space_mgr
        .get_entity(ctx.target_id)
        .is_some_and(|e| e.extensions.contains::<PetState>())
    {
        return true;
    }
    tracing::warn!(
        target: "pets.buff",
        event = "pet_script_skipped",
        reason = "target_not_a_pet",
        script,
        entity_id = ctx.target_id,
        source_id = ctx.source_id,
        effect_id = ctx.effect.effect_id,
        ability_id = ctx.effect.ability_id,
        "a pet effect script ran on something that is not a pet; nothing applied"
    );
    false
}

// ── PetStatBuff ──────────────────────────────────────────────────────────

/// Stat buff on the owner's pet, from the effect's [`PET_STAT_NVPS`].
///
/// - A **Toggled** ability (`AF_TOGGLED`, 2824 Holy Warrior) switches: the
///   first press applies the buff with no expiry, the next takes it off.
/// - Otherwise the buff lasts the effect's `pulse_duration` (4121: 60 s)
///   and the owner-pet tick takes it off. Applying it again refreshes it.
///
/// The row's `pulse_count = 1` would make it a one-shot in the pulsing
/// layer, which never registers a single pulse; the buff ledger on the pet
/// (`SpaceManager::apply_pet_buff`) carries the duration instead.
pub struct PetStatBuff;

impl EffectScript for PetStatBuff {
    fn on_apply(&self, ctx: &mut EffectContext) {
        if !target_is_pet(ctx, "PetStatBuff") {
            return;
        }
        let mods = pet_stat_mods(ctx);
        let effect_id = ctx.effect.effect_id;
        let ability_id = ctx.effect.ability_id;
        if mods.is_empty() {
            tracing::warn!(
                target: "pets.buff",
                event = "pet_script_skipped",
                reason = "no_stat_nvps",
                script = "PetStatBuff",
                entity_id = ctx.target_id,
                effect_id,
                ability_id,
                "PetStatBuff effect has no stat NVP; nothing applied"
            );
            return;
        }
        let toggled = ctx
            .space_mgr
            .ability_defs
            .get(&ability_id)
            .is_some_and(|d| d.flags & AF_TOGGLED != 0);
        if toggled {
            if ctx
                .space_mgr
                .remove_pet_buff(ctx.target_id, effect_id, BuffRemoval::ToggledOff)
                .is_none()
            {
                ctx.space_mgr
                    .apply_pet_buff(ctx.target_id, effect_id, ability_id, &mods, None);
            }
            return;
        }
        if ctx.effect.pulse_duration <= 0.0 {
            tracing::warn!(
                target: "pets.buff",
                event = "pet_script_skipped",
                reason = "no_duration",
                script = "PetStatBuff",
                entity_id = ctx.target_id,
                effect_id,
                ability_id,
                "PetStatBuff effect is neither toggled nor timed; nothing applied"
            );
            return;
        }
        let expires_at = Instant::now() + Duration::from_secs_f32(ctx.effect.pulse_duration);
        ctx.space_mgr.apply_pet_buff(
            ctx.target_id,
            effect_id,
            ability_id,
            &mods,
            Some(expires_at),
        );
    }

    fn on_remove(&self, ctx: &mut EffectContext) {
        let _ = ctx.space_mgr.remove_pet_buff(
            ctx.target_id,
            ctx.effect.effect_id,
            BuffRemoval::Removed,
        );
    }
}

// ── PetDeathTimer ────────────────────────────────────────────────────────

/// Arms To The Death's doom on the pet: it dies `pulse_duration` seconds
/// from now (4119: 60 s). The death itself (4122) is the owner-pet tick's,
/// through the one death resolver, since a script cannot await it.
pub struct PetDeathTimer;

impl EffectScript for PetDeathTimer {
    fn on_apply(&self, ctx: &mut EffectContext) {
        if !target_is_pet(ctx, "PetDeathTimer") {
            return;
        }
        let secs = ctx.effect.pulse_duration;
        if secs <= 0.0 {
            tracing::warn!(
                target: "pets.buff",
                event = "pet_script_skipped",
                reason = "no_duration",
                script = "PetDeathTimer",
                entity_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                ability_id = ctx.effect.ability_id,
                "PetDeathTimer effect has no duration; the pet is not doomed"
            );
            return;
        }
        let at = Instant::now() + Duration::from_secs_f32(secs);
        let pet = ctx.target_id;
        let Some(state) = ctx
            .space_mgr
            .get_entity_mut(pet)
            .and_then(|e| e.extensions.get_mut::<PetState>())
        else {
            return;
        };
        state.doomed_at = Some(at);
        let owner_id = state.owner_id;
        let id = ctx.space_mgr.pet_summoner_identity(pet);
        tracing::debug!(
            target: "pets.buff",
            event = "doom_armed",
            decision_outcome = "doom_armed",
            entity_id = pet,
            pet_id = pet,
            owner_id,
            account_id = id.account_id,
            player_id = id.player_id,
            ability_id = ctx.effect.ability_id,
            effect_id = ctx.effect.effect_id,
            doom_secs = secs,
            "To The Death armed: the pet dies when the timer runs out"
        );
    }
}

// ── HealPetHealth ────────────────────────────────────────────────────────

/// "Heal Pet: Health": [`super::scripts::HealHealth`] on the owner's pet
/// (`HealPercentage` of its max HEALTH per pulse). A separate name only so
/// the redirect knows the ability heals the pet, not the caster's target.
pub struct HealPetHealth;

impl EffectScript for HealPetHealth {
    fn on_apply(&self, ctx: &mut EffectContext) {
        if !target_is_pet(ctx, "HealPetHealth") {
            return;
        }
        super::scripts::HealHealth.on_apply(ctx);
    }
}

// ── PetSummonSpeed ───────────────────────────────────────────────────────

/// Heed Our Calling's passive (4968, `EF_AlwaysPersist`): raises the
/// owner's `speedPet` (stat 111) by the `SpeedPet` NVP (100), so a
/// `SpeedPet` summon's warmup scales to zero (D-PT10,
/// `use_ability::warmup::effective_warmup`). Applied while the ability is
/// known (`super::passives`).
///
/// Idempotent: it sets the stat to its base plus the bonus, so a repeated
/// grant does not stack. `on_remove` puts it back to the base. Nothing else
/// writes `speedPet` today.
pub struct PetSummonSpeed;

impl PetSummonSpeed {
    fn set(ctx: &mut EffectContext, bonus: i32, applied: bool) {
        let entity_id = ctx.target_id;
        let Some(entity) = ctx.space_mgr.get_entity_mut(entity_id) else {
            return;
        };
        let Some(stat) = entity.stats.get_mut(SPEED_PET) else {
            return;
        };
        let before = stat.cur;
        let value = stat.base_cur.saturating_add(bonus);
        stat.update(stat.min.min(value), value, stat.max.max(value));
        // Server-side only: the summon's warmup is timed by the server and
        // sent as a timer, and no client UI reads `speedPet`. Left clean so
        // it never rides a later dirty-stat flush and no burst changes.
        stat.dirty = false;
        let id = entity.identity();
        tracing::debug!(
            target: "pets.buff",
            event = if applied { "summon_speed_applied" } else { "summon_speed_removed" },
            decision_outcome = if applied { "summon_speed_applied" } else { "summon_speed_removed" },
            entity_id,
            owner_id = entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            ability_id = ctx.effect.ability_id,
            effect_id = ctx.effect.effect_id,
            speed_pet_before = before,
            speed_pet_after = value,
            "passive summon speed on the owner's speedPet"
        );
    }
}

impl EffectScript for PetSummonSpeed {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let bonus = ctx.effect.param_i32("SpeedPet");
        if bonus <= 0 {
            tracing::warn!(
                target: "pets.buff",
                event = "pet_script_skipped",
                reason = "no_speed_nvp",
                script = "PetSummonSpeed",
                entity_id = ctx.target_id,
                effect_id = ctx.effect.effect_id,
                ability_id = ctx.effect.ability_id,
                "PetSummonSpeed effect has no positive SpeedPet NVP; nothing applied"
            );
            return;
        }
        Self::set(ctx, bonus, true);
    }

    fn on_remove(&self, ctx: &mut EffectContext) {
        Self::set(ctx, 0, false);
    }
}
