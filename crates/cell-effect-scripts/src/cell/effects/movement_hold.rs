//! The Prison Boot's effect scripts (MG-63b): [`MovementHold`] and
//! [`MovementRelease`].
//!
//! Chain 1023 casts ability 1597 on the player while mission 689 is open. Its
//! effect 1939 runs [`MovementHold`], which puts a movement-only ledger entry
//! on the wearer: it holds `BSF_MovementLock` for movement and nothing else,
//! so the wearer can still use item 3438 to start the escape minigame (the
//! incapacitated gates skip movement-only entries). Effect 1942, cast by chain
//! 1025 on victory, runs [`MovementRelease`], which removes the entry named by
//! its `ReleasesEffect` param (1939).
//!
//! The hold is a counted reference like a stun's: a minigame hold taken and
//! released while the boot is on leaves the lock set until the boot comes
//! off. Death leaves the entry (effect 1939 has no `EF_ClearOnDeath`), and a
//! respawn's hard reset keeps its hold, so the wearer stays held until the
//! minigame's release.
//!
//! Log target `abilities`.

use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_wire::state_field::BSF_MOVEMENT_LOCK;

use super::stat_buff::StatBuffRemoval;
use super::{EffectContext, EffectScript};

/// The effect param naming the ledger entry a release removes.
pub const RELEASES_EFFECT_NVP: &str = "ReleasesEffect";

/// Hold the target for movement only, until its entry is removed.
pub struct MovementHold;

/// Remove the movement-only entry that [`RELEASES_EFFECT_NVP`] names.
pub struct MovementRelease;

impl EffectScript for MovementHold {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let effect = ctx.effect;
        let spec = TimedEffectSpec {
            effect_id: effect.effect_id,
            ability_id: effect.ability_id,
            invoker_id: ctx.source_id,
            effect_flags: effect.flags,
            moniker_ids: ctx.space_mgr.ability_moniker_ids(effect.ability_id),
            state_flags: BSF_MOVEMENT_LOCK,
            movement_only: true,
            duration_secs: None,
            stacking: TimedStacking::PerEffect,
            ..Default::default()
        };
        if ctx
            .space_mgr
            .apply_timed_effect(ctx.target_id, spec, std::time::Instant::now())
            .is_none()
        {
            // The ledger logged why (target gone, or dead and clear-on-death).
            return;
        }
        let row = ctx.row_ids();
        tracing::info!(
            target: "abilities",
            event = "movement_hold_applied",
            account_id = row.account_id,
            account_name = row.account_name,
            player_id = row.player_id,
            player_name = row.player_name,
            target_player_id = ctx.space_mgr.player_identity(ctx.target_id).player_id,
            target_player_name = ctx.space_mgr.player_identity(ctx.target_id).player_name,
            cast_id = row.cast_id, // nt:id-only per-cast sequence number, no name exists
            entity_id = ctx.source_id,
            entity_name = ctx.space_mgr.caster_label(ctx.source_id),
            target_id = ctx.target_id,
            target_name = ctx.space_mgr.entity_label(ctx.target_id),
            effect_id = effect.effect_id,
            effect_name = cimmeria_names::book().effect(effect.effect_id),
            ability_id = effect.ability_id,
            ability_name = cimmeria_names::book().ability(effect.ability_id),
            "MovementHold: the target is held in place until its release"
        );
    }
}

impl EffectScript for MovementRelease {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let effect = ctx.effect;
        let Some(released) = effect
            .params
            .get(RELEASES_EFFECT_NVP)
            .and_then(|v| v.parse::<i32>().ok())
        else {
            tracing::warn!(
                target: "abilities",
                event = "movement_release_refused",
                cast_id = ctx.space_mgr.current_cast_id(), // nt:id-only per-cast sequence number, no name exists
                account_id = ctx.row_ids().account_id,
                account_name = ctx.row_ids().account_name,
                player_id = ctx.row_ids().player_id,
                player_name = ctx.row_ids().player_name,
                target_player_id = ctx.space_mgr.player_identity(ctx.target_id).player_id,
                target_player_name = ctx.space_mgr.player_identity(ctx.target_id).player_name,
                reason = "missing_releases_effect",
                entity_id = ctx.source_id,
                entity_name = ctx.space_mgr.caster_label(ctx.source_id),
                target_id = ctx.target_id,
                target_name = ctx.space_mgr.entity_label(ctx.target_id),
                effect_id = effect.effect_id,
                effect_name = cimmeria_names::book().effect(effect.effect_id),
                ability_id = effect.ability_id,
                ability_name = cimmeria_names::book().ability(effect.ability_id),
                "MovementRelease: the effect names no ReleasesEffect -- nothing released"
            );
            return;
        };
        let removed =
            ctx.space_mgr
                .remove_timed_effects(ctx.target_id, StatBuffRemoval::Removed, |e| {
                    e.effect_id == released && e.movement_only
                });
        if removed.is_empty() {
            crate::cell::effects::script_rows::on_remove_found_nothing(ctx, "movement_hold");
        }
    }
}
