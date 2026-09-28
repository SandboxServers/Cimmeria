//! Native `gmGotoLocation` with `(0, 0, 0)`: the world's entry point instead
//! of the origin (`travel/world_entry_point.rs`).

use super::*; // shared helpers from tests/mod.rs
use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner::RespawnerDef;
use tokio::sync::mpsc;

fn goto_location_args(world: &str, pos: [f32; 3]) -> Vec<u8> {
    let mut args = Vec::new();
    write_wstring_arg(&mut args, world);
    for c in pos {
        args.extend_from_slice(&c.to_le_bytes());
    }
    args
}

fn only_gate_travel_position(msgs: &[CellToBaseMsg]) -> [f32; 3] {
    let travels: Vec<_> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::GateTravel { position, .. } => Some(*position),
            _ => None,
        })
        .collect();
    assert_eq!(travels.len(), 1, "expected one GateTravel, got {msgs:?}");
    travels[0]
}

/// Reverting the sentinel branch sends the GM to the literal origin.
#[tokio::test]
async fn gm_goto_location_origin_lands_on_the_world_entry_point() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.respawners.push(RespawnerDef {
        respawner_id: 3,
        world_name: "Castle".to_string(),
        name: "Checkpoint".to_string(),
        pos: [800.0, 55.0, 510.0],
    });
    let (tx, mut rx) = mpsc::channel(16);

    let args = goto_location_args("castle", [0.0; 3]);
    assert!(dispatch(1, GM_GOTO_LOCATION, &args, &tx, &mut mgr, &test_engine()).await);

    let msgs = drain(&mut rx);
    assert_eq!(only_gate_travel_position(&msgs), [800.0, 55.0, 510.0]);
    let line = feedback_text(&msgs, 1).expect("the GM must be told");
    assert!(line.contains("[respawner]"), "got {line:?}");
}

/// A known world with nowhere to land is refused, not sent to the origin.
#[tokio::test]
async fn gm_goto_location_origin_without_an_entry_point_is_refused() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(16);

    let args = goto_location_args("Castle", [0.0; 3]);
    assert!(dispatch(1, GM_GOTO_LOCATION, &args, &tx, &mut mgr, &test_engine()).await);

    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GateTravel { .. })),
        "no entry point must not GateTravel: {msgs:?}"
    );
    let line = feedback_text(&msgs, 1).expect("the GM must be told");
    assert!(line.contains("no known entry point"), "got {line:?}");
    assert!(mgr.get_entity(1).is_some(), "the GM must stay in place");
}

/// Typed coordinates are untouched by the entry-point rule.
#[tokio::test]
async fn gm_goto_location_typed_coordinates_are_kept() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.respawners.push(RespawnerDef {
        respawner_id: 3,
        world_name: "Castle".to_string(),
        name: "Checkpoint".to_string(),
        pos: [800.0, 55.0, 510.0],
    });
    let (tx, mut rx) = mpsc::channel(16);

    let args = goto_location_args("Castle", [1.0, 2.0, 3.0]);
    assert!(dispatch(1, GM_GOTO_LOCATION, &args, &tx, &mut mgr, &test_engine()).await);

    assert_eq!(only_gate_travel_position(&drain(&mut rx)), [1.0, 2.0, 3.0]);
}
