use super::super::*; // gm module: dispatch + GM_* constants
use super::*; // shared helpers from tests/mod.rs
use crate::cell::messages::CellToBaseMsg;
use tokio::sync::mpsc;

/// Build `(WSTRING DesignId, FLOAT XOffset, FLOAT ZOffset)`.
fn spawn_args(design_id: &str, x_off: f32, z_off: f32) -> Vec<u8> {
    let mut args = Vec::new();
    write_wstring_arg(&mut args, design_id);
    args.extend_from_slice(&x_off.to_le_bytes());
    args.extend_from_slice(&z_off.to_le_bytes());
    args
}

#[tokio::test]
async fn gm_spawn_by_cmd_emits_spawn_with_offset_position() {
    let mut mgr = mgr_with_player(1, "Castle");
    // Place the caller at a known position so the offset is observable.
    {
        let e = mgr.get_entity_mut(1).unwrap();
        e.position = cimmeria_common::Vector3::new(100.0, 50.0, 200.0);
    }
    let (tx, mut rx) = mpsc::channel(8);

    // Template 4321, X+10, Z-5 → spawn at (110, 50, 195) (Y unchanged).
    let args = spawn_args("4321", 10.0, -5.0);
    assert!(dispatch(1, GM_SPAWN_BY_CMD, &args, &tx, &mut mgr).await);

    match rx.try_recv().expect("gmSpawnByCmd must emit GmSpawnNpc") {
        CellToBaseMsg::GmSpawnNpc {
            entity_id,
            template_id,
            space_id: _,
            world_name,
            position,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(template_id, 4321);
            assert_eq!(world_name, "Castle");
            assert_eq!(position[0], 110.0, "X offset applied");
            assert_eq!(position[1], 50.0, "Y unchanged (ground-plane offset)");
            assert_eq!(position[2], 195.0, "Z offset applied");
        }
        other => panic!("expected GmSpawnNpc, got {other:?}"),
    }

    // Base round-trip: the cell emits ONLY the spawn request — no cell-side
    // feedback. The definitive "spawned npc <id>" line is sent later, from the
    // cell's `GmSpawnNpcReady` handler once the spawn actually takes (and the
    // base sends a "template not found" line if the lookup fails). The cell no
    // longer feeds back an optimistic "requested" line here.
    assert!(
        feedback_text(&drain(&mut rx), 1).is_none(),
        "the cell must NOT emit success feedback when only requesting the spawn"
    );
}

#[tokio::test]
async fn gm_spawn_by_cmd_rejects_non_numeric_template() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    // Non-numeric DesignId — internal-name resolution isn't wired in the cell.
    assert!(
        dispatch(
            1,
            GM_SPAWN_BY_CMD,
            &spawn_args("Goauld", 0.0, 0.0),
            &tx,
            &mut mgr
        )
        .await
    );
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GmSpawnNpc { .. })),
        "non-numeric template id must not spawn"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "non-numeric template id must feed back a rejection"
    );

    // Zero / negative template id.
    assert!(
        dispatch(
            1,
            GM_SPAWN_BY_CMD,
            &spawn_args("0", 0.0, 0.0),
            &tx,
            &mut mgr
        )
        .await
    );
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GmSpawnNpc { .. })),
        "template id 0 must not spawn"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "template id 0 must feed back a rejection"
    );
}

#[tokio::test]
async fn gm_spawn_by_cmd_rejects_truncated_and_nonfinite() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    // WSTRING present but missing the two FLOAT offsets.
    let mut short = Vec::new();
    write_wstring_arg(&mut short, "4321");
    assert!(dispatch(1, GM_SPAWN_BY_CMD, &short, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GmSpawnNpc { .. })),
        "missing offsets must not spawn"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "missing offsets must feed back a rejection"
    );

    // NaN offset → non-finite computed position → reject.
    let args = spawn_args("4321", f32::NAN, 0.0);
    assert!(dispatch(1, GM_SPAWN_BY_CMD, &args, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GmSpawnNpc { .. })),
        "non-finite spawn position must not spawn"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "non-finite spawn position must feed back a rejection"
    );
}
