//! `npc_ai event=npc_off_mesh` cadence for an NPC parked where it spawned
//! (NA24, UAT-1 D): Castle_BravoOfficer3 spawned 1.57 u off the mesh and
//! wrote a WARN every 30 s for as long as the zone was up -- 402 rows with
//! nobody in Castle.

use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::AiState;
use tracing::Level;

use super::super::MoveSource;
use super::{add_npc, cellblock_mgr, rows, NPC};
use crate::test_support::LogCapture;

/// Well outside the Cellblock mesh horizontally.
const OFF_MESH: [f32; 3] = [700.0, 0.0, 700.0];

fn check(mgr: &mut crate::cell::space_manager::SpaceManager, now: Instant) {
    super::super::sweep::after_handler(mgr, NPC, AiState::Idle, "", now);
}

/// One WARN, then DEBUG on the same 30 s window while the NPC has not moved
/// since spawn; the WARN cadence comes back once something moves it.
/// Revert proof: gate on `admit_warn(.., "npc_off_mesh", 30 s)` alone again
/// and the check at +31 s is a second WARN.
#[test]
fn an_npc_parked_off_mesh_since_spawn_warns_once_then_samples_at_debug() {
    let (mut mgr, _) = cellblock_mgr();
    add_npc(&mut mgr, "Castle_CellBlock", OFF_MESH, None, AiState::Idle);
    mgr.npc_detectors.note_move_source(NPC, MoveSource::Spawn);
    let t0 = Instant::now();
    let logs = LogCapture::install();

    for k in 0..4u64 {
        check(&mut mgr, t0 + Duration::from_secs(31 * k));
    }
    let found = rows(&logs, "npc_ai", "npc_off_mesh");
    let levels: Vec<Level> = found.iter().map(|r| r.level).collect();
    assert_eq!(
        levels,
        vec![Level::WARN, Level::DEBUG, Level::DEBUG, Level::DEBUG],
        "{found:#?}"
    );
    assert!(found[0].has_field("last_move_source", "spawn"));

    // Something moved it: back to a WARN on the normal window.
    mgr.npc_detectors.note_move_source(NPC, MoveSource::Path);
    check(&mut mgr, t0 + Duration::from_secs(200));
    let found = rows(&logs, "npc_ai", "npc_off_mesh");
    assert_eq!(found.len(), 5, "{found:#?}");
    assert_eq!(found[4].level, Level::WARN);
    assert!(found[4].has_field("last_move_source", "path"));
}
