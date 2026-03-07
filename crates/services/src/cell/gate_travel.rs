//! Stargate travel handler for the CellService.
//!
//! Handles `onDialGate` cell method calls, validating the target stargate
//! address and initiating a world transition via the BaseApp.
//!
//! Reference: `python/cell/SGWPlayer.py:onDialGate()` — validates target,
//! begins dialing sequence. `python/cell/GateTravel.py:stargatePassed()` —
//! calls `moveTo()` to transition the entity to the destination world.

use tokio::sync::mpsc;

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Stargate data (from db/resources/Worlds/Seed/stargates.sql) ─────────────

/// Static stargate destination data.
struct StargateInfo {
    /// World name matching spaces.xml WorldName.
    world_name: &'static str,
    /// Arrival position at the destination stargate.
    x: f32,
    y: f32,
    z: f32,
    /// Arrival yaw rotation.
    yaw: f32,
}

/// Look up a stargate by its address ID.
///
/// Returns the destination world name and arrival position/rotation.
/// Data sourced from `db/resources/Worlds/Seed/stargates.sql`.
fn lookup_stargate(address_id: i32) -> Option<StargateInfo> {
    // Stargate seed data (26 entries from stargates.sql).
    // address_id = stargate_id column.
    match address_id {
        1 => Some(StargateInfo { world_name: "Agnos", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        2 => Some(StargateInfo { world_name: "Castle", x: 761.677, y: 63.466, z: 551.716, yaw: 2.152 }),
        3 => Some(StargateInfo { world_name: "Harset", x: -0.076, y: -67.274, z: 38.011, yaw: 3.141 }),
        4 => Some(StargateInfo { world_name: "Tollana", x: 211.045, y: 0.606, z: -547.644, yaw: -1.571 }),
        5 => Some(StargateInfo { world_name: "Lucia_PVP", x: 62.282, y: -165.575, z: -61.22, yaw: 0.0 }),
        6 => Some(StargateInfo { world_name: "Beta_Site_Evo_1", x: 617.866, y: -59.314, z: 388.971, yaw: 1.571 }),
        7 => Some(StargateInfo { world_name: "Beta_Site_Evo_2", x: 617.866, y: -59.314, z: 388.971, yaw: 1.571 }),
        8 => Some(StargateInfo { world_name: "Mountain_Bunker", x: -29.775, y: 15.757, z: -1.558, yaw: 0.0 }),
        9 => Some(StargateInfo { world_name: "Leonops", x: 19.0, y: -210.0, z: 236.0, yaw: 0.0 }),
        10 => Some(StargateInfo { world_name: "Lucia", x: 66.718, y: -165.575, z: -70.22, yaw: -3.12 }),
        11 => Some(StargateInfo { world_name: "P2X-873", x: -6.0, y: 2.0, z: 3.0, yaw: 0.0 }),
        12 => Some(StargateInfo { world_name: "Whitelands", x: -140.79, y: -75.33, z: 56.55, yaw: 0.0 }),
        13 => Some(StargateInfo { world_name: "Goauld_Homeworld_1", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        14 => Some(StargateInfo { world_name: "Goauld_Homeworld_2", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        15 => Some(StargateInfo { world_name: "Agnos", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        16 => Some(StargateInfo { world_name: "P9G-844", x: -69.0, y: -87.0, z: 38.0, yaw: 0.0 }),
        17 => Some(StargateInfo { world_name: "P2S-569", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        18 => Some(StargateInfo { world_name: "CombatSim", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        19 => Some(StargateInfo { world_name: "Alpha_Site", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        20 => Some(StargateInfo { world_name: "Castle_CellBlock", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        21 => Some(StargateInfo { world_name: "NewMexico", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        22 => Some(StargateInfo { world_name: "P5S-117", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        23 => Some(StargateInfo { world_name: "SGC", x: 172.79, y: -8.47, z: 6.014, yaw: 0.0 }),
        24 => Some(StargateInfo { world_name: "SGC_W1", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        25 => Some(StargateInfo { world_name: "Abydos_4", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        26 => Some(StargateInfo { world_name: "Abydos_5", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        27 => Some(StargateInfo { world_name: "SGC_W1", x: 172.79, y: -8.47, z: 6.014, yaw: 0.0 }),
        28 => Some(StargateInfo { world_name: "Castle_CellBlock", x: 0.0, y: 0.0, z: 0.0, yaw: 0.0 }),
        _ => None,
    }
}

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

    // Look up the destination stargate
    let gate = match lookup_stargate(target_address_id) {
        Some(g) => g,
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
            entity_id, target_address_id, world = gate.world_name,
            "onDialGate: already in destination world"
        );
        return;
    }

    tracing::info!(
        entity_id, target_address_id,
        from = %current_world, to = gate.world_name,
        "Gate travel: initiating world transition"
    );

    // Remove entity from current space (CellService side)
    space_mgr.destroy_entity(entity_id);

    // Tell BaseApp to perform the world transition (RESET_ENTITIES + new world entry)
    let _ = tx.send(CellToBaseMsg::GateTravel {
        entity_id,
        target_world_name: gate.world_name.to_string(),
        position: [gate.x, gate.y, gate.z],
        rotation: [0.0, 0.0, gate.yaw],
    }).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_known_stargates() {
        let gate = lookup_stargate(23).unwrap();
        assert_eq!(gate.world_name, "SGC");
        assert!((gate.x - 172.79).abs() < 0.01);

        let gate2 = lookup_stargate(2).unwrap();
        assert_eq!(gate2.world_name, "Castle");
    }

    #[test]
    fn lookup_unknown_stargate() {
        assert!(lookup_stargate(999).is_none());
        assert!(lookup_stargate(-2).is_none());
    }

    #[test]
    fn cancel_dial_is_negative_one() {
        // target_address_id == -1 is the cancel signal
        assert!(lookup_stargate(-1).is_none());
    }

    #[tokio::test]
    async fn dial_gate_to_unknown_address_is_noop() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        // Dial an invalid address — no message should be sent
        handle_dial_gate(1, 999, 0, &tx, &mut mgr).await;
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dial_gate_cancel_is_noop() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        // Cancel (-1) should not send anything
        handle_dial_gate(1, -1, 0, &tx, &mut mgr).await;
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn dial_gate_same_world_is_noop() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        // Stargate 1 and 15 both go to Agnos — should be a no-op since we're already there
        handle_dial_gate(1, 1, 0, &tx, &mut mgr).await;
        assert!(rx.try_recv().is_err());

        // Entity should still exist (not destroyed)
        assert!(mgr.get_entity(1).is_some());
    }

    #[tokio::test]
    async fn dial_gate_valid_sends_gate_travel() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /><Space WorldName="Castle" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /><Space WorldName="Castle" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();
        mgr.connect_entity(1);

        let (tx, mut rx) = tokio::sync::mpsc::channel(16);

        // Dial stargate 2 (Castle) from Agnos
        handle_dial_gate(1, 2, 0, &tx, &mut mgr).await;

        // Entity should be destroyed from old space
        assert!(mgr.get_entity(1).is_none());

        // Should receive a GateTravel message
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
