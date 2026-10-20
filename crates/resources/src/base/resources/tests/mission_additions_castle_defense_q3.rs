//! Castle authored catalog proof for `defense_q3` (CS-04b3s): the admitted XML,
//! its unique ids, the seed rows that back it, the absence of reward rows and
//! the giver template and spawn that offer it.

use super::super::*;
use crate::base::mission_additions::{MAX_MISSION_XML_BYTES, MISSION_ADDITIONS};
use crate::base::mission_additions_castle::CASTLE_DEFENSE_Q3_XML;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");
const SEED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../db/resources/Missions/Seed"
);
const DB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../db/resources");

/// Mission, step, objective and task ids of the sealed `defense_q3` entry, with
/// the seed file that owns each and the CS-00Be sub-block it must sit in.
const DEFENSE_Q3_IDS: [(u32, &str, (u32, u32)); 10] = [
    (10312, "missions.sql", (10312, 10312)),
    (10366, "mission_steps.sql", (10366, 10371)),
    (10486, "mission_objectives.sql", (10486, 10491)),
    (10487, "mission_objectives.sql", (10486, 10491)),
    (10488, "mission_objectives.sql", (10486, 10491)),
    (10489, "mission_objectives.sql", (10486, 10491)),
    (10606, "mission_tasks.sql", (10606, 10611)),
    (10607, "mission_tasks.sql", (10606, 10611)),
    (10608, "mission_tasks.sql", (10606, 10611)),
    (10609, "mission_tasks.sql", (10606, 10611)),
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

/// The comma-separated values of one `INSERT ... VALUES (...);` line.
fn values(line: &str) -> Vec<&str> {
    let start = line.find("VALUES (").unwrap() + "VALUES (".len();
    let body = line[start..]
        .trim_end()
        .trim_end_matches(';')
        .strip_suffix(')')
        .unwrap();
    body.split(", ").collect()
}

/// Every `<tag>text</tag>` body in the XML, in document order.
fn tag_texts(tag: &str) -> Vec<&'static str> {
    let (open, close) = (format!("<{tag}>"), format!("</{tag}>"));
    CASTLE_DEFENSE_Q3_XML
        .split(open.as_str())
        .skip(1)
        .map(|rest| rest.split(close.as_str()).next().unwrap())
        .collect()
}

/// Every `name="value"` attribute value in the XML, in document order.
fn attr_values(name: &str) -> Vec<&'static str> {
    let key = format!(" {name}=\"");
    CASTLE_DEFENSE_Q3_XML
        .split(key.as_str())
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap())
        .collect()
}

#[test]
fn castle_side_defense_q3_xml_admitted() {
    let cache = ResourceCache::load_all_with_mission_additions(DIR, MISSION_ADDITIONS).unwrap();
    let missions = cache.category(3).unwrap();
    assert_eq!(missions.elements[&10312], CASTLE_DEFENSE_Q3_XML.as_bytes());
    assert_eq!(CASTLE_DEFENSE_Q3_XML.len(), 2116);
    assert!(CASTLE_DEFENSE_Q3_XML.len() <= MAX_MISSION_XML_BYTES);
    assert!(!CASTLE_DEFENSE_Q3_XML.contains('\n'));
    assert!(CASTLE_DEFENSE_Q3_XML.contains(r#"MissionID="10312""#));
}

#[test]
fn castle_side_defense_q3_ids_in_block() {
    let ids: Vec<u32> = DEFENSE_Q3_IDS.iter().map(|&(id, ..)| id).collect();
    let unique: std::collections::HashSet<u32> = ids.iter().copied().collect();
    assert_eq!(unique.len(), ids.len());
    for &(id, file, (lo, hi)) in &DEFENSE_Q3_IDS {
        assert!((lo..=hi).contains(&id), "{id} is outside {lo}-{hi}");
        // Not a stock key: the id is the first value of exactly one seed row.
        let rows = seed(file)
            .lines()
            .filter(|l| l.contains(&format!("VALUES ({id}, ")))
            .count();
        assert_eq!(rows, 1, "{file} must hold exactly one row for {id}");
    }
    let stock = ResourceCache::load_all_with_mission_additions(DIR, &[]).unwrap();
    assert!(!stock.category(3).unwrap().elements.contains_key(&10312));
}

#[test]
fn castle_side_defense_q3_is_registered_once() {
    // Found by id, never by position, so later "s" packets can append entries.
    let mut hits = MISSION_ADDITIONS.iter().filter(|a| a.mission_id == 10312);
    let entry = hits.next().expect("mission 10312 is not registered");
    assert!(hits.next().is_none(), "mission 10312 is registered twice");
    assert_eq!(entry.xml, CASTLE_DEFENSE_Q3_XML);
    let ids: Vec<u32> = MISSION_ADDITIONS.iter().map(|a| a.mission_id).collect();
    assert!(
        ids.windows(2).all(|w| w[0] < w[1]),
        "registry ids not strictly ascending: {ids:?}"
    );
}

#[test]
fn castle_side_defense_q3_xml_matches_seed() {
    let missions = seed("missions.sql");
    let steps = seed("mission_steps.sql");
    let objectives = seed("mission_objectives.sql");
    let tasks = seed("mission_tasks.sql");
    let mission = row(&missions, "VALUES (10312, 'Hostile officers");
    assert!(mission.contains("'Hold the Bravo Route', 'General', 0, false, false, false"));
    assert!(mission.ends_with("NULL, 0, 0, NULL);"));
    assert!(row(&steps, "VALUES (10366, 10312, ")
        .contains("'Put down the hostile officers on the Bravo route.', 0);"));
    for (id, text) in [
        ("10486", "'Defeat the first Bravo officer.'"),
        ("10487", "'Defeat the second Bravo officer.'"),
        ("10488", "'Defeat the third Bravo officer.'"),
        ("10489", "'Defeat Muelbach.'"),
    ] {
        assert!(row(&objectives, &format!("VALUES ({id}, 10366, ")).contains(text));
    }
    assert!(tasks.contains("VALUES (10606, 10486, false, 1, true, 1);"));
    assert!(tasks.contains("VALUES (10607, 10487, false, 1, true, 1);"));
    assert!(tasks.contains("VALUES (10608, 10488, false, 1, true, 1);"));
    assert!(tasks.contains("VALUES (10609, 10489, false, 1, true, 1);"));
    // Every id and text the XML carries is the one the seed rows carry.
    assert_eq!(attr_values("MissionID"), ["10312"]);
    assert_eq!(attr_values("StepID"), ["10366"]);
    assert_eq!(
        attr_values("ObjectiveID"),
        ["10486", "10487", "10488", "10489"]
    );
    assert_eq!(attr_values("TaskID"), ["10606", "10607", "10608", "10609"]);
    assert_eq!(attr_values("TaskType"), ["1", "1", "1", "1"]);
    assert_eq!(attr_values("MissionDefn"), ["Hold the Bravo Route"]);
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
    for id in ["10486", "10487", "10488", "10489"] {
        assert!(objectives
            .lines()
            .any(|l| l.contains(&format!("VALUES ({id}, 10366, "))
                && l.contains("false, false, false, ")));
    }
}

#[test]
fn castle_side_defense_q3_four_required_objectives() {
    // The auto-complete rule counts required objectives only.
    assert_eq!(
        CASTLE_DEFENSE_Q3_XML
            .matches(r#"IsOptional="false" ObjectiveID="#)
            .count(),
        4
    );
    assert!(!CASTLE_DEFENSE_Q3_XML.contains(r#"IsOptional="true""#));
    let objectives = seed("mission_objectives.sql");
    for id in ["10486", "10487", "10488", "10489"] {
        let line = row(&objectives, &format!("VALUES ({id}, 10366, "));
        let v = values(line);
        assert_eq!(v[6], "false", "is_optional on {id}");
    }
}

#[test]
fn castle_side_defense_q3_reward_matches_seal() {
    for file in ["mission_rewards.sql", "mission_reward_groups.sql"] {
        let text = seed(file);
        assert!(
            !text
                .split(|c: char| !c.is_ascii_digit())
                .any(|t| t == "10312"),
            "{file} names mission 10312"
        );
    }
    let mission = seed("missions.sql");
    let v = values(row(&mission, "VALUES (10312, "));
    assert_eq!(v[v.len() - 3], "0", "reward_naq");
    assert_eq!(v[v.len() - 2], "0", "reward_xp");
}

#[test]
fn castle_side_defense_q3_giver_rows_frozen() {
    let ents = std::fs::read_to_string(format!("{DB}/Entities/Seed/entity_templates.sql")).unwrap();
    let base = values(row(&ents, "VALUES (188, "));
    let giver = values(row(&ents, "VALUES (425, "));
    assert_eq!(base.len(), giver.len());
    // Column 0 is template_id, column 9 faction, column 14 template_name.
    for (i, (b, g)) in base.iter().zip(&giver).enumerate() {
        if i != 0 && i != 14 {
            assert_eq!(b, g, "template 425 column {i} differs from template 188");
        }
    }
    assert_eq!(giver[9], "3", "giver faction");
    assert_eq!(giver[14], "'Castle_DefenseQ3_Giver'");

    let spawns = std::fs::read_to_string(format!("{DB}/Worlds/Seed/spawnlist.sql")).unwrap();
    let spawn = values(row(&spawns, "VALUES (14096, "));
    // Frozen (e): the navmesh position on component 116 and the heading.
    assert_eq!(
        spawn[1..=4],
        ["790.0", "59.7", "530.0", "5.3559"],
        "transform"
    );
    assert_eq!(spawn[5], "8", "spawn world");
    assert_eq!(spawn[6], "425", "spawn template");
    assert_eq!(spawn[7], "'Castle_DefenseQ3_Giver'", "spawn tag");
    assert_eq!(spawn[8], "NULL", "spawn set_name");
    assert_eq!(spawn[9], "true", "spawn is_stationary");
    assert_eq!(spawns.matches("'Castle_DefenseQ3_Giver'").count(), 1);
}
