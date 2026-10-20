//! Castle authored catalog proof for `return_q2` (CS-04c2s): the admitted XML,
//! its unique ids, the seed rows that back it and the absence of reward rows.

use super::super::*;
use crate::base::mission_additions::{MAX_MISSION_XML_BYTES, MISSION_ADDITIONS};
use crate::base::mission_additions_castle::CASTLE_RETURN_Q2_XML;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
const SEED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../db/resources/Missions/Seed"
);

/// Mission, step, objective and task ids of the sealed `return_q2` entry, with
/// the seed file that owns each and the CS-00Be sub-block it must sit in.
const RETURN_Q2_IDS: [(u32, &str, (u32, u32)); 7] = [
    (10321, "missions.sql", (10321, 10321)),
    (10378, "mission_steps.sql", (10378, 10383)),
    (10379, "mission_steps.sql", (10378, 10383)),
    (10498, "mission_objectives.sql", (10498, 10503)),
    (10499, "mission_objectives.sql", (10498, 10503)),
    (10618, "mission_tasks.sql", (10618, 10623)),
    (10619, "mission_tasks.sql", (10618, 10623)),
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
    CASTLE_RETURN_Q2_XML
        .split(open.as_str())
        .skip(1)
        .map(|rest| rest.split(close.as_str()).next().unwrap())
        .collect()
}

/// Every `name="value"` attribute value in the XML, in document order.
fn attr_values(name: &str) -> Vec<&'static str> {
    let key = format!(" {name}=\"");
    CASTLE_RETURN_Q2_XML
        .split(key.as_str())
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap())
        .collect()
}

/// The text as the seed files spell it: each apostrophe is doubled in SQL.
fn sql_text(text: &str) -> String {
    text.replace('\'', "''")
}

#[test]
fn castle_side_return_q2_xml_admitted() {
    let cache = ResourceCache::load_all_with_mission_additions(DIR, MISSION_ADDITIONS).unwrap();
    let missions = cache.category(3).unwrap();
    assert_eq!(missions.elements[&10321], CASTLE_RETURN_Q2_XML.as_bytes());
    assert_eq!(CASTLE_RETURN_Q2_XML.len(), 1671);
    assert!(CASTLE_RETURN_Q2_XML.len() <= MAX_MISSION_XML_BYTES);
    assert!(!CASTLE_RETURN_Q2_XML.contains('\n'));
    assert!(CASTLE_RETURN_Q2_XML.contains(r#"MissionID="10321""#));
}

#[test]
fn castle_side_return_q2_ids_unique_and_in_block() {
    let ids: Vec<u32> = RETURN_Q2_IDS.iter().map(|&(id, ..)| id).collect();
    let unique: std::collections::HashSet<u32> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len());
    for &(id, file, (lo, hi)) in &RETURN_Q2_IDS {
        assert!((lo..=hi).contains(&id), "{id} is outside {lo}-{hi}");
        // Not a stock key: the id is the first value of exactly one seed row.
        let rows = seed(file)
            .lines()
            .filter(|l| l.contains(&format!("VALUES ({id}, ")))
            .count();
        assert_eq!(rows, 1, "{file} must hold exactly one row for {id}");
    }
    let stock = ResourceCache::load_all_with_mission_additions(DIR, &[]).unwrap();
    assert!(!stock.category(3).unwrap().elements.contains_key(&10321));
}

#[test]
fn castle_side_return_q2_is_registered_once() {
    // Found by id, never by position, so later "s" packets can append entries.
    let mut hits = MISSION_ADDITIONS.iter().filter(|a| a.mission_id == 10321);
    let entry = hits.next().expect("mission 10321 is not registered");
    assert!(hits.next().is_none(), "mission 10321 is registered twice");
    assert_eq!(entry.xml, CASTLE_RETURN_Q2_XML);
    // Strictly ascending ids also prove that no two entries share an id.
    let ids: Vec<u32> = MISSION_ADDITIONS.iter().map(|a| a.mission_id).collect();
    assert!(
        ids.windows(2).all(|w| w[0] < w[1]),
        "registry ids not strictly ascending: {ids:?}"
    );
}

#[test]
fn castle_side_return_q2_xml_matches_seed() {
    let missions = seed("missions.sql");
    let steps = seed("mission_steps.sql");
    let objectives = seed("mission_objectives.sql");
    let tasks = seed("mission_tasks.sql");
    let mission = row(&missions, "VALUES (10321, 'Moh''katan");
    assert!(mission.contains("'Confirm the Gate Address', 'General', 0, false, false, false"));
    assert!(mission.ends_with("NULL, 0, 0, NULL);"));
    assert!(row(&steps, "VALUES (10378, 10321, ")
        .contains("'Speak with Moh''katan at the Castle Stargate.', 0);"));
    assert!(row(&steps, "VALUES (10379, 10321, ")
        .contains("'Dial Harset on the Castle Stargate.', 1);"));
    assert!(row(&objectives, "VALUES (10498, 10378, ")
        .contains("false, false, false, 'Speak with Moh''katan.');"));
    assert!(row(&objectives, "VALUES (10499, 10379, ")
        .contains("false, false, false, 'Dial the Harset address.');"));
    assert!(tasks.contains("VALUES (10618, 10498, false, 1, true, 1);"));
    assert!(tasks.contains("VALUES (10619, 10499, false, 1, true, 1);"));
    // The XML flags agree with the objective rows: not hidden, not optional.
    for id in ["10498", "10499"] {
        assert!(CASTLE_RETURN_Q2_XML.contains(&format!(
            r#"IsOptional="false" ObjectiveID="{id}" AwardXP="false" IsHidden="false""#
        )));
    }
    // Every id and text the XML carries is the one the seed rows carry.
    assert_eq!(attr_values("MissionID"), ["10321"]);
    assert_eq!(attr_values("StepID"), ["10378", "10379"]);
    assert_eq!(attr_values("ObjectiveID"), ["10498", "10499"]);
    assert_eq!(attr_values("TaskID"), ["10618", "10619"]);
    assert_eq!(attr_values("MissionDefn"), ["Confirm the Gate Address"]);
    assert_eq!(tag_texts("HistoryText").len(), 1);
    for text in tag_texts("HistoryText") {
        assert!(
            mission.contains(&format!("'{}'", sql_text(text))),
            "history: {text}"
        );
    }
    for text in tag_texts("StepDisplayLogText") {
        assert!(
            steps.contains(&format!("'{}'", sql_text(text))),
            "step text: {text}"
        );
    }
    for text in tag_texts("DisplayLogText") {
        assert!(
            objectives.contains(&format!("'{}'", sql_text(text))),
            "objective: {text}"
        );
    }
}

#[test]
fn castle_side_return_q2_steps_in_declaration_order() {
    // The XML declares step A (10378) before step B (10379); the seed index agrees.
    assert_eq!(attr_values("StepID"), ["10378", "10379"]);
    let steps = seed("mission_steps.sql");
    assert!(row(&steps, "VALUES (10378, 10321, ").ends_with(", 0);"));
    assert!(row(&steps, "VALUES (10379, 10321, ").ends_with(", 1);"));
}

#[test]
fn castle_side_return_q2_no_reward_rows() {
    for file in ["mission_rewards.sql", "mission_reward_groups.sql"] {
        let text = seed(file);
        assert!(
            !text
                .split(|c: char| !c.is_ascii_digit())
                .any(|t| t == "10321"),
            "{file} names mission 10321"
        );
    }
    let mission = seed("missions.sql");
    assert!(row(&mission, "VALUES (10321, ").ends_with("NULL, 0, 0, NULL);"));
}
