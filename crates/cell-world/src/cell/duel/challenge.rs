//! A challenge forwarded by the base: the cell-side checks (CAT-M-12) and
//! the prompt to the target.

use std::time::Instant;

use tokio::sync::mpsc;
use tracing::Instrument;

use cimmeria_wire::cell::client_methods::duel::{
    TEXT_ALREADY_IN_DUEL, TEXT_CHALLENGE_SELF, TEXT_CHALLENGE_SENT, TEXT_CHALLENGE_UNDELIVERED,
    TEXT_NOT_CLOSE_ENOUGH, TEXT_PAIR_COOLDOWN, TEXT_TARGET_BUSY, TEXT_TARGET_NOT_ONLINE,
};

use crate::cell::messages::{CellToBaseMsg, DuelBaseToCell};
use crate::cell::space_manager::SpaceManager;

use super::connected_player;
use super::limits::CHALLENGE_RANGE;
use super::outbound::{send_challenge_prompt, send_line, Recipient};
use super::registry::ChallengeRefusal;

/// Handle one duel message from the base, on the wall clock.
pub async fn handle(msg: DuelBaseToCell, tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager) {
    handle_at(msg, tx, mgr, Instant::now()).await;
}

/// [`handle`] on an explicit clock.
pub async fn handle_at(
    msg: DuelBaseToCell,
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    now: Instant,
) {
    let DuelBaseToCell::Challenge {
        player_id,
        entity_id,
        account_id,
        target_player_id,
        target_entity_id,
    } = msg;
    let span = tracing::info_span!(
        target: "duel",
        "duel.challenge",
        account_id,
        player_id,
        entity_id,
        target_player_id,
        target_entity_id,
    );
    let req = Request {
        player_id,
        entity_id,
        account_id,
        target_player_id,
        target_entity_id,
    };
    challenge(req, tx, mgr, now).instrument(span).await;
}

#[derive(Clone, Copy)]
struct Request {
    player_id: i32,
    entity_id: u32,
    account_id: u32,
    target_player_id: i32,
    target_entity_id: u32,
}

async fn challenge(
    req: Request,
    tx: &mpsc::Sender<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    now: Instant,
) {
    let Some(challenger) = connected_player(mgr, req.entity_id, req.player_id) else {
        // The challenger left (or the entity id was recycled) between the
        // base's check and this message: nobody to answer.
        tracing::debug!(
            target: "duel",
            event = "duel.challenge_refused",
            account_id = req.account_id,
            player_id = req.player_id,
            entity_id = req.entity_id,
            target_player_id = req.target_player_id,
            reason = "challenger_gone",
            "duel challenge dropped: the challenger is no longer in the world"
        );
        return;
    };

    if req.player_id == req.target_player_id {
        refuse(&req, tx, "self_challenge", TEXT_CHALLENGE_SELF, None).await;
        return;
    }
    let Some(target) = connected_player(mgr, req.target_entity_id, req.target_player_id) else {
        refuse(&req, tx, "target_gone", TEXT_TARGET_NOT_ONLINE, None).await;
        return;
    };
    if target.space_id != challenger.space_id {
        refuse(&req, tx, "cross_space", TEXT_NOT_CLOSE_ENOUGH, None).await;
        return;
    }
    let distance = challenger.position.distance_to(&target.position);
    if distance > CHALLENGE_RANGE {
        refuse(
            &req,
            tx,
            "out_of_range",
            TEXT_NOT_CLOSE_ENOUGH,
            Some(distance),
        )
        .await;
        return;
    }

    let pending = match mgr
        .duels
        .open_challenge(req.player_id, req.target_player_id, now)
    {
        Ok(p) => p,
        Err(refusal) => {
            let text = match refusal {
                ChallengeRefusal::SelfChallenge => TEXT_CHALLENGE_SELF,
                ChallengeRefusal::ChallengerBusy => TEXT_ALREADY_IN_DUEL,
                ChallengeRefusal::TargetBusy => TEXT_TARGET_BUSY,
                ChallengeRefusal::PairCooldown => TEXT_PAIR_COOLDOWN,
            };
            refuse(&req, tx, refusal.reason(), text, Some(distance)).await;
            return;
        }
    };

    tracing::debug!(
        target: "duel",
        event = "duel.challenge_sent",
        duel_id = pending.duel_id,
        account_id = req.account_id,
        player_id = req.player_id,
        entity_id = req.entity_id,
        target_player_id = req.target_player_id,
        target_entity_id = target.entity_id,
        target_account_id = target.account_id,
        space_id = challenger.space_id,
        distance,
        expires_in_ms = (pending.expires_at - now).as_millis() as u64,
        "duel challenge pending: target prompted"
    );
    let delivered = send_challenge_prompt(
        tx,
        Recipient::at(&target, req.target_player_id, Some(req.player_id)),
        challenger.entity_id,
        pending.duel_id,
    )
    .await;
    let challenger_to = Recipient::at(&challenger, req.player_id, Some(req.target_player_id));
    if !delivered {
        // The target never saw the prompt: withdraw the challenge now rather
        // than leave both players busy until the 30 s expiry.
        mgr.duels.cancel_pending(req.target_player_id);
        tracing::warn!(
            target: "duel",
            event = "duel.challenge_undelivered",
            duel_id = pending.duel_id,
            account_id = req.account_id,
            player_id = req.player_id,
            entity_id = req.entity_id,
            target_player_id = req.target_player_id,
            target_entity_id = target.entity_id,
            reason = "prompt_not_queued",
            "duel prompt could not be queued; challenge withdrawn"
        );
        send_line(
            tx,
            challenger_to,
            TEXT_CHALLENGE_UNDELIVERED,
            Some(pending.duel_id),
        )
        .await;
        return;
    }
    send_line(
        tx,
        challenger_to,
        TEXT_CHALLENGE_SENT,
        Some(pending.duel_id),
    )
    .await;
}

/// Log one refusal and send the challenger its line. The target is never
/// told about a challenge it was not shown.
async fn refuse(
    req: &Request,
    tx: &mpsc::Sender<CellToBaseMsg>,
    reason: &'static str,
    text: &str,
    distance: Option<f32>,
) {
    tracing::debug!(
        target: "duel",
        event = "duel.challenge_refused",
        account_id = req.account_id,
        player_id = req.player_id,
        entity_id = req.entity_id,
        target_player_id = req.target_player_id,
        target_entity_id = req.target_entity_id,
        reason,
        distance,
        range = CHALLENGE_RANGE,
        "duel challenge refused"
    );
    let to = Recipient {
        entity_id: req.entity_id,
        account_id: Some(req.account_id),
        player_id: req.player_id,
        other_player_id: Some(req.target_player_id),
    };
    send_line(tx, to, text, None).await;
}
