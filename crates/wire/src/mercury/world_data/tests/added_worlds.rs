//! The Cimmeria-added world table (`ADDED_WORLDS`), the Debug Area (1300)
//! contract, and the world-name lookups for every declared space.
//!
//! Expected values are spelled out literally where they are a contract, so a
//! typo in the table fails here instead of being echoed into the test.

use super::super::added_worlds::{
    added_world, AddedWorldOrigin, ADDED_WORLDS, DEBUG_AREA_WORLD, DEBUG_AREA_WORLD_ID,
};
use super::super::*;
use super::historical_cellblocks::seed_rows;
use super::{sample_world_entry, walk_entity_method_records, TEST_KEY};
use crate::mercury::read_wstring;
use cimmeria_mercury::encryption::MercuryEncryption;
use std::collections::HashSet;

#[test]
fn debug_area_is_world_1300_on_ihpet_crater_light() {
    assert_eq!((DEBUG_AREA_WORLD_ID, DEBUG_AREA_WORLD), (1300, "DebugArea"));
    let w = added_world("DebugArea").expect("DebugArea is an added world");
    assert_eq!(w.world_id, 1300);
    assert_eq!(w.client_map, "Ihpet_Crater_Light");
    assert_eq!(w.origin, AddedWorldOrigin::DebugArea);
    assert!(!w.is_historical_cellblock());
    // Row 73's shipped `Flags`, not the CellBlock's.
    assert_eq!(w.world_info_flags, 0);

    assert_eq!(world_id_for_name("DebugArea"), 1300);
    assert_eq!(known_world_id("DebugArea"), Some(1300));
    assert_eq!(client_map_for_world("DebugArea"), "Ihpet_Crater_Light");
    // Exact match, like every other world-table lookup.
    assert!(added_world("debugarea").is_none());
}

/// Ids, names and lookups are unique, and no added world reuses a shipped
/// world's id or name (the shipped catalogue tops out at 92).
#[test]
fn added_worlds_are_unique_and_clear_of_shipped_ids() {
    let ids: HashSet<i32> = ADDED_WORLDS.iter().map(|w| w.world_id).collect();
    let names: HashSet<&str> = ADDED_WORLDS.iter().map(|w| w.world).collect();
    assert_eq!(ids.len(), ADDED_WORLDS.len());
    assert_eq!(names.len(), ADDED_WORLDS.len());
    for w in &ADDED_WORLDS {
        assert!(
            w.world_id > 92,
            "{}: {} is a shipped id",
            w.world,
            w.world_id
        );
        assert_ne!(w.world, w.client_map, "{}", w.world);
        assert_eq!(added_world(w.world), Some(w));
        assert_eq!(known_world_id(w.world), Some(w.world_id));
        assert_eq!(client_map_for_world(w.world), w.client_map);
    }
    let mut sorted: Vec<i32> = ADDED_WORLDS.iter().map(|w| w.world_id).collect();
    sorted.sort_unstable();
    assert_eq!(
        sorted,
        ADDED_WORLDS.iter().map(|w| w.world_id).collect::<Vec<_>>(),
        "the table is kept in world-id order"
    );
}

fn declared_spaces() -> Vec<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../entities/spaces.xml");
    let xml = std::fs::read_to_string(path).expect("spaces.xml readable");
    xml.split("WorldName=\"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap().to_string())
        .collect()
}

/// Every world the base may create resolves to its own seed id and client
/// map. Regression guard for the gap where Sewer_Falls (50), Dakara_E1 (61),
/// Dakara_E1_StoryRm (62), Harset_Market (69), Harset_StorageRm (70),
/// Ihpet_Crater_Dark (72), Ihpet_Crater_Light (73) and Menfa_Light (78) fell
/// through to WorldID 1 (CombatSim): `onClientMapLoad` and
/// `setupWorldParameters` told the client it was in CombatSim while it loaded
/// the right map.
#[test]
fn every_declared_space_resolves_to_its_seed_world_id_and_client_map() {
    let rows = seed_rows();
    let spaces = declared_spaces();
    assert!(spaces.len() >= 32, "parsed {} spaces", spaces.len());
    assert!(spaces.iter().any(|s| s == "DebugArea"));
    for world in &spaces {
        let row = rows
            .iter()
            .find(|r| r["world"] == *world)
            .unwrap_or_else(|| panic!("{world}: no resources.worlds seed row"));
        let seed_id: i32 = row["world_id"].parse().unwrap();
        assert_eq!(known_world_id(world), Some(seed_id), "{world}");
        assert_eq!(world_id_for_name(world), seed_id, "{world}");
        assert_eq!(client_map_for_world(world), row["client_map"], "{world}");
    }
}

/// The eight shipped worlds of the gap, spelled out so the guard does not
/// depend on `spaces.xml` keeping them.
#[test]
fn shipped_worlds_missing_from_the_old_table_resolve() {
    for (world, id) in [
        ("Sewer_Falls", 50),
        ("Dakara_E1", 61),
        ("Dakara_E1_StoryRm", 62),
        ("Harset_Market", 69),
        ("Harset_StorageRm", 70),
        ("Ihpet_Crater_Dark", 72),
        ("Ihpet_Crater_Light", 73),
        ("Menfa_Light", 78),
    ] {
        assert_eq!(known_world_id(world), Some(id), "{world}");
        assert_eq!(world_id_for_name(world), id, "{world}");
        let args = build_world_params_args(world);
        assert_eq!(i32::from_le_bytes(args[0..4].try_into().unwrap()), id);
    }
    assert_eq!(known_world_id("NoSuchWorld"), None);
    assert_eq!(world_id_for_name("NoSuchWorld"), 1);
}

/// The seed row is world 73's with a new id and name: same flags, time and
/// movement values, the same client map, advisory navmesh, no script.
#[test]
fn debug_area_seed_row_copies_ihpet_crater_light() {
    let rows = seed_rows();
    let light = rows
        .iter()
        .find(|r| r["world_id"] == "73")
        .expect("Ihpet_Crater_Light row");
    let matching: Vec<_> = rows
        .iter()
        .filter(|r| r["world_id"] == "1300" || r["world"] == "DebugArea")
        .collect();
    assert_eq!(matching.len(), 1, "exactly one seed row by id or name");
    let row = matching[0];
    assert_eq!(row["world"], "DebugArea");
    assert_eq!(row["client_map"], "Ihpet_Crater_Light");
    assert_eq!(row["has_script"], "false");
    assert_eq!(
        row.get("navmesh_mode").map(String::as_str),
        Some("advisory")
    );
    for (column, value) in light {
        if matches!(column.as_str(), "world_id" | "world") {
            continue;
        }
        assert_eq!(&row[column], value, "DebugArea.{column}");
    }
}

/// `onClientMapLoad` names world 1300 and asks the client for the shipped
/// Ihpet_Crater_Light package.
#[test]
fn on_client_map_load_names_the_debug_area() {
    use crate::mercury::method_idx;

    let mut info = sample_world_entry();
    info.world_name = "DebugArea".into();
    let pkt = build_create_player(
        &TEST_KEY,
        1,
        &[],
        &info,
        None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    let pt = MercuryEncryption::from_session_key(TEST_KEY)
        .decrypt(&pkt)
        .unwrap();
    let body = &pt[1 + 9..];
    let (_, off) = walk_entity_method_records(body)
        .into_iter()
        .find(|(idx, _)| *idx == method_idx::ON_CLIENT_MAP_LOAD)
        .expect("onClientMapLoad record");
    let args = &body[off + 8..];
    let (area, n) = read_wstring(args, 0).unwrap();
    let (map_path, m) = read_wstring(args, n).unwrap();
    let world_id = i32::from_le_bytes(args[n + m..n + m + 4].try_into().unwrap());
    assert_eq!(area, "DebugArea");
    assert_eq!(map_path, "Ihpet_Crater_Light");
    assert_eq!(world_id, 1300);

    let params = build_world_params_args("DebugArea");
    assert_eq!(i32::from_le_bytes(params[0..4].try_into().unwrap()), 1300);
}
