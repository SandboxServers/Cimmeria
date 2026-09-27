//! The historical CellBlock world contract (worlds 1201–1207).
//!
//! Expected values are spelled out literally rather than read back from
//! `HISTORICAL_CELLBLOCKS`, so a typo in the table fails here instead of
//! being echoed into the test.

use super::super::historical_cellblocks::{historical_cellblock, HISTORICAL_CELLBLOCKS};
use super::super::*;
use super::{sample_world_entry, walk_entity_method_records, TEST_KEY};
use crate::mercury::read_wstring;
use cimmeria_mercury::encryption::MercuryEncryption;
use std::collections::{HashMap, HashSet};

/// `(world name, world id, client map)` as agreed with the client patch,
/// which installs the `C<build>_CellBlock` folders under these exact names.
const CONTRACT: [(&str, i32, &str); 7] = [
    ("CellBlock43", 1201, "C43485_CellBlock"),
    ("CellBlock55", 1202, "C55124_CellBlock"),
    ("CellBlock57", 1203, "C57050_CellBlock"),
    ("CellBlock58", 1204, "C58674_CellBlock"),
    ("CellBlock60", 1205, "C60130_CellBlock"),
    ("CellBlock62", 1206, "C62429_CellBlock"),
    ("CellBlock63", 1207, "C63682_CellBlock"),
];

#[test]
fn world_id_for_name_resolves_every_historical_cellblock() {
    for (world, id, _) in CONTRACT {
        assert_eq!(world_id_for_name(world), id, "{world}");
    }
}

/// Without the explicit mapping these would fall through to the
/// `world_name` default and the client would be told to load a package
/// called `CellBlock43`, which does not exist.
#[test]
fn client_map_for_world_resolves_every_historical_cellblock() {
    for (world, _, client_map) in CONTRACT {
        assert_eq!(client_map_for_world(world), client_map, "{world}");
        assert_ne!(client_map_for_world(world), world, "{world}");
    }
}

#[test]
fn table_matches_the_contract_and_is_unique() {
    let table: Vec<(&str, i32, &str)> = HISTORICAL_CELLBLOCKS
        .iter()
        .map(|w| (w.world, w.world_id, w.client_map))
        .collect();
    assert_eq!(table, CONTRACT);

    let ids: HashSet<i32> = HISTORICAL_CELLBLOCKS.iter().map(|w| w.world_id).collect();
    let names: HashSet<&str> = HISTORICAL_CELLBLOCKS.iter().map(|w| w.world).collect();
    let maps: HashSet<&str> = HISTORICAL_CELLBLOCKS.iter().map(|w| w.client_map).collect();
    assert_eq!(ids.len(), 7);
    assert_eq!(names.len(), 7);
    assert_eq!(maps.len(), 7);
    // The build number is what the client folder is named after.
    for w in &HISTORICAL_CELLBLOCKS {
        assert_eq!(w.client_map, format!("C{}_CellBlock", w.build));
    }
}

#[test]
fn lookup_is_exact_and_leaves_stock_cellblock_alone() {
    assert!(historical_cellblock("Castle_CellBlock").is_none());
    assert!(historical_cellblock("cellblock43").is_none());
    assert_eq!(world_id_for_name("Castle_CellBlock"), 12);
    assert_eq!(client_map_for_world("Castle_CellBlock"), "Castle_CellBlock");
}

/// One `resources.worlds` seed row, keyed by column name.
fn seed_rows() -> Vec<HashMap<String, String>> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../db/resources/Worlds/Seed/worlds.sql"
    );
    let sql = std::fs::read_to_string(path).expect("worlds seed readable");
    sql.lines()
        .filter_map(|line| line.strip_prefix("INSERT INTO worlds ("))
        .map(|rest| {
            let (cols, rest) = rest.split_once(") VALUES (").expect("VALUES clause");
            let vals = rest.strip_suffix(");").expect("row terminator");
            let cols: Vec<&str> = cols.split(',').map(str::trim).collect();
            let vals: Vec<&str> = vals.split(',').map(str::trim).collect();
            assert_eq!(cols.len(), vals.len(), "column/value count: {vals:?}");
            cols.into_iter()
                .zip(vals)
                .map(|(c, v)| (c.to_string(), v.trim_matches('\'').to_string()))
                .collect()
        })
        .collect()
}

/// The seed carries each historical world exactly once, as an empty,
/// advisory-navmesh world with the stock CellBlock movement values, and no
/// other seeded world shares its id or name.
#[test]
fn seed_rows_match_the_contract_without_colliding_with_stock_worlds() {
    let rows = seed_rows();
    let stock = rows
        .iter()
        .find(|r| r["world_id"] == "12")
        .expect("stock CellBlock row");
    assert_eq!(stock["world"], "Castle_CellBlock");
    assert_eq!(stock["client_map"], "Castle_CellBlock");

    for (world, id, client_map) in CONTRACT {
        let matching: Vec<_> = rows
            .iter()
            .filter(|r| r["world_id"] == id.to_string() || r["world"] == world)
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "{world}: exactly one seed row by id or name"
        );
        let row = matching[0];
        assert_eq!(row["world_id"], id.to_string(), "{world}");
        assert_eq!(row["world"], world);
        assert_eq!(row["client_map"], client_map, "{world}");
        assert_eq!(row["flags"], "1", "{world}");
        assert_eq!(row["has_script"], "false", "{world}");
        assert_eq!(
            row.get("navmesh_mode").map(String::as_str),
            Some("advisory"),
            "{world}"
        );
        for column in [
            "min_per_day",
            "min_to_real_min",
            "gravity",
            "run_speed",
            "walk_speed",
            "jump_speed",
            "swim_speed",
        ] {
            assert_eq!(row[column], stock[column], "{world}.{column}");
        }
    }
}

/// `onClientMapLoad` is what makes the client load the historical package:
/// `areaName` is the world name, `mapPath` the alias, `WorldID` the new id.
#[test]
fn on_client_map_load_names_the_historical_package() {
    use crate::mercury::method_idx;

    let mut info = sample_world_entry();
    info.world_name = "CellBlock43".into();
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
    // Extended encoding: marker, u16 length, u32 entity id, sub-index.
    let args = &body[off + 8..];
    let (area, n) = read_wstring(args, 0).unwrap();
    let (map_path, m) = read_wstring(args, n).unwrap();
    let world_id = i32::from_le_bytes(args[n + m..n + m + 4].try_into().unwrap());
    assert_eq!(area, "CellBlock43");
    assert_eq!(map_path, "C43485_CellBlock");
    assert_eq!(world_id, 1201);
}

#[test]
fn setup_world_parameters_carries_the_historical_world_id() {
    for (world, id, _) in CONTRACT {
        let args = build_world_params_args(world);
        assert_eq!(
            i32::from_le_bytes(args[0..4].try_into().unwrap()),
            id,
            "{world}"
        );
    }
}
