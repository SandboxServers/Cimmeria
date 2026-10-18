//! CB-N06 *Before the Patrol Returns* (mission 62060): frozen bytes, stock absence, served cache and seeds.

use super::super::*;
use crate::base::mission_additions::{PRISONER_CHOICE_XML, REMAINING_PODS_XML};
use crate::test_support::LogCapture;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");

const SEED_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../db/resources/Missions/Seed/"
);

/// Independent copy of frozen literal F1 (w5-frozen.md), not a reference to the production const.
const EXPECTED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<COOKED_MISSION xmlns:SOAP-ENV="http://schemas.xmlsoap.org/soap/envelope/" xmlns:SOAP-ENC="http://schemas.xmlsoap.org/soap/encoding/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" xmlns:CookedData1="SGW" AwardXP="false" ShowFactionChangeIcon="false" ShowInstanceIcon="false" ShowPVPIcon="false" IsShareable="false" NumRepeats="0" CanAbandon="true" CanRepeatOnFail="false" CanFail="false" Level="1" IsHidden="false" IsOverrideMission="false" IsEnabled="true" IsAStory="true" Difficulty="1" MissionLabel="General" MissionDefn="Before the Patrol Returns" MissionID="62060"><HistoryText>Two prisoners in the cell block are too weak to escape with you. Before the patrol returns there is time for one thing: help them hide, or jam the cell-door panel so the guards cannot seal the block.</HistoryText><Steps StepEnabled="false" StepID="62061" AwardXP="false" Difficulty="1"><StepDisplayLogText>Help the prisoners hide, or jam the cell-door panel.</StepDisplayLogText><Objectives IsOptional="false" ObjectiveID="62062" AwardXP="false" IsHidden="false" IsEnabled="false" Difficulty="1"><DisplayLogText>Help the prisoners or jam the panel.</DisplayLogText><Tasks TaskType="1" TaskID="62063" AwardXP="false" IsEnabled="true" Difficulty="1"></Tasks></Objectives></Steps></COOKED_MISSION>"#;

/// Seed rows from frozen literal F2 (w5-frozen.md), one entry per file, copied byte for byte.
const SEED_ROWS: &[(&str, &str)] = &[
    ("missions.sql", "INSERT INTO missions (mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, is_shareable, level, mission_defn, mission_label, num_repeats, show_faction_change_icon, show_instance_icon, show_pvp_icon, script_name, reward_naq, reward_xp, script_spaces) VALUES (62060, 'Two prisoners in the cell block are too weak to escape with you. Before the patrol returns there is time for one thing: help them hide, or jam the cell-door panel so the guards cannot seal the block.', false, true, false, false, 1, true, true, false, false, false, 1, 'Before the Patrol Returns', 'General', 0, false, false, false, NULL, 0, 0, NULL);"),
    ("mission_steps.sql", "INSERT INTO mission_steps (step_id, mission_id, award_xp, difficulty, step_enabled, step_display_log_text, index) VALUES (62061, 62060, false, 1, false, 'Help the prisoners hide, or jam the cell-door panel.', 0);"),
    ("mission_objectives.sql", "INSERT INTO mission_objectives (objective_id, step_id, award_xp, difficulty, is_enabled, is_hidden, is_optional, display_log_text) VALUES (62062, 62061, false, 1, false, false, false, 'Help the prisoners or jam the panel.');"),
    ("mission_tasks.sql", "INSERT INTO mission_tasks (task_id, objective_id, award_xp, difficulty, is_enabled, task_type) VALUES (62063, 62062, false, 1, true, 1);"),
];

#[test]
fn prisoner_choice_xml_is_the_frozen_bytes() {
    assert_eq!(PRISONER_CHOICE_XML, EXPECTED);
    assert_eq!(PRISONER_CHOICE_XML.len(), 1369);
    assert_eq!(PRISONER_CHOICE_XML.matches('\n').count(), 1);
    assert!(!PRISONER_CHOICE_XML.contains('\r'));
}

#[test]
fn prisoner_choice_ids_are_absent_from_the_stock_pak() {
    let raw = ResourceCache::load_raw_mission_category_for_test(DIR).unwrap();
    assert!(!raw.elements.contains_key(&62060));
    for id in 62060..=62063u32 {
        let needle = format!("ID=\"{id}\"");
        for xml in raw.elements.values() {
            let text = std::str::from_utf8(xml).unwrap();
            assert!(!text.contains(&needle), "stock pak already has {needle}");
        }
    }
}

#[test]
fn production_cache_serves_prisoner_choice() {
    let served = ResourceCache::load_all(DIR).unwrap();
    let raw = ResourceCache::load_raw_mission_category_for_test(DIR).unwrap();
    assert_eq!(
        served.get(3, 62060).unwrap(),
        PRISONER_CHOICE_XML.as_bytes()
    );
    assert_eq!(served.get(3, 62050).unwrap(), REMAINING_PODS_XML.as_bytes());
    assert_eq!(
        served.category(3).unwrap().elements.len(),
        raw.elements.len() + 2
    );
}

#[test]
fn production_load_logs_the_62060_insert() {
    let capture = LogCapture::install();
    ResourceCache::load_all(DIR).unwrap();
    let events = capture.all();
    let inserted: Vec<_> = events
        .iter()
        .filter(|e| e.has_field("event", "cooked_data.mission_catalog_inserted"))
        .filter(|e| e.has_field("mission_id", "62060"))
        .collect();
    assert_eq!(inserted.len(), 1);
}

#[test]
fn prisoner_choice_seed_rows_match_the_catalog() {
    for &(file, row) in SEED_ROWS {
        let seed = std::fs::read_to_string(format!("{SEED_DIR}{file}")).unwrap();
        let seed = seed.replace("\r\n", "\n");
        let hits = seed.lines().filter(|line| *line == row).count();
        assert_eq!(hits, 1, "{file} must contain one exact row: {row}");
    }
    let xml = PRISONER_CHOICE_XML;
    assert!(xml.contains("MissionDefn=\"Before the Patrol Returns\""));
    for text in [
        "Two prisoners in the cell block are too weak to escape with you. Before the patrol returns there is time for one thing: help them hide, or jam the cell-door panel so the guards cannot seal the block.",
        "Help the prisoners hide, or jam the cell-door panel.",
        "Help the prisoners or jam the panel.",
    ] {
        assert!(xml.contains(text), "XML lacks seed text: {text}");
    }
}
