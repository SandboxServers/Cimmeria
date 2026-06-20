//! `Action::CrossWorldTeleport` executor coverage — gate-travel send
//! shape, no-ring-id discriminator, and the missing-local-entity case.

use super::*;

/// `Action::CrossWorldTeleport` must produce exactly one
/// `CellToBaseMsg::GateTravel` send carrying the right world name and
/// position, with `destination_ring_id: None` (the discriminator that
/// tells the base side NOT to emit `BaseToCellMsg::AdvanceRingDestination`
/// — there's no destination ring FSM to advance for a chain-driven
/// hop). Pinning this guards against the action accidentally being
/// rerouted through the `Action::Teleport` (same-space) path.
#[tokio::test]
async fn cross_world_teleport_action_emits_gate_travel_with_no_ring_id() {
    use crate::cell::messages::CellToBaseMsg;

    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(42);
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1109,
            Action::CrossWorldTeleport {
                world_name: "Castle".to_string(),
                position: [466.365, 70.397, 991.466],
            },
        )],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    // Drain the channel and find the GateTravel message. The action
    // also flushes dirty bandolier ammo before sending GateTravel —
    // we don't assert on that here (the player has no dirty ammo) but
    // it's why we drain rather than just `try_recv`.
    let mut gate_travel: Option<(u32, String, [f32; 3], [f32; 3], Option<i32>)> = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name,
            position,
            rotation,
            destination_ring_id,
        } = msg
        {
            gate_travel = Some((
                entity_id,
                target_world_name,
                position,
                rotation,
                destination_ring_id,
            ));
        }
    }
    let (eid, world, pos, rot, ring_id) =
        gate_travel.expect("CrossWorldTeleport action must produce a GateTravel send");
    assert_eq!(
        eid, 1,
        "GateTravel.entity_id must be the player's entity_id"
    );
    assert_eq!(world, "Castle", "GateTravel.target_world_name must match");
    assert!(
        (pos[0] - 466.365).abs() < 0.001
            && (pos[1] - 70.397).abs() < 0.001
            && (pos[2] - 991.466).abs() < 0.001,
        "GateTravel.position must be propagated verbatim; got {pos:?}"
    );
    assert_eq!(rot, [0.0, 0.0, 0.0], "rotation defaults to identity");
    assert_eq!(
        ring_id, None,
        "destination_ring_id must be None — chain-driven cross-world hop \
         skips the destination ring FSM, so base must not emit \
         AdvanceRingDestination"
    );

    // The cell-side entity must be torn down on this world before the
    // GateTravel send (matches the ring's TeleportCrossWorld arm
    // ordering). Without this, the player exists in two worlds at once
    // until base destroys via RESET_ENTITIES.
    assert!(
        mgr.get_entity(1).is_none(),
        "cell entity must be destroyed locally before cross-world hop",
    );
}

/// CrossWorldTeleport against an unknown local entity_id still
/// dispatches `GateTravel` — base may hold a connection record for the
/// player at this address that the cell hasn't synced yet, and the
/// ring's `Effect::TeleportCrossWorld` arm uses the same shape. The
/// load-bearing invariant is "no panic": the action runs inside
/// `execute_actions` iterating a resolved chain action list, and a
/// malformed chain or a desync between the resolved entity_id and
/// the cell's entity table must not crash the cell loop.
#[tokio::test]
async fn cross_world_teleport_action_with_unknown_entity_dispatches_gate_travel() {
    use crate::cell::messages::CellToBaseMsg;

    let mut mgr = make_space_mgr();
    // Note: no create_entity — eid 999 doesn't exist.

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1109,
            Action::CrossWorldTeleport {
                world_name: "Castle".to_string(),
                position: [466.365, 70.397, 991.466],
            },
        )],
    };
    // Per-action handlers fail-soft on missing entities; execute_actions
    // returning Ok here is the regression guard.
    execute_actions(resolved, 999, 42, &tx, &mut mgr, &engine).await;

    // The current implementation still emits GateTravel even when the
    // entity is missing locally, because the base side may have an
    // entity record for the player at this addr that the cell hasn't
    // synced yet. That's the same shape the ring's
    // `Effect::TeleportCrossWorld` arm uses. Confirm the message is
    // sent with the requested target.
    let mut got_gate_travel = false;
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::GateTravel { .. }) {
            got_gate_travel = true;
        }
    }
    assert!(
        got_gate_travel,
        "CrossWorldTeleport must dispatch GateTravel even when local \
         cell entity is absent — base may still hold the connection"
    );
}
