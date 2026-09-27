//! A corpse must be introduced dead. Python `SGWBeing.createOnClient` sends
//! the live `stateField` and stats (`deprecated/python/cell/SGWBeing.py:507`,
//! `:514`); the AoI enter event is where the cell hands those to the base.
//!
//! Colo 2026-09-26 (tester, 02:38:42): Hallway01_Guard 100162 died at
//! 02:32:56, the player died and reanchored at 02:34:47, and the guard
//! re-entered his AoI at 02:34:53 — rebuilt standing and alive, because the
//! cascade hardcoded `onStateFieldUpdate(0)`. He reported a guard that
//! "is not hostile to player".

use cimmeria_entity::stats::{FOCUS, HEALTH};

use super::super::super::messages::CellToBaseMsg;
use super::make_manager;
use cimmeria_wire::state_field::{BSF_DEAD, BSF_MOVEMENT_LOCK};

/// A dead NPC entering a witness's AoI carries its live `state_field`
/// (BSF_DEAD set) and its live 0-HP health in the enter event.
#[test]
fn aoi_entry_carries_a_corpses_live_state_and_health() {
    let mut mgr = make_manager();
    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc(npc_id, "Agnos", [12.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(n) = mgr.get_entity_mut(npc_id) {
        n.state_field = BSF_DEAD | BSF_MOVEMENT_LOCK;
        n.stats.get_mut(HEALTH).unwrap().update(0, 0, 250);
        n.stats.get_mut(FOCUS).unwrap().update(0, 0, 200);
    }
    mgr.connect_entity(100);

    let events = mgr.compute_aoi_changes();

    let npc_data = events
        .iter()
        .find_map(|e| match e {
            CellToBaseMsg::EnteredAoI {
                witness_id: 100,
                entity_id,
                npc_data,
                ..
            } if *entity_id == npc_id => npc_data.clone(),
            _ => None,
        })
        .expect("the corpse enters the witness's AoI with NPC data");

    assert_eq!(
        npc_data.state_field,
        BSF_DEAD | BSF_MOVEMENT_LOCK,
        "the corpse's live state_field must reach the cascade -- 0 rebuilds \
         it standing and alive on the witness's client"
    );
    let vitals = npc_data.vitals.expect("live vitals ride along");
    assert_eq!(vitals.health, [0, 0, 250], "live health, not 100/100");
    assert_eq!(vitals.focus, [0, 0, 200], "live focus");
}
