//! Castle authored catalog proof for `repeatable_q1` (CS-05b1s): the admitted XML,
//! its unique ids, the seed rows that back it, the absence of reward rows and the
//! hub giver template and spawn that carry its turn-in.

use super::super::*;
use crate::base::mission_additions::{MAX_MISSION_XML_BYTES, MISSION_ADDITIONS};
use crate::base::mission_additions_castle::CASTLE_REPEATABLE_Q1_XML;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
const SEED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../db/resources/Missions/Seed"
);
const ENTITIES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../db/resources/Entities/Seed"
);
const WORLDS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../db/resources/Worlds/Seed"
);

/// Mission, step, objective and task ids of the sealed `repeatable_q1` entry, with
/// the seed file that owns each and the CS-00Be sub-block it must sit in.
const REPEATABLE_Q1_IDS: [(u32, &str, (u32, u32)); 7] = [
    (10330, "missions.sql", (10330, 10330)),
    (10384, "mission_steps.sql", (10384, 10389)),
    (10385, "mission_steps.sql", (10384, 10389)),
    (10504, "mission_objectives.sql", (10504, 10509)),
    (10505, "mission_objectives.sql", (10504, 10509)),
    (10624, "mission_tasks.sql", (10624, 10629)),
    (10625, "mission_tasks.sql", (10624, 10629)),
];

fn seed_in(dir: &str, file: &str) -> String {
    std::fs::read_to_string(format!("{dir}/{file}")).unwrap()
}

fn seed(file: &str) -> String {
    seed_in(SEED, file)
}

/// The single seed line that contains `needle`; panics if none does.
fn row<'a>(text: &'a str, needle: &str) -> &'a str {
    text.lines()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no seed row containing `{needle}`"))
}

/// Every `<tag>text</tag>` body in the XML, in document order.
fn tag_texts(tag: &str) -> Vec<&'static str> {
    let (open, close) = (format!("<{tag}>"), format!("</{tag}>"));
    CASTLE_REPEATABLE_Q1_XML
        .split(open.as_str())
        .skip(1)
        .map(|rest| rest.split(close.as_str()).next().unwrap())
        .collect()
}

/// Every `name="value"` attribute value in the XML, in document order.
fn attr_values(name: &str) -> Vec<&'static str> {
    let key = format!(" {name}=\"");
    CASTLE_REPEATABLE_Q1_XML
        .split(key.as_str())
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap())
        .collect()
}

#[test]
fn castle_side_repeatable_q1_xml_admitted() {
    let cache = ResourceCache::load_all_with_mission_additions(DIR, MISSION_ADDITIONS).unwrap();
    let missions = cache.category(3).unwrap();
    assert_eq!(
        missions.elements[&10330],
        CASTLE_REPEATABLE_Q1_XML.as_bytes()
    );
    assert_eq!(CASTLE_REPEATABLE_Q1_XML.len(), 1757);
    assert!(CASTLE_REPEATABLE_Q1_XML.len() <= MAX_MISSION_XML_BYTES);
    assert!(!CASTLE_REPEATABLE_Q1_XML.contains('\n'));
    assert!(CASTLE_REPEATABLE_Q1_XML.contains(r#"MissionID="10330""#));
    assert!(CASTLE_REPEATABLE_Q1_XML.contains(r#"NumRepeats="2""#));
}

#[test]
fn castle_side_repeatable_q1_ids_unique_and_in_block() {
    let ids: Vec<u32> = REPEATABLE_Q1_IDS.iter().map(|&(id, ..)| id).collect();
    let unique: std::collections::HashSet<u32> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len());
    for &(id, file, (lo, hi)) in &REPEATABLE_Q1_IDS {
        assert!((lo..=hi).contains(&id), "{id} is outside {lo}-{hi}");
        // Not a stock key: the id is the first value of exactly one seed row.
        let rows = seed(file)
            .lines()
            .filter(|l| l.contains(&format!("VALUES ({id}, ")))
            .count();
        assert_eq!(rows, 1, "{file} must hold exactly one row for {id}");
    }
    let stock = ResourceCache::load_all_with_mission_additions(DIR, &[]).unwrap();
    assert!(!stock.category(3).unwrap().elements.contains_key(&10330));
}

#[test]
fn castle_side_repeatable_q1_is_registered_once() {
    // Found by id, never by position, so later "s" packets can append entries.
    let mut hits = MISSION_ADDITIONS.iter().filter(|a| a.mission_id == 10330);
    let entry = hits.next().expect("mission 10330 is not registered");
    assert!(hits.next().is_none(), "mission 10330 is registered twice");
    assert_eq!(entry.xml, CASTLE_REPEATABLE_Q1_XML);
    // Strictly ascending ids also prove that no two entries share an id.
    let ids: Vec<u32> = MISSION_ADDITIONS.iter().map(|a| a.mission_id).collect();
    assert!(
        ids.windows(2).all(|w| w[0] < w[1]),
        "registry ids not strictly ascending: {ids:?}"
    );
}

#[test]
fn castle_side_repeatable_q1_xml_matches_seed() {
    let missions = seed("missions.sql");
    let steps = seed("mission_steps.sql");
    let objectives = seed("mission_objectives.sql");
    let tasks = seed("mission_tasks.sql");
    let mission = row(&missions, "VALUES (10330, 'The Castle supply officer");
    assert!(mission.contains("'Supply Run', 'General', 2, false, false, false"));
    assert!(mission.ends_with("NULL, 0, 0, NULL);"));
    assert!(row(&steps, "VALUES (10384, 10330, ")
        .contains("'Carry the requisition slips to the Castle Infirmary.', 0);"));
    assert!(row(&steps, "VALUES (10385, 10330, ")
        .contains("'Report back to the supply officer in the Armory.', 1);"));
    assert!(row(&objectives, "VALUES (10504, 10384, ")
        .contains("false, false, false, 'Reach the Castle Infirmary.');"));
    assert!(row(&objectives, "VALUES (10505, 10385, ")
        .contains("false, false, false, 'Report to the supply officer.');"));
    assert!(tasks.contains("VALUES (10624, 10504, false, 1, true, 1);"));
    assert!(tasks.contains("VALUES (10625, 10505, false, 1, true, 1);"));
    // The XML flags agree with the objective rows: not hidden, not optional.
    for id in ["10504", "10505"] {
        assert!(CASTLE_REPEATABLE_Q1_XML.contains(&format!(
            r#"IsOptional="false" ObjectiveID="{id}" AwardXP="false" IsHidden="false""#
        )));
    }
    // Every id and text the XML carries is the one the seed rows carry.
    assert_eq!(attr_values("MissionID"), ["10330"]);
    assert_eq!(attr_values("NumRepeats"), ["2"]);
    assert_eq!(attr_values("StepID"), ["10384", "10385"]);
    assert_eq!(attr_values("ObjectiveID"), ["10504", "10505"]);
    assert_eq!(attr_values("TaskID"), ["10624", "10625"]);
    assert_eq!(attr_values("MissionDefn"), ["Supply Run"]);
    assert_eq!(tag_texts("HistoryText").len(), 1);
    for text in tag_texts("HistoryText") {
        assert!(mission.contains(&format!("'{text}'")), "history: {text}");
    }
    for text in tag_texts("StepDisplayLogText") {
        assert!(steps.contains(&format!("'{text}'")), "step text: {text}");
    }
    for text in tag_texts("DisplayLogText") {
        assert!(
            objectives.contains(&format!("'{text}'")),
            "objective: {text}"
        );
    }
}

#[test]
fn castle_side_repeatable_q1_reward_matches_seal() {
    // Frozen (f): zero rewards, and no reward row names mission 10330.
    for file in ["mission_rewards.sql", "mission_reward_groups.sql"] {
        let text = seed(file);
        assert!(
            !text.contains("10330"),
            "{file} must hold no row for mission 10330"
        );
    }
    let missions = seed("missions.sql");
    let mission = row(&missions, "VALUES (10330, 'The Castle supply officer");
    assert!(
        mission.ends_with("NULL, 0, 0, NULL);"),
        "reward_naq and reward_xp are 0"
    );
    assert!(!CASTLE_REPEATABLE_Q1_XML.contains("AwardXP=\"true\""));
}

#[test]
fn castle_side_repeatable_q1_template_and_spawn_rows() {
    // Frozen (e): template 424 is the non-vendor hub giver and spawn 13809 is its
    // only spawn, placed in world 8 with the interact tag the chains key on.
    let templates = seed_in(ENTITIES, "entity_templates.sql");
    let template = row(&templates, "VALUES (424, NULL, ");
    assert!(template.contains("'Castle_HubSupplyOfficer', 'mob', NULL, NULL, NULL, NULL"));
    let spawns = seed_in(WORLDS, "spawnlist.sql");
    let spawn = row(
        &spawns,
        "VALUES (13809, 440.0, 70.22, 984.0, 1.5708, 8, 424, ",
    );
    assert!(spawn.contains("'Castle_HubSupplyOfficer', NULL, true);"));
    let spawns_of_424 = spawns
        .lines()
        .filter(|l| l.contains("VALUES (") && l.contains(", 424, "))
        .count();
    assert_eq!(spawns_of_424, 1, "template 424 must have exactly one spawn");
}
