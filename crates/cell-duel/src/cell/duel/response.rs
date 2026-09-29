//! `sendDuelResponse` (CM 102): the target's answer (CAT-M-13).
//!
//! The only challenge a response can act on is the one addressed to the
//! caller's own `player_id`, read from the caller's cell entity. The payload
//! carries one byte and no ids, so a client cannot answer someone else's
//! challenge. The challenge is consumed on every path, so a replayed
//! response finds nothing.

use super::DuelResources;
use std::time::Instant;

use tokio::sync::mpsc;
use tracing::Instrument;

use cimmeria_common::Vector3;
use cimmeria_wire::cell::cell_methods::player::duel::{decode_send_duel_response, DuelResponse};
use cimmeria_wire::cell::client_methods::duel::{
    TEXT_DUEL_ABORTED, TEXT_DUEL_ACCEPTED, TEXT_NO_PENDING_CHALLENGE,
};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::limits::COUNTDOWN;
use super::outbound::{send_countdown, send_line, Recipient};
use super::registry::{PendingChallenge, ResponseRefusal};
use super::{connected_player, find_player};

/// Handle `sendDuelResponse` from `entity_id`, on the wall clock.
pub async fn handle(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
) {
    handle_at(entity_id, args, tx, mgr, Instant::now()).await;
}

/// [`handle`] on an explicit clock.
pub async fn handle_at(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    now: Instant,
) {
    let id = mgr.player_identity(entity_id);
    let span = tracing::info_span!(
        target: "duel",
        "duel.response",
        account_id = id.account_id,
        player_id = id.player_id,
        entity_id,
    );
    respond(entity_id, args, tx, mgr, now)
        .instrument(span)
        .await;
}

async fn respond(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    now: Instant,
) {
    let id = mgr.player_identity(entity_id);
    let response = match decode_send_duel_response(args) {
        Ok(r) => r,
        Err(e) => {
            // A forged or corrupted call: logged, not answered, and the
            // pending challenge (if any) is left for a well-formed answer.
            let pending = id
                .player_id
                .and_then(|pid| mgr.resources.duels().pending_for(pid));
            tracing::warn!(
                target: "duel",
                event = "duel.response_malformed",
                account_id = id.account_id,
                player_id = id.player_id,
                entity_id,
                target_player_id = pending.map(|p| p.challenger),
                duel_id = pending.map(|p| p.duel_id),
                args_len = args.len(),
                reason = e.reason(),
                error = ?e,
                "sendDuelResponse payload did not decode"
            );
            return;
        }
    };
    let Some(responder) = id
        .player_id
        .and_then(|pid| connected_player(mgr, entity_id, pid).map(|p| (pid, p)))
    else {
        tracing::warn!(
            target: "duel",
            event = "duel.response_refused",
            account_id = id.account_id,
            entity_id,
            response = response.name(),
            reason = "not_a_player",
            "sendDuelResponse from an entity that is not a connected player"
        );
        return;
    };
    let (responder_pid, responder) = responder;

    let pending = match mgr
        .resources
        .duels_mut()
        .take_pending_for(responder_pid, now)
    {
        Ok(p) => p,
        Err(ResponseRefusal::NoPending) => {
            tracing::debug!(
                target: "duel",
                event = "duel.response_refused",
                account_id = id.account_id,
                player_id = responder_pid,
                entity_id,
                response = response.name(),
                reason = "no_pending_challenge",
                "sendDuelResponse with no challenge addressed to the caller"
            );
            send_line(
                tx,
                Recipient::at(&responder, responder_pid, None),
                TEXT_NO_PENDING_CHALLENGE,
                None,
            )
            .await;
            return;
        }
        Err(ResponseRefusal::Expired(p)) => {
            tracing::debug!(
                target: "duel",
                event = "duel.response_refused",
                duel_id = p.duel_id,
                account_id = id.account_id,
                player_id = responder_pid,
                entity_id,
                target_player_id = p.challenger,
                response = response.name(),
                reason = "expired",
                expired_ms_ago = (now - p.expires_at).as_millis() as u64,
                "sendDuelResponse after the challenge expired"
            );
            abort_both(tx, mgr, &p, "expired").await;
            return;
        }
    };

    match response {
        DuelResponse::Decline => {
            mgr.resources.duels_mut().decline(&pending, now);
            tracing::debug!(
                target: "duel",
                event = "duel.declined",
                duel_id = pending.duel_id,
                account_id = id.account_id,
                player_id = responder_pid,
                entity_id,
                target_player_id = pending.challenger,
                "duel challenge declined"
            );
            abort_both(tx, mgr, &pending, "declined").await;
        }
        DuelResponse::Accept => {
            let challenger =
                find_player(mgr, pending.challenger).filter(|c| c.space_id == responder.space_id);
            let Some(challenger) = challenger else {
                // The challenger logged off or left the space while the
                // prompt was up. No duel; tell the responder.
                mgr.resources.duels_mut().decline(&pending, now);
                tracing::debug!(
                    target: "duel",
                    event = "duel.accept_refused",
                    duel_id = pending.duel_id,
                    account_id = id.account_id,
                    player_id = responder_pid,
                    entity_id,
                    target_player_id = pending.challenger,
                    reason = "challenger_gone",
                    "duel accepted but the challenger is no longer in the space"
                );
                abort_both(tx, mgr, &pending, "challenger_gone").await;
                return;
            };
            let centre = midpoint(challenger.position, responder.position);
            let duel =
                mgr.resources
                    .duels_mut()
                    .start_duel(&pending, responder.space_id, centre, now);
            tracing::debug!(
                target: "duel",
                event = "duel.accepted",
                duel_id = duel.duel_id,
                account_id = id.account_id,
                player_id = responder_pid,
                entity_id,
                target_player_id = pending.challenger,
                target_entity_id = challenger.entity_id,
                target_account_id = challenger.account_id,
                space_id = duel.space_id,
                state = "start_pending",
                "duel accepted: countdown started"
            );
            // The line, then the countdown the client shows as splash numbers
            // (`onTimerUpdate` type 14, SS-D2's trace of D-Q1).
            for to in [
                Recipient::at(&challenger, pending.challenger, Some(responder_pid)),
                Recipient::at(&responder, responder_pid, Some(pending.challenger)),
            ] {
                send_line(tx, to, TEXT_DUEL_ACCEPTED, Some(duel.duel_id)).await;
                send_countdown(tx, to, COUNTDOWN.as_secs_f32(), duel.duel_id).await;
            }
        }
    }
}

/// Tell both sides of `pending` "Duel aborted" (878), whichever of them is
/// still in the world. Shared with the expiry sweep.
pub(super) async fn abort_both(
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &SpaceManager,
    pending: &PendingChallenge,
    why: &'static str,
) {
    for (player_id, other) in [
        (pending.challenger, pending.target),
        (pending.target, pending.challenger),
    ] {
        match find_player(mgr, player_id) {
            Some(p) => {
                send_line(
                    tx,
                    Recipient::at(&p, player_id, Some(other)),
                    TEXT_DUEL_ABORTED,
                    Some(pending.duel_id),
                )
                .await;
            }
            None => tracing::debug!(
                target: "duel",
                event = "duel.notify_skipped",
                duel_id = pending.duel_id,
                player_id,
                target_player_id = other,
                why,
                reason = "player_not_in_world",
                "duel abort line not sent: the player is no longer in the world"
            ),
        }
    }
}

fn midpoint(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5, (a.z + b.z) * 0.5)
}
