//! Stargate travel handler for the CellService.
//!
//! Handles `onDialGate` cell method calls, validating the target stargate
//! address and initiating a world transition via the BaseApp.
//!
//! Stargate destinations are loaded from `resources.stargates` at startup
//! and cached in `SpaceManager.stargates`.
//!
//! Reference: `python/cell/SGWPlayer.py:onDialGate()` — validates target,
//! begins dialing sequence. `python/cell/GateTravel.py:stargatePassed()` —
//! calls `moveTo()` to transition the entity to the destination world.

use tokio::sync::mpsc;

use super::arrival::validate_gate_arrival;
use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Handler ──────────────────────────────────────────────────────────────────

/// Handle the `onDialGate` cell method call.
///
/// Validates the target stargate address, removes the entity from the current
/// space, and sends a `GateTravel` message to BaseApp to initiate the world
/// transition.
///
/// Reference: `python/cell/SGWPlayer.py:onDialGate()` — the Python version
/// starts a 4-second dial timer; we skip the timer and travel immediately
/// for simplicity.
#[tracing::instrument(
    name = "gate_travel.dial",
    level = "info",
    skip_all,
    fields(entity_id, target_address_id)
)]
pub async fn handle_dial_gate(
    entity_id: u32,
    target_address_id: i32,
    _source_address_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // target_address_id == -1 means cancel dialing (no-op for us)
    if target_address_id == -1 {
        tracing::debug!(entity_id, "onDialGate: cancel dial (no-op)");
        return;
    }

    // Look up the destination stargate from the DB cache
    let gate = match space_mgr.stargates.get(&target_address_id) {
        Some(g) => g.clone(),
        None => {
            tracing::warn!(
                entity_id,
                target_address_id,
                "onDialGate: invalid stargate address"
            );
            return;
        }
    };

    // Validate the entity exists and get its current world
    let current_world = match space_mgr.get_entity_world_name(entity_id) {
        Some(w) => w,
        None => {
            tracing::warn!(entity_id, "onDialGate: entity not found");
            return;
        }
    };

    // Don't travel to the same world (Python also checks this implicitly)
    if gate.world_name == current_world {
        tracing::debug!(
            entity_id, target_address_id, world = %gate.world_name,
            "onDialGate: already in destination world"
        );
        return;
    }

    tracing::info!(
        entity_id, target_address_id,
        from = %current_world, to = %gate.world_name,
        "Gate travel: initiating world transition"
    );

    // Resolve a standable arrival before anything destructive happens. Prefers
    // the gate's authored `arrival_*` pin, falls back to the gate row, and
    // replaces either with the destination world's respawner when the
    // destination has a navmesh that rejects it — see `cell::arrival`.
    //
    // **This one line is the whole arrival contract.** Castle CA10 moves the
    // placement from here to the gate-volume entry path; carry this call with
    // it so the walk-through-the-event-horizon arrival gets the same
    // validation the dial arrival does.
    let arrival = validate_gate_arrival(space_mgr, &gate);

    // Stage D: world transition destroys the cell entity and re-creates it on
    // the destination world. Flush any pending bandolier ammo writes before
    // teardown — anything still in `bandolier_ammo_dirty` after this is lost
    // to the cross-world re-spawn.
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        if let Some(player_id) = entity.player_id {
            super::cell_methods::inventory::flush_dirty_bandolier_ammo(entity, player_id, tx).await;
        }
    }

    // Tell BaseApp to perform the world transition (RESET_ENTITIES + new world
    // entry) BEFORE removing the entity locally. A closed base channel must
    // not leave the player destroyed cell-side with no transfer in flight —
    // that is an "un-spaced" player who can only recover by relogging. Same
    // ordering the native `gmGotoLocation` handler already uses.
    if let Err(e) = tx
        .send(CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name: gate.world_name.clone(),
            position: arrival.position,
            rotation: [0.0, 0.0, arrival.yaw],
            destination_ring_id: None,
            // Stargate travel resolves the destination by world name.
            destination_space_id: None,
        })
        .await
    {
        tracing::error!(
            entity_id, world = %gate.world_name, error = %e,
            "onDialGate: base channel closed — entity left in place, no transfer"
        );
        return;
    }

    // Cancel any open trade before the entity goes. `destroy_entity` doesn't
    // clean trade state, and this helper early-returns once `get_entity`
    // misses — so gating on it afterwards would be a silent no-op, leaving the
    // traveller's partner with a dangling `trade_partner_entity_id` and no
    // `onTradeResults(Cancelled)`. Both lifecycle arms call it for the same
    // reason; stargate travel is just as much a departure.
    super::cell_methods::player::trade::cancel_trade_on_disconnect(entity_id, tx, space_mgr).await;

    // Remove entity from current space (CellService side)
    space_mgr.destroy_entity(entity_id);
}

#[cfg(test)]
mod tests {
    use super::super::spawner::StargateEntry;
    use super::*;

    fn make_manager_with_stargates() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" />
            <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
        </Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Agnos" />
            <Space WorldName="Castle" />
        </Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();

        // Populate stargates cache (simulates DB load)
        mgr.stargates.insert(
            1,
            StargateEntry {
                world_name: "Agnos".to_string(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                yaw: 0.0,
                address_origin: 15,
                arrival: None,
            },
        );
        mgr.stargates.insert(
            2,
            StargateEntry {
                world_name: "Castle".to_string(),
                x: 761.677,
                y: 63.466,
                z: 551.716,
                yaw: 2.152,
                address_origin: 18,
                arrival: None,
            },
        );
        mgr.stargates.insert(
            15,
            StargateEntry {
                world_name: "Agnos".to_string(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                yaw: 0.0,
                address_origin: 15,
                arrival: None,
            },
        );
        // Same Castle destination, but with an authored arrival pin. Synthetic
        // — this packet seeds no arrival coordinate for any gate; Harset's is
        // pinned in-game during milestone M0.
        mgr.stargates.insert(
            99,
            StargateEntry {
                world_name: "Castle".to_string(),
                x: 761.677,
                y: 63.466,
                z: 551.716,
                yaw: 2.152,
                address_origin: 18,
                arrival: Some(([700.5, 60.25, 540.75], 1.0)),
            },
        );

        mgr
    }

    #[tokio::test]
    async fn dial_gate_to_unknown_address_is_noop() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 999, 0, &tx, &mut mgr).await;
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dial_gate_cancel_is_noop() {
        let mut mgr = make_manager_with_stargates();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, -1, 0, &tx, &mut mgr).await;
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dial_gate_same_world_is_noop() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 1, 0, &tx, &mut mgr).await;
        assert!(rx.try_recv().is_err());
        assert!(mgr.get_entity(1).is_some());
    }

    /// A closed base channel must leave the traveller in place. Destroying
    /// the entity first and *then* discovering the send failed produces a
    /// player who is in no space with no transfer in flight — recoverable
    /// only by relogging.
    #[tokio::test]
    async fn dial_gate_with_closed_base_channel_leaves_the_entity_in_place() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        let space_before = mgr.get_entity_space_id(1);

        let (tx, rx) = tokio::sync::mpsc::channel(16);
        drop(rx); // base side is gone

        handle_dial_gate(1, 2, 0, &tx, &mut mgr).await;

        assert!(
            mgr.get_entity(1).is_some(),
            "a failed GateTravel enqueue must not tear the traveller out of their space"
        );
        assert_eq!(
            mgr.get_entity_space_id(1),
            space_before,
            "the traveller must still be in their origin space"
        );
    }

    #[tokio::test]
    async fn dial_gate_valid_sends_gate_travel() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 2, 0, &tx, &mut mgr).await;

        assert!(mgr.get_entity(1).is_none());

        let msg = rx.try_recv().expect("Expected GateTravel message");
        match msg {
            CellToBaseMsg::GateTravel {
                entity_id,
                target_world_name,
                position,
                ..
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(target_world_name, "Castle");
                assert!((position[0] - 761.677).abs() < 0.01);
            }
            _ => panic!("Expected GateTravel message, got {:?}", msg),
        }
    }

    /// An unpinned gate arrives on the gate row, facing the row's authored
    /// yaw — the pre-H01 behaviour, pinned here so the arrival change can't
    /// silently alter it for the 28 gates that have no pin.
    #[tokio::test]
    async fn dial_gate_without_an_arrival_pin_uses_the_gate_row() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 2, 0, &tx, &mut mgr).await;

        let (position, rotation) = expect_gate_travel(&mut rx);
        assert!((position[0] - 761.677).abs() < 0.01);
        assert!((position[1] - 63.466).abs() < 0.01);
        assert!((position[2] - 551.716).abs() < 0.01);
        assert!((rotation[2] - 2.152).abs() < 0.01);
    }

    /// A pinned gate arrives on the pin, yaw included. Reverting
    /// `handle_dial_gate` to `[gate.x, gate.y, gate.z]` / `gate.yaw` — the
    /// pre-H01 line — fails this.
    #[tokio::test]
    async fn dial_gate_with_an_arrival_pin_uses_the_pin() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 99, 0, &tx, &mut mgr).await;

        let (position, rotation) = expect_gate_travel(&mut rx);
        // Exact, not tolerance-based: these literals are copied verbatim
        // through the call chain with no arithmetic applied.
        assert_eq!(position, [700.5, 60.25, 540.75]);
        assert_eq!(rotation[2], 1.0);
        assert_ne!(
            position,
            [761.677, 63.466, 551.716],
            "the prefab-origin gate row must not win over an authored pin"
        );
    }

    /// End to end, on the wire the base actually receives: a gate whose
    /// arrival is off the destination world's navmesh must hand BaseApp the
    /// respawner position, not the authored one.
    ///
    /// `cell::arrival`'s own tests prove the *decision*; the two tests above
    /// prove the *pin plumbing*. Without this one the defect the packet
    /// exists to prevent — an off-mesh coordinate reaching
    /// `CellToBaseMsg::GateTravel` — is only covered in two disjoint halves.
    #[tokio::test]
    async fn dial_gate_to_an_off_mesh_arrival_sends_the_respawner_position() {
        use crate::cell::arrival::{test_fixture_mesh, test_insert_navmesh_space};
        use crate::cell::spawner::RespawnerDef;

        let Some(mesh) = test_fixture_mesh() else {
            return;
        };
        // Same fixture coordinates as `cell::arrival::tests`.
        const ON_MESH: [f32; 3] = [-289.465, 68.542, -154.276];
        const OFF_MESH: [f32; 3] = [-289.465, 268.542, -154.276];

        let mut mgr = make_manager_with_stargates();
        test_insert_navmesh_space(&mut mgr, "Castle_CellBlock", mesh);
        mgr.respawners.push(RespawnerDef {
            respawner_id: 1,
            world_name: "Castle_CellBlock".to_string(),
            name: "test".to_string(),
            pos: ON_MESH,
        });
        mgr.stargates.insert(
            98,
            StargateEntry {
                world_name: "Castle_CellBlock".to_string(),
                x: OFF_MESH[0],
                y: OFF_MESH[1],
                z: OFF_MESH[2],
                yaw: 1.75,
                address_origin: 18,
                arrival: None,
            },
        );
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 98, 0, &tx, &mut mgr).await;

        let (position, rotation) = expect_gate_travel(&mut rx);
        assert_eq!(position, ON_MESH, "the off-mesh gate row must be replaced");
        assert_ne!(
            position, OFF_MESH,
            "an off-navmesh arrival must never reach the base — that is the \
             silent-freeze bug"
        );
        assert_eq!(
            rotation[2], 1.75,
            "a respawner fallback carries the authored facing through"
        );
    }

    fn expect_gate_travel(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> ([f32; 3], [f32; 3]) {
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::GateTravel {
                position, rotation, ..
            } = msg
            {
                return (position, rotation);
            }
        }
        panic!("Expected GateTravel message");
    }
}
