use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

use super::super::dispatch::dispatch;
use super::super::REQUEST_ACTIVE_SLOT_CHANGE;
use super::make_test_space_mgr;

/// REQUEST_ACTIVE_SLOT_CHANGE must reject any wire slot that maps outside
/// the bandolier's server-side range (`0..bag_max_slots(3)` = `0..4`)
/// before mutating active_bandolier_slot or sending ActiveSlotUpdate. A
/// forged value would otherwise leave the entity in an impossible state.
///
/// Wire ↔ server translation is `server = wire - 1`, so the rejected wire
/// values are: 0 (→ server -1), 5+ (→ server 4+), and any negative wire
/// value (which the legacy client never sends but a forged packet might).
#[tokio::test]
async fn request_active_slot_change_rejects_out_of_range_slot() {
    use crate::cell::content::build_engine;

    // Wire values that translate to invalid server slots:
    //   0  → server -1 (below range)
    //   5  → server  4 (above the 4-slot bandolier)
    //   99 → server 98
    //   -1 → server -2 (already below the floor)
    //   i32::MAX → server i32::MAX-1 (above range)
    //   i32::MIN → server i32::MIN (saturating_sub guards against debug-overflow panic)
    for bad_wire_slot in [0i32, 5, 99, -1, i32::MAX, i32::MIN] {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.active_bandolier_slot = 0;
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        let engine = build_engine(None).await;

        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&3i32.to_le_bytes()); // bag_id = 3
        args.extend_from_slice(&bad_wire_slot.to_le_bytes());

        let handled = dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;
        assert!(
            handled,
            "handler claims the index even when slot is invalid (bad_wire_slot={bad_wire_slot})"
        );
        assert!(
            rx.try_recv().is_err(),
            "no messages emitted for bad_wire_slot={bad_wire_slot}"
        );
        assert_eq!(
            mgr.get_entity(1).unwrap().active_bandolier_slot,
            0,
            "active_bandolier_slot must not change for bad_wire_slot={bad_wire_slot}"
        );
    }
}

/// Wire slot IDs are 1-indexed by client convention (`Bag.py:369`,
/// `SGWPlayer.py:2192`). The wire decoder must subtract 1 before any
/// mutation so the cell's `active_bandolier_slot` and the
/// `ActiveSlotUpdate` message both carry the 0-indexed server slot.
///
/// Pins the translation: wire 1..=4 ↔ server 0..=3 across the entire
/// 4-slot bandolier range. A regression here was the original cause of
/// "switching between slots does not work at all" — the server treated
/// wire 1 as server slot 1 (the second weapon, not the first), so
/// keypress 1 either no-op'd or jumped to the wrong weapon.
#[tokio::test]
async fn request_active_slot_change_translates_wire_to_server_slot() {
    use crate::cell::content::build_engine;
    use cimmeria_entity::cell_entity::BandolierItem;

    for (wire_slot, expected_server_slot) in [(1i32, 0), (2, 1), (3, 2), (4, 3)] {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            // Seed all 4 slots so the swap doesn't bail on "empty target".
            for slot_id in 0..4 {
                e.bandolier_items.insert(
                    slot_id,
                    BandolierItem {
                        item_id: 100 + slot_id,
                        clip_size: 30,
                        default_ammo_type: 1,
                        current_ammo: 30,
                        cur_ammo_type: 1,
                    },
                );
            }
            // Start at a slot that's distinct from every test case so each
            // iteration genuinely exercises the swap path.
            e.active_bandolier_slot = if expected_server_slot == 0 { 3 } else { 0 };
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        let engine = build_engine(None).await;

        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&3i32.to_le_bytes());
        args.extend_from_slice(&wire_slot.to_le_bytes());

        dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;

        assert_eq!(
            mgr.get_entity(1).unwrap().active_bandolier_slot,
            expected_server_slot,
            "wire slot {wire_slot} must land in cell as server slot {expected_server_slot}"
        );

        // Confirm two pieces of state both reflect the new server slot:
        //   1. `ActiveSlotUpdate` to base: the persisted `bandolier_slot`
        //      column must store the server-side index, not the wire-side
        //      one (would cause the appearance query to filter the wrong
        //      bandolier row and render the player without a weapon).
        //   2. `onActiveSlotUpdate` (method 70) to the client: the
        //      bandolier UI indicator must learn the new slot, otherwise
        //      the LUA `getActiveSlotForContainer(...) ~= N` guard turns
        //      subsequent keypresses for the slot it thinks is selected
        //      into client-side no-ops. That's the bug behind "switching
        //      back to the first bandolier slot does not give me my
        //      weapon back" during play-testing.
        let mut saw_active_slot_update = false;
        let mut saw_client_indicator = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::ActiveSlotUpdate { slot_id, .. } => {
                    assert_eq!(
                        slot_id, expected_server_slot,
                        "ActiveSlotUpdate must carry the server slot, not the wire slot"
                    );
                    saw_active_slot_update = true;
                }
                CellToBaseMsg::EntityMethodCall {
                    method_index, args, ..
                } if method_index
                    == crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE =>
                {
                    let bag_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                    let wire_slot_field = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                    assert_eq!(bag_id, 3, "bandolier bag id on the wire indicator");
                    assert_eq!(
                        wire_slot_field,
                        expected_server_slot + 1,
                        "client indicator must carry wire slot (= server slot + 1)"
                    );
                    saw_client_indicator = true;
                }
                _ => {}
            }
        }
        assert!(
            saw_active_slot_update,
            "expected an ActiveSlotUpdate message for wire_slot={wire_slot}"
        );
        assert!(
            saw_client_indicator,
            "expected an onActiveSlotUpdate (client UI indicator) for wire_slot={wire_slot}"
        );
    }
}
