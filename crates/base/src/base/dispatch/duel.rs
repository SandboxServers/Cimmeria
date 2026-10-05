//! `sendDuelChallenge` (SGWPlayer base method 0xD9): the base half of a
//! duel challenge (SS-D1).
//!
//! The base checks what only it can see, in this order, then forwards the
//! challenge to the cell as `DuelBaseToCell::Challenge`:
//!
//! 1. the duel bucket (D-SS21): first, so every refused challenge costs a
//!    token and a client cannot turn a flood of bad packets into a flood of
//!    feedback lines (the chat path's order, `dispatch/chat.rs`);
//! 2. the challenger must be client-ready ([`is_client_ready`]): not mid
//!    world entry or gate travel;
//! 3. squad duels are refused (the ledger's first check; see
//!    [`TEXT_SQUAD_DUEL_UNSUPPORTED`] for why not text 874);
//! 4. the target, resolved against the online index (D-SS13): exact name,
//!    else a unique case-insensitive match, and client-ready too;
//! 5. Ignore (D-SS15): a target who ignores the challenger refuses it.
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
    TEXT_CHALLENGER_LOADING, TEXT_CHALLENGE_UNDELIVERED, TEXT_SQUAD_DUEL_UNSUPPORTED,
    TEXT_TARGET_AMBIGUOUS, TEXT_TARGET_IGNORING, TEXT_TARGET_LOADING, TEXT_TARGET_NOT_ONLINE,
};
use tokio::sync::mpsc;

use crate::cell::messages::{BaseToCellMsg, DuelBaseToCell};

use super::super::feedback::{send_feedback_line, FeedbackCtx};
use super::super::rate_limit::{log_exceeded, RateActor, RateCategory, RateDecision};
use super::super::ConnectedClientState;
use cimmeria_base_session::base::player_index::{NameLookup, OnlinePlayerIndex};
use cimmeria_base_session::base::session_identity::session_identity;

/// Longest prefix of a typed name a log row carries.
const LOGGED_NAME_CHARS: usize = 64;

/// The challenger, from this session.
#[derive(Debug, Clone, Copy)]
struct Actor {
    player_id: i32,
    account_id: u32,
    entity_id: u32,
    player_name: Option<&'static str>,
    account_name: Option<&'static str>,
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

    // The bucket, then the identity, under one lock. The token is taken
    // before anything else is checked, so a session with no player in the
    // world cannot flood 0xD9 for free either.
    let (actor, decision, challenger_ready, session) = {
        let mut clients = connected.lock().unwrap();
        let Some(c) = clients.get_mut(&addr) else {
            return;
        };
        let identity = session_identity(c);
        let session = (
            c.account_id,
            c.active_player_id,
            c.player_entity_id,
            identity,
        );
        let actor = match (c.active_player_id, c.player_entity_id) {
            (Some(player_id), Some(entity_id)) => Some(Actor {
                player_id,
                account_id: c.account_id,
                entity_id,
                player_name: identity.player_name,
                account_name: identity.account_name,
            }),
            _ => None,
        };
        let decision = c.rate_limits.check(RateCategory::DuelChallenge, now);
        if let RateDecision::Limited { notify } = decision {
            let rate_actor = RateActor::of(addr, c);
            log_exceeded(
                RateCategory::DuelChallenge,
                rate_actor,
                notify,
                &c.rate_limits,
                now,
            );
        }
        (actor, decision, is_client_ready(c), session)
    };

    if let RateDecision::Limited { notify } = decision {
        if notify {
            send_feedback_line(&feedback, addr, RateCategory::DuelChallenge.feedback_text()).await;
        }
        return;
    }

    let Some(actor) = actor else {
        let (account_id, player_id, entity_id, identity) = session;
        tracing::warn!(
            target: "duel",
            event = "duel.challenge_refused",
            %addr,
            account_id,
            account_name = identity.account_name,
            player_id,
            player_name = identity.player_name,
            entity_id,
            entity_name = identity.player_name,
            reason = "not_in_world",
            "sendDuelChallenge from a session with no player in the world"
        );
        return;
    };

    let call = match decode_send_duel_challenge(payload) {
        Ok(call) => call,
        Err(e) => {
            // A forged or corrupted call: logged, not answered.
            tracing::warn!(
                target: "duel",
                event = "duel.challenge_malformed",
                %addr,
                account_id = actor.account_id,
                account_name = actor.account_name,
                player_id = actor.player_id,
                player_name = actor.player_name,
                entity_id = actor.entity_id,
                entity_name = actor.player_name,
                reason = e.reason(),
                error = %e,
                "sendDuelChallenge payload did not decode"
            );
            return;
        }
    };
    let shown: String = call.player_name.chars().take(LOGGED_NAME_CHARS).collect();

    let refuse = |reason: &'static str, target: Option<(i32, Option<&'static str>)>| {
        let target_player_id = target.map(|t| t.0);
        let target_player_name = target.and_then(|t| t.1);
        tracing::debug!(
            target: "duel",
            event = "duel.challenge_refused",
            %addr,
            account_id = actor.account_id,
            account_name = actor.account_name,
            player_id = actor.player_id,
            player_name = actor.player_name,
            entity_id = actor.entity_id,
            entity_name = actor.player_name,
            target_player_id,
            target_player_name,
            target_name = %shown,
            squad_duel = call.squad_duel,
            reason,
            "duel challenge refused at the base"
        );
    };

    if !challenger_ready {
        refuse("challenger_loading", None);
        send_feedback_line(&feedback, addr, TEXT_CHALLENGER_LOADING).await;
        return;
    }

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
                let target_name = target_state.and_then(|t| {
                    cimmeria_entity::name_intern::intern_opt(t.player_name.as_deref())
                });
                match target_state.and_then(|t| t.player_entity_id) {
                    Some(_) if !target_state.is_some_and(is_client_ready) => Err((
                        "target_loading",
                        TEXT_TARGET_LOADING,
                        Some((target.player_id, target_name)),
                    )),
                    Some(target_entity_id) => Ok((
                        (target.player_id, target_name),
                        target_entity_id,
                        target_state.is_some_and(|t| ignores(t, actor.player_id)),
                    )),
                    None => Err((
                        "target_not_in_world",
                        TEXT_TARGET_NOT_ONLINE,
                        Some((target.player_id, target_name)),
                    )),
                }
            }
            NameLookup::Ambiguous => Err(("target_ambiguous", TEXT_TARGET_AMBIGUOUS, None)),
            NameLookup::NotFound => Err(("target_not_online", TEXT_TARGET_NOT_ONLINE, None)),
        }
    };
    let ((target_player_id, target_player_name), target_entity_id, ignored) = match resolved {
        Ok(t) => t,
        Err((reason, text, target)) => {
            refuse(reason, target);
            send_feedback_line(&feedback, addr, text).await;
            return;
        }
    };
    if ignored {
        refuse(
            "target_ignoring",
            Some((target_player_id, target_player_name)),
        );
        send_feedback_line(&feedback, addr, TEXT_TARGET_IGNORING).await;
        return;
    }

    let Some(tx) = cell_tx else {
        tracing::warn!(
            target: "duel",
            event = "duel.challenge_refused",
            %addr,
            account_id = actor.account_id,
            account_name = actor.account_name,
            player_id = actor.player_id,
            player_name = actor.player_name,
            entity_id = actor.entity_id,
            entity_name = actor.player_name,
            target_player_id,
            target_player_name,
            reason = "no_cell_channel",
            "duel challenge not forwarded: no cell channel"
        );
        send_feedback_line(&feedback, addr, TEXT_CHALLENGE_UNDELIVERED).await;
        return;
    };
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
            account_name = actor.account_name,
            player_id = actor.player_id,
            player_name = actor.player_name,
            entity_id = actor.entity_id,
            entity_name = actor.player_name,
            target_player_id,
            target_player_name,
            reason = "cell_channel_closed",
            "duel challenge could not be forwarded to the cell"
        );
        send_feedback_line(&feedback, addr, TEXT_CHALLENGE_UNDELIVERED).await;
        return;
    }
    // Logged only once the cell has the challenge: the error path above
    // logs `cell_channel_closed` instead, never both.
    tracing::debug!(
        target: "duel",
        event = "duel.challenge_forwarded",
        %addr,
        account_id = actor.account_id,
        account_name = actor.account_name,
        player_id = actor.player_id,
        player_name = actor.player_name,
        entity_id = actor.entity_id,
        entity_name = actor.player_name,
        target_player_id,
        target_player_name,
        target_entity_id,
        target_entity_name = target_player_name,
        "duel challenge passed the base checks; forwarded to the cell"
    );
}

/// The session's character is in the world and its client has created the
/// player entity: listed online (`onClientReady` done, no `logOff`) and no
/// world-entry step outstanding. Gate travel keeps the listing but sets
/// `pending_world_entry`, then `pending_map_loaded`, then
/// `pending_client_ready`, so a traveller mid-load is not ready either.
fn is_client_ready(c: &ConnectedClientState) -> bool {
    c.listed_online
        && c.pending_world_entry.is_none()
        && c.pending_map_loaded.is_none()
        && c.pending_client_ready.is_none()
}

/// D-SS15 seam: does the target's session ignore `challenger_player_id`?
///
/// The target's cached Ignore list (SS-C1) holds the challenger's
/// character. The cache resolves every entry to `player_id`s at each world
/// entry and Ignore change, matching names case-insensitively (D-SS13), so
/// the check works from the id the session gives us.
fn ignores(target: &ConnectedClientState, challenger_player_id: i32) -> bool {
    target.ignore.ignores_player(challenger_player_id)
}
