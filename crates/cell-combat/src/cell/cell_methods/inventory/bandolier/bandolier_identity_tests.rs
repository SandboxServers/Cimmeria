//! Bandolier row identity and numeric ids (TG-CMB-05): `active_slot_change`
//! and `weapon_ability_swap` carry the Rule 5 identity quartet, and `item_id`
//! is a bare number rather than the `Some(55)` string a `?opt` would log.

use tokio::sync::mpsc;
use tracing::Level;

use crate::test_support::{make_space_manager_with_player, LogCapture};
use cimmeria_entity::cell_entity::BandolierItem;

use super::handle_request_active_slot_change;

const ENTITY_ID: u32 = 1;

/// A player with a weapon (item 55) in slot 1 and the active slot at 0, so a
/// request for slot 1 is an immediate change with no holster choreography.
fn player_mgr() -> crate::cell::space_manager::SpaceManager {
    let mut mgr = make_space_manager_with_player(ENTITY_ID);
    // Item 55 grants ability 579 on the ranged event, so equipping it changes
    // the known set and the swap row fires.
    mgr.item_event_set_abilities
        .insert((55, crate::cell::spawner::EVENT_ITEM_RANGED), 579);
    let e = mgr.get_entity_mut(ENTITY_ID).unwrap();
    e.is_player = true;
    e.player_id = Some(72);
    e.account_id = Some(700);
    e.stamp_log_names(Some("Tester"), Some("tester_login"));
    e.bandolier_items.insert(
        1,
        BandolierItem {
            instance_id: 1011,
            item_id: 55,
            clip_size: 12,
            default_ammo_type: 7,
            current_ammo: 12,
            cur_ammo_type: 7,
        },
    );
    mgr
}

#[tokio::test]
async fn bandolier_rows_carry_identity_and_numeric_ids() {
    let capture = LogCapture::install();
    let mut mgr = player_mgr();
    let (tx, _rx) = mpsc::channel(16);
    // Args: bag 3 (bandolier), wire slot 2 (1-indexed), so server slot 1.
    handle_request_active_slot_change(ENTITY_ID, &[3, 0, 0, 0, 2, 0, 0, 0], &tx, &mut mgr).await;

    let change = capture
        .find_message(Level::INFO, "Active bandolier slot changed")
        .expect("active_slot_change row");
    assert!(change.has_field("event", "active_slot_change"));
    assert!(change.has_field("player_id", "72"));
    assert!(change.has_field("account_id", "700"));
    assert!(change.has_field("player_name", "Tester"));
    assert!(change.has_field("account_name", "tester_login"));
    assert!(change.has_field("item_id", "55"));

    let swap = capture
        .find_message(Level::INFO, "Per-weapon ability grant")
        .expect("weapon_ability_swap row");
    assert!(swap.has_field("event", "weapon_ability_swap"));
    assert!(swap.has_field("player_id", "72"));
    assert!(swap.has_field("account_id", "700"));
    assert!(swap.has_field("item_id", "55"));
}
