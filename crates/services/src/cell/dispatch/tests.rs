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

// ── GM gate (#475 / CAT-N-03) ─────────────────────────────────────────

/// **#475 negative case.** A non-GM caller (access_level 0) sending a
/// GM-gated cell method (`onWorldInstanceReset`, CM 92) must be rejected
/// at the dispatch layer: a `warn!` audit log fires, an `onErrorCode`
/// wire response goes back to the caller, and the method never reaches
/// its handler. Reverting the gate lets the call fall through to the
/// stub handler (which would, once implemented, tear down the space).
#[tokio::test]
async fn gm_gated_method_rejected_for_non_gm_caller() {
    use crate::test_support::LogCapture;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.player_id = Some(100);
        e.access_level = 0; // Player — explicit for clarity
    }

    let capture = LogCapture::install();
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    dispatch_cell_method(1, CM_WORLD_INSTANCE_RESET, &[], &tx, &mut mgr, &engine).await;

    // Audit warn with the structured fields ops would pivot on.
    let event = capture
        .find_message(tracing::Level::WARN, "GM-gated cell method rejected")
        .expect("non-GM call to a GM-gated method must emit the rejection warn");
    assert!(event.has_field("method_index", "92"));
    assert!(event.has_field("access_level", "0"));

    // Wire-visible rejection: onErrorCode (121) back to the caller.
    let msg = rx
        .try_recv()
        .expect("rejection must send a wire-visible onErrorCode response");
    match msg {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(method_index, 121, "onErrorCode");
            // SystemID(u8) + InstanceID(i32 LE = method index) + ErrorCodeID(u16)
            assert_eq!(args.len(), 7);
            assert_eq!(
                i32::from_le_bytes([args[1], args[2], args[3], args[4]]),
                92,
                "InstanceID carries the rejected method index"
            );
        }
        other => panic!("expected onErrorCode EntityMethodCall, got {other:?}"),
    }
    // Nothing else on the wire — the handler never ran.
    assert!(
        rx.try_recv().is_err(),
        "a rejected GM call must produce only the onErrorCode response"
    );
}

/// **#475 positive case.** A GM caller (access_level 2 = GameMaster)
/// passes the gate: an authorization `info!` fires and no `onErrorCode`
/// rejection is sent. The method then reaches its (stub) handler, which
/// for CM 92 logs `UNIMPLEMENTED` and sends nothing — so the absence of
/// an onErrorCode is the observable signal that the gate let it through.
#[tokio::test]
async fn gm_gated_method_allowed_for_gm_caller() {
    use crate::test_support::LogCapture;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.player_id = Some(100);
        e.access_level = 2; // GameMaster
    }

    let capture = LogCapture::install();
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    dispatch_cell_method(1, CM_WORLD_INSTANCE_RESET, &[], &tx, &mut mgr, &engine).await;

    assert!(
        capture
            .find_message(tracing::Level::INFO, "GM-gated cell method authorized")
            .is_some(),
        "GM caller must pass the gate with an authorization info log"
    );
    // No onErrorCode rejection — the call was authorized through to the
    // (stub) handler.
    assert!(
        rx.try_recv().is_err(),
        "authorized GM call must not emit an onErrorCode rejection"
    );
}

/// An ordinary (non-gated) player method must be completely unaffected by
/// the gate even for an access_level 0 caller — the gate only intercepts
/// the restricted index set.
#[tokio::test]
async fn non_gated_method_unaffected_by_gate_for_player() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.player_id = Some(100);
        e.access_level = 0;
    }

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    // setTargetID (CM 0) with a target id — an ordinary player method.
    let args = 5i32.to_le_bytes().to_vec();
    dispatch_cell_method(1, CM_SET_TARGET_ID, &args, &tx, &mut mgr, &engine).await;

    // No onErrorCode (the gate didn't fire); the method ran normally.
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            assert_ne!(
                method_index, 121,
                "a non-gated player method must never trip the GM-gate onErrorCode"
            );
        }
    }
}

// ── SGWGmPlayer tail (109+) routing through the gate (#473 / CAT-N-04) ──

/// **#473 negative case + regression guard.** A non-GM caller (access_level
/// 0) sending an SGWGmPlayer cell method (>= 109) must be rejected at the
/// dispatch gate BEFORE any handler runs: the rejection `warn!` fires, an
/// `onErrorCode` goes back, and the gm handler never executes (so no
/// GrantItem / TeleportPlayer side effect leaks onto the wire).
///
/// We use `gmGiveItem` (133) with valid args — if the gate were reverted, the
/// handler WOULD run and emit a `GrantItem`. The assertion that no `GrantItem`
/// appears (only the `onErrorCode`) is the revert-verifier: drop the
/// `index >= 109` arm in `requires_gm` and a non-GM reaches the handler,
/// failing this test.
#[tokio::test]
async fn gm_tail_method_rejected_for_non_gm_caller() {
    use crate::test_support::LogCapture;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.player_id = Some(100);
        e.access_level = 0; // Player — NOT a GM
    }

    let capture = LogCapture::install();
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    // gmGiveItem(133): WSTRING "1234" + INT32 qty=1 — well-formed, so the only
    // reason it wouldn't grant is the gate intercepting it.
    let mut args = Vec::new();
    crate::mercury::write_wstring(&mut args, "1234");
    args.extend_from_slice(&1i32.to_le_bytes());

    dispatch_cell_method(
        1,
        crate::cell::cell_methods::gm::GM_GIVE_ITEM,
        &args,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    // Rejection audit log with the gm* index.
    let event = capture
        .find_message(tracing::Level::WARN, "GM-gated cell method rejected")
        .expect("non-GM call to a 109+ gm method must be rejected by the gate");
    assert!(event.has_field("method_index", "133"));
    assert!(event.has_field("access_level", "0"));

    // Exactly one wire message — the onErrorCode. NO GrantItem (the handler
    // never ran). This is the revert-verifier assertion.
    let msg = rx
        .try_recv()
        .expect("rejection must send an onErrorCode response");
    match msg {
        CellToBaseMsg::EntityMethodCall { method_index, .. } => {
            assert_eq!(method_index, 121, "onErrorCode expected");
        }
        other => panic!("expected onErrorCode EntityMethodCall, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_err(),
        "a rejected gm call must produce ONLY the onErrorCode — no GrantItem \
         side effect. A GrantItem here means a non-GM reached the handler \
         (the `index >= 109` gate arm was reverted)."
    );
}

/// **#473 positive case.** A GM (access_level 2) sending an implemented gm*
/// index (gmGiveItem 133) passes the gate and the handler executes — the
/// observable proof is a `GrantItem` on the wire (and no onErrorCode).
#[tokio::test]
async fn gm_tail_method_executes_for_gm_caller() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.player_id = Some(100);
        e.access_level = 2; // GameMaster
    }

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    let mut args = Vec::new();
    crate::mercury::write_wstring(&mut args, "1234");
    args.extend_from_slice(&3i32.to_le_bytes());

    dispatch_cell_method(
        1,
        crate::cell::cell_methods::gm::GM_GIVE_ITEM,
        &args,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let mut saw_grant = false;
    let mut saw_error = false;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::GrantItem { item_id, count, .. } => {
                assert_eq!(item_id, 1234);
                assert_eq!(count, 3);
                saw_grant = true;
            }
            CellToBaseMsg::EntityMethodCall {
                method_index: 121, ..
            } => {
                saw_error = true;
            }
            _ => {}
        }
    }
    assert!(saw_grant, "authorized gmGiveItem must emit GrantItem");
    assert!(!saw_error, "authorized gm call must not emit onErrorCode");
}

/// A GM sending an UNimplemented 109+ index (gmSetGodMode 142) passes the gate
/// and hits the auth-gated router fall-through without panic and without any
/// stray side-effect message.
#[tokio::test]
async fn gm_tail_unimplemented_index_falls_through_without_panic() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.player_id = Some(100);
        e.access_level = 2; // GameMaster
    }

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    // 142 = gmSetGodMode — gated (>= 109) but no handler wired yet.
    dispatch_cell_method(1, 142, &[1u8], &tx, &mut mgr, &engine).await;

    // No onErrorCode (gate passed), no side-effect message (no handler).
    assert!(
        rx.try_recv().is_err(),
        "an authorized-but-unimplemented gm index must produce no wire message"
    );
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
