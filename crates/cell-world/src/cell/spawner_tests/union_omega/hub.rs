//! UO-03: Omega's hub respawner 180 (`Omega Site Gate Plaza`).
//!
//! A death anywhere on the gate plaza resolves to respawner 180, which
//! stands on the hub mesh, connected to Modi. Before UO-03, world 18 had
//! no respawner, and a hub death respawned in place.

use cimmeria_cell_catalog::cell::respawner_fallback::nearest_valid_respawner_def;
use cimmeria_cell_catalog::cell::spawner::load_respawners;
use cimmeria_entity::movement_validation::SpaceBounds;

use super::{mesh, standable_and_connected, vector, HUB_MESH, MODI};
use crate::test_support::require_db_or_skip;

const RESPAWNER_HUB: i32 = 180;
/// UO-01 worknote, § C4: where SGC's US-32 lands a player.
const ARRIVAL: [f32; 3] = [34.70, -51.28, -142.00];

#[tokio::test]
async fn live_db_a_hub_death_resolves_to_the_gate_plaza_respawner() {
    let pool = require_db_or_skip!();
    let respawners = load_respawners(&pool).await.expect("load_respawners");
    let row = respawners
        .iter()
        .find(|r| r.respawner_id == RESPAWNER_HUB)
        .expect("respawner 180 must be seeded");
    assert_eq!(row.world_name, "Omega_Site");
    assert_eq!(row.name, "Omega Site Gate Plaza");
    let hub = mesh(HUB_MESH);
    standable_and_connected(&hub, row.pos, MODI, "respawner 180");
    for (label, death) in [("Modi", MODI), ("the C4 arrival point", ARRIVAL)] {
        let chosen = nearest_valid_respawner_def(
            &respawners,
            "Omega_Site",
            vector(death),
            Some(&hub),
            &SpaceBounds::FALLBACK,
        )
        .unwrap_or_else(|| panic!("a death at {label} found no respawner"));
        assert_eq!(chosen.respawner_id, RESPAWNER_HUB, "a death at {label}");
    }
    let dx = row.pos[0] - ARRIVAL[0];
    let dz = row.pos[2] - ARRIVAL[2];
    assert!(
        (dx * dx + dz * dz).sqrt() >= 3.0,
        "respawner 180 must stand at least 3 m from the arrival point"
    );
}
