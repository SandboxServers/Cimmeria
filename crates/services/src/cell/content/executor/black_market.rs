//! Black Market action handler: open the client auction-house window.
//!
//! `Action::OpenBlackMarket` is the player-reachable entry into the Black
//! Market. It mirrors [`super::dialog::display`] exactly in how it routes
//! to the client: resolve the in-world auctioneer NPC, then push the
//! `onBMOpen(entityId)` client method (index 90) through the
//! cell→base `EntityMethodCall` channel — the same path dialogs and
//! kismet sequences use.

use tokio::sync::mpsc;

use crate::base::black_market::wire;
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
    fields(entity_id, chain_id, auctioneer_entity_id = tracing::field::Empty)
)]
pub(super) async fn open(
    entity_id: u32,
    chain_id: i64,
    params: &std::collections::HashMap<String, serde_json::Value>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let auctioneer_entity_id = params
        .get("target_entity_id")
        .and_then(|v| v.as_u64())
        .map(|v| v as i32)
        .or_else(|| {
            space_mgr
                .get_entity(entity_id)
                .and_then(|p| p.last_interaction_target)
                .map(|id| id as i32)
        });

    let auctioneer_entity_id = match auctioneer_entity_id {
        Some(id) => id,
        None => {
            tracing::warn!(
                entity_id,
                chain_id,
                "OpenBlackMarket: no auctioneer entity id in chain params or \
                 last_interaction_target -- cannot send onBMOpen (would open the \
                 Black Market with no bound auctioneer)"
            );
            return;
        }
    };

    tracing::Span::current().record("auctioneer_entity_id", auctioneer_entity_id);
    tracing::info!(
        entity_id,
        auctioneer_entity_id,
        chain_id,
        "Content: opening Black Market window"
    );

    let args = wire::serialize_on_bm_open(auctioneer_entity_id);
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: crate::mercury::method_idx::ON_BM_OPEN,
            args,
        })
        .await
    {
        // Dropped onBMOpen leaves the player stuck — they interacted with
        // the auctioneer and the window never opened. warn! because it's
        // player-visible (same shape as send_dialog_display).
        tracing::warn!(
            entity_id,
            auctioneer_entity_id,
            chain_id,
            "OpenBlackMarket: cell→base send failed -- Black Market not opened on client: {e}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{make_space_manager, LogCapture};
    use std::collections::HashMap;
    use tracing::Level;

    fn empty_params() -> HashMap<String, serde_json::Value> {
        HashMap::new()
    }

    /// Interacting with the auctioneer (interact_tag chain) stamps
    /// `params["target_entity_id"]`; the `onBMOpen` wire `entityId` must
    /// be that auctioneer id, not the player's.
    #[tokio::test]
    async fn open_uses_target_entity_id_from_chain_params() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        const AUCTIONEER_ID: u32 = 0xA5C7;
        let mut params = empty_params();
        params.insert(
            "target_entity_id".into(),
            serde_json::json!(AUCTIONEER_ID as u64),
        );

        let (tx, mut rx) = mpsc::channel(4);
        open(
            /* entity_id */ 1, /* chain_id */ 7000, &params, &tx, &mgr,
        )
        .await;

        let msg = rx.try_recv().expect("must emit onBMOpen");
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(
                    method_index,
                    crate::mercury::method_idx::ON_BM_OPEN,
                    "must dispatch the onBMOpen client method (90)"
                );
                assert_eq!(
                    entity_id, 1,
                    "onBMOpen is delivered to the player's own client"
                );
                let wire_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(
                    wire_entity_id as u32, AUCTIONEER_ID,
                    "wire entityId must be the auctioneer, not the player"
                );
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }
    }

    /// Follow-up path: chain didn't stamp `target_entity_id`, so the
    /// handler falls back to the player's `last_interaction_target` pin.
    #[tokio::test]
    async fn open_falls_back_to_last_interaction_target() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        const AUCTIONEER_ID: u32 = 0xBEEF;
        if let Some(p) = mgr.get_entity_mut(1) {
            p.last_interaction_target = Some(AUCTIONEER_ID);
        }

        let params = empty_params();
        let (tx, mut rx) = mpsc::channel(4);
        open(1, 7001, &params, &tx, &mgr).await;

        let msg = rx.try_recv().expect("must emit onBMOpen");
        match msg {
            CellToBaseMsg::EntityMethodCall { args, .. } => {
                let wire_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(wire_entity_id as u32, AUCTIONEER_ID);
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }
    }

    /// With neither source available, the handler must abort with a WARN
    /// rather than open a window bound to nothing. The WARN level is
    /// load-bearing: operators correlate "Black Market never opened" with
    /// a chain not fired off an interact path.
    #[tokio::test]
    async fn open_aborts_with_warn_when_no_auctioneer_resolves() {
        let capture = LogCapture::install();
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        // last_interaction_target intentionally None.

        let params = empty_params();
        let (tx, mut rx) = mpsc::channel(4);
        open(1, 9999, &params, &tx, &mgr).await;

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
}
