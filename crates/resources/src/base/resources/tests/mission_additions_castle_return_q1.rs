//! Castle authored catalog proof for `return_q1` (CS-04c1s): the admitted XML,
//! its unique ids, the seed rows that back it and the absence of reward rows.

use super::super::*;
use crate::base::mission_additions::{MAX_MISSION_XML_BYTES, MISSION_ADDITIONS};
use crate::base::mission_additions_castle::CASTLE_RETURN_Q1_XML;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
const SEED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../db/resources/Missions/Seed"
);

/// Mission, step, objective and task ids of the sealed `return_q1` entry, with
/// the seed file that owns each and the D-CA20 row (`r4-dispatch-plan.md` §4)
/// it must sit in.
const RETURN_Q1_IDS: [(u32, &str, (u32, u32)); 4] = [
    (10320, "missions.sql", (10320, 10320)),
    (10372, "mission_steps.sql", (10372, 10377)),
    (10492, "mission_objectives.sql", (10492, 10497)),
    (10612, "mission_tasks.sql", (10612, 10617)),
];

fn seed(file: &str) -> String {
    std::fs::read_to_string(format!("{SEED}/{file}")).unwrap()
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
    CASTLE_RETURN_Q1_XML
        .split(open.as_str())
        .skip(1)
        .map(|rest| rest.split(close.as_str()).next().unwrap())
        .collect()
}

/// Every `name="value"` attribute value in the XML, in document order.
fn attr_values(name: &str) -> Vec<&'static str> {
    let key = format!(" {name}=\"");
    CASTLE_RETURN_Q1_XML
        .split(key.as_str())
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap())
        .collect()
}

#[test]
fn castle_side_return_q1_xml_admitted() {
    let cache = ResourceCache::load_all_with_mission_additions(DIR, MISSION_ADDITIONS).unwrap();
    let missions = cache.category(3).unwrap();
    assert_eq!(missions.elements[&10320], CASTLE_RETURN_Q1_XML.as_bytes());
    assert_eq!(CASTLE_RETURN_Q1_XML.len(), 1235);
    assert!(CASTLE_RETURN_Q1_XML.len() <= MAX_MISSION_XML_BYTES);
    assert!(!CASTLE_RETURN_Q1_XML.contains('\n'));
    assert!(CASTLE_RETURN_Q1_XML.contains(r#"MissionID="10320""#));
}

#[test]
fn castle_side_return_q1_ids_unique_and_in_block() {
    let ids: Vec<u32> = RETURN_Q1_IDS.iter().map(|&(id, ..)| id).collect();
    let unique: std::collections::HashSet<u32> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len());
    for &(id, file, (lo, hi)) in &RETURN_Q1_IDS {
        assert!((lo..=hi).contains(&id), "{id} is outside {lo}-{hi}");
        // Not a stock key: the id is the first value of exactly one seed row.
        let rows = seed(file)
            .lines()
            .filter(|l| l.contains(&format!("VALUES ({id}, ")))
            .count();
        assert_eq!(rows, 1, "{file} must hold exactly one row for {id}");
    }
    let stock = ResourceCache::load_all_with_mission_additions(DIR, &[]).unwrap();
    assert!(!stock.category(3).unwrap().elements.contains_key(&10320));
}

#[test]
fn castle_side_return_q1_is_registered_once() {
    // Found by id, never by position, so later "s" packets can append entries.
    let mut hits = MISSION_ADDITIONS.iter().filter(|a| a.mission_id == 10320);
    let entry = hits.next().expect("mission 10320 is not registered");
    assert!(hits.next().is_none(), "mission 10320 is registered twice");
    assert_eq!(entry.xml, CASTLE_RETURN_Q1_XML);
    // Strictly ascending ids also prove that no two entries share an id.
    let ids: Vec<u32> = MISSION_ADDITIONS.iter().map(|a| a.mission_id).collect();
    assert!(
        ids.windows(2).all(|w| w[0] < w[1]),
        "registry ids not strictly ascending: {ids:?}"
    );
}

#[test]
fn castle_side_return_q1_xml_matches_seed() {
    let missions = seed("missions.sql");
    let steps = seed("mission_steps.sql");
    let objectives = seed("mission_objectives.sql");
    let tasks = seed("mission_tasks.sql");
    let mission = row(&missions, "VALUES (10320, 'Back from Harset");
    assert!(mission.contains("true, true, false, false, false, 4, 'Report from Harset', 'General', 0, false, false, false"));
    assert!(mission.ends_with("NULL, 0, 0, NULL);"));
    assert!(row(&steps, "VALUES (10372, 10320, ")
        .contains("'Report to Colonel Marsh at the Castle Stargate.', 0);"));
    assert!(row(&objectives, "VALUES (10492, 10372, ")
        .contains("false, false, false, 'Report to Colonel Marsh.');"));
    assert!(tasks.contains("VALUES (10612, 10492, false, 1, true, 1);"));
    // The XML flags agree with the objective row: not hidden, not optional.
    assert!(CASTLE_RETURN_Q1_XML
        .contains(r#"IsOptional="false" ObjectiveID="10492" AwardXP="false" IsHidden="false""#));
    assert!(CASTLE_RETURN_Q1_XML.contains(r#"Level="4""#));
    // Every id and text the XML carries is the one the seed rows carry.
    assert_eq!(attr_values("MissionID"), ["10320"]);
    assert_eq!(attr_values("StepID"), ["10372"]);
    assert_eq!(attr_values("ObjectiveID"), ["10492"]);
    assert_eq!(attr_values("TaskID"), ["10612"]);
    assert_eq!(attr_values("MissionDefn"), ["Report from Harset"]);
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
fn castle_side_return_q1_no_reward_rows() {
    for file in ["mission_rewards.sql", "mission_reward_groups.sql"] {
        let text = seed(file);
        assert!(
            !text
                .split(|c: char| !c.is_ascii_digit())
                .any(|t| t == "10320"),
            "{file} names mission 10320"
        );
    }
    let mission = seed("missions.sql");
    assert!(row(&mission, "VALUES (10320, ").ends_with("NULL, 0, 0, NULL);"));
}
