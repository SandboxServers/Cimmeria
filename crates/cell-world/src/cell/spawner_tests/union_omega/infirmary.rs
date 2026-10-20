//! UO-34: a death beside the Infirmary resolves to respawner 181, and a death
//! on the gate plaza still resolves to respawner 180 (UO-03).
//!
//! Respawner 181 (`Omega Infirmary`) stands 3.0-6.0 m from the medic (UO-33)
//! on the hub mesh, well clear of Modi and the C4 arrival point.

use cimmeria_cell_catalog::cell::respawner_fallback::nearest_valid_respawner_def;
use cimmeria_cell_catalog::cell::spawner::load_respawners;
use cimmeria_entity::movement_validation::SpaceBounds;

use super::{mesh, standable, standable_and_connected, vector, HUB_MESH, MODI};
use crate::test_support::require_db_or_skip;

const RESPAWNER_HUB: i32 = 180;
const RESPAWNER_INFIRMARY: i32 = 181;
/// UO-01 worknote, § C4: where SGC's US-32 lands a player.
const ARRIVAL: [f32; 3] = [34.70, -51.28, -142.00];
/// UO-20 worknote, § Death probes: 1.41 m from the medic (63818).
const INFIRMARY_DEATH: [f32; 3] = [38.00, -44.67, 6.00];

#[tokio::test]
async fn live_db_an_infirmary_death_resolves_to_respawner_181() {
    let pool = require_db_or_skip!();
    let respawners = load_respawners(&pool).await.expect("load_respawners");
    let row = respawners
        .iter()
        .find(|r| r.respawner_id == RESPAWNER_INFIRMARY)
        .expect("respawner 181 must be seeded");
    assert_eq!(row.world_name, "Omega_Site");
    assert_eq!(row.name, "Omega Infirmary");
    let hub = mesh(HUB_MESH);
    standable_and_connected(&hub, row.pos, MODI, "respawner 181");
    standable(
        &hub,
        INFIRMARY_DEATH,
        "the Infirmary death (INFIRMARY_DEATH)",
    );
    let dx = row.pos[0] - INFIRMARY_DEATH[0];
    let dz = row.pos[2] - INFIRMARY_DEATH[2];
    let reach = (dx * dx + dz * dz).sqrt();
    assert!(
        reach <= 6.0,
        "respawner 181 must stand within 6 m (x/z) of the Infirmary death (INFIRMARY_DEATH), is {reach:.2} m"
    );
    let chosen = nearest_valid_respawner_def(
        &respawners,
        "Omega_Site",
        vector(INFIRMARY_DEATH),
        Some(&hub),
        &SpaceBounds::FALLBACK,
    )
    .unwrap_or_else(|| panic!("the Infirmary death (INFIRMARY_DEATH) found no respawner"));
    assert_eq!(
        chosen.respawner_id, RESPAWNER_INFIRMARY,
        "respawner 181 must catch the Infirmary death (INFIRMARY_DEATH)"
    );
    for (label, death) in [("Modi", MODI), ("the C4 arrival point", ARRIVAL)] {
        let chosen = nearest_valid_respawner_def(
            &respawners,
            "Omega_Site",
            vector(death),
            Some(&hub),
            &SpaceBounds::FALLBACK,
        )
        .unwrap_or_else(|| panic!("a death at {label} found no respawner"));
        assert_eq!(
            chosen.respawner_id, RESPAWNER_HUB,
            "a death at {label} must stay on respawner 180, not 181"
        );
    }
    for (label, anchor) in [("Modi", MODI), ("the C4 arrival point", ARRIVAL)] {
        let dx = row.pos[0] - anchor[0];
        let dz = row.pos[2] - anchor[2];
        assert!(
            (dx * dx + dz * dz).sqrt() >= 12.0,
            "respawner 181 must stand at least 12 m (x/z) from {label}"
        );
    }
}
