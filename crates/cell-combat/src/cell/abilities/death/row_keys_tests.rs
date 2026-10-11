//! TG-CMB-06 LogCapture guards for the death rows: `player.death` carries the
//! world bare and no `killer_name` for an environment kill, and `target_killed`
//! names the player side of the kill by identity (Rule 5).

use super::*;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;
use tracing::Level;

/// A Castle space with a player at id 1 (`player_id` 100) and an NPC at id 2.
fn castle_with_player_and_npc() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    mgr
}

/// Environment kill of a player (no attacker entity): the `player.death` row
/// names the world as a bare string and omits `killer_name`, never `""`.
/// Reverting `world` to `?world_name` records `Some("Castle")` and fails here.
#[tokio::test]
async fn environment_kill_death_row_names_world_and_no_killer() {
    let capture = LogCapture::install();
    let mut mgr = castle_with_player_and_npc();
    let (tx, _rx) = mpsc::channel(64);

    resolve_death(1, 0, None, false, false, &tx, &mut mgr).await;

    let all = capture.all();
    let death = all
        .iter()
        .find(|c| c.target == "player.death")
        .expect("player.death must be emitted for a player target");
    assert!(
        death.has_field("world", "Castle"),
        "player.death must carry world bare: {death:#?}"
    );
    assert!(
        !death.fields.contains_key("killer_name"),
        "an environment kill has no killer_name, not an empty one: {death:#?}"
    );
}

/// A player attacker's kill row names that player by `player_id` (Rule 5),
/// with `target_id` for the entity killed. Dropping the identity fields fails.
#[tokio::test]
async fn player_kill_target_killed_row_names_the_attacker_player_id() {
    let capture = LogCapture::install();
    let mut mgr = castle_with_player_and_npc();
    let (tx, _rx) = mpsc::channel(64);

    resolve_death(2, 1, None, true, false, &tx, &mut mgr).await;

    let all = capture.all();
    let killed = all
        .iter()
        .find(|c| c.message_contains("Target killed!"))
        .expect("target_killed must be emitted for a kill");
    assert!(
        killed.has_field("player_id", "100"),
        "target_killed must carry the player attacker's player_id: {killed:#?}"
    );
    assert!(
        killed.has_field("target_id", "2"),
        "target_killed must name the target as target_id: {killed:#?}"
    );
}

/// TG-SOC-15: an unnamed player (no `character_name`) that dies sends no
/// `ContactListPresenceEvent`, so no `entity:{eid}` placeholder reaches the
/// contact lists. The skip logs one DEBUG `contacts.presence_skipped` row
/// naming the player by identity. Restoring the placeholder send makes the
/// event appear here and fails the test.
#[tokio::test]
async fn unnamed_player_death_skips_presence_and_logs_the_row() {
    let capture = LogCapture::install();
    let mut mgr = castle_with_player_and_npc();
    let (tx, mut rx) = mpsc::channel(64);

    resolve_death(1, 0, None, false, false, &tx, &mut mgr).await;

    let mut msgs = Vec::new();
    while let Ok(m) = rx.try_recv() {
        msgs.push(m);
    }
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::ContactListPresenceEvent { .. })),
        "an unnamed player's death must send no ContactListPresenceEvent: {msgs:?}"
    );

    let all = capture.all();
    let skipped = all
        .iter()
        .find(|c| c.has_field("event", "contacts.presence_skipped"))
        .expect("the skipped presence must log a contacts.presence_skipped row");
    assert_eq!(skipped.level, Level::DEBUG, "{skipped:#?}");
    assert!(
        skipped.has_field("reason", "no_character_name"),
        "the skip must carry reason = no_character_name: {skipped:#?}"
    );
    assert!(
        skipped.has_field("entity_id", "1"),
        "the skip must name the player's entity_id: {skipped:#?}"
    );
    assert!(
        skipped.has_field("player_id", "100"),
        "the skip must carry the player's player_id: {skipped:#?}"
    );
}
