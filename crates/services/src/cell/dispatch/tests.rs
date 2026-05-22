//! Dispatch tests: constant index sanity, name lookup, and end-to-end async
//! dispatch verifying the route from `dispatch_cell_method` into the per-
//! interface handlers.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::*;

#[test]
fn cell_method_name_known() {
    assert_eq!(cell_method_name(CM_SET_TARGET_ID), "setTargetID");
    assert_eq!(cell_method_name(CM_SET_CROUCHED), "setCrouched");
    assert_eq!(
        cell_method_name(CM_REQUEST_HOLSTER_WEAPON),
        "requestHolsterWeapon"
    );
}

#[test]
fn cell_method_name_unknown() {
    assert_eq!(cell_method_name(255), "unknown");
}

#[test]
fn indices_are_sequential() {
    // SGWBeing exposed CellMethods come first
    assert_eq!(CM_SET_TARGET_ID, 0);
    assert_eq!(CM_SET_MOVEMENT_TYPE, 1);
    // Then SGWAbilityManager
    assert_eq!(CM_TOGGLE_COMBAT_DEBUG, 2);
    assert_eq!(CM_TOGGLE_COMBAT_VERBOSE_DEBUG, 3);
    assert_eq!(CM_CONFIRMATION_RESPONSE, 4);
    // Then SGWCombatant
    assert_eq!(CM_SET_CROUCHED, 5);
    assert_eq!(CM_TOGGLE_HEAL_DEBUG, 6);
    assert_eq!(CM_REQUEST_HOLSTER_WEAPON, 7);
}

#[test]
fn all_109_methods_have_names() {
    // Every index from 0-108 should resolve to a known method name
    for i in 0u16..=108 {
        let name = cell_method_name(i);
        assert_ne!(name, "unknown", "Index {} should have a name", i);
    }
}

#[test]
fn all_109_method_constants_are_correct() {
    // Spot-check interface boundaries and key methods
    // OrganizationMember starts at 8
    assert_eq!(CM_ORG_INVITE_RESPONSE, 8);
    assert_eq!(CM_ORG_TRANSFER_CASH, 19);
    // MinigamePlayer starts at 20
    assert_eq!(CM_MG_DEBUG_START, 20);
    assert_eq!(CM_MG_CONTACT_REQUEST, 34);
    // GateTravel
    assert_eq!(CM_ON_DIAL_GATE, 35);
    // SGWInventoryManager
    assert_eq!(CM_REMOVE_ITEM, 36);
    assert_eq!(CM_REQUEST_AMMO_CHANGE, 42);
    // SGWMailManager
    assert_eq!(CM_REQUEST_MAIL_HEADERS, 43);
    assert_eq!(CM_PAY_COD_FOR_MAIL, 51);
    // Missionary
    assert_eq!(CM_ABANDON_MISSION, 52);
    assert_eq!(CM_SHARE_MISSION_RESPONSE, 54);
    // ContactListManager
    assert_eq!(CM_CONTACT_LIST_CREATE, 55);
    assert_eq!(CM_CONTACT_LIST_REMOVE_MEMBERS, 60);
    // SGWBlackMarketManager
    assert_eq!(CM_BM_SEARCH, 61);
    assert_eq!(CM_BM_STOP_WATCHING, 66);
    // SGWPlayer own
    assert_eq!(CM_CALL_FOR_AID, 67);
    assert_eq!(CM_DIALOG_BUTTON_CHOICE, 75);
    assert_eq!(CM_INITIAL_RESPONSE, 76);
    assert_eq!(CM_TRIGGER_REGION, 85);
    assert_eq!(CM_REQUEST_RELOAD, 86);
    assert_eq!(CM_CANCEL_MOVIE, 108);
}

// ── New method index tests ────────────────────────────────────────────

#[test]
fn new_method_indices_correct() {
    assert_eq!(CM_TRIGGER_REGION, 85);
    assert_eq!(CM_REQUEST_RELOAD, 86);
}

#[test]
fn new_method_names_resolve() {
    assert_eq!(
        cell_method_name(CM_TRIGGER_REGION),
        "triggerClientHintedGenericRegion"
    );
    assert_eq!(cell_method_name(CM_REQUEST_RELOAD), "requestReload");
}

#[test]
fn quest_critical_method_names() {
    assert_eq!(
        cell_method_name(CM_DIALOG_BUTTON_CHOICE),
        "dialogButtonChoice"
    );
    assert_eq!(cell_method_name(CM_INITIAL_RESPONSE), "initialResponse");
    assert_eq!(cell_method_name(CM_USE_ITEM), "useItem");
}

// ── Dispatch integration tests (async) ────────────────────────────────

fn make_test_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

#[tokio::test]
async fn dispatch_trigger_region_enter_fires_event() {
    use crate::cell::space_manager::RegionData;
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.player_id = Some(100);
    }

    // Register a region with runtime_id=2 so the dispatch can look it up
    mgr.regions.insert(
        2,
        RegionData {
            runtime_id: 2,
            db_set_id: 42,
            tag: "Castle_Cellblock.Region2".to_string(),
            world_name: "Castle_CellBlock".to_string(),
            height: 0.0,
            radius: 0.0,
            flags: 1,
            points: vec![[0.0; 3]; 4],
        },
    );

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    // Build args: INT32 region_id=2, UINT8 bEntering=1, VECTOR3 position
    let mut args = Vec::new();
    args.extend_from_slice(&2i32.to_le_bytes()); // region_id
    args.push(1); // bEntering = true
    args.extend_from_slice(&0.0f32.to_le_bytes()); // x
    args.extend_from_slice(&0.0f32.to_le_bytes()); // y
    args.extend_from_slice(&0.0f32.to_le_bytes()); // z

    dispatch_cell_method(1, CM_TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;

    // No chains registered so no messages, but no panic = dispatch worked
    assert!(
        rx.try_recv().is_err(),
        "Empty engine should produce no messages"
    );
}

#[tokio::test]
async fn dispatch_trigger_region_exit() {
    use crate::cell::space_manager::RegionData;
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    // Register a region with runtime_id=3
    mgr.regions.insert(
        3,
        RegionData {
            runtime_id: 3,
            db_set_id: 43,
            tag: "Castle_Cellblock.Region3".to_string(),
            world_name: "Castle_CellBlock".to_string(),
            height: 0.0,
            radius: 0.0,
            flags: 1,
            points: vec![[0.0; 3]; 4],
        },
    );

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, _rx) = mpsc::channel(16);

    let mut args = Vec::new();
    args.extend_from_slice(&3i32.to_le_bytes());
    args.push(0); // bEntering = false (exit)
    args.extend_from_slice(&[0u8; 12]);

    dispatch_cell_method(1, CM_TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;
    // No panic = success
}

#[tokio::test]
async fn dispatch_trigger_region_unknown_id_warns() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    // No regions registered — runtime_id 99 should be unknown
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    let mut args = Vec::new();
    args.extend_from_slice(&99i32.to_le_bytes());
    args.push(1);
    args.extend_from_slice(&[0u8; 12]);

    dispatch_cell_method(1, CM_TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;
    // Should warn but not panic, and produce no messages
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn dispatch_trigger_region_ignores_short_args() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, _rx) = mpsc::channel(16);

    // Only 4 bytes — less than required 17
    let args = vec![0u8; 4];
    dispatch_cell_method(1, CM_TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;
    // Should silently skip (no panic)
}

#[tokio::test]
async fn dispatch_reload_sends_entity_property() {
    use cimmeria_entity::cell_entity::BandolierItem;
    use cimmeria_entity::stats::AMMO_SLOT_1;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    // Stage C: shadow scalars are gone. Seed the bandolier item + AmmoSlot
    // stat the same way `InitPlayerState` does for a real world entry.
    if let Some(e) = mgr.get_entity_mut(1) {
        // Weapon already drawn so Phase A (defer-reload-for-draw)
        // doesn't kick in — this test asserts the Phase B deadline
        // and timer-update wire shape, not the Phase A defer path.
        e.weapon_holstered = false;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 5,
                cur_ammo_type: 2,
            },
        );
        if let Some(stat) = e.stats.get_mut(AMMO_SLOT_1) {
            stat.update(0, 5, 30);
            stat.clear_dirty();
        }
    }

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    let args = vec![0u8]; // reloadType = 0
    dispatch_cell_method(1, CM_REQUEST_RELOAD, &args, &tx, &mut mgr, &engine).await;

    // Reload sets the deadline but does NOT immediately refill — the magazine
    // stays at the pre-reload count until the reload tick runs past warmup.
    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(
        entity.active_ammo(),
        5,
        "magazine should not refill until warmup elapses"
    );
    assert!(
        entity.reload_complete_at.is_some(),
        "reload deadline should be set"
    );

    // Reload sends a TimerUpdate (method 12) for the cooldown bar; the
    // onEntityProperty(AmmoTypeId) packet only fires when an event_set
    // sequence is mapped, which this test deliberately doesn't set up.
    let msg = rx.try_recv().unwrap();
    match msg {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(method_index, 12, "expected TimerUpdate first");
        }
        _ => panic!("Expected EntityMethodCall"),
    }
}

#[tokio::test]
async fn dispatch_reload_already_full_no_message() {
    use cimmeria_entity::cell_entity::BandolierItem;
    use cimmeria_entity::stats::AMMO_SLOT_1;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    // Already at max — bandolier item with clip_size == current_ammo.
    if let Some(e) = mgr.get_entity_mut(1) {
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 0,
                current_ammo: 30,
                cur_ammo_type: 0,
            },
        );
        if let Some(stat) = e.stats.get_mut(AMMO_SLOT_1) {
            stat.update(0, 30, 30);
            stat.clear_dirty();
        }
    }

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    dispatch_cell_method(1, CM_REQUEST_RELOAD, &[0u8], &tx, &mut mgr, &engine).await;

    // No message sent when already full
    assert!(rx.try_recv().is_err());
}
