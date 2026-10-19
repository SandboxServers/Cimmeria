//! DK-10fs: the production Dakara entry 63100 (D-DK1 F). Its XML is the
//! exact fixture transform, its ids are new, the served catalog gains it
//! and nothing else, and the seed rows say what the XML says.

use super::super::*;
use crate::base::mission_additions::{MISSION_ADDITIONS, TEST_MISSION_XML};
use crate::base::mission_additions_dakara::DAKARA_HOME_XML;
use crate::base::mission_overrides::{MISSION_OVERRIDES, STEP_TEXT_OVERRIDES};
use quick_xml::events::Event;
use quick_xml::Reader;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
const IDS: [u32; 4] = [63100, 63101, 63102, 63103];

#[test]
fn dakara_home_is_the_exact_fixture_transform() {
    let expected = TEST_MISSION_XML
        .replace("CB-N05a catalog fixture", "Home to Dakara")
        .replace("MissionID=\"61000\"", "MissionID=\"63100\"")
        .replace("StepID=\"61001\"", "StepID=\"63101\"")
        .replace("ObjectiveID=\"61002\"", "ObjectiveID=\"63102\"")
        .replace("TaskID=\"61003\"", "TaskID=\"63103\"")
        .replace(
            "Verify the authored mission catalog.",
            "Speak with Rak'nor at the Stargate.",
        )
        .replace(
            "<DisplayLogText> </DisplayLogText>",
            "<DisplayLogText>Rak'nor keeps watch over the Stargate plaza. Tell him what you saw at Stargate Command.</DisplayLogText>",
        );
    assert_eq!(DAKARA_HOME_XML, expected);
    assert_eq!(DAKARA_HOME_XML.len(), 1206);
    assert!(!DAKARA_HOME_XML.ends_with('\n'));
}

#[test]
fn dakara_home_ids_are_absent_from_the_stock_catalog() {
    let stock = ResourceCache::load_raw_mission_category_for_test(DIR).unwrap();
    for id in IDS {
        assert!(!stock.elements.contains_key(&id), "stock mission {id}");
    }
    for xml in stock.elements.values() {
        let mut reader = Reader::from_str(std::str::from_utf8(xml).unwrap());
        loop {
            match reader.read_event().unwrap() {
                Event::Start(e) | Event::Empty(e) => {
                    for attr in e.attributes() {
                        let attr = attr.unwrap();
                        if matches!(
                            attr.key.as_ref(),
                            "MissionID" | "StepID" | "ObjectiveID" | "TaskID"
                        ) {
                            assert!(!IDS.contains(&attr.value.parse::<u32>().unwrap()));
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
    }
}

#[test]
fn load_all_serves_dakara_home_and_changes_nothing_else() {
    let served = ResourceCache::load_all(DIR).unwrap();
    let empty = ResourceCache::load_all_with_mission_additions(DIR, &[]).unwrap();
    for &(id, _) in CATEGORY_PAKS {
        if id != 3 {
            assert_eq!(served.category(id), empty.category(id), "category {id}");
        }
    }
    let missions = served.category(3).unwrap();
    let baseline = empty.category(3).unwrap();
    assert_eq!(missions.elements.len(), baseline.elements.len() + 1);
    assert_eq!(missions.elements.len(), 1041);
    assert_eq!(missions.elements[&63100], DAKARA_HOME_XML.as_bytes());
    for (id, bytes) in &baseline.elements {
        assert_eq!(
            &missions.elements[id], bytes,
            "existing served mission {id}"
        );
    }
    assert_eq!(
        missions.metadata,
        7538u32.wrapping_add(compute_metadata_bump(
            MISSION_OVERRIDES,
            STEP_TEXT_OVERRIDES,
            MISSION_ADDITIONS,
        ))
    );
    assert_ne!(missions.metadata, baseline.metadata);
    assert_eq!(
        served
            .overridden_elements(3)
            .iter()
            .filter(|&&id| id == 63100)
            .count(),
        1
    );
}

#[test]
fn dakara_seed_rows_match_the_xml() {
    let seed = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../db/resources/Missions/Seed/missions_dakara_e1.sql"
    ))
    .unwrap();
    for row in [
        "VALUES (63100, 'Home to Dakara',",
        "VALUES (63101, 63100,",
        "'Speak with Rak''nor at the Stargate.'",
        "VALUES (63102, 63101,",
        "'Rak''nor keeps watch over the Stargate plaza. Tell him what you saw at Stargate Command.'",
        "VALUES (63103, 63102,",
    ] {
        assert!(
            seed.contains(row),
            "missions_dakara_e1.sql drifted from DAKARA_HOME_XML: change both together"
        );
    }
}
