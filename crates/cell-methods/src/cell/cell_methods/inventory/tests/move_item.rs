use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

use super::super::dispatch::dispatch;
use super::super::MOVE_ITEM;
use super::make_test_space_mgr;

/// `handle_move_item`'s wire decoder must subtract 1 from the inbound
/// `target_slot_id` so the base-side handler operates in 0-indexed
/// server-internal coordinates. Mirrors legacy `SGWPlayer.py:2157`
/// `moveItem(itemId, targetBag, targetSlot - 1, quantity)`.
///
/// Without this translation a drag onto wire slot 1 (intended fist /
/// leftmost slot) would land at server slot 1 (the second slot from
/// the left), corrupting the player's bandolier layout.
#[tokio::test]
async fn handle_move_item_translates_wire_slot_to_server_slot() {
    use crate::cell::content::build_engine;

    for (wire_slot, expected_server_slot) in [(1i32, 0), (2, 1), (3, 2), (4, 3)] {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        let engine = build_engine(None).await;

        // moveItem wire layout (16 bytes):
        //   item_id              i32 LE @ 0..4
        //   target_container_id  i32 LE @ 4..8
        //   wire_slot_id         i32 LE @ 8..12   (1-indexed)
        //   quantity             i32 LE @ 12..16
        let mut args = Vec::with_capacity(16);
        args.extend_from_slice(&5000i32.to_le_bytes()); // item_id (sentinel)
        args.extend_from_slice(&3i32.to_le_bytes()); // target_container = bandolier
        args.extend_from_slice(&wire_slot.to_le_bytes());
        args.extend_from_slice(&1i32.to_le_bytes()); // quantity = 1

        dispatch(1, MOVE_ITEM, &args, &tx, &mut mgr, &engine).await;

        let msg = rx
            .try_recv()
            .expect("handle_move_item should forward MoveInventoryItem to base");
        match msg {
            CellToBaseMsg::MoveInventoryItem {
                target_slot_id,
                target_container_id,
                ..
            } => {
                assert_eq!(target_container_id, 3, "bag id forwarded as-is");
                assert_eq!(
                    target_slot_id, expected_server_slot,
                    "wire slot {wire_slot} must arrive at base as server slot \
                     {expected_server_slot} (= wire_slot - 1)"
                );
            }
            other => panic!("expected MoveInventoryItem, got {other:?}"),
        }
    }
}
