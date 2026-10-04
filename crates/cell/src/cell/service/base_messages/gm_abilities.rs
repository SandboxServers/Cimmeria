//! `BaseToCellMsg::GmAbilitiesChanged`: the cell half of
//! `gmGiveAllAbilities` (154) and `gmResetAbilities` (153), AB-N2.
//!
//! The base has written the row. The cell mirrors it and sends one burst,
//! whatever the number of ids:
//!
//! 1. the interrupt frames of a warming cast of a removed ability (AT-10);
//! 2. `onKnownAbilitiesUpdate`, once: the Ability window and the hotbar's
//!    known list;
//! 3. for a reset, `onEntityProperty(TrainingPoints)` with the refund;
//! 4. the `onTrainerOpen` re-send while a trainer is pinned;
//! 5. the GM's result line.
//!
//! Passives follow the known set: a removed passive's effect comes off, an
//! added one holds at once (pets PT-08), as for a respec and a GM grant.

use tokio::sync::mpsc;

use crate::cell::console::gm::feedback::send_gm_feedback;
use crate::cell::effects::passives::PassiveChange;
use crate::cell::messages::{CellToBaseMsg, GmAbilitiesChanged, GmAbilityChange};
use crate::cell::space_manager::SpaceManager;

use super::ability_granted::{resend_trainer_if_pinned, send_training_points};
use super::passive_sync::apply_passives_and_sync;
use super::player_init::send_known_abilities_update;

/// Handle `BaseToCellMsg::GmAbilitiesChanged`.
pub(super) async fn handle_gm_abilities_changed(
    changed: GmAbilitiesChanged,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let GmAbilitiesChanged {
        entity_id,
        player_id,
        change,
        added,
        removed,
        training_points,
    } = changed;
    let cmd = change.command();
    // The base checked the session before its write, not before this reply.
    let current = space_mgr.get_entity(entity_id).and_then(|e| e.player_id);
    if current != Some(player_id) {
        tracing::warn!(
            target: "abilities",
            event = "gm_ability_bulk_mirror",
            decision_outcome = "ignored",
            reason = "player_mismatch",
            entity_id,
            player_id,
            current_player_id = ?current,
            cmd,
            "GmAbilitiesChanged: entity no longer plays the changed character"
        );
        return;
    }
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        return; // checked above
    };
    for &id in &removed {
        entity.abilities.remove_ability(id);
    }
    for &id in &added {
        entity.abilities.add_ability(id);
    }
    if change == GmAbilityChange::Reset {
        let progress = &mut entity.tree_progress;
        progress.trained_abilities.clear();
        progress.tree_points_spent = 0;
        progress.training_points = training_points;
    }
    let regranted = reconcile_weapon_grants(entity_id, space_mgr);
    apply_passives_and_sync(entity_id, &removed, PassiveChange::Unlearned, tx, space_mgr).await;
    apply_passives_and_sync(entity_id, &added, PassiveChange::Learned, tx, space_mgr).await;
    let interrupted =
        crate::cell::abilities::interrupt_unlearned_cast(entity_id, &removed, tx, space_mgr).await;

    let identity = space_mgr.player_identity(entity_id);
    tracing::info!(
        target: "abilities",
        event = "gm_ability_bulk_mirror",
        decision_outcome = "applied",
        entity_id,
        account_id = identity.account_id,
        player_id,
        cmd,
        added = added.len(),
        removed = removed.len(),
        weapon_regranted = ?regranted,
        training_points,
        interrupted_warmup = interrupted,
        "GmAbilitiesChanged: cell mirrored + one hotbar burst"
    );

    send_known_abilities_update(entity_id, tx, space_mgr).await;
    if change == GmAbilityChange::Reset {
        send_training_points(entity_id, training_points, tx).await;
    }
    resend_trainer_if_pinned(entity_id, training_points, tx, space_mgr).await;
    let text = match change {
        GmAbilityChange::GrantAll => format!(
            "{cmd}: granted {} abilities from your tree (saved; no points spent)",
            added.len()
        ),
        GmAbilityChange::Reset => format!(
            "{cmd}: back to your starter abilities; removed {}, restored {}, \
             training points now {training_points}",
            removed.len(),
            added.len()
        ),
    };
    send_gm_feedback(entity_id, &text, tx).await;
}

/// Re-run the active weapon's grant against the post-change known set.
///
/// The row can hold an ability the equipped weapon also grants (579, the
/// pistol shot, trained or persisted before the pistol went in the slot).
/// The weapon then never tagged it, so a reset that removes it from the row
/// would leave the player unable to fire the weapon until the next slot
/// change. Swapping in the slot's own set adds back, and tags, every weapon
/// ability the change took away. Returns those ids.
fn reconcile_weapon_grants(entity_id: u32, space_mgr: &mut SpaceManager) -> Vec<i32> {
    let Some(slot) = space_mgr
        .get_entity(entity_id)
        .map(|e| e.active_bandolier_slot)
    else {
        return Vec::new();
    };
    let set = cimmeria_cell_combat::cell::cell_methods::inventory::bandolier::weapon_ability_set(
        space_mgr, entity_id, slot,
    );
    match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e.abilities.swap_weapon_granted_abilities(set).1,
        None => Vec::new(),
    }
}
