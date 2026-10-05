//! Rule 6 on the MinigamePlayer stubs: their rows name the caller and every
//! entity ID the client sent. Removing a name field fails these.

use tokio::sync::mpsc;
use tracing::Level;

use super::{dispatch, END_CURRENT};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

fn space_with_player() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(1).unwrap();
    e.is_player = true;
    e.player_id = Some(100);
    e.character_name = Some("Minigamer".to_string());
    mgr
}

#[tokio::test]
async fn end_current_minigame_row_names_the_caller_winner_and_loser() {
    let capture = LogCapture::install();
    let mut mgr = space_with_player();
    let (tx, _rx) = mpsc::channel(4);
    let mut args = Vec::new();
    for v in [7i32, 1, 1] {
        args.extend_from_slice(&v.to_le_bytes());
    }
    assert!(dispatch(1, END_CURRENT, &args, &tx, &mut mgr).await);

    let row = capture
        .find_message(Level::INFO, "UNIMPLEMENTED: endCurrentMinigame")
        .expect("the stub logs its call");
    for key in ["entity_name", "winner_name", "loser_name"] {
        assert!(row.has_field(key, "Minigamer"), "{key} missing: {row:#?}");
    }
}
