//! A template's literal nameplate (`entity_templates.display_name`, the
//! Debug Area's Visual NPC Lineup) rides every AoI introduction, so a
//! witness who arrives after the first one gets the label too: the cascade
//! that sends `onBeingNameUpdate` is built from the enter event's NPC data.

use super::super::super::messages::CellToBaseMsg;
use super::make_manager;

fn entered_name(events: &[CellToBaseMsg], witness: u32, npc: u32) -> Option<Option<String>> {
    events.iter().find_map(|e| match e {
        CellToBaseMsg::EnteredAoI {
            witness_id,
            entity_id,
            npc_data,
            ..
        } if *witness_id == witness && *entity_id == npc => {
            Some(npc_data.as_ref().and_then(|d| d.display_name.clone()))
        }
        _ => None,
    })
}

/// The first witness and a late joiner both get the NPC's `display_name` in
/// their enter event; an NPC without one carries `None`. Revert proof: drop
/// `display_name` from `NpcAoIData::from_entity` and both witnesses lose it.
#[test]
fn every_witness_gets_the_npcs_display_name_late_joiners_included() {
    let mut mgr = make_manager();
    let labelled = mgr.allocate_npc_id();
    mgr.spawn_npc(labelled, "Agnos", [12.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(labelled).unwrap().display_name =
        Some("Teal'c #30 BS_JaffaMale".to_string());
    let plain = mgr.allocate_npc_id();
    mgr.spawn_npc(plain, "Agnos", [14.0, 0.0, 10.0], [0.0; 3])
        .unwrap();

    mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    let first = mgr.compute_aoi_changes();
    assert_eq!(
        entered_name(&first, 100, labelled),
        Some(Some("Teal'c #30 BS_JaffaMale".to_string())),
        "the first witness gets the label"
    );
    assert_eq!(entered_name(&first, 100, plain), Some(None));

    mgr.create_entity(101, "Agnos", [11.0, 0.0, 11.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(101);
    let late = mgr.compute_aoi_changes();
    assert_eq!(
        entered_name(&late, 101, labelled),
        Some(Some("Teal'c #30 BS_JaffaMale".to_string())),
        "a late joiner gets the label too"
    );
}
