//! The cell half of the unreadable-missions lock (TG-DB-12, D-TG18).
//!
//! When the base cannot read `sgw_mission` at world entry it sends
//! `MissionStateUnknown` before `InitPlayerState`, and [`mark_state_unknown`]
//! locks the player's tracker. The tracker is then empty but is *not* the
//! player's state, so a chain gated on `mission_status eq not_active` would
//! re-offer, and a re-accept would overwrite, a completed mission (#411).
//! While locked:
//!
//! - [`gate_mission_chains`] drops every chain in a resolved action list
//!   that changes a mission, whole, so a chain never half-runs (its item
//!   grant without its accept). Called from `execute_actions` before the
//!   fire-once gate, so a dropped once-chain stays armed for after recovery.
//! - [`refuse_if_state_unknown`] guards each primitive (accept, abandon,
//!   advance step, complete objective, complete) and the persist, which
//!   covers the client and GM paths that do not run through a chain.
//!
//! Each refusal tells the player ([`UNAVAILABLE_LINE`]) and logs WARN
//! `event = mission_state_unknown`, `reason = saved_missions_unreadable`.
//! `SavedMissionsRecovered` ends the lock through [`recover_state`].

use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_entity::missions::MissionInstance;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// The line a player sees when a mission change is refused.
pub const UNAVAILABLE_LINE: &str = "Mission data unavailable, try again shortly.";

/// The line a player sees when the saved missions arrive late.
pub const RESTORED_LINE: &str = "Mission data restored.";

/// Whether `entity_id`'s mission tracker is locked.
pub fn is_state_unknown(entity_id: u32, space_mgr: &SpaceManager) -> bool {
    space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| e.missions.is_state_unknown())
}

/// Lock `entity_id`'s tracker (`MissionStateUnknown`).
pub fn mark_state_unknown(entity_id: u32, space_mgr: &mut SpaceManager) {
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        tracing::warn!(
            event = "mission_state_unknown",
            reason = "entity_missing",
            entity_id, // nt:id-only the entity is gone, nothing left to name
            "MissionStateUnknown for an entity the cell does not have; nothing to lock"
        );
        return;
    };
    entity.missions.mark_state_unknown();
    let who = space_mgr.player_identity(entity_id);
    tracing::info!(
        event = "mission_state_locked",
        reason = "saved_missions_unreadable",
        entity_id,
        entity_name = who.player_name,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        "Mission tracker locked: the saved missions are unreadable; accepts, advances and \
         saves are refused until a background read succeeds"
    );
}

/// Refuse a mission change while the tracker is locked: one feedback line
/// to the player and a WARN. Returns `true` when the caller must stop.
pub async fn refuse_if_state_unknown(
    entity_id: u32,
    mission_id: i32,
    site: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    if !is_state_unknown(entity_id, space_mgr) {
        return false;
    }
    warn_refused(entity_id, mission_id, site, None, space_mgr);
    send_feedback(entity_id, UNAVAILABLE_LINE, tx, space_mgr).await;
    true
}

/// The persist backstop: refuse quietly (WARN, no feedback line; the
/// primitive that mutated already told the player). Returns `true` when the
/// `MissionUpdate` must not be sent.
pub fn refuse_persist_if_unknown(
    entity_id: u32,
    mission_id: i32,
    site: &'static str,
    space_mgr: &SpaceManager,
) -> bool {
    if !is_state_unknown(entity_id, space_mgr) {
        return false;
    }
    warn_refused(entity_id, mission_id, site, None, space_mgr);
    true
}

/// The mission an action changes, if it changes one.
fn mission_of(action: &Action) -> Option<i32> {
    match action {
        Action::AcceptMission { mission_id }
        | Action::AdvanceMission { mission_id }
        | Action::CompleteMission { mission_id }
        | Action::AdvanceStep { mission_id, .. }
        | Action::AbandonMission { mission_id }
        | Action::FailObjective { mission_id, .. }
        | Action::CompleteObjective { mission_id, .. } => Some(*mission_id),
        _ => None,
    }
}

/// Drop, whole, every chain in `actions` that changes a mission while
/// `entity_id`'s tracker is locked. One feedback line per call, one WARN per
/// dropped chain. Delays stay index-aligned with the survivors.
pub(crate) async fn gate_mission_chains(
    actions: Vec<(i64, Action)>,
    action_delays: Vec<i32>,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> (Vec<(i64, Action)>, Vec<i32>) {
    if !is_state_unknown(entity_id, space_mgr) {
        return (actions, action_delays);
    }
    let mut blocked: Vec<(i64, i32)> = Vec::new();
    for (chain_id, action) in &actions {
        if let Some(mission_id) = mission_of(action) {
            if !blocked.iter().any(|(c, _)| c == chain_id) {
                blocked.push((*chain_id, mission_id));
            }
        }
    }
    if blocked.is_empty() {
        return (actions, action_delays);
    }
    for &(chain_id, mission_id) in &blocked {
        warn_refused(entity_id, mission_id, "chain", Some(chain_id), space_mgr);
    }
    send_feedback(entity_id, UNAVAILABLE_LINE, tx, space_mgr).await;

    let delays_len = action_delays.len();
    let mut kept = Vec::with_capacity(actions.len());
    let mut kept_delays = Vec::with_capacity(delays_len);
    for (i, (chain_id, action)) in actions.into_iter().enumerate() {
        if blocked.iter().any(|(c, _)| *c == chain_id) {
            continue;
        }
        if i < delays_len {
            kept_delays.push(action_delays[i]);
        }
        kept.push((chain_id, action));
    }
    (kept, kept_delays)
}

/// Unlock `entity_id`'s tracker with the missions read late
/// (`SavedMissionsRecovered`), resend the journal and tell the player.
/// Returns `false`, changing nothing, when the entity is gone or was not
/// locked (a duplicate recovery must not overwrite progress made since).
pub async fn recover_state(
    entity_id: u32,
    restored: Vec<MissionInstance>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let count = restored.len();
    let recovered = space_mgr
        .get_entity_mut(entity_id)
        .is_some_and(|e| e.missions.recover_state(restored));
    let who = space_mgr.player_identity(entity_id);
    if !recovered {
        tracing::debug!(
            event = "mission_state_recovery_ignored",
            reason = "not_locked",
            entity_id,
            entity_name = who.player_name,
            player_id = who.player_id,
            player_name = who.player_name,
            "SavedMissionsRecovered for a tracker that is not locked (or an entity that is \
             gone); ignored"
        );
        return false;
    }
    tracing::info!(
        event = "mission_state_recovered",
        entity_id,
        entity_name = who.player_name,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        count,
        "Saved missions restored after a late read; the mission tracker is unlocked"
    );
    super::resend_missions(entity_id, tx, space_mgr).await;
    send_feedback(entity_id, RESTORED_LINE, tx, space_mgr).await;
    true
}

fn warn_refused(
    entity_id: u32,
    mission_id: i32,
    site: &'static str,
    chain_id: Option<i64>,
    space_mgr: &SpaceManager,
) {
    let who = space_mgr.player_identity(entity_id);
    let names = cimmeria_names::book();
    tracing::warn!(
        event = "mission_state_unknown",
        reason = "saved_missions_unreadable",
        site,
        entity_id,
        entity_name = who.player_name,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        mission_id,
        mission_name = names.mission(mission_id),
        chain_id,
        chain_name = chain_id.and_then(|c| names.chain(c)),
        "Mission change refused: the saved missions are unreadable, so the tracker is locked"
    );
}

async fn send_feedback(
    entity_id: u32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
        })
        .await
    {
        tracing::warn!(
            event = "mission_state_feedback_lost",
            reason = "base_channel_closed",
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            error = %e,
            "Mission lock feedback line not queued"
        );
    }
}
