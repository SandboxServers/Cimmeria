//! `Action::OpenLoot` — a loot window on a live container (Decision
//! (@Cadacious, 2026-09-28)): per-looter rolls, the once-per-character gate,
//! pending reopen, repeatable re-roll, and a feedback line on every press
//! that opens nothing.

use std::collections::HashMap;

use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::mercury::method_idx::ON_LOOT_DISPLAY;

use super::*;
use crate::cell::spawner::LootTableEntry;

const CHEST: u32 = 50;
const P1: u32 = 1;
const P2: u32 = 2;
const TABLE: i32 = 99;

/// A tagged chest at (10, 0, 10), two players beside it, and a loot table
/// that always drops one Slappack and 10 naquadah.
fn rig() -> SpaceManager {
    let mut mgr = make_space_mgr();
    mgr.create_entity(CHEST, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(CHEST).unwrap().tag = Some("Chest".into());
    for (eid, pid) in [(P1, 101), (P2, 102)] {
        mgr.create_entity(eid, "Agnos", [11.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        let p = mgr.get_entity_mut(eid).unwrap();
        p.is_player = true;
        p.player_id = Some(pid);
    }
    mgr.loot_tables.insert(
        TABLE,
        vec![
            LootTableEntry {
                design_id: Some(2893),
                min_quantity: 1,
                max_quantity: 1,
                probability: 1.0,
            },
            LootTableEntry {
                design_id: None,
                min_quantity: 10,
                max_quantity: 10,
                probability: 1.0,
            },
        ],
    );
    mgr
}

fn open(table: Option<i32>, once: bool) -> ResolvedActions {
    let mut params = HashMap::new();
    params.insert("target_entity_id".to_string(), serde_json::json!(CHEST));
    ResolvedActions {
        actions: vec![(
            1274,
            Action::OpenLoot {
                loot_table_id: table,
                once_per_character: once,
                container_key: None,
            },
        )],
        action_delays: vec![0],
        params,
    }
}

async fn press(
    mgr: &mut SpaceManager,
    who: u32,
    table: Option<i32>,
    once: bool,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    let pid = mgr.get_entity(who).unwrap().player_id.unwrap();
    execute_actions(open(table, once), who, pid, &tx, mgr, &ChainEngine::new()).await;
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

fn loot_displays(msgs: &[CellToBaseMsg]) -> usize {
    msgs.iter()
        .filter(|m| {
            matches!(m, CellToBaseMsg::EntityMethodCall { method_index, .. }
                if *method_index == ON_LOOT_DISPLAY)
        })
        .count()
}

fn feedback(msgs: &[CellToBaseMsg]) -> usize {
    msgs.iter()
        .filter(|m| {
            matches!(m, CellToBaseMsg::EntityMethodCall { method_index, .. }
                if *method_index == ON_PLAYER_COMMUNICATION)
        })
        .count()
}

fn persisted(msgs: &[CellToBaseMsg]) -> Vec<String> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::ContainerLooted { container_key, .. } => Some(container_key.clone()),
            _ => None,
        })
        .collect()
}

fn roll_of(mgr: &SpaceManager, pid: i32) -> Vec<i32> {
    mgr.get_entity(CHEST)
        .unwrap()
        .container_loot
        .get(&pid)
        .map(|l| l.iter().map(|i| i.index).collect())
        .unwrap_or_default()
}

/// Each looter gets their own roll on the same chest, the chest stays a live
/// container (no corpse loot, no loot bit), and the once flag is set and
/// persisted keyed by the chest's tag.
#[tokio::test]
async fn each_looter_rolls_their_own_loot_on_a_live_chest() {
    let mut mgr = rig();

    let a = press(&mut mgr, P1, Some(TABLE), true).await;
    assert_eq!(loot_displays(&a), 1, "the window opens: {a:?}");
    assert_eq!(persisted(&a), vec!["Chest".to_string()]);
    assert!(mgr
        .get_entity(P1)
        .unwrap()
        .looted_containers
        .contains("Chest"));
    assert_eq!(mgr.get_entity(P1).unwrap().looting_entity, Some(CHEST));

    let b = press(&mut mgr, P2, Some(TABLE), true).await;
    assert_eq!(loot_displays(&b), 1);
    let (r1, r2) = (roll_of(&mgr, 101), roll_of(&mgr, 102));
    assert_eq!((r1.len(), r2.len()), (2, 2), "both rolls are whole");
    assert!(
        r1.iter().all(|i| !r2.contains(i)),
        "the two rolls are separate lists: {r1:?} vs {r2:?}"
    );
    let chest = mgr.get_entity(CHEST).unwrap();
    assert!(chest.is_loot_container);
    assert!(chest.loot.is_empty(), "nothing lands in corpse loot");
    assert_eq!(
        chest.interaction_type_flags, 0,
        "a live container keeps its own flags"
    );
}

/// Once per character: a press with loot still pending reopens it untouched;
/// once taken, a press rolls nothing and says so. The flag, not the pending
/// list, is what blocks the second roll.
#[tokio::test]
async fn a_once_chest_reopens_pending_loot_then_refuses_a_second_roll() {
    let mut mgr = rig();
    press(&mut mgr, P1, Some(TABLE), true).await;
    let first = roll_of(&mgr, 101);

    let again = press(&mut mgr, P1, Some(TABLE), true).await;
    assert_eq!(loot_displays(&again), 1, "pending loot reopens: {again:?}");
    assert!(persisted(&again).is_empty(), "no second flag write");
    assert_eq!(roll_of(&mgr, 101), first, "reopened, not re-rolled");

    // The player takes everything (the lootItem path empties and prunes).
    mgr.loot_list_mut(CHEST, 101).unwrap().clear();
    mgr.prune_container_loot(CHEST, 101);

    let spent = press(&mut mgr, P1, Some(TABLE), true).await;
    assert_eq!(loot_displays(&spent), 0, "no second roll: {spent:?}");
    assert_eq!(feedback(&spent), 1, "the press still gets a line");
    assert!(roll_of(&mgr, 101).is_empty());

    // The same flag after a relog: a fresh entity stamped from the DB row.
    let pid = 101;
    mgr.get_entity_mut(P1).unwrap().looted_containers.clear();
    mgr.get_entity_mut(P1)
        .unwrap()
        .looted_containers
        .insert("Chest".into());
    let relogged = press(&mut mgr, P1, Some(TABLE), true).await;
    assert_eq!(loot_displays(&relogged), 0);
    assert!(roll_of(&mgr, pid).is_empty());
}

/// Without the once flag (the debug crate) every open re-rolls and replaces
/// the pending list, and nothing is persisted.
#[tokio::test]
async fn a_repeatable_chest_rerolls_on_every_open() {
    let mut mgr = rig();
    let a = press(&mut mgr, P1, Some(TABLE), false).await;
    let first = roll_of(&mgr, 101);
    let b = press(&mut mgr, P1, Some(TABLE), false).await;
    let second = roll_of(&mgr, 101);
    assert_eq!((loot_displays(&a), loot_displays(&b)), (1, 1));
    assert!(persisted(&a).is_empty() && persisted(&b).is_empty());
    assert_eq!(second.len(), 2);
    assert!(
        second.iter().all(|i| !first.contains(i)),
        "a re-roll, not a reopen: {first:?} then {second:?}"
    );
}

/// Reopen-only (`loot_table_id: None`) never rolls: with nothing pending it
/// answers with a line, with loot pending it reopens it.
#[tokio::test]
async fn reopen_only_never_rolls() {
    let mut mgr = rig();
    let none = press(&mut mgr, P1, None, false).await;
    assert_eq!((loot_displays(&none), feedback(&none)), (0, 1));
    assert!(roll_of(&mgr, 101).is_empty());

    press(&mut mgr, P1, Some(TABLE), true).await;
    let pending = press(&mut mgr, P1, None, false).await;
    assert_eq!(loot_displays(&pending), 1, "pending loot reopens");
}

/// A chest out of interact range opens nothing and sets no flag.
#[tokio::test]
async fn an_out_of_range_chest_opens_nothing() {
    let mut mgr = rig();
    mgr.update_entity_position(P1, [60.0, 0.0, 60.0], [0; 3], [0.0; 3]);
    let msgs = press(&mut mgr, P1, Some(TABLE), true).await;
    assert_eq!((loot_displays(&msgs), feedback(&msgs)), (0, 1));
    assert!(persisted(&msgs).is_empty());
    assert!(!mgr
        .get_entity(P1)
        .unwrap()
        .looted_containers
        .contains("Chest"));
}
