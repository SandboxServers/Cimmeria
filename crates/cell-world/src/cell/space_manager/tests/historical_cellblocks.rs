//! The shipped `entities/` world tables for the historical CellBlock worlds
//! (1201–1207). Each must be declared instanced in `spaces.xml` and never
//! listed in `cell_spaces.xml`, so every arrival gets a fresh space of its own
//! that is never the stock CellBlock's.

use super::super::*;
use cimmeria_wire::mercury::world_data::historical_cellblocks::HISTORICAL_CELLBLOCKS;

fn entities_file(name: &str) -> String {
    let path = format!("{}/../../entities/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn shipped_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(&entities_file("spaces.xml")).unwrap();
    mgr.create_startup_spaces(&entities_file("cell_spaces.xml"))
        .unwrap();
    mgr
}

#[test]
fn every_historical_cellblock_is_declared_instanced() {
    let mgr = shipped_manager();
    for world in &HISTORICAL_CELLBLOCKS {
        assert!(
            mgr.is_world_instanced(world.world),
            "{} must be Instanced=\"true\" in spaces.xml",
            world.world
        );
        assert_eq!(
            mgr.canonical_world_name(&world.world.to_lowercase()),
            Some(world.world),
            "a typed name must canonicalise to the declared spelling"
        );
    }
    assert!(
        mgr.is_world_instanced("Castle_CellBlock"),
        "the stock CellBlock is unchanged"
    );
}

/// `cell_spaces.xml` preloads non-instanced worlds. A historical world listed
/// there would be skipped with a warning today; keep it out of the file.
#[test]
fn no_historical_cellblock_is_a_startup_space() {
    let cell_spaces = entities_file("cell_spaces.xml");
    let mgr = shipped_manager();
    for world in &HISTORICAL_CELLBLOCKS {
        assert!(
            !cell_spaces.contains(&format!("\"{}\"", world.world)),
            "{} must not appear in cell_spaces.xml",
            world.world
        );
        assert!(!mgr.has_space_for_world(world.world));
        assert_eq!(mgr.default_space_for_world(world.world), None);
    }
}

#[test]
fn each_arrival_gets_its_own_space_in_its_own_world() {
    let mut mgr = shipped_manager();
    let stock = mgr.find_or_create_space("Castle_CellBlock").unwrap();
    for world in &HISTORICAL_CELLBLOCKS {
        let first = mgr.find_or_create_space(world.world).unwrap();
        let second = mgr.find_or_create_space(world.world).unwrap();
        assert_ne!(
            first, second,
            "{}: instanced, one space per arrival",
            world.world
        );
        assert_ne!(first, stock, "{}", world.world);
        assert_eq!(mgr.world_name_for_space(first), Some(world.world));
        assert_eq!(mgr.world_name_for_space(second), Some(world.world));
    }
    assert_eq!(mgr.world_name_for_space(stock), Some("Castle_CellBlock"));
}
