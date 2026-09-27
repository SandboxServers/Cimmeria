//! `sendDuelChallenge` (SGWPlayer base method 0xD9): the base half of a
//! duel challenge (SS-D1).
//!
//! The base checks what only it can see, in this order, then forwards the
//! challenge to the cell as `DuelBaseToCell::Challenge`:
//!
//! 1. the duel bucket (D-SS21): first, so every refused challenge costs a
//!    token and a client cannot turn a flood of bad packets into a flood of
//!    feedback lines (the chat path's order, `dispatch/chat.rs`);
//! 2. squad duels are refused (the ledger's first check; see
//!    [`TEXT_SQUAD_DUEL_UNSUPPORTED`] for why not text 874);
//! 3. the target, resolved against the online index (D-SS13): exact name,
//!    else a unique case-insensitive match;
//! 4. Ignore (D-SS15): a target who ignores the challenger refuses it.
//!
//! The cell then checks self, space, range, busy and the pair cooldown
//! (`cell::duel::challenge`). Every id in the forward is read from this
//! session map, never from the payload, which carries only a name and a
//! byte.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::base::duel::decode_send_duel_challenge;
use cimmeria_wire::cell::client_methods::duel::{
    TEXT_SQUAD_DUEL_UNSUPPORTED, TEXT_TARGET_AMBIGUOUS, TEXT_TARGET_IGNORING,
    TEXT_TARGET_NOT_ONLINE,
};
use tokio::sync::mpsc;

use crate::cell::messages::{BaseToCellMsg, DuelBaseToCell};

use super::super::feedback::{send_feedback_line, FeedbackCtx};
use super::super::rate_limit::{log_exceeded, RateActor, RateCategory, RateDecision};
use super::super::ConnectedClientState;
use cimmeria_base_session::base::player_index::{NameLookup, OnlinePlayerIndex};

/// Longest prefix of a typed name a log row carries.
const LOGGED_NAME_CHARS: usize = 64;

/// The challenger, from this session.
#[derive(Debug, Clone, Copy)]
struct Actor {
    player_id: i32,
    account_id: u32,
    entity_id: u32,
}

/// Handle `sendDuelChallenge` on the wall clock.
pub(super) async fn handle_send_duel_challenge(
    payload: &[u8],
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    send_duel_challenge_at(payload, addr, transport, connected, cell_tx, Instant::now()).await;
}

/// [`handle_send_duel_challenge`] on an explicit clock, so the rate-limit
/// guard can step time exactly.
#[tracing::instrument(
    name = "duel.challenge_request",
    target = "duel",
    level = "info",
    skip_all,
    fields(peer = %addr, payload_len = payload.len()),
)]
pub(super) async fn send_duel_challenge_at(
    payload: &[u8],
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    now: Instant,
) {
    let feedback = FeedbackCtx {
        transport,
        connected,
    };

    // Identity and the bucket, under one lock.
    let (actor, decision) = {
        let mut clients = connected.lock().unwrap();
        let Some(c) = clients.get_mut(&addr) else {
            return;
        };
        let (Some(player_id), Some(entity_id)) = (c.active_player_id, c.player_entity_id) else {
            tracing::warn!(
                target: "duel",
                event = "duel.challenge_refused",
                %addr,
                account_id = c.account_id,
                player_id = c.active_player_id,
                entity_id = c.player_entity_id,
                reason = "not_in_world",
                "sendDuelChallenge from a session with no player in the world"
            );
            return;
        };
        let actor = Actor {
            player_id,
            account_id: c.account_id,
            entity_id,
        };
        let decision = c.rate_limits.check(RateCategory::DuelChallenge, now);
        if let RateDecision::Limited { notify } = decision {
            let rate_actor = RateActor {
                addr,
                player_id: Some(player_id),
                account_id: c.account_id,
                entity_id: Some(entity_id),
            };
            log_exceeded(
                RateCategory::DuelChallenge,
                rate_actor,
                notify,
                &c.rate_limits,
                now,
            );
        }
        (actor, decision)
    };

    if let RateDecision::Limited { notify } = decision {
        if notify {
            send_feedback_line(&feedback, addr, RateCategory::DuelChallenge.feedback_text()).await;
        }
        return;
    }

    let call = match decode_send_duel_challenge(payload) {
        Ok(call) => call,
        Err(e) => {
            // A forged or corrupted call: logged, not answered.
            tracing::warn!(
                target: "duel",
                event = "duel.challenge_malformed",
                %addr,
                account_id = actor.account_id,
                player_id = actor.player_id,
                entity_id = actor.entity_id,
                reason = e.reason(),
                error = %e,
                "sendDuelChallenge payload did not decode"
            );
            return;
        }
    };
    let shown: String = call.player_name.chars().take(LOGGED_NAME_CHARS).collect();

    let refuse = |reason: &'static str, target_player_id: Option<i32>| {
        tracing::debug!(
            target: "duel",
            event = "duel.challenge_refused",
            %addr,
            account_id = actor.account_id,
            player_id = actor.player_id,
            entity_id = actor.entity_id,
            target_player_id,
            target_name = %shown,
            squad_duel = call.squad_duel,
            reason,
            "duel challenge refused at the base"
        );
    };

    if call.is_squad() {
        refuse("squad_duel", None);
        send_feedback_line(&feedback, addr, TEXT_SQUAD_DUEL_UNSUPPORTED).await;
        return;
    }

    // Resolve the target and read what the forward needs, under one lock.
    let resolved = {
        let clients = connected.lock().unwrap();
        match OnlinePlayerIndex::new(&clients).lookup(&call.player_name) {
            NameLookup::Found(target) => {
                let target_state = clients.get(&target.addr);
                match target_state.and_then(|t| t.player_entity_id) {
                    Some(target_entity_id) => Ok((
                        target.player_id,
                        target_entity_id,
                        target_state.is_some_and(|t| ignores(t, actor.player_id)),
                    )),
                    None => Err(("target_not_in_world", TEXT_TARGET_NOT_ONLINE)),
                }
            }
            NameLookup::Ambiguous => Err(("target_ambiguous", TEXT_TARGET_AMBIGUOUS)),
            NameLookup::NotFound => Err(("target_not_online", TEXT_TARGET_NOT_ONLINE)),
        }
    };
    let (target_player_id, target_entity_id, ignored) = match resolved {
        Ok(t) => t,
        Err((reason, text)) => {
            refuse(reason, None);
            send_feedback_line(&feedback, addr, text).await;
            return;
        }
    };
    if ignored {
        refuse("target_ignoring", Some(target_player_id));
        send_feedback_line(&feedback, addr, TEXT_TARGET_IGNORING).await;
        return;
    }

    let Some(tx) = cell_tx else {
        tracing::warn!(
            target: "duel",
            event = "duel.challenge_refused",
            %addr,
            account_id = actor.account_id,
            player_id = actor.player_id,
            entity_id = actor.entity_id,
            target_player_id,
            reason = "no_cell_channel",
            "duel challenge not forwarded: no cell channel"
        );
        return;
    };
    tracing::debug!(
        target: "duel",
        event = "duel.challenge_forwarded",
        %addr,
        account_id = actor.account_id,
        player_id = actor.player_id,
        entity_id = actor.entity_id,
        target_player_id,
        target_entity_id,
        "duel challenge passed the base checks; forwarded to the cell"
    );
    let msg = BaseToCellMsg::Duel(DuelBaseToCell::Challenge {
        player_id: actor.player_id,
        entity_id: actor.entity_id,
        account_id: actor.account_id,
        target_player_id,
        target_entity_id,
    });
    if tx.send(msg).await.is_err() {
        tracing::warn!(
            target: "duel",
            event = "duel.challenge_refused",
            %addr,
            account_id = actor.account_id,
            player_id = actor.player_id,
            entity_id = actor.entity_id,
            target_player_id,
            reason = "cell_channel_closed",
            "duel challenge could not be forwarded to the cell"
        );
    }
}

/// D-SS15 seam: does the target's session ignore `challenger_player_id`?
///
/// **Integration edit for SS-C1.** SS-C1 is building the Ignore cache on
/// `ConnectedClientState` in parallel; when it merges, this body becomes
/// that cache's membership check. Until then nothing is ignored, and the
/// refusal path above (`reason = target_ignoring`) is wired but unreachable.
fn ignores(_target: &ConnectedClientState, _challenger_player_id: i32) -> bool {
    false
}
