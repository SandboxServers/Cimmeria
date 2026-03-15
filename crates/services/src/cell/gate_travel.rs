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
            tracing::warn!(entity_id, target_address_id, "onDialGate: invalid stargate address");
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

    // Remove entity from current space (CellService side)
    space_mgr.destroy_entity(entity_id);

    // Tell BaseApp to perform the world transition (RESET_ENTITIES + new world entry)
    let _ = tx.send(CellToBaseMsg::GateTravel {
        entity_id,
        target_world_name: gate.world_name.clone(),
        position: [gate.x, gate.y, gate.z],
        rotation: [0.0, 0.0, gate.yaw],
    }).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::spawner::StargateEntry;

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
        mgr.stargates.insert(1, StargateEntry {
            world_name: "Agnos".to_string(), x: 0.0, y: 0.0, z: 0.0, yaw: 0.0,
        });
        mgr.stargates.insert(2, StargateEntry {
            world_name: "Castle".to_string(), x: 761.677, y: 63.466, z: 551.716, yaw: 2.152,
        });
        mgr.stargates.insert(15, StargateEntry {
            world_name: "Agnos".to_string(), x: 0.0, y: 0.0, z: 0.0, yaw: 0.0,
        });

        mgr
    }

    #[tokio::test]
    async fn dial_gate_to_unknown_address_is_noop() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();

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
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 1, 0, &tx, &mut mgr).await;
        assert!(rx.try_recv().is_err());
        assert!(mgr.get_entity(1).is_some());
    }

    #[tokio::test]
    async fn dial_gate_valid_sends_gate_travel() {
        let mut mgr = make_manager_with_stargates();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();
        mgr.connect_entity(1);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        handle_dial_gate(1, 2, 0, &tx, &mut mgr).await;

        assert!(mgr.get_entity(1).is_none());

        let msg = rx.try_recv().expect("Expected GateTravel message");
        match msg {
            CellToBaseMsg::GateTravel { entity_id, target_world_name, position, .. } => {
                assert_eq!(entity_id, 1);
                assert_eq!(target_world_name, "Castle");
                assert!((position[0] - 761.677).abs() < 0.01);
            }
            _ => panic!("Expected GateTravel message, got {:?}", msg),
        }
    }
}
