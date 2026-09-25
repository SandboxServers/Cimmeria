//! Base-originated fan-out of a player's entity-method call to the *other*
//! players who can see it.
//!
//! The base owns several pieces of player state that other players render —
//! the `BeingAppearance` it rebuilds from the DB on equip / holster / slot
//! change, the level it bumps on an XP grant — but it does not know who is
//! looking. Witness sets live on the cell, and mirroring them here would be a
//! second copy to leak on disconnect. So the base hands the already-built
//! args to the cell, which fans them out through the same
//! `send_entity_method_to_witnesses` path every cell-originated state change
//! uses, and they come back as one `CellToBaseMsg::WitnessEntityMethod` per
//! observer.

use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;

/// Ask the cell to send `method_index(args)` on `entity_id` to every player
/// currently witnessing it. Never addresses `entity_id`'s own client — the
/// caller has already done that.
///
/// A missing `cell_tx` is the no-cell-service configuration (unit tests,
/// base-only tooling) and is silent. A failed send means the cell loop is
/// gone; other players keep the stale view until they re-enter AoI, so it is
/// logged per docs/architecture/negative-logging-convention.md.
pub(crate) async fn broadcast_to_witnesses(
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
) {
    let Some(tx) = cell_tx else {
        return;
    };
    if let Err(e) = tx
        .send(BaseToCellMsg::BroadcastToWitnesses {
            entity_id,
            method_index,
            args,
        })
        .await
    {
        tracing::warn!(
            target: "aoi.witness_broadcast_failed",
            entity_id,
            method_index,
            reason = "cell_channel_closed",
            "BroadcastToWitnesses: base->cell send failed -- other players keep \
             the stale view of this entity until they re-enter its AoI: {e}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    /// Negative-log seam: a closed base->cell channel must not swallow the
    /// fan-out silently — other players are now looking at a stale entity.
    #[tokio::test]
    async fn closed_cell_channel_warns_with_entity_and_reason() {
        let capture = LogCapture::install();
        let (tx, rx) = mpsc::channel::<BaseToCellMsg>(1);
        drop(rx);

        broadcast_to_witnesses(&Some(tx), 42, 26, vec![1, 2, 3]).await;

        let warn = capture
            .find_event(Level::WARN, "BroadcastToWitnesses", "cell_channel_closed")
            .expect("closed cell channel must WARN");
        assert!(warn.has_field("entity_id", "42"), "{warn:#?}");
        assert!(warn.has_field("method_index", "26"), "{warn:#?}");
    }

    /// No cell service configured is a legitimate, silent no-op.
    #[tokio::test]
    async fn missing_cell_channel_is_silent() {
        let capture = LogCapture::install();

        broadcast_to_witnesses(&None, 42, 26, vec![]).await;

        assert!(capture
            .find_event(Level::WARN, "BroadcastToWitnesses", "cell_channel_closed")
            .is_none());
    }
}
