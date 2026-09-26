//! XML loading, world catalogue, and space-id allocation tests.

use super::super::*;
use super::{make_manager, TEST_SPACES_XML};

#[test]
fn parse_spaces_xml_loads_all_worlds() {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(TEST_SPACES_XML).unwrap();
    assert_eq!(mgr.world_count(), 4);
    assert!(mgr.worlds.contains_key("Agnos"));
    assert!(mgr.worlds.contains_key("Castle_CellBlock"));
    assert!(mgr.worlds["Castle_CellBlock"].instanced);
    assert!(!mgr.worlds["Agnos"].instanced);
}

#[test]
fn startup_spaces_get_correct_ids() {
    let mgr = make_manager();
    assert_eq!(mgr.space_count(), 2);
    // cell_id=1: first space = (1<<16)|0 = 65536, second = 65537
    assert_eq!(mgr.space_id_for_world("Agnos"), Some(65536));
    assert_eq!(mgr.space_id_for_world("Castle"), Some(65537));
}

#[test]
fn instanced_space_created_on_demand() {
    let mut mgr = make_manager();
    assert_eq!(mgr.space_id_for_world("Castle_CellBlock"), None);

    let id1 = mgr.find_or_create_space("Castle_CellBlock").unwrap();
    assert_eq!(id1, 65538); // next after 65536, 65537

    // Each call creates a NEW instance — they should NOT share a space
    let id2 = mgr.find_or_create_space("Castle_CellBlock").unwrap();
    assert_eq!(id2, 65539);
    assert_ne!(id1, id2);

    // Instanced spaces are NOT cached in world_spaces
    assert_eq!(mgr.space_id_for_world("Castle_CellBlock"), None);
}

#[test]
fn unknown_world_returns_error() {
    let mut mgr = make_manager();
    assert!(mgr.find_or_create_space("Narnia").is_err());
}

#[test]
fn space_id_scheme() {
    let mut mgr = SpaceManager::new(1);
    assert_eq!(mgr.allocate_space_id(), 65536); // (1 << 16) | 0
    assert_eq!(mgr.allocate_space_id(), 65537); // (1 << 16) | 1
    assert_eq!(mgr.allocate_space_id(), 65538); // (1 << 16) | 2
}

#[test]
fn full_xml_file_loading() {
    // Test with the actual XML content (same structure as files)
    let mut mgr = SpaceManager::new(1);

    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Agnos_Library" Instanced="false" MinX="-600" MaxX="600" MinY="-600" MaxY="600" />
    <Space WorldName="Beta_Site_Evo_1" Instanced="false" MinX="-1600" MaxX="2600" MinY="-3000" MaxY="3000" />
    <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
    <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    <Space WorldName="SGC_W1" Instanced="true" MinX="-400" MaxX="400" MinY="-400" MaxY="800" />
</Spaces>"#;

    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" />
    <Space WorldName="Agnos_Library" />
    <Space WorldName="Beta_Site_Evo_1" />
    <Space WorldName="Castle" />
</Spaces>"#;

    mgr.parse_spaces_xml(spaces_xml).unwrap();
    assert_eq!(mgr.world_count(), 6);

    mgr.create_startup_spaces(cell_spaces_xml).unwrap();
    assert_eq!(mgr.space_count(), 4);

    // Startup spaces get sequential IDs
    assert_eq!(mgr.space_id_for_world("Agnos"), Some(65536));
    assert_eq!(mgr.space_id_for_world("Agnos_Library"), Some(65537));
    assert_eq!(mgr.space_id_for_world("Beta_Site_Evo_1"), Some(65538));
    assert_eq!(mgr.space_id_for_world("Castle"), Some(65539));

    // Instanced worlds not yet created
    assert_eq!(mgr.space_id_for_world("Castle_CellBlock"), None);
    assert_eq!(mgr.space_id_for_world("SGC_W1"), None);
}
