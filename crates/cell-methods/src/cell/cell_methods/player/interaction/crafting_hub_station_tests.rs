//! Live-DB end-to-end guard for the crafting stations of the stasis-room
//! debug hub (templates 310-313, spawns tagged `CraftHub_*`): built from
//! their real seed rows by the startup spawn path, the stations are what
//! the station tick and the crafting forward find beside the supplies
//! vendor, for every crafting verb.

use crate::cell::interactions::crafting_stations::{station_mask, stations_in_range};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

const PLAYER: u32 = 1;

/// Every `ECraftTypeFlags` bit: craft, research, reverse engineering and
/// alloying.
const EVERY_VERB: u8 = 0x0F;

/// The stasis room with the crafting corner spawned exactly as the cell
/// spawns it at startup, and a player at `pos`. Returns the manager and
/// each spawn's entity id by tag.
fn staged(records: &[spawner::SpawnRecord], pos: [f32; 3]) -> (SpaceManager, Vec<(String, u32)>) {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-1000" MaxX="1000" MinY="-1000" MaxY="1000" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    let mut spawned = Vec::new();
    for record in records
        .iter()
        .filter(|r| r.tag.as_deref().is_some_and(|t| t.starts_with("CraftHub_")))
    {
        let eid = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(eid, record)
            .expect("a crafting-hub entity must spawn from its record");
        spawned.push((record.tag.clone().unwrap(), eid));
    }
    mgr.create_entity(PLAYER, "Castle_CellBlock", pos, [0.0; 3])
        .expect("the player must stage in the stasis room");
    mgr.get_entity_mut(PLAYER).unwrap().is_player = true;
    (mgr, spawned)
}

fn eid_of(spawned: &[(String, u32)], tag: &str) -> u32 {
    spawned
        .iter()
        .find(|(t, _)| t == tag)
        .map(|(_, eid)| *eid)
        .unwrap_or_else(|| panic!("{tag} was not spawned: {spawned:?}"))
}

/// A player at the supplies vendor reaches a station for every verb, and
/// the one reported is the nearest (the BioMedical station, 2.6 units
/// away). From the respawner, 7.4 units from the nearest station, none is
/// in reach.
#[tokio::test]
async fn a_player_at_the_supplies_vendor_reaches_a_station_for_every_verb() {
    let pool = require_db_or_skip!();
    let records = spawner::load_spawns_from_db(&pool)
        .await
        .expect("spawns must load");
    let vendor = records
        .iter()
        .find(|r| r.tag.as_deref() == Some("CraftHub_Supplies"))
        .expect("the supplies vendor must be seeded");

    let (mgr, spawned) = staged(&records, [vendor.x, vendor.y, vendor.z]);
    assert_eq!(
        spawned.len(),
        5,
        "four stations and the vendor: {spawned:?}"
    );
    let stations = stations_in_range(&mgr, PLAYER);
    let bio = eid_of(&spawned, "CraftHub_Station_BioMedical");
    assert_eq!(stations, [Some(bio); 4]);
    assert_eq!(station_mask(&stations), EVERY_VERB);

    let (mgr, _) = staged(&records, [-334.231, 73.472, -228.026]);
    assert_eq!(
        stations_in_range(&mgr, PLAYER),
        [None; 4],
        "no station reaches the spot a new character wakes up on"
    );
}
