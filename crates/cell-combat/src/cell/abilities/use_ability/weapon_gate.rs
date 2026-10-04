//! The launch gates that apply to weapon attacks only: the
//! attack-while-holstered queue, the one-queued-shot rule and the bandolier
//! slot-swap lockout. Lifted out of `handle.rs` (AB-12) unchanged.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::AbilityDef;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::metrics::{self, CastOutcome, RefusalReason};

/// `true` when a weapon-attack gate holds the cast back: the caller returns
/// `false` with no cooldown charged. Non-weapon abilities (heals, buffs,
/// self-casts) always pass.
pub(super) async fn hold_weapon_attack(
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    ability_def: Option<&AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // Attack-while-holstered queue: when the player presses fire while
    // the weapon is holstered, defer the ability dispatch until the
    // draw animation has had time to play. Mirrors the
    // reload-while-holstered Phase A — draw the weapon, fire
    // `Item_Equip`, stash the ability + target, and let
    // `pending_attack_tick` re-invoke `handle_use_ability` after
    // `UNHOLSTER_DRAW_DURATION`.
    //
    // Only weapon attacks (`required_ammo > 0`) gate on this queue.
    // Non-weapon abilities (heals, buffs, self-casts) bypass entirely
    // — they don't need the weapon drawn to function, and they
    // shouldn't be locked out while a queued weapon shot is mid-draw.
    //
    // Subsequent weapon-attack presses during the draw window are
    // rejected so the first press locks in the queue. Ammo is NOT
    // checked here — the deferred re-invocation runs the normal ammo
    // check at fire time.
    let is_weapon_attack = ability_def.is_some_and(|d| d.required_ammo > 0);
    if !is_weapon_attack {
        return false;
    }
    let who = space_mgr.player_identity(entity_id);
    let queued_attack_already_pending = space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| e.pending_attack_at.is_some());
    if queued_attack_already_pending {
        tracing::debug!(
            target: "abilities",
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id,
            ability_id,
            "useAbility: weapon attack already queued (mid-draw), ignoring input"
        );
        metrics::refused_in(space_mgr, entity_id, RefusalReason::WeaponAttackQueued);
        return true;
    }

    // Block weapon attacks while a bandolier slot swap is in progress.
    // The player's hands are physically holstering the old weapon and
    // drawing the new one; firing through that window would defeat the
    // animation penalty that makes weapon swaps a real loadout choice.
    // Non-weapon abilities (heals, buffs) are still permitted — the
    // queue is about the FIRE pose, not a global ability lockout.
    let slot_swap_in_progress = space_mgr.get_entity(entity_id).is_some_and(|e| {
        e.pending_slot_swap_at
            .is_some_and(|t| std::time::Instant::now() < t)
    });
    if slot_swap_in_progress {
        tracing::debug!(
            target: "abilities",
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id,
            ability_id,
            "useAbility: bandolier slot swap in progress, weapon attack blocked"
        );
        metrics::refused_in(space_mgr, entity_id, RefusalReason::SlotSwapInProgress);
        return true;
    }

    let needs_unholster_queue = space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| e.is_player && e.weapon_holstered && e.threatened_mobs.is_empty());
    if !needs_unholster_queue {
        return false;
    }
    if let Some(e) = space_mgr.get_entity_mut(entity_id) {
        e.set_weapon_holstered(false);
        e.combat_exit_at = Some(std::time::Instant::now());
        e.holster_animation_complete_at = None;
        e.pending_attack_at = Some(
            std::time::Instant::now()
                + super::super::super::cell_methods::player::world::UNHOLSTER_DRAW_DURATION,
        );
        e.pending_attack_ability_id = Some(ability_id);
        e.pending_attack_target_id = Some(target_id);
    }
    tracing::info!(
        target: "abilities",
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id,
        ability_id,
        target_id,
        "useAbility: holstered → queueing attack, drawing weapon first"
    );
    metrics::cast_in(space_mgr, entity_id, CastOutcome::Held);
    super::super::messaging::request_appearance_refresh(entity_id, tx, space_mgr).await;
    super::super::super::cell_methods::player::world::fire_item_sequence(
        entity_id,
        super::super::super::spawner::EVENT_ITEM_EQUIP,
        tx,
        space_mgr,
    )
    .await;
    true
}
