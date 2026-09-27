//! `BaseToCellMsg::UpdateIgnoreList` (SS-C1): the base's Ignore set lands on
//! the cell entity that spatial chat reads, and a push for a missing entity
//! is logged, not applied.

use std::collections::HashSet;

use super::*;
use crate::test_support::LogCapture;

fn names(v: &[&str]) -> HashSet<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[tokio::test]
async fn update_ignore_list_replaces_the_cell_entity_set() {
    let capture = LogCapture::install();
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(7, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(7).unwrap().player_id = Some(70);
    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    for set in [names(&["Spammer", "Jerk"]), names(&["Jerk"])] {
        handle_base_message(
            BaseToCellMsg::UpdateIgnoreList {
                entity_id: 7,
                player_id: 70,
                account_id: 700,
                ignore_names: set.clone(),
            },
            &tx,
            &mut mgr,
            &engine,
            &[],
        )
        .await;
        assert_eq!(
            mgr.get_entity(7).unwrap().ignore_names,
            set,
            "each push replaces the set; it never merges"
        );
    }
    assert!(rx.try_recv().is_err(), "the arm sends nothing to the base");
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "chat.ignore_set_applied"))
        .expect("chat.ignore_set_applied logged");
    assert!(
        ev.has_field("account_id", "700") && ev.has_field("player_id", "70"),
        "the applied row names the owner's account and player (rule 5)"
    );
}

#[tokio::test]
async fn update_ignore_list_for_missing_entity_logs_reason() {
    let capture = LogCapture::install();
    let mut mgr = SpaceManager::new(1);
    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    handle_base_message(
        BaseToCellMsg::UpdateIgnoreList {
            entity_id: 404,
            player_id: 70,
            account_id: 700,
            ignore_names: names(&["X"]),
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "chat.ignore_set_dropped"))
        .expect("a push for a missing entity must log chat.ignore_set_dropped");
    assert!(ev.has_field("reason", "entity_missing"));
    assert!(ev.has_field("player_id", "70") && ev.has_field("entity_id", "404"));
    assert!(
        ev.has_field("account_id", "700"),
        "entity_missing carries the owner's account from the message (rule 5)"
    );
}

/// Keyed by player identity: a push naming entity 7 for player 70 must not
/// land on entity 7 once that id belongs to player 71 (entity ids are
/// recycled; gate travel moves a character to a new id). Fails when the
/// handler applies the set by entity id alone.
#[tokio::test]
async fn update_ignore_list_for_another_players_entity_is_dropped() {
    let capture = LogCapture::install();
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(7, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(7).unwrap().player_id = Some(71);
    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    handle_base_message(
        BaseToCellMsg::UpdateIgnoreList {
            entity_id: 7,
            player_id: 70,
            account_id: 700,
            ignore_names: names(&["Pest"]),
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    assert!(
        mgr.get_entity(7).unwrap().ignore_names.is_empty(),
        "player 71 must not inherit player 70's Ignore list"
    );
    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "chat.ignore_set_dropped"))
        .expect("chat.ignore_set_dropped logged");
    assert!(ev.has_field("reason", "player_mismatch"));
    assert!(
        ev.has_field("account_id", "700"),
        "the message owner's account"
    );
}
