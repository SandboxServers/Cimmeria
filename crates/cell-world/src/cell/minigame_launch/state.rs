//! The pre-game minigame dialog (MG-21, D-MG19): the optional step between a
//! launch and the `StartMinigame` message.
//!
//! With the dialog switch on, a launch stores a [`PendingOffer`] and shows the
//! client its pre-game dialog (client 53). The player answers with cell 24
//! (`startMinigame`, [`accept_offer`]) or cell 30 (`minigameStartCancel`,
//! [`cancel_offer`]). With the switch off, [`start_or_offer`] is the direct
//! launch of MG-22.
//!
//! Two deviations from the original flow, both deliberate: the hold is taken
//! at launch, not when the dialog opens, and expiry is checked lazily when
//! cell 24 arrives. An unanswered dialog therefore never locks a player.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_wire::cell::client_methods::minigame::{
    ON_START_MINIGAME_DIALOG, ON_START_MINIGAME_DIALOG_CLOSE,
};
use cimmeria_wire::mercury::write_wstring;
use cimmeria_wire::state_field::{BSF_DEAD, BSF_PLAYING_MINIGAME};
use tokio::sync::mpsc;

use super::{launch_minigame, MinigameLaunchRequest, MINIGAME_TECH_COMPETENCY};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// How long an offer waits for the player's answer.
pub const OFFER_TTL: Duration = Duration::from_secs(60);

/// The offers waiting on a player's answer, and the launches queued behind
/// them (MG-24 fills the queue).
#[derive(Debug, Default)]
pub struct MinigameLaunchState {
    /// Open offers by entity id; a newer offer replaces the old one.
    pub offers: HashMap<u32, PendingOffer>,
    /// Launches waiting their turn.
    pub queue: Vec<MinigameLaunchRequest>,
}

/// A launch the player has been asked to confirm. `space_id` is the space the
/// entity was in when the dialog opened; accept refuses if it has moved.
#[derive(Debug, Clone)]
pub struct PendingOffer {
    pub req: MinigameLaunchRequest,
    pub space_id: u32,
    pub expires_at: Instant,
}

/// Off: launch now, as MG-22 does. On: store the offer (expiring after
/// [`OFFER_TTL`]) and send client 53. True when the launch was sent or the
/// dialog was shown.
pub async fn start_or_offer(
    req: MinigameLaunchRequest,
    start_dialog: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    if !start_dialog {
        return launch_minigame(req, tx, space_mgr).await;
    }
    let entity_id = req.entity_id;
    if space_mgr.get_entity(entity_id).is_none() {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            game_name = %req.game_name,
            reason = "entity_missing",
            "StartMinigame: entity not in the cell -- start dialog not shown"
        );
        return false;
    }
    let args = dialog_args(&req);
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_START_MINIGAME_DIALOG,
            args,
        })
        .await
    {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            game_name = %req.game_name,
            reason = "cell_to_base_send_failed",
            "StartMinigame: start dialog not sent -- minigame will not launch: {e}"
        );
        return false;
    }
    let expires_at = Instant::now() + OFFER_TTL;
    let space_id = space_mgr
        .entity_space
        .get(&entity_id)
        .copied()
        .unwrap_or_default();
    space_mgr.minigame_launch.offers.insert(
        entity_id,
        PendingOffer {
            req,
            space_id,
            expires_at,
        },
    );
    true
}

/// Cell 24. A live offer launches with source `"start_dialog"`. A missing or
/// expired offer, an offer whose entity now has another player or is dead or
/// has left its space, or a player already in a minigame, sends client 54 and
/// logs a WARN. True only when the launch was sent.
pub async fn accept_offer(
    entity_id: u32,
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(offer) = space_mgr.minigame_launch.offers.remove(&entity_id) else {
        return refuse_offer(entity_id, "no_offer", tx, space_mgr).await;
    };
    if now >= offer.expires_at {
        return refuse_offer(entity_id, "offer_expired", tx, space_mgr).await;
    }
    // Fails closed: an entity with no player id never matches an offer.
    let same_player = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.player_id)
        .is_some_and(|pid| pid == offer.req.player_id);
    if !same_player {
        return refuse_offer(entity_id, "player_mismatch", tx, space_mgr).await;
    }
    let dead = space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| e.state_field & BSF_DEAD != 0);
    let moved = space_mgr.entity_space.get(&entity_id) != Some(&offer.space_id);
    if dead || moved {
        return refuse_offer(entity_id, "stale_offer", tx, space_mgr).await;
    }
    let already_playing = space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| e.state_field & BSF_PLAYING_MINIGAME != 0);
    if already_playing {
        return refuse_offer(entity_id, "already_playing", tx, space_mgr).await;
    }
    let req = MinigameLaunchRequest {
        source: "start_dialog",
        ..offer.req
    };
    launch_minigame(req, tx, space_mgr).await
}

/// Cell 30. Remove the offer, INFO when one existed. Nothing is sent: no 54,
/// no chains and no base message.
pub fn cancel_offer(entity_id: u32, space_mgr: &mut SpaceManager) -> bool {
    let Some(offer) = space_mgr.minigame_launch.offers.remove(&entity_id) else {
        return false;
    };
    tracing::info!(
        entity_id,
        entity_name = space_mgr.entity_names(entity_id).entity_name,
        game_name = %offer.req.game_name,
        reason = "player_cancelled",
        "StartMinigame: start dialog cancelled"
    );
    true
}

/// WARN with `reason`, then send client 54 so the client closes its dialog.
async fn refuse_offer(
    entity_id: u32,
    reason: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> bool {
    tracing::warn!(
        entity_id,
        entity_name = space_mgr.entity_names(entity_id).entity_name,
        reason,
        "StartMinigame: start dialog answer refused"
    );
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_START_MINIGAME_DIALOG_CLOSE,
            args: Vec::new(),
        })
        .await
    {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            reason = "cell_to_base_send_failed",
            "StartMinigame: dialog close not sent: {e}"
        );
    }
    false
}

/// The client 53 arguments in the order `onStartMinigameDialog` declares them:
/// `Name`, `Difficulty`, `TCLevel`, `Verb`, `ArchetypeBitfield`, `CanPlay`,
/// `CanCall`, `CanSpectate`. Strings are wide strings, integers little-endian.
fn dialog_args(req: &MinigameLaunchRequest) -> Vec<u8> {
    let mut buf = Vec::new();
    write_wstring(&mut buf, &req.game_name);
    write_wstring(&mut buf, &format!("Difficulty {}", req.difficulty));
    buf.extend_from_slice(&(MINIGAME_TECH_COMPETENCY as i32).to_le_bytes());
    write_wstring(&mut buf, &req.game_name);
    buf.extend_from_slice(&0xFFFF_i32.to_le_bytes());
    buf.push(1); // CanPlay
    buf.push(0); // CanCall
    buf.push(0); // CanSpectate
    buf
}
