//! Black Market action handler: open the client auction-house window.
//!
//! `Action::OpenBlackMarket` is the player-reachable entry into the Black
//! Market. It mirrors [`super::dialog::display`] exactly in how it routes
//! to the client: resolve the in-world auctioneer NPC, then push the
//! `onBMOpen(entityId)` client method (index 90) through the
//! cell→base `EntityMethodCall` channel — the same path dialogs and
//! kismet sequences use.
//!
//! It also records the auctioneer in the player's Black Market session
//! (`cell::black_market`, BM-02): cell methods 62-64 are honoured only for
//! a player the server sent to an auctioneer this way.
//!
//! Before it sends anything it checks the NPC really is an auctioneer
//! (BM-07, `auctioneer_check`): `NpcInteractionType::Auctioneer`, which only
//! a template's seeded `INT_AUCTION` bit gives, in the same space and within
//! interact distance. A chain bound to any other NPC opens nothing. Either
//! way the player gets one chat line, so the click is never silent, even on
//! a client without the patch that draws the window.

use cimmeria_cell_world::cell::black_market::{
    auctioneer_check, count_bm_outcome, BlackMarketReject,
};
use cimmeria_wire::black_market::serialize_on_bm_open;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use tokio::sync::mpsc;

use crate::cell::client_methods::black_market::ON_BM_OPEN;
use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `Action::OpenBlackMarket` — send `onBMOpen(auctioneerEntityId)` to the
/// triggering player so the client opens the Black Market window.
///
/// The wire `entityId` is the auctioneer NPC the player interacted with —
/// the client binds the window to it as the conversation partner. It is
/// resolved with the same precedence `dialog::display` uses:
///
/// 1. Chain `params["target_entity_id"]` — stamped by `fire_interact_tag`
///    when the chain fires off an `interact_tag` trigger (the normal path
///    for the auctioneer).
/// 2. Player's `last_interaction_target` pin (set by `handle_interact`) —
///    covers follow-up chains where the trigger carries no NPC.
/// 3. Abort with a `warn` — no auctioneer could be resolved, so opening
///    the window with the player's own id would mis-bind the partner.
///    Bail loud so a mis-wired chain (e.g. an `open_black_market` not
///    fired off an interact path) is diagnosable rather than silently
///    opening an empty window.
#[tracing::instrument(
    name = "black_market.open",
    level = "info",
    skip_all,
    fields(
        entity_id,
        chain_id,
        account_id = tracing::field::Empty,
        player_id = tracing::field::Empty,
        auctioneer_entity_id = tracing::field::Empty
    )
)]
pub(super) async fn open(
    entity_id: u32,
    chain_id: i64,
    params: &std::collections::HashMap<String, serde_json::Value>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let span = tracing::Span::current();
    if let Some(account_id) = id.account_id {
        span.record("account_id", account_id);
    }
    if let Some(player_id) = id.player_id {
        span.record("player_id", player_id);
    }

    let auctioneer_entity_id = params
        .get("target_entity_id")
        .and_then(|v| v.as_u64())
        .map(|v| v as i32)
        .or_else(|| {
            space_mgr
                .get_entity(entity_id)
                .and_then(|p| p.last_interaction_target)
                .map(|target| target as i32)
        });

    let auctioneer_entity_id = match auctioneer_entity_id {
        Some(auctioneer) => auctioneer,
        None => {
            tracing::warn!(
                entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                chain_id,
                "OpenBlackMarket: no auctioneer entity id in chain params or \
                 last_interaction_target -- cannot send onBMOpen (would open the \
                 Black Market with no bound auctioneer)"
            );
            return;
        }
    };

    span.record("auctioneer_entity_id", auctioneer_entity_id);

    // BM-07 authority: only a seeded auctioneer, here and in reach, opens
    // the Black Market. A negative id names no entity.
    let checked = u32::try_from(auctioneer_entity_id)
        .map_err(|_| BlackMarketReject::AuctioneerGone)
        .and_then(|auctioneer| auctioneer_check(entity_id, auctioneer, space_mgr));
    if let Err(reject) = checked {
        refuse_open(
            entity_id,
            chain_id,
            auctioneer_entity_id,
            reject,
            tx,
            space_mgr,
        )
        .await;
        return;
    }

    tracing::info!(
        entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        auctioneer_entity_id,
        chain_id,
        "Content: opening Black Market window"
    );

    let args = serialize_on_bm_open(auctioneer_entity_id);
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_BM_OPEN,
            args,
        })
        .await
    {
        // Dropped onBMOpen leaves the player stuck — they interacted with
        // the auctioneer and the window never opened. warn! because it's
        // player-visible (same shape as send_dialog_display).
        tracing::warn!(
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            auctioneer_entity_id,
            chain_id,
            "OpenBlackMarket: cell→base send failed -- Black Market not opened on client: {e}"
        );
        return;
    }

    count_bm_outcome("open", "ok");
    send_feedback(entity_id, OPENED_LINE, tx).await;

    // The window is open: this auctioneer is now the one cell methods 62-64
    // are checked against. `auctioneer_check` passed, so the id is a live
    // entity's.
    if let (Some(player_id), Ok(auctioneer)) = (id.player_id, u32::try_from(auctioneer_entity_id)) {
        space_mgr.black_market.open(player_id, auctioneer);
        let opens = space_mgr.black_market.get(player_id).map_or(0, |s| s.opens);
        tracing::debug!(
            event = "bm.open",
            entity_id,
            account_id = id.account_id,
            player_id,
            auctioneer_entity_id,
            opens,
            "Black Market session opened at an auctioneer"
        );
    }
}

/// The line a player gets on every open. A client without the patch drops
/// `onBMOpen`, so the line is the only thing that player sees.
pub(crate) const OPENED_LINE: &str =
    "The auctioneer opens the Black Market. (No window? The Black Market needs the Cimmeria client patch.)";

/// The line a refused open gives, by reason.
fn refusal_line(reject: BlackMarketReject) -> &'static str {
    match reject {
        BlackMarketReject::OutOfRange { .. } => "You are too far from the auctioneer.",
        BlackMarketReject::AuctioneerGone | BlackMarketReject::AuctioneerOtherSpace => {
            "The auctioneer is no longer here."
        }
        _ => "Nobody here runs the Black Market.",
    }
}

/// A refused open: nothing is sent but the player's chat line, no session
/// is recorded. `bm.open_refused` carries `reason = not_at_auctioneer` (the
/// `BMError` label the trade methods use for the same rule) and the exact
/// check in `access`. `not_an_auctioneer` is WARN: a chain bound to the
/// wrong NPC, an authoring bug. Walking away is INFO.
async fn refuse_open(
    entity_id: u32,
    chain_id: i64,
    auctioneer_entity_id: i32,
    reject: BlackMarketReject,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let dist = match reject {
        BlackMarketReject::OutOfRange { dist } => Some(dist),
        _ => None,
    };
    if reject == BlackMarketReject::NotAnAuctioneer {
        tracing::warn!(
            event = "bm.open_refused",
            reason = "not_at_auctioneer",
            access = reject.label(),
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            auctioneer_entity_id,
            chain_id,
            "OpenBlackMarket refused: the NPC is not an auctioneer (its template has no \r
             INT_Auction bit) -- the chain is bound to the wrong NPC"
        );
    } else {
        tracing::info!(
            event = "bm.open_refused",
            reason = "not_at_auctioneer",
            access = reject.label(),
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            auctioneer_entity_id,
            chain_id,
            dist,
            "OpenBlackMarket refused: the player is not at the auctioneer"
        );
    }
    count_bm_outcome("open", "not_at_auctioneer");
    send_feedback(entity_id, refusal_line(reject), tx).await;
}

/// One `SYSTEM` line on the feedback channel to the player's own client.
async fn send_feedback(entity_id: u32, text: &str, tx: &mpsc::Sender<CellToBaseMsg>) {
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
        })
        .await
    {
        tracing::warn!(
            event = "bm.feedback_send_failed",
            entity_id,
            reason = "base_channel_closed",
            "Black Market feedback line not queued: {e}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{make_space_manager, LogCapture};
    use cimmeria_entity::cell_entity::NpcInteractionType;
    use std::collections::HashMap;
    use tracing::Level;

    fn empty_params() -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }

    /// A player (entity 1, `player_id` 77) at the origin and an NPC at
    /// `pos` with `interaction`, in the Agnos space. Returns the NPC's id.
    fn player_and_npc(
        pos: [f32; 3],
        interaction: Option<NpcInteractionType>,
    ) -> (SpaceManager, u32) {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        mgr.get_entity_mut(1).unwrap().player_id = Some(77);
        let npc = mgr.allocate_npc_id();
        mgr.spawn_npc(npc, "Agnos", pos, [0.0; 3]).unwrap();
        mgr.get_entity_mut(npc).unwrap().interaction_type = interaction;
        (mgr, npc)
    }

    /// An auctioneer 2 units from the player: what the hub's template 305
    /// spawns as.
    fn at_auctioneer() -> (SpaceManager, u32) {
        player_and_npc([2.0, 0.0, 0.0], Some(NpcInteractionType::Auctioneer))
    }

    fn target(npc: u32) -> HashMap<String, serde_json::Value> {
        let mut params = empty_params();
        params.insert("target_entity_id".into(), serde_json::json!(npc as u64));
        params
    }

    fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    }

    /// The `onBMOpen` calls among `msgs`: (recipient, wire `entityId`).
    fn bm_opens(msgs: &[CellToBaseMsg]) -> Vec<(u32, i32)> {
        msgs.iter()
            .filter_map(|m| match m {
                CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index,
                    args,
                } if *method_index == ON_BM_OPEN => Some((
                    *entity_id,
                    i32::from_le_bytes([args[0], args[1], args[2], args[3]]),
                )),
                _ => None,
            })
            .collect()
    }

    /// Whether `msgs` carries, to the player, the exact bytes a feedback
    /// line of `text` serializes to.
    fn has_line(msgs: &[CellToBaseMsg], text: &str) -> bool {
        let want = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
        msgs.iter().any(|m| {
            matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: 1, method_index, args }
                if *method_index == ON_PLAYER_COMMUNICATION && *args == want)
        })
    }

    /// Interacting with the auctioneer (interact_tag chain) stamps
    /// `params["target_entity_id"]`; `onBMOpen` goes to the player's own
    /// client and its wire `entityId` is that auctioneer, not the player.
    /// The player also gets the open line.
    #[tokio::test]
    async fn open_uses_target_entity_id_from_chain_params() {
        let (mut mgr, npc) = at_auctioneer();
        let (tx, mut rx) = mpsc::channel(4);
        open(1, 7000, &target(npc), &tx, &mut mgr).await;

        let msgs = drain(&mut rx);
        assert_eq!(bm_opens(&msgs), vec![(1, npc as i32)]);
        assert!(has_line(&msgs, OPENED_LINE), "{msgs:?}");
    }

    /// Follow-up path: chain didn't stamp `target_entity_id`, so the
    /// handler falls back to the player's `last_interaction_target` pin.
    #[tokio::test]
    async fn open_falls_back_to_last_interaction_target() {
        let (mut mgr, npc) = at_auctioneer();
        mgr.get_entity_mut(1).unwrap().last_interaction_target = Some(npc);

        let (tx, mut rx) = mpsc::channel(4);
        open(1, 7001, &empty_params(), &tx, &mut mgr).await;

        assert_eq!(bm_opens(&drain(&mut rx)), vec![(1, npc as i32)]);
    }

    /// With neither source available, the handler must abort with a WARN
    /// rather than open a window bound to nothing. The WARN level is
    /// load-bearing: operators correlate "Black Market never opened" with
    /// a chain not fired off an interact path.
    #[tokio::test]
    async fn open_aborts_with_warn_when_no_auctioneer_resolves() {
        let capture = LogCapture::install();
        let (mut mgr, _npc) = at_auctioneer();
        // last_interaction_target intentionally None.

        let (tx, mut rx) = mpsc::channel(4);
        open(1, 9999, &empty_params(), &tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "must not emit onBMOpen when no auctioneer id can be resolved"
        );
        assert!(
            capture
                .find_message(Level::WARN, "OpenBlackMarket: no auctioneer entity id")
                .is_some(),
            "abort must surface a WARN — silent return masks a mis-wired chain"
        );
    }

    /// BM-02 authority: a successful open records the auctioneer in the
    /// player's session, which is what cell methods 62-64 are checked
    /// against. Without it every create, bid and cancel is refused.
    #[tokio::test]
    async fn open_records_the_auctioneer_in_the_session() {
        let (mut mgr, npc) = at_auctioneer();
        let (tx, _rx) = mpsc::channel(4);
        open(1, 7002, &target(npc), &tx, &mut mgr).await;

        let session = mgr.black_market.get(77).expect("session recorded");
        assert_eq!(session.auctioneer_id, Some(npc));
        assert_eq!(session.opens, 1);
    }

    /// No session is recorded when `onBMOpen` could not be sent: the player
    /// never saw a window, so there is nothing to trade at.
    #[tokio::test]
    async fn failed_open_records_no_session() {
        let (mut mgr, npc) = at_auctioneer();
        let (tx, rx) = mpsc::channel(4);
        drop(rx);
        open(1, 7003, &target(npc), &tx, &mut mgr).await;

        assert!(mgr.black_market.get(77).is_none());
    }

    /// BM-07 authority: a chain that runs `open_black_market` off any NPC
    /// but an auctioneer (a vendor, a quest giver, an NPC with no static
    /// type) opens nothing. No `onBMOpen`, no session, a WARN
    /// `bm.open_refused reason=not_at_auctioneer access=not_an_auctioneer`
    /// and a chat line. Fails if the open stops calling `auctioneer_check`.
    #[tokio::test]
    async fn open_at_an_npc_that_is_not_an_auctioneer_is_refused() {
        for other in [
            None,
            Some(NpcInteractionType::Vendor),
            Some(NpcInteractionType::Dialog { dialog_id: 5 }),
        ] {
            let capture = LogCapture::install();
            let (mut mgr, npc) = player_and_npc([2.0, 0.0, 0.0], other.clone());
            let (tx, mut rx) = mpsc::channel(4);
            open(1, 7004, &target(npc), &tx, &mut mgr).await;

            let msgs = drain(&mut rx);
            assert!(bm_opens(&msgs).is_empty(), "{other:?}: {msgs:?}");
            assert!(has_line(&msgs, "Nobody here runs the Black Market."));
            assert!(mgr.black_market.get(77).is_none(), "{other:?}");
            let row = capture
                .find_event(Level::WARN, "OpenBlackMarket refused", "not_at_auctioneer")
                .unwrap_or_else(|| panic!("{other:?}: WARN bm.open_refused"));
            assert!(row.has_field("access", "not_an_auctioneer"));
            assert!(row.has_field("event", "bm.open_refused"));
        }
    }

    /// A player who walked away from the auctioneer before a follow-up
    /// chain ran is refused at INFO with the distance, and told.
    #[tokio::test]
    async fn open_out_of_range_is_refused() {
        let capture = LogCapture::install();
        let (mut mgr, npc) = player_and_npc([40.0, 0.0, 0.0], Some(NpcInteractionType::Auctioneer));
        let (tx, mut rx) = mpsc::channel(4);
        open(1, 7005, &target(npc), &tx, &mut mgr).await;

        let msgs = drain(&mut rx);
        assert!(bm_opens(&msgs).is_empty());
        assert!(has_line(&msgs, "You are too far from the auctioneer."));
        let row = capture
            .find_event(Level::INFO, "OpenBlackMarket refused", "not_at_auctioneer")
            .expect("INFO bm.open_refused");
        assert!(row.has_field("access", "auctioneer_out_of_range"));
    }
}
