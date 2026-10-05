//! NT-25 (Rule 6): the `npc_ai.leash` `event=enter` row names the NPC, its
//! template and the target it gave up on, next to their ids.

use std::time::Instant;

use cimmeria_names::{NameBook, Table};

use super::leash::{on_enter, LeashEntry};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

const NPC: u32 = 900;
const PLAYER: u32 = 1;

fn agnos_with_guard_and_player() -> SpaceManager {
    let mut book = NameBook::empty();
    book.insert(Table::Templates, 77, "NT25_Leash_Template");
    book.insert(Table::Texts, 78, "Leashing Guard");
    cimmeria_names::global().store(book);

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    mgr.spawn_npc(NPC, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.template_id = Some(77);
    npc.name_id = Some(78);
    mgr.create_entity(PLAYER, "Agnos", [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.character_name = Some("Daniel".to_string());
    mgr
}

/// Revert proof: drop `npc_name`, `template_name` or `target_name` from the
/// `enter` row and the matching assertion fails.
#[test]
fn leash_enter_row_names_the_npc_its_template_and_its_target() {
    let mut mgr = agnos_with_guard_and_player();
    let logs = LogCapture::install();

    on_enter(
        &mut mgr,
        NPC,
        LeashEntry {
            target: Some((PLAYER, cimmeria_common::Vector3::new(5.0, 0.0, 5.0))),
            leash_distance: 30.0,
            reason: "leash_out",
            trigger: "beyond_band",
        },
        Instant::now(),
    );

    let all = logs.all();
    let row = all
        .iter()
        .find(|c| c.target == "npc_ai.leash" && c.has_field("event", "enter"))
        .expect("the leash enter row");
    assert!(row.has_field("npc_name", "Leashing Guard"), "{row:?}");
    assert!(
        row.has_field("template_name", "NT25_Leash_Template"),
        "{row:?}"
    );
    assert!(row.has_field("target_id", "1"), "{row:?}");
    assert!(row.has_field("target_name", "Daniel"), "{row:?}");
}
