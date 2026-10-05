//! DA-10: `activateSpawnSet` (214) / `deactivateSpawnSet` (215) and
//! `.spawnset`, over the two fixture lineup sets.
//!
//! Filter prefix: `spawn_set_`.
//!
//! Bug shapes: a non-GM reaching the switch; a set switched off leaving its
//! actors on a witness's client (no `LeftAoI`); two groups of one kind on at
//! once (the full lineup ran the 32-bit client out of memory); a repeat
//! press spawning a second copy of a set, or sending a second `LeftAoI`.
//!
//! Revert proofs: drop the `active_peers` loop in `show_spawn_set` and
//! `spawn_set_showing_one_group_switches_the_other_off_for_every_witness`
//! fails on set A's count; swap `despawn_npc_releasing_combat` for a bare
//! `remove_entity` and the same test fails on the `LeftAoI` rows; drop the
//! `is_active` early return in `show_spawn_set` and
//! `spawn_set_repeat_presses_change_nothing` fails on the entity count.

use cimmeria_cell_world::test_fixtures::{
    install_lineup_sets, LINEUP_SET_A, LINEUP_SET_A_NAME, LINEUP_SET_A_SIZE, LINEUP_SET_B,
    LINEUP_SET_B_NAME, LINEUP_SET_B_SIZE,
};
use cimmeria_common::EntityId;

use super::*;
use crate::cell::dispatch::gm_gate::{enforce_gm_gate, requires_gm};

const GM: u32 = 1;
/// A connected player standing by, who sees every lineup actor.
const WITNESS: u32 = 2;

fn fixture() -> SpaceManager {
    let mut mgr = mgr_with_player(GM, "Castle");
    mgr.connect_entity(GM);
    mgr.create_entity(WITNESS, "Castle", [4.0, 0.0, 4.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(WITNESS);
    let w = mgr.get_entity_mut(WITNESS).unwrap();
    w.is_player = true;
    w.player_id = Some(101);
    let leftover = install_lineup_sets(&mut mgr, "Castle");
    assert_eq!(
        leftover.len(),
        1,
        "fixture: only the non-member stays a startup record"
    );
    mgr
}

/// What the AoI tick does once the actors are in range: both players see
/// every live lineup actor.
fn witness_all(mgr: &mut SpaceManager, set_id: i32) {
    let live = mgr.spawn_sets.get(set_id).unwrap().live.clone();
    for p in [GM, WITNESS] {
        let e = mgr.get_entity_mut(p).unwrap();
        for &id in &live {
            e.witnesses.insert(EntityId(id as i32));
        }
    }
}

async fn call(mgr: &mut SpaceManager, index: u16, set_id: i32) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    assert!(
        dispatch(GM, index, &set_id.to_le_bytes(), &tx, mgr, &test_engine()).await,
        "index {index} must be handled"
    );
    drain(&mut rx)
}

fn left_aoi(msgs: &[CellToBaseMsg]) -> Vec<(u32, u32)> {
    let mut rows: Vec<(u32, u32)> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::LeftAoI {
                witness_id,
                entity_id,
            } => Some((*witness_id, *entity_id)),
            _ => None,
        })
        .collect();
    rows.sort_unstable();
    rows
}

fn npc_count(mgr: &SpaceManager) -> usize {
    mgr.spaces
        .values()
        .flat_map(|s| s.entities.values())
        .filter(|e| !e.is_player)
        .count()
}

/// **Server authority.** Both indices sit in the GM tail: a player gets
/// `onErrorCode` and no handler runs; a GM passes.
#[tokio::test]
async fn spawn_set_indices_are_gated_for_non_gms() {
    for idx in [ACTIVATE_SPAWN_SET, DEACTIVATE_SPAWN_SET] {
        assert!(requires_gm(idx), "{idx} must be GM-gated");
        let mut mgr = fixture();
        mgr.get_entity_mut(GM).unwrap().access_level = 0;
        let (tx, mut rx) = mpsc::channel(8);
        assert!(!enforce_gm_gate(GM, idx, &tx, &mgr).await, "{idx}: refused");
        let msgs = drain(&mut rx);
        assert!(
            matches!(
                msgs.as_slice(),
                [CellToBaseMsg::EntityMethodCall {
                    method_index: 121,
                    ..
                }]
            ),
            "{idx}: only onErrorCode: {msgs:?}"
        );
        mgr.get_entity_mut(GM).unwrap().access_level = 2;
        assert!(
            enforce_gm_gate(GM, idx, &tx, &mgr).await,
            "{idx}: GM passes"
        );
    }
}

/// Nothing in a set spawns at boot; activating it spawns every member and
/// says so on the first press.
#[tokio::test]
async fn spawn_set_activate_spawns_every_member() {
    let mut mgr = fixture();
    assert_eq!(npc_count(&mgr), 0, "no lineup actor at boot");
    let msgs = call(&mut mgr, ACTIVATE_SPAWN_SET, LINEUP_SET_A).await;
    assert_eq!(npc_count(&mgr), LINEUP_SET_A_SIZE);
    let set = mgr.spawn_sets.get(LINEUP_SET_A).unwrap();
    assert!(set.is_active());
    for id in &set.live {
        assert!(mgr.get_entity(*id).is_some(), "live id {id} exists");
    }
    let line = feedback_text(&msgs, GM).expect("a line on the first press");
    assert!(
        line.contains(LINEUP_SET_A_NAME)
            && line.contains(&format!("showing {LINEUP_SET_A_SIZE} actors"))
            && line.contains("one group at a time"),
        "{line}"
    );
}

/// Exclusive: showing B switches A off first, and every player who saw an
/// A actor is told it left.
#[tokio::test]
async fn spawn_set_showing_one_group_switches_the_other_off_for_every_witness() {
    let mut mgr = fixture();
    call(&mut mgr, ACTIVATE_SPAWN_SET, LINEUP_SET_A).await;
    witness_all(&mut mgr, LINEUP_SET_A);
    let a_ids = mgr.spawn_sets.get(LINEUP_SET_A).unwrap().live.clone();

    let msgs = call(&mut mgr, ACTIVATE_SPAWN_SET, LINEUP_SET_B).await;

    assert!(!mgr.spawn_sets.get(LINEUP_SET_A).unwrap().is_active());
    assert!(mgr.spawn_sets.get(LINEUP_SET_B).unwrap().is_active());
    assert_eq!(npc_count(&mgr), LINEUP_SET_B_SIZE, "only B is loaded");
    for id in &a_ids {
        assert!(mgr.get_entity(*id).is_none(), "A actor {id} is gone");
    }
    let mut want: Vec<(u32, u32)> = a_ids
        .iter()
        .flat_map(|&id| [(GM, id), (WITNESS, id)])
        .collect();
    want.sort_unstable();
    assert_eq!(left_aoi(&msgs), want, "every witness drops every A actor");
    let line = feedback_text(&msgs, GM).unwrap();
    assert!(
        line.contains(LINEUP_SET_B_NAME)
            && line.contains(&format!(
                "Cleared first: {LINEUP_SET_A_NAME} ({LINEUP_SET_A_SIZE})"
            )),
        "{line}"
    );
}

/// Idempotent: a second activate spawns no second copy, a second deactivate
/// sends no second `LeftAoI`, and each still answers.
#[tokio::test]
async fn spawn_set_repeat_presses_change_nothing() {
    let mut mgr = fixture();
    call(&mut mgr, ACTIVATE_SPAWN_SET, LINEUP_SET_A).await;
    witness_all(&mut mgr, LINEUP_SET_A);

    let again = call(&mut mgr, ACTIVATE_SPAWN_SET, LINEUP_SET_A).await;
    assert_eq!(npc_count(&mgr), LINEUP_SET_A_SIZE, "no second copy");
    assert!(left_aoi(&again).is_empty());
    assert!(feedback_text(&again, GM)
        .unwrap()
        .contains("already showing"));

    let off = call(&mut mgr, DEACTIVATE_SPAWN_SET, LINEUP_SET_A).await;
    assert_eq!(npc_count(&mgr), 0);
    assert_eq!(left_aoi(&off).len(), 2 * LINEUP_SET_A_SIZE);
    assert!(feedback_text(&off, GM).unwrap().contains("Cleared"));

    let off_again = call(&mut mgr, DEACTIVATE_SPAWN_SET, LINEUP_SET_A).await;
    assert!(left_aoi(&off_again).is_empty(), "nothing left to leave");
    assert!(feedback_text(&off_again, GM)
        .unwrap()
        .contains("is not showing"));

    // And it can come back.
    call(&mut mgr, ACTIVATE_SPAWN_SET, LINEUP_SET_A).await;
    assert_eq!(npc_count(&mgr), LINEUP_SET_A_SIZE);
}

/// An unknown id and a missing argument both answer, and change nothing.
#[tokio::test]
async fn spawn_set_unknown_id_and_bad_args_answer() {
    let mut mgr = fixture();
    let msgs = call(&mut mgr, ACTIVATE_SPAWN_SET, 9999).await;
    assert!(feedback_text(&msgs, GM)
        .unwrap()
        .contains("no spawn set 9999"));
    let (tx, mut rx) = mpsc::channel(8);
    assert!(
        dispatch(
            GM,
            DEACTIVATE_SPAWN_SET,
            &[1, 2],
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    let msgs = drain(&mut rx);
    assert!(feedback_text(&msgs, GM).unwrap().contains("missing INT32"));
    assert_eq!(npc_count(&mgr), 0);
}

/// `.spawnset` drives the same switch: `on`, `list`, `clear`.
#[tokio::test]
async fn spawn_set_dot_console_on_list_clear() {
    let mut mgr = fixture();
    let (tx, mut rx) = mpsc::channel(64);
    let engine = test_engine();
    crate::cell::console::handle_console_command(
        GM,
        &format!(".spawnset on {LINEUP_SET_B}"),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert_eq!(npc_count(&mgr), LINEUP_SET_B_SIZE);
    drain(&mut rx);

    crate::cell::console::handle_console_command(GM, ".spawnset", &tx, &mut mgr, &engine).await;
    let listed = drain(&mut rx);
    let lines: Vec<String> = listed
        .iter()
        .filter_map(|m| feedback_text(std::slice::from_ref(m), GM))
        .collect();
    assert_eq!(lines.len(), 2, "one line per set: {lines:?}");
    assert!(
        lines[0].contains(LINEUP_SET_A_NAME) && lines[0].ends_with("off"),
        "{lines:?}"
    );
    assert!(
        lines[1].contains(LINEUP_SET_B_NAME) && lines[1].contains("ON"),
        "{lines:?}"
    );

    witness_all(&mut mgr, LINEUP_SET_B);
    crate::cell::console::handle_console_command(GM, ".spawnset clear", &tx, &mut mgr, &engine)
        .await;
    let cleared = drain(&mut rx);
    assert_eq!(npc_count(&mgr), 0);
    assert_eq!(left_aoi(&cleared).len(), 2 * LINEUP_SET_B_SIZE);
}
