//! Fenced cell request and same-attempt retry for Frost's Letter.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_content_engine::actions::Action;
use cimmeria_entity::known_names;
use cimmeria_entity::missions::{MISSION_ACTIVE, STATUS_ACTIVE};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::messages::{
    LetterTurnInAttempt, LetterTurnInRequest, LetterTurnInSession, PluginMsg,
};
use tokio::sync::mpsc;

use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{interact_range, SpaceManager};

const MARSH_TAG: &str = "CmdCenter_Marsh";
const MARSH_TEMPLATE: i32 = 10;
const WORLD: &str = "Harset_CmdCenter";
const RETRY_AFTER: Duration = Duration::from_secs(3);
const PROCESSING: &str = "Checking Frost's Letter...";
const UNAVAILABLE: &str = "Cannot hand in Frost's Letter right now.";

/// One accepted interaction, bound to one player, entry and Marsh entity.
/// Entity destruction drops this extension with the player.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PendingLetterTurnIn {
    pub(crate) request: LetterTurnInRequest,
    pub(crate) marsh_entity_id: u32,
    pub(crate) next_retry_at: Instant,
}

fn same_entry(pending: &PendingLetterTurnIn, space_mgr: &SpaceManager) -> bool {
    let request = pending.request;
    let Some(player) = space_mgr.get_entity(request.entity_id) else {
        return false;
    };
    player.is_player
        && player.player_id == Some(request.player_id)
        && player
            .extensions
            .get::<LetterTurnInSession>()
            .is_some_and(|session| {
                session.player_id == request.player_id
                    && session.token == request.token
                    && session.generation == request.generation
                    && session.epoch == request.epoch
            })
        && space_mgr
            .get_entity_world_name(request.entity_id)
            .as_deref()
            == Some(WORLD)
        && space_mgr.get_entity_world_id(request.entity_id) == Some(68)
}

fn valid_interaction(pending: &PendingLetterTurnIn, space_mgr: &SpaceManager) -> bool {
    let request = pending.request;
    same_entry(pending, space_mgr)
        && space_mgr
            .get_entity(request.entity_id)
            .is_some_and(|player| player.last_interaction_target == Some(pending.marsh_entity_id))
        && space_mgr
            .get_entity(pending.marsh_entity_id)
            .is_some_and(|marsh| {
                marsh.tag.as_deref() == Some(MARSH_TAG) && marsh.template_id == Some(MARSH_TEMPLATE)
            })
        && interact_range(request.entity_id, pending.marsh_entity_id, space_mgr).is_ok()
}

fn eligible_mission(entity_id: u32, space_mgr: &SpaceManager) -> bool {
    let Some(player) = space_mgr.get_entity(entity_id) else {
        return false;
    };
    if player.missions.is_state_unknown() || player.missions.is_load_pending() {
        return false;
    }
    let Some(letter) = player.missions.get_mission(1360) else {
        return false;
    };
    letter.status == MISSION_ACTIVE
        && letter.current_step_id == Some(4038)
        && letter
            .active_objectives
            .iter()
            .any(|obj| obj.objective_id == 4651 && obj.status == STATUS_ACTIVE)
        && !player.missions.get_mission(1361).is_some_and(|praxis| {
            praxis.status == MISSION_ACTIVE && praxis.current_step_id == Some(4694)
        })
}

fn feedback(entity_id: u32, player_id: i32, text: &str, tx: &mpsc::Sender<CellToBaseMsg>) {
    if let Err(error) = tx.try_send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: ON_PLAYER_COMMUNICATION,
        args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
    }) {
        tracing::warn!(entity_id, entity_name = known_names::player_name(player_id),
            reason = "feedback_channel_unavailable", %error,
            "Frost letter feedback could not be queued");
    }
}

fn send_request(pending: &mut PendingLetterTurnIn, tx: &mpsc::Sender<CellToBaseMsg>) {
    match tx.try_send(CellToBaseMsg::Plugin(PluginMsg::new(pending.request))) {
        Ok(()) => {
            pending.next_retry_at = Instant::now() + RETRY_AFTER;
            tracing::info!(
                entity_id = pending.request.entity_id,
                entity_name = known_names::player_name(pending.request.player_id),
                player_id = pending.request.player_id,
                player_name = known_names::player_name(pending.request.player_id),
                marsh_entity_id = pending.marsh_entity_id,
                marsh_entity_name = MARSH_TAG,
                "Frost letter turn-in request queued"
            );
        }
        Err(error) => {
            pending.next_retry_at = Instant::now() + RETRY_AFTER;
            tracing::warn!(
                entity_id = pending.request.entity_id,
                entity_name = known_names::player_name(pending.request.player_id),
                player_id = pending.request.player_id,
                player_name = known_names::player_name(pending.request.player_id),
                reason = if matches!(error, mpsc::error::TrySendError::Full(_)) {
                    "base_queue_full"
                } else {
                    "base_queue_closed"
                },
                "Frost letter turn-in request not queued; same attempt retained"
            );
        }
    }
}

/// Request a transaction for the exact loaded action and live Marsh click.
#[allow(clippy::too_many_arguments)]
pub(super) async fn request(
    action: Action,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    params: &HashMap<String, serde_json::Value>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Action::CompleteMissionWithItem {
        mission_id: 1360,
        expected_step_id: 4038,
        item_id: 3730,
        quantity: 1,
        dialog_id: 4576,
        dialog_set_id: 120101,
        slot: 10,
    } = action
    else {
        tracing::warn!(
            entity_id,
            entity_name = known_names::player_name(player_id),
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            reason = "invalid_action",
            "Frost letter action refused"
        );
        feedback(entity_id, player_id, UNAVAILABLE, tx);
        return;
    };

    // A previously pending request may belong to a destroyed/reused player,
    // a different entry or a mission that is no longer eligible. Clear it first.
    let stale = space_mgr
        .get_entity(entity_id)
        .and_then(|player| player.extensions.get::<PendingLetterTurnIn>())
        .is_some_and(|pending| {
            !same_entry(pending, space_mgr) || !eligible_mission(entity_id, space_mgr)
        });
    if stale {
        if let Some(player) = space_mgr.get_entity_mut(entity_id) {
            player.extensions.remove::<PendingLetterTurnIn>();
        }
    }
    if space_mgr
        .get_entity(entity_id)
        .is_some_and(|player| player.extensions.contains::<PendingLetterTurnIn>())
    {
        feedback(entity_id, player_id, PROCESSING, tx);
        return;
    }

    let target = params
        .get("target_entity_id")
        .and_then(serde_json::Value::as_u64)
        .and_then(|id| u32::try_from(id).ok());
    let Some(target) = target else {
        tracing::info!(
            entity_id,
            entity_name = known_names::player_name(player_id),
            player_id,
            player_name = known_names::player_name(player_id),
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            reason = "missing_target",
            "Frost letter turn-in click refused"
        );
        feedback(entity_id, player_id, UNAVAILABLE, tx);
        return;
    };
    let Some(session) = space_mgr
        .get_entity(entity_id)
        .and_then(|player| player.extensions.get::<LetterTurnInSession>())
        .copied()
    else {
        tracing::info!(
            entity_id,
            entity_name = known_names::player_name(player_id),
            player_id,
            player_name = known_names::player_name(player_id),
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            reason = "missing_session",
            "Frost letter turn-in click refused"
        );
        feedback(entity_id, player_id, UNAVAILABLE, tx);
        return;
    };
    let request = LetterTurnInRequest {
        entity_id,
        player_id,
        attempt: LetterTurnInAttempt::from_bytes(rand::random()),
        token: session.token,
        generation: session.generation,
        epoch: session.epoch,
    };
    let mut pending = PendingLetterTurnIn {
        request,
        marsh_entity_id: target,
        next_retry_at: Instant::now(),
    };
    let actor_matches = space_mgr
        .get_entity(entity_id)
        .is_some_and(|player| player.player_id == Some(player_id) && player_id > 0);
    let tag_matches =
        params.get("entity_tag").and_then(serde_json::Value::as_str) == Some(MARSH_TAG);
    if !actor_matches
        || !tag_matches
        || !valid_interaction(&pending, space_mgr)
        || !eligible_mission(entity_id, space_mgr)
    {
        tracing::info!(
            entity_id,
            entity_name = known_names::player_name(player_id),
            player_id,
            player_name = known_names::player_name(player_id),
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            reason = "ineligible_click",
            "Frost letter turn-in click refused"
        );
        feedback(entity_id, player_id, UNAVAILABLE, tx);
        return;
    }

    feedback(entity_id, player_id, PROCESSING, tx);
    send_request(&mut pending, tx);
    space_mgr
        .get_entity_mut(entity_id)
        .expect("validated live actor")
        .extensions
        .insert(pending);
}

/// Retry uncertain requests with their original attempt, and clear stale entries.
/// Runs on the cell tick; request admission is nonblocking.
pub fn letter_turn_in_retry_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let now = Instant::now();
    for entity_id in space_mgr.all_player_entity_ids() {
        let pending = space_mgr
            .get_entity(entity_id)
            .and_then(|player| player.extensions.get::<PendingLetterTurnIn>())
            .copied();
        let Some(mut pending) = pending else {
            continue;
        };
        if !same_entry(&pending, space_mgr) || !eligible_mission(entity_id, space_mgr) {
            if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                player.extensions.remove::<PendingLetterTurnIn>();
            }
            continue;
        }
        if now < pending.next_retry_at {
            continue;
        }
        send_request(&mut pending, tx);
        if let Some(player) = space_mgr.get_entity_mut(entity_id) {
            player.extensions.insert(pending);
        }
    }
}
