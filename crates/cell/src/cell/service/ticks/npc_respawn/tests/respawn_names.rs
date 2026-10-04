//! NT-25 (Rule 6): the `npc_respawn_recreate` row names the NPC, its
//! template and its world next to their ids, so a respawn reads in SigNoz
//! without a seed lookup.

use super::super::*;
use super::fixtures::make_mgr_with_dead_npc;
use crate::test_support::LogCapture;
use cimmeria_names::{NameBook, Table};

const TEMPLATE_ID: i32 = 4242;
const NAME_ID: i32 = 4243;

/// The book is process-global (nextest runs each test in its own process).
fn install_test_book() {
    let mut book = NameBook::empty();
    book.insert(Table::Templates, TEMPLATE_ID.into(), "NT25_Jaffa_Template");
    book.insert(Table::Texts, NAME_ID.into(), "Jaffa Guard");
    cimmeria_names::global().store(book);
}

#[tokio::test]
async fn the_respawn_row_names_the_npc_its_template_and_its_world() {
    install_test_book();
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    if let Some(npc) = mgr.get_entity_mut(50) {
        npc.template_id = Some(TEMPLATE_ID);
        npc.name_id = Some(NAME_ID);
    }
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    npc_respawn_tick(&tx, &mut mgr).await;

    let all = logs.all();
    let row = all
        .iter()
        .find(|c| c.fields.get("event").map(String::as_str) == Some("npc_respawn_recreate"))
        .expect("the respawn must log npc_respawn_recreate");
    assert!(row.has_field("npc_id", "50"), "{row:?}");
    assert!(row.has_field("npc_name", "Jaffa Guard"), "{row:?}");
    assert!(row.has_field("template_id", "4242"), "{row:?}");
    assert!(
        row.has_field("template_name", "NT25_Jaffa_Template"),
        "{row:?}"
    );
    assert!(row.has_field("world", "Castle"), "{row:?}");
    assert!(
        !row.fields.contains_key("world_name"),
        "`world_name` was renamed to `world` (Rule 6): {row:?}"
    );
}
