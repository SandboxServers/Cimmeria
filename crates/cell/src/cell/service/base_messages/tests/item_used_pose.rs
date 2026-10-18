use super::*;
use crate::test_support::LogCapture;

/// Builds a Castle scene with a player at the origin and an NPC at `[2.0, 0.0, 0.0]`.
/// Returns the manager and the NPC id.
fn castle_scene(player: u32) -> (SpaceManager, u32) {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(player, "Castle", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(player) {
        e.is_player = true;
        e.player_id = Some(12);
        e.session_kind = Some("player");
    }
    mgr.connect_entity(player);
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Castle", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    (mgr, npc)
}

/// Drives one `ItemUsed` message for `player` with the given `target_id`.
async fn use_item(mgr: &mut SpaceManager, player: u32, target_id: i32) {
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::ItemUsed {
            entity_id: player,
            instance_id: 1001,
            type_id: 42,
            target_id,
        },
        &tx,
        mgr,
        &engine,
        &[],
    )
    .await;
}

/// An item used on a live NPC writes one `use_item` row with the item type and a 2 m stand-off.
#[tokio::test]
async fn item_used_on_npc_writes_use_item_row() {
    let player = 9401;
    let (mut mgr, npc) = castle_scene(player);
    let logs = LogCapture::install();
    use_item(&mut mgr, player, npc as i32).await;

    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "interaction.pose")
        .collect();
    assert_eq!(rows.len(), 1, "one pose row per use: {rows:?}");
    let row = &rows[0];
    assert!(row.has_field("kind", "use_item"));
    assert!(row.has_field("route", "item_used"));
    assert!(row.has_field("item_type_id", "42"));
    let radius: f32 = row.fields["radius_m"].parse().unwrap();
    assert!((radius - 2.0).abs() < 0.01, "radius_m {radius}");
}

/// A use with no target (`target_id = 0`) writes no pose row, since no entity is named.
#[tokio::test]
async fn item_used_without_target_writes_no_row() {
    let player = 9402;
    let (mut mgr, _npc) = castle_scene(player);
    let logs = LogCapture::install();
    use_item(&mut mgr, player, 0).await;

    assert!(
        logs.all()
            .into_iter()
            .all(|c| c.target != "interaction.pose"),
        "a use without a target must not write interaction.pose"
    );
}

/// A use aimed at the user's own entity writes no pose row, because there is no stand-off.
#[tokio::test]
async fn item_used_on_self_writes_no_row() {
    let player = 9403;
    let (mut mgr, _npc) = castle_scene(player);
    let logs = LogCapture::install();
    use_item(&mut mgr, player, player as i32).await;

    assert!(
        logs.all()
            .into_iter()
            .all(|c| c.target != "interaction.pose"),
        "a self-targeted use must not write interaction.pose"
    );
}
