//! The cell half of the unreadable-missions lock (TG-DB-12, D-TG18).
//!
//! When the base cannot read `sgw_mission` at world entry it sends
//! `MissionStateUnknown` before `InitPlayerState`, and [`mark_state_unknown`]
//! locks the player's tracker. The tracker is then empty but is *not* the
//! player's state, so a chain gated on `mission_status eq not_active` would
//! re-offer, and a re-accept would overwrite, a completed mission (#411).
//!
//! The tracker is also held from `CreateEntity` until `InitPlayerState`
//! ([`mark_load_pending`]): until then nobody knows whether the read
//! succeeded, so a mission change in that window could land on an empty
//! tracker before `MissionStateUnknown` arrives.
//!
//! While locked or pending:
//!
//! - [`gate_mission_chains`] drops every chain in a resolved action list
//!   that changes a mission, whole, so a chain never half-runs (its item
//!   grant without its accept). Called from `execute_actions` before the
//!   fire-once gate, so a dropped once-chain stays armed for after recovery.
//! - [`refuse_if_state_unknown`] guards each primitive (accept, abandon,
//!   advance step, complete objective, complete, GM fail) and the persist,
//!   which covers the client and GM paths that do not run through a chain.
//!
//! Each refusal tells the player ([`UNAVAILABLE_LINE`]) and logs WARN
//! `event = mission_state_unknown`, at most once per entity per
//! [`REFUSAL_WINDOW`]; the next row carries `suppressed`, the refusals
//! swallowed since (Pattern D). `SavedMissionsRecovered` ends the lock
//! through [`recover_state`], and the `player_loaded` chains dropped at
//! world entry ([`defer_dropped_chains`]) are then replayed.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

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

/// One refusal row (WARN + feedback line) per entity per window.
pub const REFUSAL_WINDOW: Duration = Duration::from_secs(5);

/// Per-entity refusal throttle: when the last row went out and how many
/// refusals were swallowed since.
static THROTTLE: LazyLock<Mutex<HashMap<u32, (Instant, u64)>>> = LazyLock::new(Default::default);

/// Whether a refusal for `entity_id` at `now` may log and tell the player.
/// `Some(suppressed)` admits it, carrying the refusals swallowed since the
/// last admitted one; `None` swallows it (adding `count`).
fn admit_refusal(entity_id: u32, now: Instant, count: u64) -> Option<u64> {
    let Ok(mut map) = THROTTLE.lock() else {
        return Some(0);
    };
    if map.len() > 1024 {
        map.retain(|_, (last, _)| now.saturating_duration_since(*last) < REFUSAL_WINDOW);
    }
    match map.get_mut(&entity_id) {
        Some((last, suppressed)) if now.saturating_duration_since(*last) < REFUSAL_WINDOW => {
            *suppressed += count;
            None
        }
        Some(entry) => {
            let suppressed = entry.1;
            *entry = (now, 0);
            Some(suppressed)
        }
        None => {
            map.insert(entity_id, (now, 0));
            Some(0)
        }
    }
}

fn reset_throttle(entity_id: u32) {
    if let Ok(mut map) = THROTTLE.lock() {
        map.remove(&entity_id);
    }
}

/// Why `entity_id`'s tracker is locked, if it is: the saved missions are
/// unreadable, or their load outcome has not arrived yet.
pub fn lock_reason(entity_id: u32, space_mgr: &SpaceManager) -> Option<&'static str> {
    let missions = &space_mgr.get_entity(entity_id)?.missions;
    if missions.is_state_unknown() {
        Some("saved_missions_unreadable")
    } else if missions.is_load_pending() {
        Some("saved_missions_pending")
    } else {
        None
    }
}

/// Whether `entity_id`'s mission tracker is locked (or still pending).
pub fn is_state_unknown(entity_id: u32, space_mgr: &SpaceManager) -> bool {
    lock_reason(entity_id, space_mgr).is_some()
}

/// Hold `entity_id`'s tracker until `InitPlayerState` (a base-created
/// player's `CreateEntity`).
pub fn mark_load_pending(entity_id: u32, space_mgr: &mut SpaceManager) {
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        entity.missions.mark_load_pending();
        reset_throttle(entity_id);
    }
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
/// to the player and a WARN (throttled). Returns `true` when the caller
/// must stop.
pub async fn refuse_if_state_unknown(
    entity_id: u32,
    mission_id: i32,
    site: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    let Some(reason) = lock_reason(entity_id, space_mgr) else {
        return false;
    };
    if let Some(suppressed) = admit_refusal(entity_id, Instant::now(), 1) {
        warn_refused(
            entity_id, mission_id, site, reason, None, suppressed, space_mgr,
        );
        send_feedback(entity_id, UNAVAILABLE_LINE, tx, space_mgr).await;
    }
    true
}

/// The persist backstop: refuse quietly (throttled WARN, no feedback line;
/// the primitive that mutated already told the player). Returns `true`
/// when the `MissionUpdate` must not be sent.
pub fn refuse_persist_if_unknown(
    entity_id: u32,
    mission_id: i32,
    site: &'static str,
    space_mgr: &SpaceManager,
) -> bool {
    let Some(reason) = lock_reason(entity_id, space_mgr) else {
        return false;
    };
    if let Some(suppressed) = admit_refusal(entity_id, Instant::now(), 1) {
        warn_refused(
            entity_id, mission_id, site, reason, None, suppressed, space_mgr,
        );
    }
    true
}

/// The mission an action changes, if it changes one.
fn mission_of(action: &Action) -> Option<i32> {
    match action {
        Action::AcceptMission { mission_id }
        | Action::AdvanceMission { mission_id }
        | Action::CompleteMission { mission_id }
        | Action::CompleteMissionWithItem { mission_id, .. }
        | Action::AdvanceStep { mission_id, .. }
        | Action::AbandonMission { mission_id }
        | Action::FailObjective { mission_id, .. }
        | Action::CompleteObjective { mission_id, .. } => Some(*mission_id),
        _ => None,
    }
}

/// Remember, for replay on recovery, each chain in `actions` that the lock
/// is about to drop. Called by `fire_player_loaded` only: the chains of
/// other triggers re-fire on their own trigger, `player_loaded` never
/// does in that session.
pub fn defer_dropped_chains(
    entity_id: u32,
    actions: &[(i64, Action)],
    space_mgr: &mut SpaceManager,
) {
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        return;
    };
    if !entity.missions.is_state_unknown() && !entity.missions.is_load_pending() {
        return;
    }
    for (chain_id, action) in actions {
        if mission_of(action).is_some() {
            entity.missions.defer_player_loaded_chain(*chain_id);
        }
    }
}

/// Drop, whole, every chain in `actions` that changes a mission while
/// `entity_id`'s tracker is locked. One feedback line per call, one WARN per
/// dropped chain (throttled together). Delays stay index-aligned with the
/// survivors.
pub(crate) async fn gate_mission_chains(
    actions: Vec<(i64, Action)>,
    action_delays: Vec<i32>,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> (Vec<(i64, Action)>, Vec<i32>) {
    let Some(reason) = lock_reason(entity_id, space_mgr) else {
        return (actions, action_delays);
    };
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
    if let Some(suppressed) = admit_refusal(entity_id, Instant::now(), blocked.len() as u64) {
        for &(chain_id, mission_id) in &blocked {
            warn_refused(
                entity_id,
                mission_id,
                "chain",
                reason,
                Some(chain_id),
                suppressed,
                space_mgr,
            );
        }
        send_feedback(entity_id, UNAVAILABLE_LINE, tx, space_mgr).await;
    }

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
/// The caller then replays the deferred `player_loaded` chains.
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
    reset_throttle(entity_id);
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
    reason: &'static str,
    chain_id: Option<i64>,
    suppressed: u64,
    space_mgr: &SpaceManager,
) {
    let who = space_mgr.player_identity(entity_id);
    let names = cimmeria_names::book();
    tracing::warn!(
        event = "mission_state_unknown",
        reason,
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
        suppressed,
        "Mission change refused: the saved missions are unreadable or not loaded yet, so the \
         tracker is locked"
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

#[cfg(test)]
pub(crate) fn admit_refusal_at(entity_id: u32, now: Instant) -> Option<u64> {
    admit_refusal(entity_id, now, 1)
}
