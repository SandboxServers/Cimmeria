//! `/give` — grant an item to the caller via the shared `GrantItem` sink.

use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use super::{CellToBaseMsg, SpaceManager};

/// Hard cap on `/give` count. The count reaches `sgw_inventory.stack_size`
/// (an `integer` column with no CHECK constraint) via `GrantItem`. A
/// **negative** count would subtract from an existing stack on the
/// stack-merge path — a fat-finger (`/give 55 -5`) that silently destroys
/// items — and an absurd count is an overflow/abuse surface. The `/give`
/// handler rejects non-positive counts and clamps the upper bound here so
/// the raw client-typed value never reaches the inventory SQL (CAT-N-11
/// signed-quantity footgun; flagged by the server-authority audit).
const GIVE_COUNT_CAP: i32 = 1000;

/// `/give <item_id> [count]` — grant an item to the caller. Base persists to
/// `sgw_inventory` and pushes the client inventory update; container 1 is the
/// backpack default (matching `content::executor::inventory::grant`).
pub(super) async fn handle_give(
    caller_entity_id: u32,
    item_id: i32,
    count: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(player_id) = space_mgr
        .get_entity(caller_entity_id)
        .and_then(|e| e.player_id)
    else {
        send_gm_feedback(caller_entity_id, "Give failed: you are not a player.", tx).await;
        return;
    };

    // Reject non-positive counts and clamp the upper bound BEFORE the value
    // reaches `GrantItem` → the inventory SQL. A negative count subtracts
    // from an existing stack on the merge path (silent item destruction);
    // the column has no CHECK constraint, so the guard lives here
    // (server-authority audit, CAT-N-11). The shared `handle_grant_item`
    // sink is NOT the place to clamp — the content-engine grant path also
    // uses it with trusted counts.
    if count < 1 {
        send_gm_feedback(
            caller_entity_id,
            "Give failed: count must be a positive number.",
            tx,
        )
        .await;
        return;
    }
    let count = count.min(GIVE_COUNT_CAP);

    const DEFAULT_CONTAINER: i32 = 1; // backpack
    if let Err(e) = tx
        .send(CellToBaseMsg::GrantItem {
            entity_id: caller_entity_id,
            player_id,
            item_id,
            container_id: DEFAULT_CONTAINER,
            count,
        })
        .await
    {
        tracing::warn!(caller_entity_id, item_id, error = %e, "gm_command: GrantItem send failed");
        send_gm_feedback(caller_entity_id, "Give failed: internal error.", tx).await;
        return;
    }

    send_gm_feedback(
        caller_entity_id,
        &format!("Gave you {count}x item {item_id}."),
        tx,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::super::tests_common::*;
    use super::super::{handle_gm_command, GmCommandIntent};
    use super::*;

    #[tokio::test]
    async fn give_emits_grant_item_to_caller() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::Give {
                item_id: 55,
                count: 2,
            },
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let msgs = drain(&mut rx);
        let grant = msgs.iter().find_map(|m| match m {
            CellToBaseMsg::GrantItem {
                entity_id,
                player_id,
                item_id,
                container_id,
                count,
            } => Some((*entity_id, *player_id, *item_id, *container_id, *count)),
            _ => None,
        });
        assert_eq!(
            grant,
            Some((1, 100, 55, 1, 2)),
            "GrantItem must target the caller"
        );
        let fb = feedback_text_to(&msgs, 1).unwrap();
        assert!(fb.contains("item 55"), "got: {fb}");
    }

    /// **HIGH-1 (CAT-N-11) regression guard.** A negative `/give` count
    /// must be REJECTED — never reach `GrantItem`, because a negative
    /// stack_size subtracts from an existing stack (silent item
    /// destruction). Reverting the `count < 1` guard trips this: the test
    /// would see a `GrantItem` emitted with count -5.
    #[tokio::test]
    async fn give_rejects_negative_count() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::Give {
                item_id: 55,
                count: -5,
            },
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let msgs = drain(&mut rx);
        assert!(
            !msgs
                .iter()
                .any(|m| matches!(m, CellToBaseMsg::GrantItem { .. })),
            "a negative count must NOT emit GrantItem (would corrupt stack_size)"
        );
        let fb = feedback_text_to(&msgs, 1).unwrap();
        assert!(fb.contains("must be a positive"), "got: {fb}");
    }

    /// **HIGH-1 upper-bound guard.** An absurd `/give` count is clamped to
    /// `GIVE_COUNT_CAP` before it reaches the inventory SQL.
    #[tokio::test]
    async fn give_clamps_absurd_count() {
        let mut mgr = mgr_with_player();
        let (tx, mut rx) = mpsc::channel(8);
        handle_gm_command(
            1,
            GmCommandIntent::Give {
                item_id: 55,
                count: 2_000_000_000,
            },
            &tx,
            &mut mgr,
            &[],
        )
        .await;

        let granted_count = drain(&mut rx).into_iter().find_map(|m| match m {
            CellToBaseMsg::GrantItem { count, .. } => Some(count),
            _ => None,
        });
        assert_eq!(
            granted_count,
            Some(GIVE_COUNT_CAP),
            "an absurd count must be clamped to GIVE_COUNT_CAP before GrantItem"
        );
    }
}
