//! Category 12 (`CookedWorldInfo.pak`): the committed PAK stays the file
//! clients ship with, and the historical CellBlock worlds (1201–1207) ride
//! the per-key handshake on top of it.

use super::super::*;
use crate::base::world_info_overrides::{
    generate_world_info_xml, WorldInfoOverride, WORLD_INFO_OVERRIDES,
};

/// Version of `CookedWorldInfo.pak` that shipped clients hold.
const CLIENT_SHIPPED_WORLD_INFO_VERSION: u32 = 5959;

fn data_dir() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache")
}

fn shipped_world_info() -> CategoryData {
    ResourceCache::load_pak(&format!("{}/CookedWorldInfo.pak", data_dir()))
        .expect("committed world info PAK loads")
}

/// The on-disk PAK must stay the file clients already hold: same version, no
/// Cimmeria entries. New worlds belong in `world_info_overrides`, which
/// delivers them per key.
#[test]
fn on_disk_world_info_pak_is_the_client_shipped_file() {
    let shipped = shipped_world_info();
    assert_eq!(
        shipped.metadata, CLIENT_SHIPPED_WORLD_INFO_VERSION,
        "do not edit the PAK's MetaData; add a WorldInfoOverride instead"
    );
    for ov in WORLD_INFO_OVERRIDES {
        assert!(
            !shipped.elements.contains_key(&ov.world_id),
            "world {} is in the on-disk PAK; overrides must not shadow shipped worlds",
            ov.world_id
        );
    }
}

/// The handshake contract: category 12 names exactly the seven historical
/// worlds in `InvalidKeys`, serves a version a shipped client does not hold
/// (so it learns about them), and leaves every shipped world untouched.
#[test]
fn world_info_takes_the_per_key_handshake_for_the_historical_cellblocks() {
    let cache = ResourceCache::load_all(data_dir()).expect("committed PAKs load");
    assert_eq!(
        cache.overridden_elements(CATEGORY_WORLD_INFO),
        [1201, 1202, 1203, 1204, 1205, 1206, 1207].as_slice()
    );

    let served = cache.category(CATEGORY_WORLD_INFO).expect("category 12");
    let bump = compute_world_info_metadata_bump(WORLD_INFO_OVERRIDES);
    assert_eq!(
        served.metadata,
        CLIENT_SHIPPED_WORLD_INFO_VERSION + bump,
        "served version is the shipped one plus the content-derived bump"
    );
    assert_ne!(served.metadata, CLIENT_SHIPPED_WORLD_INFO_VERSION);

    let shipped = shipped_world_info();
    assert_eq!(
        served.elements.len(),
        shipped.elements.len() + WORLD_INFO_OVERRIDES.len()
    );
    for ov in WORLD_INFO_OVERRIDES {
        assert_eq!(
            cache.get(CATEGORY_WORLD_INFO, ov.world_id),
            Some(&generate_world_info_xml(ov))
        );
    }
    for (id, xml) in &shipped.elements {
        assert_eq!(
            cache.get(CATEGORY_WORLD_INFO, *id),
            Some(xml),
            "shipped world {id} must be served untouched"
        );
    }
}

/// The generator must emit exactly what the client already parses for this
/// element type. Checked against real shipped entries: the stock CellBlock,
/// and CombatSim, whose `ClientMap` differs from its `World` the way every
/// historical world's does.
#[test]
fn generated_world_info_xml_matches_shipped_entries() {
    let shipped = shipped_world_info();
    let cases = [
        WorldInfoOverride {
            world_id: 12,
            world: "Castle_CellBlock",
            client_map: "Castle_CellBlock",
            flags: 1,
            min_per_day: 1440,
            min_to_real_min: 1,
        },
        WorldInfoOverride {
            world_id: 1,
            world: "CombatSim",
            client_map: "Combat_Terrain_Test",
            flags: 0,
            min_per_day: 1440,
            min_to_real_min: 1,
        },
    ];
    for ov in &cases {
        let actual = shipped
            .elements
            .get(&ov.world_id)
            .expect("ships in the PAK");
        assert_eq!(
            String::from_utf8_lossy(&generate_world_info_xml(ov)),
            String::from_utf8_lossy(actual),
            "world {}",
            ov.world_id
        );
    }
}

/// Each generated entry is well-formed XML whose `COOKED_WORLD_INFO`
/// attributes name the historical world, its own client package and id.
#[test]
fn generated_entries_parse_and_name_the_historical_package() {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    for ov in WORLD_INFO_OVERRIDES {
        let xml = generate_world_info_xml(ov);
        let text = std::str::from_utf8(&xml).unwrap();
        let mut reader = Reader::from_str(text);
        let mut attrs = std::collections::HashMap::new();
        loop {
            match reader.read_event().expect("well-formed XML") {
                Event::Start(e) if e.name().as_ref() == b"COOKED_WORLD_INFO" => {
                    for a in e.attributes() {
                        let a = a.expect("well-formed attribute");
                        // Raw value: none of these attributes needs escaping.
                        attrs.insert(
                            String::from_utf8(a.key.as_ref().to_vec()).unwrap(),
                            String::from_utf8(a.value.to_vec()).unwrap(),
                        );
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(attrs["WorldID"], ov.world_id.to_string());
        assert_eq!(attrs["World"], ov.world);
        assert_eq!(attrs["ClientMap"], ov.client_map);
        assert_ne!(attrs["ClientMap"], attrs["World"]);
        assert_eq!(attrs["Flags"], "1");
        assert_eq!(attrs["MinPerDay"], "1440");
        assert_eq!(attrs["MinToRealMin"], "1");
    }
}

#[test]
fn world_info_metadata_bump_is_deterministic_non_zero_and_change_sensitive() {
    let a = compute_world_info_metadata_bump(WORLD_INFO_OVERRIDES);
    assert_eq!(a, compute_world_info_metadata_bump(WORLD_INFO_OVERRIDES));
    assert_ne!(a, 0);
    assert_eq!(a & 1, 1, "low bit is always set");

    let mut changed = WORLD_INFO_OVERRIDES.to_vec();
    changed[0].client_map = "C99999_CellBlock";
    assert_ne!(
        compute_world_info_metadata_bump(&changed),
        a,
        "a changed client map must change the version so clients refetch"
    );
}

#[test]
fn apply_world_info_overrides_is_a_no_op_when_the_category_is_missing() {
    let mut categories = HashMap::new();
    let overridden = ResourceCache::apply_world_info_overrides(&mut categories);
    assert!(overridden.is_empty());
    assert!(categories.is_empty());
}
