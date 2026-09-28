//! `lootItem` on a **live** container (the content `open_loot` action,
//! Decision (@Cadacious, 2026-09-28)): the looter takes from their own roll,
//! the container keeps its flags and stays standing, another looter cannot
//! take that roll, and the range gate still applies.

use cimmeria_entity::cell_entity::LootItem;
use tokio::sync::mpsc;

use super::*;
use crate::cell::space_manager::SpaceManager;

const CHEST: u32 = 50;
const P1: u32 = 1;
const P2: u32 = 2;

/// A container holding a two-item roll for player 101 (entity 1), with
/// entity 1 looting it from `looter_at`.
fn rig(looter_at: [f32; 3]) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-500" MaxX="500" MinY="-500" MaxY="500" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(CHEST, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let c = mgr.get_entity_mut(CHEST).unwrap();
    c.is_loot_container = true;
    c.interaction_type_flags = crate::cell::abilities::INT_NORMAL_LOOT;
    c.next_loot_index = 3;
    c.container_loot.insert(
        101,
        vec![
            LootItem {
                design_id: Some(2893),
                quantity: 2,
                index: 1,
            },
            LootItem {
                design_id: None,
                quantity: 40,
                index: 2,
            },
        ],
    );
    for (eid, pid, at) in [(P1, 101, looter_at), (P2, 102, [1.0, 0.0, 0.0])] {
        mgr.create_entity(eid, "Agnos", at, [0.0; 3]).unwrap();
        let p = mgr.get_entity_mut(eid).unwrap();
        p.is_player = true;
        p.player_id = Some(pid);
        p.looting_entity = Some(CHEST);
    }
    mgr
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn grants(msgs: &[CellToBaseMsg]) -> usize {
    msgs.iter()
        .filter(|m| {
            matches!(
                m,
                CellToBaseMsg::GrantItem { .. } | CellToBaseMsg::GrantCash { .. }
            )
        })
        .count()
}

/// Loot All on a live container: both items are granted, the roll is spent
/// (pruned), the window closes, and the container keeps its loot cursor
/// (no `INTERACTION_TYPE` broadcast, flags untouched).
#[tokio::test]
async fn loot_all_on_a_live_container_grants_and_spends_the_roll() {
    let mut mgr = rig([1.0, 0.0, 0.0]);
    let (tx, mut rx) = mpsc::channel(32);
    handle_loot_item(P1, 2, &tx, &mut mgr).await;
    handle_loot_item(P1, 1, &tx, &mut mgr).await;
    let msgs = drain(&mut rx);

    assert_eq!(grants(&msgs), 2, "both items granted: {msgs:?}");
    let chest = mgr.get_entity(CHEST).unwrap();
    assert!(
        !chest.container_loot.contains_key(&101),
        "the emptied roll is pruned"
    );
    assert_eq!(
        chest.interaction_type_flags,
        crate::cell::abilities::INT_NORMAL_LOOT,
        "a live container keeps its flags"
    );
    assert!(
        !msgs.iter().any(
            |m| matches!(m, CellToBaseMsg::EntityMethodCall { method_index, .. }
            if *method_index == crate::mercury::method_idx::INTERACTION_TYPE)
        ),
        "no loot-bit broadcast for a container: {msgs:?}"
    );
    assert_eq!(mgr.get_entity(P1).unwrap().looting_entity, None);

    // The roll is spent: a stale click takes nothing.
    mgr.get_entity_mut(P1).unwrap().looting_entity = Some(CHEST);
    handle_loot_item(P1, 1, &tx, &mut mgr).await;
    assert_eq!(grants(&drain(&mut rx)), 0, "an emptied roll grants nothing");
}

/// Another player cannot take player 101's roll by guessing its index.
#[tokio::test]
async fn another_looter_cannot_take_someone_elses_roll() {
    let mut mgr = rig([1.0, 0.0, 0.0]);
    let (tx, mut rx) = mpsc::channel(32);
    handle_loot_item(P2, 1, &tx, &mut mgr).await;
    assert_eq!(grants(&drain(&mut rx)), 0);
    assert_eq!(
        mgr.get_entity(CHEST).unwrap().container_loot[&101].len(),
        2,
        "player 101's roll is untouched"
    );
}

/// The #446 range gate applies to a container like a corpse.
#[tokio::test]
async fn loot_item_beyond_interact_range_takes_nothing_from_a_container() {
    let mut mgr = rig([100.0, 0.0, 0.0]);
    let (tx, mut rx) = mpsc::channel(32);
    handle_loot_item(P1, 1, &tx, &mut mgr).await;
    assert_eq!(grants(&drain(&mut rx)), 0);
    assert_eq!(mgr.get_entity(CHEST).unwrap().container_loot[&101].len(), 2);
}

/// A refused grant goes back into the looter's own roll, even when it was
/// the last item and the roll had been pruned.
#[tokio::test]
async fn a_refused_grant_goes_back_into_the_looters_roll() {
    let mut mgr = rig([1.0, 0.0, 0.0]);
    let source = LootGrantSource {
        corpse_id: CHEST,
        index: 1,
        corpse_respawn_at: None,
        corpse_template_id: None,
    };
    mgr.get_entity_mut(CHEST).unwrap().container_loot.clear();
    let (tx, mut rx) = mpsc::channel(32);
    handle_loot_grant_refused(
        P1,
        101,
        source,
        2893,
        2,
        1,
        crate::cell::messages::GrantRefusal::ContainerFull,
        &tx,
        &mut mgr,
    )
    .await;
    let roll = &mgr.get_entity(CHEST).unwrap().container_loot[&101];
    assert_eq!(roll.len(), 1, "the item is back in player 101's roll");
    assert!(mgr.get_entity(CHEST).unwrap().loot.is_empty());
    let want = cimmeria_wire::cell::chat::serialize_on_player_communication(
        "SYSTEM",
        0,
        cimmeria_wire::cell::chat::CHAN_FEEDBACK,
        "Your inventory is full. The item was left in the container.",
    );
    assert!(
        drain(&mut rx).into_iter().any(|m| matches!(
            m,
            CellToBaseMsg::EntityMethodCall { ref args, .. } if *args == want
        )),
        "the line says the item was left in the container"
    );
}
