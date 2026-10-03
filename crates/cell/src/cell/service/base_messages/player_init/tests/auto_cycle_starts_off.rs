//! Every login starts with auto-cycle off (owner decision 2026-10-03).
//!
//! #412 restored a saved `BSF_AutoCycling` here. On colo 2026-10-03 that saved
//! value was a loop the server had already stopped, so the player logged in to
//! a lit button and an armed loop. `InitPlayerState` no longer carries a
//! `state_field`; this pins what the handler leaves behind.

use super::super::*;
use crate::cell::combat::BSF_AUTO_CYCLING;
use cimmeria_entity::cell_entity::SystemOptions;

fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
    }
    mgr.connect_entity(1);
    mgr
}

/// The loop flag and the button bit are off after world entry, and no
/// `onStateFieldUpdate` lights the button.
#[tokio::test]
async fn login_leaves_auto_cycle_off_and_the_button_unlit() {
    let mut mgr = make_mgr();
    let (tx, mut rx) = mpsc::channel(64);

    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        vec![],
        vec![],
        0,
        vec![],
        SystemOptions::default(),
        0, // access_level
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let e = mgr.get_entity(1).unwrap();
    assert!(!e.abilities.auto_cycle, "the loop starts off");
    assert_eq!(
        e.state_field & BSF_AUTO_CYCLING,
        0,
        "the button starts unlit"
    );
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            args,
        } = msg
        {
            if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                let v = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(
                    v & BSF_AUTO_CYCLING,
                    0,
                    "no state broadcast lights the button"
                );
            }
        }
    }
}
