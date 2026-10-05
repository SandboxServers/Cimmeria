//! `Action::ShowTutorial`: the one-time tutorial (Class Start v6, CS-03).
//!
//! Two halves, because the database decides "first time":
//!
//! 1. [`run`] (the action): a player who already has the tutorial in
//!    `CellEntity::shown_tutorials` gets nothing (`already_shown`).
//!    Otherwise the id goes into that set at once, so a second firing in
//!    the same tick and a `tutorial_shown` gate later in the tick both see
//!    it, and `RecordTutorialShown` goes to the base.
//! 2. [`apply_tutorial_recorded`] (the base's answer): the base inserted
//!    `(player_id, tutorial_id)` into `sgw_player_tutorials` with
//!    `ON CONFLICT DO NOTHING`. Only a new row (`First`) displays the
//!    dialog; `AlreadyShown` displays nothing; `Refused` displays nothing
//!    and takes the optimistic mark back out, so a later trigger can retry.
//!
//! A relog or a world change rebuilds the set from the table
//! (`InitPlayerState.shown_tutorials`), so neither replays a tutorial.
//!
//! The dialog is sent with the player as its speaker entity: a
//! `DUIST_DefaultTutorial` window has no portrait, and binding an NPC would
//! only make the display depend on whoever the player last clicked.
//!
//! Telemetry: every outcome is one `event = "content_show_tutorial"` row on
//! target `content` with a `decision_outcome` of `shown`, `already_shown`
//! or `refused` (plus a DEBUG `forwarded`), carrying the player, the chain
//! and the tutorial with their names (Rule 6).

use tokio::sync::mpsc;

use crate::cell::messages::{
    CellToBaseMsg, RecordTutorialShown, TutorialRecordOutcome, TutorialRecorded,
};
use crate::cell::space_manager::SpaceManager;

/// `event` of every row (target `content`).
const EVENT: &str = "content_show_tutorial";

/// Run one `show_tutorial` action for `entity_id`.
pub(super) async fn run(
    tutorial_id: i32,
    entity_id: u32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let who = space_mgr.player_identity(entity_id);
    let is_player = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);
    let Some(player_id) = who.player_id.filter(|&id| id > 0 && is_player) else {
        // The chain fired on an NPC or a player with no loaded character:
        // an authoring or lifecycle bug, never ordinary play.
        tracing::warn!(
            target: "content",
            event = EVENT,
            decision_outcome = "refused",
            reason = "not_a_player",
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            tutorial_id,
            tutorial_name = cimmeria_names::book().dialog(tutorial_id),
            "show_tutorial fired for an entity that is not a loaded player; nothing shown"
        );
        return;
    };

    // Mark first: the set is what the same tick's next firing and any
    // `tutorial_shown` gate read.
    let newly_marked = space_mgr
        .get_entity_mut(entity_id)
        .is_some_and(|e| e.shown_tutorials.insert(tutorial_id));
    if !newly_marked {
        // Ordinary play (a replayed chain, the trigger firing again), so
        // INFO: the row a "why did the tutorial not show" query finds.
        tracing::info!(
            target: "content",
            event = EVENT,
            decision_outcome = "already_shown",
            reason = "cell_set",
            entity_id,
            entity_name = who.player_name,
            account_id = who.account_id,
            account_name = who.account_name,
            player_id,
            player_name = who.player_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            tutorial_id,
            tutorial_name = cimmeria_names::book().dialog(tutorial_id),
            "show_tutorial: the player has already seen this tutorial; nothing shown"
        );
        return;
    }

    let msg = CellToBaseMsg::RecordTutorialShown(RecordTutorialShown {
        entity_id,
        player_id,
        chain_id,
        tutorial_id,
    });
    if let Err(e) = tx.send(msg).await {
        if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
            entity.shown_tutorials.remove(&tutorial_id);
        }
        tracing::error!(
            target: "content",
            event = EVENT,
            decision_outcome = "refused",
            reason = "cell_to_base_closed",
            entity_id,
            entity_name = who.player_name,
            account_id = who.account_id,
            account_name = who.account_name,
            player_id,
            player_name = who.player_name,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            tutorial_id,
            tutorial_name = cimmeria_names::book().dialog(tutorial_id),
            error = %e,
            "show_tutorial: cell->base send failed; nothing recorded or shown"
        );
        return;
    }
    // Module-path target: `content` is exported at INFO only, the
    // `cimmeria_cell_content=debug` OTEL_FILTER row exports this one.
    tracing::debug!(
        event = EVENT,
        decision_outcome = "forwarded",
        entity_id,
        entity_name = who.player_name,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id,
        player_name = who.player_name,
        chain_id,
        chain_name = cimmeria_names::book().chain(chain_id),
        tutorial_id,
        tutorial_name = cimmeria_names::book().dialog(tutorial_id),
        "show_tutorial forwarded to the base to record"
    );
}

/// Handle `BaseToCellMsg::TutorialRecorded`: display the tutorial when the
/// base's record was the first one.
pub async fn apply_tutorial_recorded(
    recorded: TutorialRecorded,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let TutorialRecorded {
        entity_id,
        player_id,
        chain_id,
        tutorial_id,
        outcome,
    } = recorded;
    let who = space_mgr.player_identity(entity_id);
    if who.player_id != Some(player_id) {
        // The player logged out or swapped characters while the insert ran.
        // The row is written either way; the next world entry reads it.
        tracing::info!(
            target: "content",
            event = EVENT,
            decision_outcome = "refused",
            reason = "player_changed",
            entity_id,
            entity_name = who.player_name,
            player_id,
            current_player_id = who.player_id,
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            tutorial_id,
            tutorial_name = cimmeria_names::book().dialog(tutorial_id),
            record_outcome = outcome.as_str(),
            "TutorialRecorded: the entity no longer plays that character; nothing shown"
        );
        return;
    }

    match outcome {
        TutorialRecordOutcome::First => {
            tracing::info!(
                target: "content",
                event = EVENT,
                decision_outcome = "shown",
                entity_id,
                entity_name = who.player_name,
                account_id = who.account_id,
                account_name = who.account_name,
                player_id,
                player_name = who.player_name,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                tutorial_id,
                tutorial_name = cimmeria_names::book().dialog(tutorial_id),
                "show_tutorial: first time for this character; displaying the tutorial"
            );
            crate::cell::playtest_friction::dialog_shown(entity_id, tutorial_id);
            crate::cell::player_journal::note(
                entity_id,
                crate::cell::player_journal::kinds::DIALOG,
                format!("tutorial={tutorial_id} chain={chain_id}"),
            );
            // The player is the speaker entity: a tutorial window has no
            // portrait (see the module docs).
            crate::cell::interactions::send_dialog_display(
                entity_id,
                entity_id as i32,
                tutorial_id,
                tx,
                space_mgr,
            )
            .await;
        }
        TutorialRecordOutcome::AlreadyShown => {
            // The table had the row but the cell's set did not (a world
            // entry raced the action). Keep the mark; show nothing.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.shown_tutorials.insert(tutorial_id);
            }
            tracing::info!(
                target: "content",
                event = EVENT,
                decision_outcome = "already_shown",
                reason = "db_row",
                entity_id,
                entity_name = who.player_name,
                account_id = who.account_id,
                account_name = who.account_name,
                player_id,
                player_name = who.player_name,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                tutorial_id,
                tutorial_name = cimmeria_names::book().dialog(tutorial_id),
                "show_tutorial: sgw_player_tutorials already has this tutorial; nothing shown"
            );
        }
        TutorialRecordOutcome::Refused => {
            // Nothing was persisted, so nothing is shown (a tutorial shown
            // without a row would replay on the next relog). Forget the
            // mark so a later trigger can try again. The base logged why.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.shown_tutorials.remove(&tutorial_id);
            }
            tracing::warn!(
                target: "content",
                event = EVENT,
                decision_outcome = "refused",
                reason = "not_recorded",
                entity_id,
                entity_name = who.player_name,
                account_id = who.account_id,
                account_name = who.account_name,
                player_id,
                player_name = who.player_name,
                chain_id,
                chain_name = cimmeria_names::book().chain(chain_id),
                tutorial_id,
                tutorial_name = cimmeria_names::book().dialog(tutorial_id),
                "show_tutorial: the base could not record the tutorial; nothing shown"
            );
        }
    }
}
