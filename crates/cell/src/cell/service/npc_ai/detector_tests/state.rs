//! The avatar-update flag: the one `detector_tests::state` test that drives
//! the service loop's movement tick. The rest of `state` is in
//! `cimmeria-cell-combat`.

use cimmeria_entity::cell_entity::AiState;

use super::{add_npc, add_threat_player, castle_mgr, movement_tick, NPC};
use crate::cell::messages::CellToBaseMsg;

/// The AoI relay says whether the NPC it is sending a velocity for actually
/// moved: `Some(false)` with a non-zero velocity is running in place.
#[test]
fn the_aoi_relay_carries_npc_moved_since_last() {
    let mut mgr = castle_mgr();
    add_npc(&mut mgr, "Castle", [0.0; 3], None, AiState::Fighting);
    add_threat_player(&mut mgr, "Castle", [5.0, 0.0, 0.0]);
    mgr.get_entity_mut(NPC).unwrap().velocity = [6.0, 0.0, 0.0];
    movement_tick(&mut mgr);
    movement_tick(&mut mgr);
    let moved: Vec<Option<bool>> = mgr
        .compute_aoi_changes()
        .into_iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMoved {
                entity_id,
                npc_moved_since_last,
                ..
            } if entity_id == NPC => Some(npc_moved_since_last),
            _ => None,
        })
        .collect();
    assert_eq!(moved, vec![Some(false)]);
}
