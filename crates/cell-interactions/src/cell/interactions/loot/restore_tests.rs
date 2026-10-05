//! A refused loot grant puts the item back on its corpse, and only there.
//!
//! Each test takes an item with `handle_loot_item` first, so the source the
//! refusal carries is the one the take captured. Removing the put-back in
//! `handle_loot_grant_refused` leaves the corpse empty and fails the first
//! test; removing the unsent-grant return fails the last.

use cimmeria_entity::cell_entity::{LootItem, NpcInteractionType};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use tokio::sync::mpsc;
use tracing::Level;

use super::*;
use crate::cell::abilities::INT_NORMAL_LOOT;
use crate::cell::messages::GrantRefusal;
use crate::mercury::method_idx;
use crate::test_support::LogCapture;

const LOOTER: u32 = 1;
const PLAYER_ID: i32 = 42;
const ACCOUNT_ID: u32 = 7;
const DESIGN: i32 = 5228;
const TEMPLATE: i32 = 304;

/// A looter (entity 1) standing on a dead crate that holds `items` item
/// drops (indices 1..), each one of `DESIGN`, and is waiting to respawn.
fn looting_mgr(items: i32) -> (SpaceManager, u32) {
    let mut mgr = SpaceManager::new(1);
    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(spaces_xml).unwrap();
    mgr.create_startup_spaces(cell_spaces_xml).unwrap();
    mgr.create_entity(LOOTER, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let corpse = mgr.allocate_npc_id();
    mgr.spawn_npc(corpse, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let npc = mgr.get_entity_mut(corpse).unwrap();
    npc.template_id = Some(TEMPLATE);
    npc.respawn_at = Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
    for index in 1..=items {
        npc.loot.push(LootItem {
            design_id: Some(DESIGN),
            quantity: 1,
            index,
        });
    }
    npc.next_loot_index = items + 1;
    npc.interaction_type_flags |= INT_NORMAL_LOOT;
    npc.interaction_type = Some(NpcInteractionType::Loot);
    let p = mgr.get_entity_mut(LOOTER).unwrap();
    p.player_id = Some(PLAYER_ID);
    p.account_id = Some(ACCOUNT_ID);
    p.is_player = true;
    p.looting_entity = Some(corpse);
    (mgr, corpse)
}

/// Take index 1 and return the source the grant carried.
async fn take(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) -> LootGrantSource {
    handle_loot_item(LOOTER, 1, tx, mgr).await;
    let mut source = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::GrantItem { loot, item_id, .. } = msg {
            assert_eq!(item_id, DESIGN);
            source = loot;
        }
    }
    source.expect("a looted item must carry its corpse on GrantItem")
}

fn sent_to_looter(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u16, Vec<u8>)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: LOOTER,
            method_index,
            args,
        } = msg
        {
            out.push((method_index, args));
        }
    }
    out
}

fn feedback(text: &str) -> (u16, Vec<u8>) {
    (
        method_idx::ON_PLAYER_COMMUNICATION,
        serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
    )
}

/// The last item came off the crate, which cleared its loot bit; the base
/// refused the grant (the crafting bag is full). The item is back at its
/// index, the loot bit is back, the looter is told, and the restore is
/// logged with the full identity. A second copy of the same refusal finds
/// the index taken, so the item is never doubled.
#[tokio::test]
async fn refused_grant_goes_back_on_the_emptied_corpse() {
    let (mut mgr, corpse) = looting_mgr(1);
    let (tx, mut rx) = mpsc::channel(64);
    let source = take(&mut mgr, &tx, &mut rx).await;
    assert_eq!(source.corpse_id, corpse);
    assert!(mgr.get_entity(corpse).unwrap().loot.is_empty());
    assert_eq!(
        mgr.get_entity(corpse).unwrap().interaction_type_flags & INT_NORMAL_LOOT,
        0
    );
    let _ = sent_to_looter(&mut rx);

    let capture = LogCapture::install();
    handle_loot_grant_refused(
        LOOTER,
        PLAYER_ID,
        source,
        DESIGN,
        1,
        15,
        GrantRefusal::ContainerFull,
        &tx,
        &mut mgr,
    )
    .await;

    let npc = mgr.get_entity(corpse).unwrap();
    assert_eq!(npc.loot.len(), 1, "the item must be back on the corpse");
    assert_eq!(
        (
            npc.loot[0].design_id,
            npc.loot[0].quantity,
            npc.loot[0].index
        ),
        (Some(DESIGN), 1, 1)
    );
    assert_ne!(
        npc.interaction_type_flags & INT_NORMAL_LOOT,
        0,
        "loot bit restored"
    );
    assert!(matches!(
        npc.interaction_type,
        Some(NpcInteractionType::Loot)
    ));
    assert!(
        sent_to_looter(&mut rx).contains(&feedback(
            "Your crafting bag is full. The item was left on the corpse."
        )),
        "the looter must be told why the pickup failed"
    );
    let event = capture
        .find_event(Level::INFO, "loot_restored", "container_full")
        .expect("loot_restored");
    assert_eq!(event.target, "inventory");
    for (key, value) in [
        ("account_id", ACCOUNT_ID.to_string()),
        ("player_id", PLAYER_ID.to_string()),
        ("entity_id", LOOTER.to_string()),
        ("corpse_id", corpse.to_string()),
        ("index", "1".to_string()),
        ("item_id", DESIGN.to_string()),
        ("qty", "1".to_string()),
        ("container_id", "15".to_string()),
    ] {
        assert_eq!(event.fields.get(key), Some(&value), "field `{key}`");
    }

    handle_loot_grant_refused(
        LOOTER,
        PLAYER_ID,
        source,
        DESIGN,
        1,
        15,
        GrantRefusal::ContainerFull,
        &tx,
        &mut mgr,
    )
    .await;
    assert_eq!(
        mgr.get_entity(corpse).unwrap().loot.len(),
        1,
        "never doubled"
    );
    assert!(capture
        .find_event(Level::WARN, "loot_restore_failed", "index_taken")
        .is_some());
}

/// A looter still at the crate sees the window refresh with the item back
/// in it.
#[tokio::test]
async fn looter_with_the_window_open_sees_the_item_again() {
    let (mut mgr, _corpse) = looting_mgr(2);
    let (tx, mut rx) = mpsc::channel(64);
    let source = take(&mut mgr, &tx, &mut rx).await;
    let _ = sent_to_looter(&mut rx);

    handle_loot_grant_refused(
        LOOTER,
        PLAYER_ID,
        source,
        DESIGN,
        1,
        1,
        GrantRefusal::ContainerFull,
        &tx,
        &mut mgr,
    )
    .await;

    let sent = sent_to_looter(&mut rx);
    assert!(sent.contains(&feedback(
        "Your inventory is full. The item was left on the corpse."
    )));
    let display = sent
        .iter()
        .find(|(m, _)| *m == method_idx::ON_LOOT_DISPLAY)
        .expect("the open loot window must refresh");
    // entityId, then an ARRAY count of 2: both items are listed again.
    assert_eq!(u32::from_le_bytes(display.1[4..8].try_into().unwrap()), 2);
}

/// The crate respawned while the grant was in flight: the item does not go
/// onto the new body. It is lost, logged as such, and the looter is told.
#[tokio::test]
async fn respawned_corpse_does_not_get_the_item() {
    let (mut mgr, corpse) = looting_mgr(1);
    let (tx, mut rx) = mpsc::channel(64);
    let source = take(&mut mgr, &tx, &mut rx).await;
    let _ = sent_to_looter(&mut rx);
    mgr.get_entity_mut(corpse).unwrap().respawn_at = None;

    let capture = LogCapture::install();
    handle_loot_grant_refused(
        LOOTER,
        PLAYER_ID,
        source,
        DESIGN,
        1,
        15,
        GrantRefusal::DatabaseError,
        &tx,
        &mut mgr,
    )
    .await;

    assert!(mgr.get_entity(corpse).unwrap().loot.is_empty());
    let event = capture
        .find_event(Level::WARN, "loot_restore_failed", "corpse_changed")
        .expect("loot_restore_failed reason=corpse_changed");
    assert_eq!(
        event.fields.get("refusal").map(String::as_str),
        Some("database_error")
    );
    assert_eq!(
        event.fields.get("account_id"),
        Some(&ACCOUNT_ID.to_string())
    );
    assert!(sent_to_looter(&mut rx).contains(&feedback(
        "That item could not be picked up, and the corpse it was on is gone."
    )));
}

/// The base channel is closed, so the grant never left the cell: the item
/// stays on the corpse.
#[tokio::test]
async fn unsent_grant_stays_on_the_corpse() {
    let (mut mgr, corpse) = looting_mgr(1);
    let (tx, rx) = mpsc::channel(64);
    drop(rx);
    let capture = LogCapture::install();

    handle_loot_item(LOOTER, 1, &tx, &mut mgr).await;

    let npc = mgr.get_entity(corpse).unwrap();
    assert_eq!(
        npc.loot.len(),
        1,
        "an unsent grant must leave the item on the corpse"
    );
    assert_ne!(npc.interaction_type_flags & INT_NORMAL_LOOT, 0);
    let event = capture
        .find_event(Level::WARN, "loot_grant_send_failed", "restored")
        .expect("loot_grant_send_failed");
    assert_eq!(
        event.fields.get("restored").map(String::as_str),
        Some("true")
    );
}

/// `Player looted item` carries a numeric `item_id` (absent for cash),
/// never the Debug-formatted `"Some(5228)"` / `"None"` the 2026-09-29
/// playtest rows had, plus the looter's `account_id` and `player_id`.
/// Reverting to `item_id = ?removed_item.design_id` fails the first
/// assertion; dropping the identity fails the next two.
#[tokio::test]
async fn looted_item_row_carries_numeric_design_id_and_identity() {
    let (mut mgr, corpse) = looting_mgr(1);
    mgr.get_entity_mut(corpse).unwrap().loot.push(LootItem {
        design_id: None,
        quantity: 25,
        index: 2,
    });
    let (tx, mut rx) = mpsc::channel(64);
    let capture = LogCapture::install();
    let _ = take(&mut mgr, &tx, &mut rx).await;
    handle_loot_item(LOOTER, 2, &tx, &mut mgr).await;

    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.level == Level::INFO && c.message_contains("Player looted item"))
        .collect();
    assert_eq!(rows.len(), 2, "{rows:#?}");
    let item = &rows[0];
    assert!(item.has_field("item_id", &DESIGN.to_string()), "{item:#?}");
    assert!(item.has_field("account_id", &ACCOUNT_ID.to_string()));
    assert!(item.has_field("player_id", &PLAYER_ID.to_string()));
    assert!(item.has_field("loot_kind", "item"));
    assert!(item.has_field("corpse_template_id", &TEMPLATE.to_string()));
    let cash = &rows[1];
    assert!(
        !cash.fields.contains_key("item_id"),
        "cash has no design id, not a \"None\" string: {cash:#?}"
    );
    assert!(cash.has_field("loot_kind", "cash"));
    assert!(cash.has_field("quantity", "25"));
}
