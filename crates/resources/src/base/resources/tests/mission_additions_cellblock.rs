//! CB-N05 *The Remaining Pods* (mission 62050): frozen bytes, stock absence, served cache and seeds.

use super::super::*;
use crate::base::mission_additions::{MISSION_ADDITIONS, REMAINING_PODS_XML};
use crate::test_support::LogCapture;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");

const SEED_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../db/resources/Missions/Seed/"
);

/// Independent copy of frozen literal F1 (w3-frozen.md), not a reference to the production const.
const EXPECTED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="The Remaining Pods" MissionID="62050"><HistoryText>Prisoner 329 heard other captives in nearby pods. Tell Col. Marsh so they are not left behind.</HistoryText><Steps StepEnabled="false" StepID="62051" AwardXP="false" Difficulty="1"><StepDisplayLogText>Ask Prisoner 329 about the other pods.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62052" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Speak to Prisoner 329.</DisplayLogText><Tasks TaskType="1" TaskID="62053" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps><Steps StepEnabled="false" StepID="62054" AwardXP="false" Difficulty="1"><StepDisplayLogText>Tell Col. Marsh there may be prisoners in the other pods.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62055" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Warn Col. Marsh.</DisplayLogText><Tasks TaskType="1" TaskID="62056" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>"#;

/// Seed rows from frozen literal F2 (w3-frozen.md), one entry per file, copied byte for byte.
const SEED_ROWS: &[(&str, &str)] = &[
    ("missions.sql", "INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (62050, 'Prisoner 329 heard other captives in nearby pods. Tell Col. Marsh so they are not left behind.', false, true, false, false, 1, true, true, false, false, false, 1, 'The Remaining Pods', 'General', 0, false, false, false, NULL, 0, 0, NULL);"),
    ("mission_steps.sql", "INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (62051, 62050, false, 1, false, 'Ask Prisoner 329 about the other pods.', 0);"),
    ("mission_steps.sql", "INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (62054, 62050, false, 1, false, 'Tell Col. Marsh there may be prisoners in the other pods.', 1);"),
    ("mission_objectives.sql", "INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (62052, 62051, false, 1, false, false, false, 'Speak to Prisoner 329.');"),
    ("mission_objectives.sql", "INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (62055, 62054, false, 1, false, false, false, 'Warn Col. Marsh.');"),
    ("mission_tasks.sql", "INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (62053, 62052, false, 1, true, 1);"),
    ("mission_tasks.sql", "INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (62056, 62055, false, 1, true, 1);"),
];

#[test]
fn remaining_pods_xml_is_the_frozen_bytes() {
    assert_eq!(REMAINING_PODS_XML, EXPECTED);
    assert_eq!(REMAINING_PODS_XML.len(), 1678);
    assert_eq!(REMAINING_PODS_XML.matches('\n').count(), 1);
    assert!(!REMAINING_PODS_XML.contains('\r'));
}

#[test]
fn remaining_pods_ids_are_absent_from_the_stock_pak() {
    let raw = ResourceCache::load_raw_mission_category_for_test(DIR).unwrap();
    assert!(!raw.elements.contains_key(&62050));
    for id in 62050..=62056u32 {
        let needle = format!("ID=\"{id}\"");
        for xml in raw.elements.values() {
            let text = std::str::from_utf8(xml).unwrap();
            assert!(!text.contains(&needle), "stock pak already has {needle}");
        }
    }
}

#[test]
fn production_cache_serves_remaining_pods() {
    let served = ResourceCache::load_all(DIR).unwrap();
    let raw = ResourceCache::load_raw_mission_category_for_test(DIR).unwrap();
    let none = ResourceCache::load_all_with_mission_additions(DIR, &[]).unwrap();
    assert_eq!(served.get(3, 62050).unwrap(), REMAINING_PODS_XML.as_bytes());
    assert_eq!(
        served.category(3).unwrap().elements.len(),
        raw.elements.len() + MISSION_ADDITIONS.len()
    );
    assert_ne!(
        served.category(3).unwrap().metadata,
        none.category(3).unwrap().metadata
    );
    for &(id, _) in CATEGORY_PAKS {
        if id != 3 {
            assert_eq!(served.category(id), none.category(id));
        }
    }
}

#[test]
fn production_load_logs_the_insert() {
    let capture = LogCapture::install();
    ResourceCache::load_all(DIR).unwrap();
    let events = capture.all();
    let inserted: Vec<_> = events
        .iter()
        .filter(|e| e.has_field("event", "cooked_data.mission_catalog_inserted"))
        .collect();
    assert_eq!(
        inserted
            .iter()
            .filter(|e| e.has_field("mission_id", "62050"))
            .count(),
        1
    );
}

#[test]
fn mission_seed_rows_match_the_catalog() {
    for &(file, row) in SEED_ROWS {
        let seed = std::fs::read_to_string(format!("{SEED_DIR}{file}")).unwrap();
        let seed = seed.replace("\r\n", "\n");
        let hits = seed.lines().filter(|line| *line == row).count();
        assert_eq!(hits, 1, "{file} must contain one exact row: {row}");
    }
    let xml = REMAINING_PODS_XML;
    assert!(xml.contains("MissionDefn=\"The Remaining Pods\""));
    for text in [
        "Prisoner 329 heard other captives in nearby pods. Tell Col. Marsh so they are not left behind.",
        "Ask Prisoner 329 about the other pods.",
        "Tell Col. Marsh there may be prisoners in the other pods.",
        "Speak to Prisoner 329.",
        "Warn Col. Marsh.",
    ] {
        assert!(xml.contains(text), "XML lacks seed text: {text}");
    }
}
