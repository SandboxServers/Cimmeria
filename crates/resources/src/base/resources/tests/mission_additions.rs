//! Authored admission, exact stock provenance, atomic refusal and telemetry.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use quick_xml::events::Event;
use quick_xml::Reader;

use super::super::*;
use crate::base::mission_additions::{
    MissionAddition, MAX_MISSION_XML_BYTES, MISSION_ADDITIONS, TEST_MISSION_ADDITIONS,
    TEST_MISSION_XML,
};
use crate::base::mission_overrides::{MISSION_OVERRIDES, STEP_TEXT_OVERRIDES};
use crate::test_support::LogCapture;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cache");

fn raw() -> CategoryData {
    ResourceCache::load_raw_mission_category_for_test(DIR).unwrap()
}

fn stock_xml() -> String {
    String::from_utf8(raw().elements[&1624].clone()).unwrap()
}

fn addition(mission_id: u32, xml: String) -> MissionAddition {
    MissionAddition {
        mission_id,
        xml: Box::leak(xml.into_boxed_str()),
    }
}

fn assert_atomic_rejection(id: u32, bad: String, reason: &str) {
    let capture = LogCapture::install();
    let mut categories = HashMap::from([(3, raw())]);
    let before = categories.clone();
    let batch = [
        addition(61000, TEST_MISSION_XML.to_owned()),
        addition(id, bad),
    ];
    let error = ResourceCache::apply_mission_overrides(&mut categories, &batch).unwrap_err();
    assert!(error.starts_with(reason), "expected {reason}, got {error}");
    assert_eq!(
        categories, before,
        "{reason} must leave every byte/key/version unchanged"
    );
    let events = capture.all();
    assert!(
        events.iter().any(|e| e.level == tracing::Level::WARN
            && e.has_field("event", "cooked_data.mission_catalog_rejected")
            && e.has_field("reason", reason)
            && e.has_field("category_id", "3")
            && e.has_field("category_name", "CookedDataMissions")
            && e.has_field("mission_id", &id.to_string())
            && e.has_field("bytes", &batch[1].xml.len().to_string())),
        "missing refusal log for {reason}"
    );
    assert!(!events
        .iter()
        .any(|e| e.has_field("event", "cooked_data.mission_catalog_inserted")));
    assert!(!events
        .iter()
        .any(|e| e.message_contains("Applied Cimmeria")));
}

#[test]
fn fixture_is_exact_stock_transform_and_ids_are_test_only() {
    let stock = raw();
    assert_eq!(stock.metadata, 7538);
    assert_eq!(stock.elements.len(), 1040);
    assert!(!stock.elements.contains_key(&61000));
    assert_eq!(stock.elements[&1624].len(), 1093);
    let transformed = stock_xml()
        .replace("MissionID=\"1624\"", "MissionID=\"61000\"")
        .replace("StepID=\"4818\"", "StepID=\"61001\"")
        .replace("ObjectiveID=\"6198\"", "ObjectiveID=\"61002\"")
        .replace("TaskID=\"8213\"", "TaskID=\"61003\"")
        .replace("Ambush", "CB-N05a catalog fixture")
        .replace(
            "Eliminate the Mutated Asgard.",
            "Verify the authored mission catalog.",
        )
        .replace(
            "<DisplayLogText></DisplayLogText>",
            "<DisplayLogText> </DisplayLogText>",
        );
    assert_eq!(TEST_MISSION_XML.as_bytes(), transformed.as_bytes());
    assert_eq!(TEST_MISSION_XML.len(), 1139);
    assert!(!TEST_MISSION_XML.ends_with('\n'));
    let reserved = [61000, 61001, 61002, 61003];
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
                            assert!(!reserved.contains(&attr.value.parse::<u32>().unwrap()));
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
    }
    let seed_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../db/resources/Missions/Seed");
    for entry in std::fs::read_dir(seed_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "sql") {
            let seed = std::fs::read_to_string(path).unwrap();
            for token in seed.split(|c: char| !c.is_ascii_digit()) {
                if let Ok(id) = token.parse::<u32>() {
                    assert!(!reserved.contains(&id), "fixture id {id} was seeded");
                }
            }
        }
    }
    assert!(MISSION_ADDITIONS.is_empty());
}

#[test]
fn insertion_preserves_served_baseline_and_bumps_once() {
    let baseline = ResourceCache::load_all(DIR).unwrap();
    let empty = ResourceCache::load_all_with_mission_additions(DIR, &[]).unwrap();
    for &(id, _) in CATEGORY_PAKS {
        assert_eq!(baseline.category(id), empty.category(id));
        assert_eq!(
            baseline.overridden_elements(id),
            empty.overridden_elements(id)
        );
    }
    assert!(!baseline.category(3).unwrap().elements.contains_key(&61000));
    let first =
        ResourceCache::load_all_with_mission_additions(DIR, TEST_MISSION_ADDITIONS).unwrap();
    let second =
        ResourceCache::load_all_with_mission_additions(DIR, TEST_MISSION_ADDITIONS).unwrap();
    let missions = first.category(3).unwrap();
    assert_eq!(missions, second.category(3).unwrap());
    assert_eq!(missions.elements.len(), 1041);
    assert_eq!(missions.elements[&61000], TEST_MISSION_XML.as_bytes());
    assert_eq!(
        first
            .overridden_elements(3)
            .iter()
            .filter(|&&id| id == 61000)
            .count(),
        1
    );
    assert_eq!(
        missions.metadata,
        7538u32.wrapping_add(compute_metadata_bump(
            MISSION_OVERRIDES,
            STEP_TEXT_OVERRIDES,
            TEST_MISSION_ADDITIONS,
        ))
    );
    for (id, bytes) in &baseline.category(3).unwrap().elements {
        assert_eq!(
            &missions.elements[id], bytes,
            "existing served mission {id}"
        );
    }
    let patched: HashSet<_> = MISSION_OVERRIDES
        .iter()
        .map(|ov| ov.mission_id)
        .chain(STEP_TEXT_OVERRIDES.iter().map(|ov| ov.mission_id))
        .collect();
    for (id, bytes) in raw().elements {
        if !patched.contains(&id) {
            assert_eq!(
                missions.elements[&id], bytes,
                "unpatched stock mission {id}"
            );
        }
    }
    for id in [622, 641] {
        assert_ne!(
            missions.elements[&id],
            raw().elements[&id],
            "old patches still apply"
        );
    }
    // Repeat application is deliberately not a hot-reload API.
    let mut categories = HashMap::from([(3, missions.clone())]);
    let before = categories.clone();
    assert!(
        ResourceCache::apply_mission_overrides(&mut categories, TEST_MISSION_ADDITIONS)
            .unwrap_err()
            .starts_with("stock_id_collision:")
    );
    assert_eq!(categories, before);
}

#[test]
fn hash_and_synthetic_batch_are_order_independent_and_empty_is_legacy_neutral() {
    // Existing stock 1624 is used ONLY in a synthetic empty category, never
    // registered as a production addition or loaded on top of its stock key.
    let stock = addition(1624, stock_xml());
    let fixture = addition(61000, TEST_MISSION_XML.to_owned());
    let batch = [stock, fixture];
    let reversed = [
        addition(61000, TEST_MISSION_XML.to_owned()),
        addition(1624, stock_xml()),
    ];
    assert_eq!(
        compute_metadata_bump(&[], &[], &batch),
        compute_metadata_bump(&[], &[], &reversed)
    );
    let apply = |additions: &[MissionAddition]| {
        let mut categories = HashMap::from([(
            3,
            CategoryData {
                metadata: 7538,
                elements: HashMap::new(),
            },
        )]);
        let ids = ResourceCache::apply_mission_overrides(&mut categories, additions).unwrap();
        (categories, ids)
    };
    assert_eq!(apply(&batch), apply(&reversed));
    let bump = compute_metadata_bump(&[], &[], TEST_MISSION_ADDITIONS);
    assert_ne!(
        bump,
        compute_metadata_bump(
            &[],
            &[],
            &[addition(
                61000,
                TEST_MISSION_XML.replace("catalog fixture", "catalog edited")
            )]
        )
    );
    assert_ne!(
        bump,
        compute_metadata_bump(&[], &[], &[addition(1624, TEST_MISSION_XML.to_owned())])
    );
    let mut legacy = std::collections::hash_map::DefaultHasher::new();
    for ov in MISSION_OVERRIDES {
        ov.mission_id.hash(&mut legacy);
        ov.insert_after_step_id.hash(&mut legacy);
        ov.injected_steps_xml.hash(&mut legacy);
    }
    for ov in STEP_TEXT_OVERRIDES {
        ov.mission_id.hash(&mut legacy);
        ov.step_id.hash(&mut legacy);
        ov.new_step_display_log_text.hash(&mut legacy);
    }
    assert_eq!(
        compute_metadata_bump(MISSION_OVERRIDES, STEP_TEXT_OVERRIDES, &[]),
        ((legacy.finish() as u32) & 0xffff) | 1
    );
    let (missing_targets, ids) = apply(TEST_MISSION_ADDITIONS);
    assert_eq!(missing_targets[&3].elements.len(), 1);
    assert_eq!(ids[&3], [61000]);
}

#[test]
fn every_bad_second_entry_refuses_atomically_before_stock_patches_or_success_logs() {
    let xml = stock_xml();
    let duplicate_nested = xml.replace("StepID=\"4818\"", "StepID=\"61001\"");
    let too_large = format!("{xml}{}", " ".repeat(MAX_MISSION_XML_BYTES + 1 - xml.len()));
    assert_eq!(too_large.len(), MAX_MISSION_XML_BYTES + 1);
    let cases = vec![
        (
            1624,
            xml.trim_end_matches("</COOKED_MISSION>").to_owned(),
            "malformed_xml",
        ),
        (
            1624,
            xml.replace("COOKED_MISSION", "BAD_ROOT"),
            "wrong_root",
        ),
        (
            1624,
            xml.replace("<HistoryText>", "<Steps>"),
            "invalid_nesting_or_order",
        ),
        (
            1624,
            xml.replace(" AwardXP=\"true\"", ""),
            "missing_required_attribute",
        ),
        (
            1624,
            xml.replace("AwardXP=\"true\"", "AwardXP=\"maybe\""),
            "invalid_boolean",
        ),
        (
            1624,
            xml.replace("Level=\"1\"", "Level=\"1.5\""),
            "invalid_integer",
        ),
        (
            1624,
            xml.replace("StepID=\"4818\"", "StepID=\"2147483648\""),
            "invalid_integer",
        ),
        (
            1624,
            xml.replace("StepID=\"4818\"", "StepID=\"0\""),
            "nonpositive_id",
        ),
        (1624, xml.replace("Ambush", "&unknown;"), "invalid_entity"),
        (
            1624,
            xml.replace("<HistoryText>Ambush", "<HistoryText>&unknown;"),
            "invalid_entity",
        ),
        (
            1624,
            xml.replace(" Level=\"1\"", " Level=\"1\" Level=\"1\""),
            "invalid_or_duplicate_attribute",
        ),
        (61000, TEST_MISSION_XML.to_owned(), "duplicate_registry_id"),
        (1624, duplicate_nested, "duplicate_nested_id"),
        (61001, xml.clone(), "mission_id_mismatch"),
        (1624, format!("{xml}{xml}"), "invalid_declaration"),
        (1624, format!("{xml}<COOKED_MISSION/>"), "multiple_roots"),
        (1624, format!("{xml}trailing"), "unexpected_text"),
        (1624, format!("{xml}<![CDATA[ ]]>"), "unexpected_text"),
        (
            1624,
            xml.replace("<HistoryText>Ambush", "<HistoryText>\0"),
            "invalid_xml_character",
        ),
        (
            1624,
            xml.replace("<HistoryText>Ambush", "<HistoryText>&#0;"),
            "invalid_entity",
        ),
        (
            1624,
            xml.replace("TaskID=\"8213\"", "TaskID=\"6198\""),
            "duplicate_nested_id",
        ),
        (
            1624,
            xml.replace(" TaskType=\"1\"", ""),
            "missing_required_attribute",
        ),
        (
            1624,
            xml.replace(
                "<COOKED_MISSION",
                "<!DOCTYPE COOKED_MISSION [<!ENTITY x 'boom'>]><COOKED_MISSION",
            ),
            "dtd_forbidden",
        ),
        (1624, xml.clone(), "stock_id_collision"),
        (0, xml.clone(), "root_id_out_of_range"),
        (65536, xml.clone(), "root_id_out_of_range"),
        (1624, too_large, "xml_too_large"),
        (
            1624,
            xml.replace(
                "<StepDisplayLogText>Eliminate the Mutated Asgard.</StepDisplayLogText>",
                "",
            ),
            "invalid_nesting_or_order",
        ),
        (
            1624,
            xml.replace("<DisplayLogText></DisplayLogText>", ""),
            "invalid_nesting_or_order",
        ),
        (
            1624,
            xml.replace("<HistoryText>Ambush</HistoryText>", ""),
            "invalid_nesting_or_order",
        ),
    ];
    for (id, bad, reason) in cases {
        assert_atomic_rejection(id, bad, reason);
    }
}

#[test]
fn invalid_declaration_grammar_and_placement_refuse_atomically() {
    let xml = stock_xml();
    let (_, body) = xml.split_once("?>").unwrap();
    for declaration in [
        "<?xml version=\"1.0\" version=\"1.0\"?>",
        "<?xml version=\"1.0\" encoding=\"UTF-8\" encoding=\"UTF-16\"?>",
        "<?xml version=\"1.0\" standalone=\"yes\" standalone=\"yes\"?>",
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"bogus\"?>",
        "<?xml encoding=\"UTF-8\" version=\"1.0\"?>",
        "<?xml version=\"1.0\" standalone=\"no\" encoding=\"UTF-8\"?>",
        "<?xml version=\"1.0\" extra=\"value\"?>",
        "<?xml version=\"1.1\"?>",
        "<?xml version=\"1.0\" encoding=\"UTF-16\"?>",
        "<?xml standalone=\"yes\"?>",
        "<?xml version=\"1.0\"encoding=\"UTF-8\"?>",
        "<?xml version=\"1.0\" standalone=\"y&#101;s\"?>",
        "<?xml version=\"1.0\" \u{a0}encoding=\"UTF-8\"?>",
        "<?xml version=\"1.0\" junk?>",
    ] {
        assert_atomic_rejection(1624, format!("{declaration}{body}"), "invalid_declaration");
    }
    for prefix in [" ", "\n", "<!--before declaration-->"] {
        assert_atomic_rejection(1624, format!("{prefix}{xml}"), "invalid_declaration");
    }
    assert_atomic_rejection(
        1624,
        format!("<?xml version=\"1.0\"?>{xml}"),
        "invalid_declaration",
    );
}

#[test]
fn invalid_xml_characters_text_and_outside_whitespace_refuse_atomically() {
    let xml = stock_xml();
    let cases = [
        (
            xml.replace("<HistoryText>Ambush", "<HistoryText>]]>"),
            "malformed_xml",
        ),
        (format!("{xml}\u{a0}"), "unexpected_text"),
        (format!("\u{a0}{xml}"), "unexpected_text"),
        (xml.replace("<Steps ", "\u{a0}<Steps "), "unexpected_text"),
        (format!("{xml}<!--\0-->"), "invalid_xml_character"),
        (format!("{xml}<!--\u{fffe}-->"), "invalid_xml_character"),
        (
            xml.replace("MissionDefn=\"Ambush\"", "MissionDefn=\"\u{ffff}\""),
            "invalid_xml_character",
        ),
        (
            xml.replace("</HistoryText>", "</HistoryText\0>"),
            "invalid_xml_character",
        ),
    ];
    for (bad, reason) in cases {
        assert_atomic_rejection(1624, bad, reason);
    }
}

#[test]
fn missing_final_required_children_refuse_atomically() {
    let xml = stock_xml();
    let (before_steps, _) = xml.split_once("<Steps ").unwrap();
    let (before_objectives, _) = xml.split_once("<Objectives ").unwrap();
    let (before_display, _) = xml.split_once("<DisplayLogText>").unwrap();
    for bad in [
        format!("{before_steps}</COOKED_MISSION>"),
        format!("{before_objectives}</Steps></COOKED_MISSION>"),
        format!("{before_display}</Objectives></Steps></COOKED_MISSION>"),
    ] {
        assert_atomic_rejection(1624, bad, "missing_required_child");
    }
}

#[test]
fn supported_declarations_and_xml_whitespace_preserve_exact_bytes() {
    let (_, body) = TEST_MISSION_XML.split_once("?>").unwrap();
    for declaration in [
        "",
        "<?xml version='1.0'?>",
        "<?xml version=\"1.0\" standalone='yes'?>",
        "<?xml version='1.0' encoding='utf-8' standalone=\"no\" ?>",
        "\u{feff}<?xml\tversion \r=\n'1.0'\tencoding='UTF-8'?>",
    ] {
        let xml = format!("{declaration}{body}\t\r\n ");
        let batch = [addition(61000, xml)];
        let mut categories = HashMap::from([(3, raw())]);
        ResourceCache::apply_mission_overrides(&mut categories, &batch).unwrap();
        assert_eq!(categories[&3].elements[&61000], batch[0].xml.as_bytes());
    }
}

#[test]
fn missing_category_still_validates_batch_and_warns_without_creating_category() {
    let capture = LogCapture::install();
    let mut categories = HashMap::new();
    assert!(
        ResourceCache::apply_mission_overrides(&mut categories, TEST_MISSION_ADDITIONS)
            .unwrap()
            .is_empty()
    );
    assert!(categories.is_empty());
    let duplicate = [
        addition(61000, TEST_MISSION_XML.to_owned()),
        addition(61000, TEST_MISSION_XML.to_owned()),
    ];
    assert!(
        ResourceCache::apply_mission_overrides(&mut categories, &duplicate)
            .unwrap_err()
            .starts_with("duplicate_registry_id")
    );
    assert!(categories.is_empty());
    assert!(capture
        .all()
        .iter()
        .any(|e| e.has_field("reason", "category_not_loaded")
            && e.has_field("category_id", "3")
            && e.level == tracing::Level::WARN));
    let batch = [
        addition(61000, TEST_MISSION_XML.to_owned()),
        addition(1624, "<bad/>".to_owned()),
    ];
    assert!(
        ResourceCache::apply_mission_overrides(&mut categories, &batch)
            .unwrap_err()
            .starts_with("wrong_root")
    );
    assert!(categories.is_empty());
    assert!(ResourceCache::apply_mission_overrides(&mut categories, &[])
        .unwrap()
        .is_empty());
}

#[test]
fn supported_optional_tasks_entities_and_size_boundary_preserve_exact_bytes_and_log_final_version()
{
    let no_tasks = TEST_MISSION_XML.replace("<Tasks TaskType=\"1\" TaskID=\"61003\" AwardXP=\"false\" IsEnabled=\"true\" Difficulty=\"1\"></Tasks>", "");
    let escaped = no_tasks.replace("catalog fixture", "catalog &amp; &#65; fixture");
    let padded = format!(
        "{escaped}{}",
        " ".repeat(MAX_MISSION_XML_BYTES - escaped.len())
    );
    assert_eq!(padded.len(), 355_840);
    let batch = [addition(61000, padded)];
    let capture = LogCapture::install();
    let mut categories = HashMap::from([(3, raw())]);
    ResourceCache::apply_mission_overrides(&mut categories, &batch).unwrap();
    assert_eq!(categories[&3].elements[&61000], batch[0].xml.as_bytes());
    let events = capture.all();
    let inserted: Vec<_> = events
        .iter()
        .filter(|e| e.has_field("event", "cooked_data.mission_catalog_inserted"))
        .collect();
    assert_eq!(inserted.len(), 1);
    let event = inserted[0];
    assert_eq!(event.level, tracing::Level::INFO);
    assert!(event.has_field("category_id", "3"));
    assert!(event.has_field("category_name", "CookedDataMissions"));
    assert!(event.has_field("mission_id", "61000"));
    assert!(event.has_field("mission_name", "CB-N05a catalog & A fixture"));
    assert!(event.has_field("bytes", "355840"));
    assert!(event.has_field("metadata_version", &categories[&3].metadata.to_string()));
    assert!(!event.fields.contains_key("xml"));
}
