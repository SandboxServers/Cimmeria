//! `BaseToCellMsg::ContentAbilitiesGranted`: the cell half of the
//! `grant_ability` content action (Class Start v6, CS-01a).
//!
//! The base has committed the grant. The cell mirrors it, then tells the
//! player, in this order:
//!
//! 1. `onKnownAbilitiesUpdate`, once, when anything was learned: the
//!    Ability window and the hotbar's known list;
//! 2. `onEntityProperty(TrainingPoints)` when a converted trainer purchase
//!    refunded points (OD-CS06);
//! 3. one `You have learned <ability>.` feedback line per newly learned
//!    ability, so a grant never arrives silently (OD-CS04), then one line
//!    per converted purchase saying it is now free;
//! 4. the `onTrainerOpen` re-send while a trainer is pinned, since a
//!    learned prerequisite or new branch credit can open a node.
//!
//! The credited ids join `TreeProgress::credited_grants`, which the spend
//! gate reads. A learned passive holds at once, as for any other grant.

use tokio::sync::mpsc;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::effects::passives::PassiveChange;
use crate::cell::messages::{CellToBaseMsg, ContentAbilitiesGranted};
use crate::cell::space_manager::SpaceManager;

use super::ability_granted::{resend_trainer_if_pinned, send_training_points};
use super::passive_sync::apply_passives_and_sync;
use super::player_init::send_known_abilities_update;

const EVENT: &str = "content_grant_mirror";

/// The line a player reads for one newly learned ability.
pub(super) fn learned_line(ability_name: Option<&str>, ability_id: i32) -> String {
    match ability_name {
        Some(name) => format!("You have learned {name}."),
        None => format!("You have learned a new ability (#{ability_id})."),
    }
}

/// The line for a bought ability the grant made free (OD-CS06).
pub(super) fn converted_line(ability_name: Option<&str>, ability_id: i32) -> String {
    let name = ability_name.map_or_else(|| format!("ability #{ability_id}"), str::to_string);
    format!("{name} is now yours for free; its training points were refunded.")
}

/// Handle `BaseToCellMsg::ContentAbilitiesGranted`.
pub(super) async fn handle_content_abilities_granted(
    granted: ContentAbilitiesGranted,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let ContentAbilitiesGranted {
        entity_id,
        player_id,
        chain_id,
        source_kind,
        learned,
        credited,
        converted,
        training_points,
        tree_points_spent,
    } = granted;
    // The base checked the session before its write, not before this reply.
    let current = space_mgr.get_entity(entity_id).and_then(|e| e.player_id);
    if current != Some(player_id) {
        tracing::warn!(
            target: "abilities",
            event = EVENT,
            decision_outcome = "ignored",
            reason = "player_mismatch",
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            player_id, // nt:id-only the granted character no longer plays this entity
            current_player_id = current,
            current_player_name = space_mgr.player_identity(entity_id).player_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            learned = ?learned,
            "ContentAbilitiesGranted: entity no longer plays the granted character; \
             the abilities show after that character's next login"
        );
        return;
    }
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        return; // checked above
    };
    for &id in &learned {
        entity.abilities.add_ability(id);
    }
    let progress = &mut entity.tree_progress;
    let credit_before = progress.credited_grants.len();
    for &id in &credited {
        if !progress.credited_grants.contains(&id) {
            progress.credited_grants.push(id);
        }
    }
    let credit_added = progress.credited_grants.len() - credit_before;
    // A bought node the grant converted (OD-CS06): no longer trained, its
    // cost refunded. The base's values are the row's, after the commit.
    progress
        .trained_abilities
        .retain(|id| !converted.contains(id));
    let points_changed = progress.training_points != training_points;
    progress.training_points = training_points;
    progress.tree_points_spent = tree_points_spent;

    apply_passives_and_sync(entity_id, &learned, PassiveChange::Learned, tx, space_mgr).await;

    let identity = space_mgr.player_identity(entity_id);
    // Lines and log rows are built before any await: the NameBook guard is
    // not held across one.
    let lines: Vec<(i32, String)> = {
        let book = cimmeria_names::book();
        for &ability_id in &learned {
            tracing::info!(
                target: "abilities",
                event = EVENT,
                decision_outcome = "learned",
                entity_id,
                entity_name = identity.player_name,
                account_id = identity.account_id,
                account_name = identity.account_name,
                player_id,
                player_name = identity.player_name,
                chain_id,
                chain_name = book.chain(chain_id),
                source_kind = source_kind.as_str(),
                ability_id,
                ability_name = book.ability(ability_id),
                "ContentAbilitiesGranted: cell mirrored"
            );
        }
        tracing::debug!(
            target: "abilities",
            event = EVENT,
            decision_outcome = "applied",
            entity_id,
            entity_name = identity.player_name,
            player_id,
            player_name = identity.player_name,
            chain_id,
            chain_name = book.chain(chain_id),
            source_kind = source_kind.as_str(),
            learned = learned.len(),
            credit_added,
            "ContentAbilitiesGranted: known set and branch credit updated"
        );
        let name_of = |ability_id: i32| {
            book.ability(ability_id).or(space_mgr
                .ability_defs
                .get(&ability_id)
                .map(|d| d.name.as_str()))
        };
        learned
            .iter()
            .map(|&id| (id, learned_line(name_of(id), id)))
            .chain(
                converted
                    .iter()
                    .map(|&id| (id, converted_line(name_of(id), id))),
            )
            .collect()
    };

    if !learned.is_empty() {
        send_known_abilities_update(entity_id, "content_ability_granted", tx, space_mgr).await;
    }
    if points_changed {
        send_training_points(entity_id, training_points, tx, space_mgr).await;
    }
    for (ability_id, text) in lines {
        let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, &text);
        if let Err(e) = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_PLAYER_COMMUNICATION,
                args,
            })
            .await
        {
            tracing::warn!(
                target: "abilities",
                event = EVENT,
                decision_outcome = "feedback_send_failed",
                reason = "base_channel_closed",
                entity_id,
                entity_name = identity.player_name,
                player_id,
                player_name = identity.player_name,
                ability_id,
                ability_name = cimmeria_names::book().ability(ability_id),
                error = %e,
                "ContentAbilitiesGranted: learned line not queued"
            );
        }
    }
    if !learned.is_empty() || credit_added > 0 || points_changed {
        resend_trainer_if_pinned(entity_id, training_points, tx, space_mgr).await;
    }
}
