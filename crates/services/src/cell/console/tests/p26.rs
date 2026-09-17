//! Packet P26 regression suite: `.gotoxyz` (absent, new registration).
//!
//! Same-space authoritative teleport of the selected target, falling back to
//! the caller when nothing is selected. Reuses the native `gmGotoXYZ`/
//! `gmSummon` mechanism cell-side
//! (`crates/services/src/cell/cell_methods/gm/travel.rs`):
//! `update_entity_position` + `note_authorized_teleport`, then
//! `TeleportPlayer` for a player target only. See
//! `crates/services/src/cell/cell_methods/gm/tests/travel.rs` for the native
//! handlers' own coverage of the shared mechanism (this suite does not
//! re-prove `update_entity_position`/AoI internals, only the `.gotoxyz`
//! command's own target-resolution, player/NPC split, and feedback-routing
//! behavior).
//!
//! Filter prefix: `legacy_p26_`.

use cimmeria_common::Vector3;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::exec;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Create a fresh connected player entity in `setup()`'s "Agnos" space.
fn player_target(mgr: &mut SpaceManager, entity_id: u32, pos: [f32; 3]) {
    mgr.create_entity(entity_id, "Agnos", pos, [0.0; 3])
        .unwrap();
    mgr.connect_entity(entity_id);
    if let Some(e) = mgr.get_entity_mut(entity_id) {
        e.is_player = true;
    }
}

/// Drain every queued message, returning the (at most one) `TeleportPlayer`
/// payload and every decoded feedback line.
fn drain_teleport(
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) -> (Option<(u32, u32, [f32; 3], [f32; 3])>, Vec<String>) {
    let mut teleport = None;
    let mut feedback = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match &msg {
            CellToBaseMsg::TeleportPlayer {
                entity_id,
                space_id,
                position,
                prev_pos,
            } => {
                let payload = (*entity_id, *space_id, *position, *prev_pos);
                assert!(
                    teleport.replace(payload).is_none(),
                    "received more than one TeleportPlayer message"
                );
            }
            _ => {
                if let Some(text) = decode_feedback(&msg) {
                    feedback.push(text);
                }
            }
        }
    }
    (teleport, feedback)
}

/// Happy path, NPC target: exactly the target's grid position moves, no
/// `TeleportPlayer` is emitted (an NPC has no client to snap), and the
/// caller receives the feedback line.
#[tokio::test]
async fn legacy_p26_gotoxyz_moves_selected_npc_target() {
    let (mut mgr, gm, npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec(
        "gotoxyz",
        gm,
        &["10", "20", "30"],
        Some(npc),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let p = mgr.get_entity(npc).unwrap().position;
    assert_eq!([p.x, p.y, p.z], [10.0, 20.0, 30.0], "NPC must be moved");

    let (teleport, feedback) = drain_teleport(&mut rx);
    assert_eq!(
        teleport, None,
        "an NPC target must never emit TeleportPlayer"
    );
    assert!(
        feedback.iter().any(|t| t.contains("moved entity")),
        "the caller must receive a feedback line; got {feedback:?}"
    );
}

/// Exact final position + spatial-grid consistency: after the move, the
/// spatial grid's own position lookup — not just `cell_entity.position` in
/// isolation — reflects the destination. Proven via the AoI tick: a witness
/// already tracking the target must see it moved on the next tick, which
/// only happens if `SpaceGrid::update_position` actually ran.
#[tokio::test]
async fn legacy_p26_gotoxyz_npc_move_broadcast_to_witness() {
    let (mut mgr, gm, _npc) = setup();
    mgr.connect_entity(gm); // AoI walks players; ensure the caller is tracked
    mgr.get_entity_mut(gm).unwrap().position = Vector3 {
        x: 5.0,
        y: 0.0,
        z: 5.0,
    };
    // A fresh NPC within the caller's AoI radius so tick 1 makes it a witness.
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [6.0, 0.0, 6.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(gm).unwrap().current_target_id = Some(npc as i32);

    let _ = mgr.compute_aoi_changes(); // tick 1: NPC enters AoI

    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(16);
    exec(
        "gotoxyz",
        gm,
        &["40", "0", "40"],
        Some(npc),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let moved = mgr.compute_aoi_changes().into_iter().find_map(|m| match m {
        CellToBaseMsg::EntityMoved {
            witness_id,
            entity_id,
            position,
            ..
        } if witness_id == gm && entity_id == npc => Some(position),
        _ => None,
    });
    assert_eq!(
        moved,
        Some([40.0, 0.0, 40.0]),
        "the moved NPC must be broadcast to the caller-witness at the new position"
    );
}

/// Happy path, distinct player target: exactly one `TeleportPlayer` naming
/// the TARGET (never the caller), the exact destination position, and the
/// prior position captured before the grid update. Feedback goes to the
/// caller — `TeleportPlayer` itself carries no GM-feedback field, so this is
/// the console handler's own immediate cell-side line (D03: caller/subject
/// split, matching the native `gmSummon`/`gmGotoXYZ` pattern of feeding back
/// to the actor who issued the command, not the moved entity).
#[tokio::test]
async fn legacy_p26_gotoxyz_player_target_emits_teleport_player_exact_bytes() {
    let (mut mgr, gm, _npc) = setup();
    let target = 2u32;
    player_target(&mut mgr, target, [1.0, 0.0, 1.0]);
    let expected_space_id = mgr.get_entity(target).unwrap().space_id.0 as u32;
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    mgr.get_entity_mut(gm).unwrap().current_target_id = Some(target as i32);

    exec(
        "gotoxyz",
        gm,
        &["99.5", "1.0", "-42.25"],
        Some(target),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let (teleport, feedback) = drain_teleport(&mut rx);
    assert_eq!(
        teleport,
        Some((
            target,
            expected_space_id,
            [99.5, 1.0, -42.25],
            [1.0, 0.0, 1.0]
        )),
        "TeleportPlayer must name the target, carry the exact destination \
         and the pre-move position"
    );
    assert!(
        feedback.iter().any(|t| t.contains("moved entity")),
        "the caller must receive the feedback line, not the target; got {feedback:?}"
    );

    let p = mgr.get_entity(target).unwrap().position;
    assert_eq!([p.x, p.y, p.z], [99.5, 1.0, -42.25]);
}

/// A player target's move is still broadcast to other witnesses via the
/// normal AoI tick, independent of the `TeleportPlayer` snap (which only
/// updates the *moved player's own* client).
#[tokio::test]
async fn legacy_p26_gotoxyz_player_target_move_broadcast_to_witness() {
    let (mut mgr, gm, _npc) = setup();
    mgr.connect_entity(gm);
    mgr.get_entity_mut(gm).unwrap().position = Vector3 {
        x: 5.0,
        y: 0.0,
        z: 5.0,
    };
    let target = 2u32;
    player_target(&mut mgr, target, [6.0, 0.0, 6.0]);
    mgr.get_entity_mut(gm).unwrap().current_target_id = Some(target as i32);

    let _ = mgr.compute_aoi_changes(); // tick 1: target enters caller's AoI

    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(16);
    exec(
        "gotoxyz",
        gm,
        &["40", "0", "40"],
        Some(target),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let moved = mgr.compute_aoi_changes().into_iter().find_map(|m| match m {
        CellToBaseMsg::EntityMoved {
            witness_id,
            entity_id,
            position,
            ..
        } if witness_id == gm && entity_id == target => Some(position),
        _ => None,
    });
    assert_eq!(
        moved,
        Some([40.0, 0.0, 40.0]),
        "the moved player must be broadcast to the caller-witness at the new position"
    );
}

/// No selection (`Target::None` with nothing currently targeted) falls back
/// to the caller — legacy `entity = target or player`.
#[tokio::test]
async fn legacy_p26_gotoxyz_falls_back_to_caller_when_no_target() {
    let (mut mgr, gm, _npc) = setup();
    mgr.get_entity_mut(gm).unwrap().current_target_id = None;
    let caller_before = mgr.get_entity(gm).unwrap().position;
    let expected_space_id = mgr.get_entity(gm).unwrap().space_id.0 as u32;
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    // `Target::None` means dispatch resolves no target (`None`); `exec` is
    // driven directly here with `target_id: None` to isolate the handler's
    // own fallback from `resolve_target`'s selection lookup.
    exec(
        "gotoxyz",
        gm,
        &["7", "8", "9"],
        None,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let p = mgr.get_entity(gm).unwrap().position;
    assert_eq!([p.x, p.y, p.z], [7.0, 8.0, 9.0], "the caller must be moved");
    let (teleport, feedback) = drain_teleport(&mut rx);
    assert_eq!(
        teleport,
        Some((
            gm,
            expected_space_id,
            [7.0, 8.0, 9.0],
            [caller_before.x, caller_before.y, caller_before.z]
        )),
        "the caller is a player, so the fallback move still snaps via TeleportPlayer"
    );
    assert!(feedback.iter().any(|t| t.contains("moved entity")));
}

/// A distinct selected target leaves the caller's own position unchanged —
/// the same D03 concern P05 resolved for grants, here for a position write.
#[tokio::test]
async fn legacy_p26_gotoxyz_caller_unchanged_when_moving_selection() {
    let (mut mgr, gm, npc) = setup();
    let caller_start = mgr.get_entity(gm).unwrap().position;
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(16);

    exec(
        "gotoxyz",
        gm,
        &["77", "0", "88"],
        Some(npc),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let p = mgr.get_entity(gm).unwrap().position;
    assert_eq!(
        [p.x, p.y, p.z],
        [caller_start.x, caller_start.y, caller_start.z],
        "moving a distinct selection must not move the caller"
    );
}

/// Non-finite coordinates (NaN/inf) are rejected by the shared `parse_f32`
/// finite filter — no grid mutation, no `TeleportPlayer`.
#[tokio::test]
async fn legacy_p26_gotoxyz_rejects_non_finite_coordinates() {
    for bad in ["NaN", "inf", "-inf"] {
        let (mut mgr, gm, npc) = setup();
        let start = mgr.get_entity(npc).unwrap().position;
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        exec(
            "gotoxyz",
            gm,
            &[bad, "0", "0"],
            Some(npc),
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let p = mgr.get_entity(npc).unwrap().position;
        assert_eq!(
            [p.x, p.y, p.z],
            [start.x, start.y, start.z],
            "{bad} coordinate must not move the entity"
        );
        let (teleport, feedback) = drain_teleport(&mut rx);
        assert_eq!(teleport, None, "{bad} must not emit TeleportPlayer");
        assert!(
            feedback.iter().any(|t| t.contains("finite")),
            "{bad} must feed back a finite-number rejection; got {feedback:?}"
        );
    }
}

/// Malformed (non-numeric) or missing coordinate args are rejected the same
/// way, leaving the entity unmoved.
#[tokio::test]
async fn legacy_p26_gotoxyz_rejects_malformed_args() {
    let (mut mgr, gm, npc) = setup();
    let start = mgr.get_entity(npc).unwrap().position;
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec(
        "gotoxyz",
        gm,
        &["abc", "0", "0"],
        Some(npc),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let p = mgr.get_entity(npc).unwrap().position;
    assert_eq!([p.x, p.y, p.z], [start.x, start.y, start.z]);
    let (teleport, feedback) = drain_teleport(&mut rx);
    assert_eq!(teleport, None);
    assert!(feedback.iter().any(|t| t.contains("x")));
}

/// A cross-space current selection is dropped to "no target" by
/// `resolve_target`'s existing `Target::None` semantics (matches every other
/// optional-target command, e.g. `.info`) — `.gotoxyz` then falls back to
/// the caller rather than erroring.
#[tokio::test]
async fn legacy_p26_gotoxyz_cross_space_selection_falls_back_to_caller() {
    let (mut mgr, gm, _npc) = setup();
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Other" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Other" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(2, "Other", [50.0, 0.0, 60.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(gm).unwrap().current_target_id = Some(2);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    // Drive through `handle_console_command` so `resolve_target` actually
    // runs (unlike the other tests here, which call `exec` directly).
    crate::cell::console::handle_console_command(gm, ".gotoxyz 3 0 3", &tx, &mut mgr, &engine)
        .await;

    let p = mgr.get_entity(gm).unwrap().position;
    assert_eq!(
        [p.x, p.y, p.z],
        [3.0, 0.0, 3.0],
        "the caller must be moved when the selection is out of space"
    );
    let other = mgr.get_entity(2).unwrap().position;
    assert_eq!(
        [other.x, other.y, other.z],
        [50.0, 0.0, 60.0],
        "the cross-space entity must be left untouched"
    );
    let (_, feedback) = drain_teleport(&mut rx);
    assert!(feedback.iter().any(|t| t.contains("moved entity")));
}

/// Base channel closed before the `TeleportPlayer` send: the function must
/// return cleanly (not panic/hang) rather than unconditionally claiming a
/// snap that never reached the base. The cell-side grid write already
/// happened (position is cell-authoritative the instant
/// `update_entity_position` runs — the base round-trip only pushes the wire
/// packet and persists), matching the native `gmGotoXYZ`/`gmSummon` handlers'
/// documented behavior.
#[tokio::test]
async fn legacy_p26_gotoxyz_player_target_survives_closed_channel() {
    let (mut mgr, gm, _npc) = setup();
    let target = 2u32;
    player_target(&mut mgr, target, [1.0, 0.0, 1.0]);
    mgr.get_entity_mut(gm).unwrap().current_target_id = Some(target as i32);
    let engine = ChainEngine::new();
    let (tx, rx) = mpsc::channel(16);
    drop(rx); // simulate a closed base channel

    exec(
        "gotoxyz",
        gm,
        &["5", "0", "5"],
        Some(target),
        &tx,
        &mut mgr,
        &engine,
    )
    .await; // must not panic

    let p = mgr.get_entity(target).unwrap().position;
    assert_eq!(
        [p.x, p.y, p.z],
        [5.0, 0.0, 5.0],
        "the cell-side grid write is authoritative even if the base push failed"
    );
}
