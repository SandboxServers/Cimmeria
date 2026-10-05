//! Tests for combat dispatch: the `make_mgr_with_player` fixture and the
//! `dispatch` routing tests. The respawn fork's tests moved with the respawn
//! core to `cell::respawn::tests`, with a copy of the fixture.

use super::*;

/// The ground-cast `entity_health_below` guard, which drives this
/// dispatcher (Harset H04).
mod aoe_health_below;

/// The dead gate and offered-respawner check on callForAid / respawn.
mod respawn_gate;

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
    let handled = dispatch(1, 9999, &[], &tx, &mut mgr, &engine, None).await;
    assert!(!handled);
}

/// **Regression guard (AB-T1, rule 5).** The `useAbility` receipt row is
/// queryable under the `abilities` target with its stable `event`, the
/// player's ids, and the target id the client sent. Before AB-T1 it logged
/// under the module path with no player identity.
#[tokio::test]
async fn use_ability_receipt_row_names_the_player() {
    let mut mgr = make_mgr_with_player("Castle_CellBlock");
    mgr.get_entity_mut(1).unwrap().account_id = Some(900);
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    let logs = crate::test_support::LogCapture::install();
    let mut args = 7i32.to_le_bytes().to_vec();
    args.extend_from_slice(&42i32.to_le_bytes());

    assert!(dispatch(1, USE_ABILITY, &args, &tx, &mut mgr, &engine, Some(4711)).await);

    let recv = logs
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "use_ability_recv"))
        .expect("the receipt row");
    assert_eq!(recv.target, "abilities");
    for (k, v) in [
        ("stage", "recv"),
        ("account_id", "900"),
        ("player_id", "100"),
        ("ability_id", "7"),
        ("wire_target_id", "42"),
        // AB-T2: the inbound Mercury seq, the join to the client's press.
        ("mercury_seq", "4711"),
    ] {
        assert!(recv.has_field(k, v), "{k} = {v}: {recv:?}");
    }
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

    let logs = crate::test_support::LogCapture::install();
    let handled = dispatch(
        1,
        USE_ABILITY,
        &[1u8, 2, 3],
        &tx,
        &mut mgr,
        &engine,
        Some(9),
    )
    .await;
    assert!(handled);
    // AB-T2: the drop is no longer silent. One DEBUG row under `abilities`
    // names the player, the lengths and the packet.
    let row = logs
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "use_ability_args_short"))
        .expect("a short useAbility logs its drop");
    assert_eq!(row.target, "abilities");
    assert_eq!(row.level, tracing::Level::DEBUG);
    for (k, v) in [
        ("player_id", "100"),
        ("args_len", "3"),
        ("expected_len", "8"),
        ("mercury_seq", "9"),
    ] {
        assert!(row.has_field(k, v), "{k} = {v}: {row:?}");
    }
    assert!(
        rx.try_recv().is_err(),
        "short USE_ABILITY must not emit packets"
    );
    assert!(
        !mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7),
        "short USE_ABILITY must not start a cooldown"
    );
}
