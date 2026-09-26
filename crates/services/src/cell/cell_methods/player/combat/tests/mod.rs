//! Tests for combat dispatch: the `make_mgr_with_player` fixture and the
//! `dispatch` routing tests. The respawn fork's tests moved with the respawn
//! core to `cell::respawn::tests`, with a copy of the fixture.

use super::*;

/// Build a SpaceManager with one player at id=1 in the
/// Castle_CellBlock instanced space (every dispatch test sees a
/// fresh world). Caller can override is_player and stats.
fn make_mgr_with_player(world: &str) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = format!(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="{world}" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    );
    mgr.parse_spaces_xml(&xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, world, [42.0, 1.0, 17.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    mgr.connect_entity(1);
    mgr
}

#[tokio::test]
async fn dispatch_returns_false_for_unknown_method() {
    let mut mgr = make_mgr_with_player("Castle_CellBlock");
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    let handled = dispatch(1, 9999, &[], &tx, &mut mgr, &engine).await;
    assert!(!handled);
}

/// USE_ABILITY with a too-short payload (< 8 bytes) must return
/// true (handler took the method) but not start any cooldown,
/// not consume any state, and not emit packets — the args are
/// silently ignored. Pre-seed an ability + cooldown-free state so
/// a regression that decodes garbage args and starts a cooldown
/// gets caught.
#[tokio::test]
async fn use_ability_with_short_args_silently_drops() {
    let mut mgr = make_mgr_with_player("Castle_CellBlock");
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);

    let handled = dispatch(1, USE_ABILITY, &[1u8, 2, 3], &tx, &mut mgr, &engine).await;
    assert!(handled);
    assert!(
        rx.try_recv().is_err(),
        "short USE_ABILITY must not emit packets"
    );
    assert!(
        !mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7),
        "short USE_ABILITY must not start a cooldown"
    );
}
