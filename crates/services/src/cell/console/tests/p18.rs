//! Packet P18 regression suite: `.location`, `.rotation` (both absent, new
//! registrations).
//!
//! Dual-mode selected-spawnable placement: zero args reports, a complete
//! three-tuple sets. Legacy `deprecated/python/cell/commands/Entity.py:101-134`
//! gated the write on `z is not None`, so a 1- or 2-argument invocation
//! silently did nothing and reported anyway; D02 says to correct that, so the
//! partial-tuple tests below assert an explicit rejection *and* no mutation.
//!
//! The set path reuses `.gotoxyz`'s snap abstraction
//! (`update_entity_position` + `note_authorized_teleport` + `TeleportPlayer`
//! for players only) — see [`super::p26`] for that mechanism's own coverage.
//! What is proven here and nowhere else:
//!
//!   * the 0-or-3 arity gate, including the no-mutation guarantee;
//!   * `.location`'s orientation preservation across the grid write
//!     (`update_entity_position` zeroes `direction` from its `[i8; 3]`
//!     parameter — the regression this suite exists for);
//!   * `.rotation` writing `direction` as `[pitch, yaw, roll]` radians
//!     directly, with no `atan2`/vector conversion;
//!   * both changes reaching a witness through `EntityMoved`.
//!
//! Filter prefix: `legacy_p18_`.

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
fn drain(
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

// ── .location ───────────────────────────────────────────────────────────────

/// Zero args reports the target's current position verbatim and mutates
/// nothing — neither position nor orientation, and no client snap.
#[tokio::test]
async fn legacy_p18_location_no_args_reports_without_mutating() {
    let (mut mgr, gm, npc) = setup();
    mgr.get_entity_mut(npc).unwrap().direction = Vector3::new(0.25, 1.5, -0.75);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec("location", gm, &[], Some(npc), &tx, &mut mgr, &engine).await;

    let e = mgr.get_entity(npc).unwrap();
    assert_eq!(
        [e.position.x, e.position.y, e.position.z],
        [12.0, 0.0, 12.0],
        "a report must not move the entity"
    );
    assert_eq!(
        [e.direction.x, e.direction.y, e.direction.z],
        [0.25, 1.5, -0.75],
        "a report must not touch orientation"
    );

    let (teleport, feedback) = drain(&mut rx);
    assert_eq!(teleport, None, "a report must not emit TeleportPlayer");
    assert_eq!(
        feedback,
        vec![format!(
            "Position of entity {npc} is: (12.000, 0.000, 12.000)"
        )],
        "exactly the legacy report line, naming the target"
    );
}

/// Complete tuple sets the NPC's exact final position, updates the spatial
/// grid, and reports the *new* value back to the caller. No `TeleportPlayer`
/// for an NPC (no client to snap).
#[tokio::test]
async fn legacy_p18_location_sets_exact_position_on_npc() {
    let (mut mgr, gm, npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec(
        "location",
        gm,
        &["99.5", "1.25", "-42.5"],
        Some(npc),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let p = mgr.get_entity(npc).unwrap().position;
    assert_eq!([p.x, p.y, p.z], [99.5, 1.25, -42.5]);

    let (teleport, feedback) = drain(&mut rx);
    assert_eq!(
        teleport, None,
        "an NPC target must never emit TeleportPlayer"
    );
    assert_eq!(
        feedback,
        vec![format!(
            "Position of entity {npc} is: (99.500, 1.250, -42.500)"
        )],
        "the report must echo the post-write position"
    );
}

/// **Primary regression guard.** `update_entity_position` overwrites
/// `direction` from its `[i8; 3]` parameter, so the `[0, 0, 0]` the snap
/// abstraction passes would zero the entity's facing. Legacy `location`
/// assigned `target.position` and nothing else, so `.location` must restore
/// the orientation after the grid write.
///
/// Reverting the `e.direction = facing` write-back in
/// `console::placement::location` turns the asserted vector into
/// `(0.0, 0.0, 0.0)` and fails this test.
#[tokio::test]
async fn legacy_p18_location_set_preserves_orientation() {
    let (mut mgr, gm, npc) = setup();
    mgr.get_entity_mut(npc).unwrap().direction = Vector3::new(0.125, 2.5, -1.25);
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(16);

    exec(
        "location",
        gm,
        &["5", "0", "5"],
        Some(npc),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let e = mgr.get_entity(npc).unwrap();
    assert_eq!(
        [e.position.x, e.position.y, e.position.z],
        [5.0, 0.0, 5.0],
        "the move itself must still land"
    );
    assert_eq!(
        [e.direction.x, e.direction.y, e.direction.z],
        [0.125, 2.5, -1.25],
        "moving an entity must not reset its facing"
    );
}

/// A player target additionally gets the authoritative `TeleportPlayer` snap
/// naming the target (never the caller), the exact destination, and the
/// pre-move position captured before the grid write. Feedback still goes to
/// the caller (D03).
#[tokio::test]
async fn legacy_p18_location_player_target_emits_teleport_player_exact_bytes() {
    let (mut mgr, gm, _npc) = setup();
    let target = 2u32;
    player_target(&mut mgr, target, [1.0, 0.0, 1.0]);
    let expected_space_id = mgr.get_entity(target).unwrap().space_id.0 as u32;
    mgr.get_entity_mut(gm).unwrap().current_target_id = Some(target as i32);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec(
        "location",
        gm,
        &["7.5", "2", "-3.25"],
        Some(target),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let (teleport, feedback) = drain(&mut rx);
    assert_eq!(
        teleport,
        Some((
            target,
            expected_space_id,
            [7.5, 2.0, -3.25],
            [1.0, 0.0, 1.0]
        )),
        "TeleportPlayer must name the target and carry the exact destination \
         plus the pre-move position"
    );
    assert_eq!(
        feedback,
        vec![format!(
            "Position of entity {target} is: (7.500, 2.000, -3.250)"
        )],
        "the caller receives the readout, not the moved player"
    );
}

/// Spatial-grid consistency, not just `cell_entity.position`: a witness
/// already tracking the target sees it at the new coordinates on the next AoI
/// tick, which only happens if `SpaceGrid::update_position` actually ran.
#[tokio::test]
async fn legacy_p18_location_set_broadcast_to_witness() {
    let (mut mgr, gm, _npc) = setup();
    mgr.connect_entity(gm);
    mgr.get_entity_mut(gm).unwrap().position = Vector3::new(5.0, 0.0, 5.0);
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [6.0, 0.0, 6.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(gm).unwrap().current_target_id = Some(npc as i32);

    let _ = mgr.compute_aoi_changes(); // tick 1: NPC enters the caller's AoI

    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(16);
    exec(
        "location",
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
        "the repositioned entity must reach the caller-witness at the new position"
    );
}

// ── .rotation ───────────────────────────────────────────────────────────────

/// Zero args reports the current orientation as pitch/yaw/roll radians plus
/// the yaw in degrees, and mutates nothing.
#[tokio::test]
async fn legacy_p18_rotation_no_args_reports_without_mutating() {
    let (mut mgr, gm, npc) = setup();
    // yaw = pi/2 -> 90 deg heading.
    mgr.get_entity_mut(npc).unwrap().direction =
        Vector3::new(0.0, std::f32::consts::FRAC_PI_2, 0.0);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec("rotation", gm, &[], Some(npc), &tx, &mut mgr, &engine).await;

    let d = mgr.get_entity(npc).unwrap().direction;
    assert_eq!(
        [d.x, d.y, d.z],
        [0.0, std::f32::consts::FRAC_PI_2, 0.0],
        "a report must not rotate the entity"
    );
    let (teleport, feedback) = drain(&mut rx);
    assert_eq!(teleport, None);
    assert_eq!(
        feedback,
        vec![format!(
            "Rotation of entity {npc} is: (pitch 0.000, yaw 1.571, roll 0.000) rad; \
             heading 90.0 deg"
        )],
        "the readout must present yaw from direction.y directly"
    );
}

/// **Semantic guard.** A complete tuple writes `direction` as
/// `[pitch, yaw, roll]` component-for-component — no `atan2`, no
/// normalisation, no vector interpretation. `direction.y` is yaw, which is
/// what `pack_angle(direction[1])` sends and what the player row persists as
/// `heading`.
///
/// A regression that reintroduced `atan2`-style vector handling (the bug
/// class tracked by P48) would not produce these exact components.
#[tokio::test]
async fn legacy_p18_rotation_sets_pitch_yaw_roll_components_directly() {
    let (mut mgr, gm, npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec(
        "rotation",
        gm,
        &["0.5", "-1.25", "2"],
        Some(npc),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let d = mgr.get_entity(npc).unwrap().direction;
    assert_eq!(
        [d.x, d.y, d.z],
        [0.5, -1.25, 2.0],
        "pitch/yaw/roll must land on direction.x/.y/.z unchanged"
    );

    let (_, feedback) = drain(&mut rx);
    assert_eq!(
        feedback,
        vec![format!(
            "Rotation of entity {npc} is: (pitch 0.500, yaw -1.250, roll 2.000) rad; \
             heading -71.6 deg"
        )]
    );
}

/// Rotating never moves the entity, and never emits a client position snap —
/// `BASEMSG_FORCED_POSITION` carries no orientation field, so there is
/// nothing to send.
#[tokio::test]
async fn legacy_p18_rotation_does_not_move_or_snap_player_target() {
    let (mut mgr, gm, _npc) = setup();
    let target = 2u32;
    player_target(&mut mgr, target, [1.0, 0.0, 1.0]);
    mgr.get_entity_mut(gm).unwrap().current_target_id = Some(target as i32);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    exec(
        "rotation",
        gm,
        &["0", "3.0", "0"],
        Some(target),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let e = mgr.get_entity(target).unwrap();
    assert_eq!(
        [e.position.x, e.position.y, e.position.z],
        [1.0, 0.0, 1.0],
        "rotating must not move the entity"
    );
    assert_eq!(
        [e.direction.x, e.direction.y, e.direction.z],
        [0.0, 3.0, 0.0]
    );

    let (teleport, _) = drain(&mut rx);
    assert_eq!(
        teleport, None,
        "orientation has no FORCED_POSITION representation -- no snap is emitted"
    );
}

/// The new facing reaches a witness on the next AoI tick via
/// `EntityMoved.direction`, with no explicit fan-out from the handler.
#[tokio::test]
async fn legacy_p18_rotation_broadcast_to_witness() {
    let (mut mgr, gm, _npc) = setup();
    mgr.connect_entity(gm);
    mgr.get_entity_mut(gm).unwrap().position = Vector3::new(5.0, 0.0, 5.0);
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [6.0, 0.0, 6.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(gm).unwrap().current_target_id = Some(npc as i32);

    let _ = mgr.compute_aoi_changes(); // tick 1: NPC enters the caller's AoI

    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(16);
    exec(
        "rotation",
        gm,
        &["0.25", "1.75", "-0.5"],
        Some(npc),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let dir = mgr.compute_aoi_changes().into_iter().find_map(|m| match m {
        CellToBaseMsg::EntityMoved {
            witness_id,
            entity_id,
            direction,
            ..
        } if witness_id == gm && entity_id == npc => Some(direction),
        _ => None,
    });
    assert_eq!(
        dir,
        Some([0.25, 1.75, -0.5]),
        "the new orientation must reach the caller-witness"
    );
}

// ── shared arity gate ───────────────────────────────────────────────────────

/// Partial tuples (1 or 2 args) are rejected outright with no mutation.
/// Legacy's `if z is not None` gate silently ignored these and reported
/// anyway; D02 says correct the bug rather than reproduce it.
///
/// This is the acceptance criterion "malformed partial tuple causes no
/// mutation" — reverting the `n => Rejected` arm of `parse_triple` to legacy's
/// fall-through-and-report behaviour fails the feedback assertion here.
#[tokio::test]
async fn legacy_p18_partial_tuple_is_rejected_without_mutating() {
    for name in ["location", "rotation"] {
        for args in [vec!["1"], vec!["1", "2"]] {
            let (mut mgr, gm, npc) = setup();
            mgr.get_entity_mut(npc).unwrap().direction = Vector3::new(0.1, 0.2, 0.3);
            let start = mgr.get_entity(npc).unwrap().position;
            let engine = ChainEngine::new();
            let (tx, mut rx) = mpsc::channel(16);

            exec(name, gm, &args, Some(npc), &tx, &mut mgr, &engine).await;

            let e = mgr.get_entity(npc).unwrap();
            assert_eq!(
                [e.position.x, e.position.y, e.position.z],
                [start.x, start.y, start.z],
                ".{name} {args:?} must not move the entity"
            );
            assert_eq!(
                [e.direction.x, e.direction.y, e.direction.z],
                [0.1, 0.2, 0.3],
                ".{name} {args:?} must not rotate the entity"
            );

            let (teleport, feedback) = drain(&mut rx);
            assert_eq!(teleport, None, ".{name} {args:?} must not snap");
            assert_eq!(
                feedback.len(),
                1,
                ".{name} {args:?} must produce exactly the rejection line, \
                 never a readout as well; got {feedback:?}"
            );
            assert!(
                feedback[0].starts_with(&format!("{name}: expected no arguments")),
                ".{name} {args:?} must reject explicitly; got {feedback:?}"
            );
        }
    }
}

/// Non-finite and non-numeric components are rejected by the shared
/// `parse_f32` filter, leaving the entity untouched — and crucially without
/// falling through to a readout that would imply the command succeeded.
#[tokio::test]
async fn legacy_p18_rejects_malformed_components_without_mutating() {
    for name in ["location", "rotation"] {
        for bad in ["NaN", "inf", "-inf", "abc"] {
            let (mut mgr, gm, npc) = setup();
            mgr.get_entity_mut(npc).unwrap().direction = Vector3::new(0.1, 0.2, 0.3);
            let start = mgr.get_entity(npc).unwrap().position;
            let engine = ChainEngine::new();
            let (tx, mut rx) = mpsc::channel(16);

            exec(
                name,
                gm,
                &[bad, "0", "0"],
                Some(npc),
                &tx,
                &mut mgr,
                &engine,
            )
            .await;

            let e = mgr.get_entity(npc).unwrap();
            assert_eq!(
                [e.position.x, e.position.y, e.position.z],
                [start.x, start.y, start.z],
                ".{name} {bad} must not move the entity"
            );
            assert_eq!(
                [e.direction.x, e.direction.y, e.direction.z],
                [0.1, 0.2, 0.3],
                ".{name} {bad} must not rotate the entity"
            );

            let (teleport, feedback) = drain(&mut rx);
            assert_eq!(teleport, None);
            assert_eq!(
                feedback.len(),
                1,
                ".{name} {bad} must feed back only the parse error; got {feedback:?}"
            );
            assert!(
                feedback[0].contains("must be a finite number"),
                ".{name} {bad} must report the finite-number rejection; got {feedback:?}"
            );
        }
    }
}

/// A closed base channel during `.location`'s player snap must not panic, and
/// must not claim a placement the client never received. The cell-side grid
/// write is authoritative the instant `update_entity_position` runs.
#[tokio::test]
async fn legacy_p18_location_player_target_survives_closed_channel() {
    let (mut mgr, gm, _npc) = setup();
    let target = 2u32;
    player_target(&mut mgr, target, [1.0, 0.0, 1.0]);
    mgr.get_entity_mut(gm).unwrap().current_target_id = Some(target as i32);
    let engine = ChainEngine::new();
    let (tx, rx) = mpsc::channel(16);
    drop(rx); // simulate a closed base channel

    exec(
        "location",
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
        "the cell-side grid write stands even when the base push failed"
    );
}

/// A distinct selected target is the only thing that moves or rotates — the
/// caller's own placement is untouched (D03's caller/subject split, here for
/// placement writes).
#[tokio::test]
async fn legacy_p18_caller_placement_unchanged_when_acting_on_selection() {
    for (name, args) in [
        ("location", ["77", "0", "88"]),
        ("rotation", ["0.5", "0.5", "0.5"]),
    ] {
        let (mut mgr, gm, npc) = setup();
        let before = mgr.get_entity(gm).unwrap();
        let (pos, dir) = (before.position, before.direction);
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(16);

        exec(name, gm, &args, Some(npc), &tx, &mut mgr, &engine).await;

        let after = mgr.get_entity(gm).unwrap();
        assert_eq!(
            [after.position.x, after.position.y, after.position.z],
            [pos.x, pos.y, pos.z],
            ".{name} on a selection must not move the caller"
        );
        assert_eq!(
            [after.direction.x, after.direction.y, after.direction.z],
            [dir.x, dir.y, dir.z],
            ".{name} on a selection must not rotate the caller"
        );
    }
}
