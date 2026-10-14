//! Admission for the sealed authored subset; stock XML is never re-emitted.
//! Preflight is separate from application so a bad later entry cannot leave
//! either an insertion or an existing mission patch partially applied.

use std::collections::{HashMap, HashSet};

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::base::dialog_overrides::MAX_COOKED_ELEMENT_ID;
use crate::base::mission_additions::{MissionAddition, MAX_MISSION_XML_BYTES};

use super::{category_name, CategoryData, CATEGORY_MISSIONS};

#[derive(Debug)]
pub(super) struct PreparedMission<'a> {
    pub(super) addition: &'a MissionAddition,
    pub(super) name: String,
}

pub(super) fn prepare<'a>(
    category: Option<&CategoryData>,
    additions: &'a [MissionAddition],
) -> Result<Vec<PreparedMission<'a>>, String> {
    let mut registry_ids = HashSet::new();
    let mut all_ids = HashSet::new();
    let mut prepared = Vec::with_capacity(additions.len());
    for addition in additions {
        let mut name = None;
        let result = (|| {
            if !(1..=MAX_COOKED_ELEMENT_ID).contains(&addition.mission_id) {
                return Err("root_id_out_of_range");
            }
            if !registry_ids.insert(addition.mission_id) {
                return Err("duplicate_registry_id");
            }
            if addition.xml.len() > MAX_MISSION_XML_BYTES {
                return Err("xml_too_large");
            }
            validate(addition, &mut all_ids, &mut name)?;
            if category.is_some_and(|c| c.elements.contains_key(&addition.mission_id)) {
                return Err("stock_id_collision");
            }
            Ok(())
        })();
        if let Err(reason) = result {
            tracing::warn!(
                event = "cooked_data.mission_catalog_rejected",
                category_id = CATEGORY_MISSIONS,
                category_name = category_name(CATEGORY_MISSIONS).unwrap_or("unknown"),
                mission_id = addition.mission_id,
                mission_name = name.as_deref().unwrap_or("<unavailable>"),
                reason,
                bytes = addition.xml.len(),
                "Authored mission batch rejected before application",
            );
            return Err(format!("{reason}: mission {}", addition.mission_id));
        }
        prepared.push(PreparedMission {
            addition,
            name: name.expect("validated MissionDefn"),
        });
    }
    prepared.sort_unstable_by_key(|mission| mission.addition.mission_id);
    Ok(prepared)
}

/// Preserve the exact admitted UTF-8 bytes, including declaration separator.
pub(super) fn insert(category: &mut CategoryData, prepared: &[PreparedMission<'_>]) {
    for mission in prepared {
        category.elements.insert(
            mission.addition.mission_id,
            mission.addition.xml.as_bytes().to_vec(),
        );
    }
}

#[derive(Debug)]
struct Frame {
    node: String,
    children: u32,
}

fn text_node(node: &str) -> bool {
    matches!(
        node,
        "HistoryText" | "StepDisplayLogText" | "DisplayLogText"
    )
}

fn xml_chars(text: &str) -> bool {
    text.chars().all(
        |c| matches!(c as u32, 9 | 10 | 13 | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff),
    )
}

fn check_text(stack: &[Frame], text: &str) -> Result<(), &'static str> {
    if !xml_chars(text) {
        return Err("invalid_xml_character");
    }
    if !stack.last().is_some_and(|f| text_node(&f.node)) && !text.trim().is_empty() {
        return Err("unexpected_text");
    }
    Ok(())
}

fn finish(frame: &Frame) -> Result<(), &'static str> {
    let required = match frame.node.as_str() {
        "COOKED_MISSION" | "Steps" => 2,
        "Objectives" => 1,
        _ => 0,
    };
    if frame.children < required {
        return Err("missing_required_child");
    }
    Ok(())
}

fn validate(
    addition: &MissionAddition,
    ids: &mut HashSet<i32>,
    name: &mut Option<String>,
) -> Result<(), &'static str> {
    let mut reader = Reader::from_str(addition.xml);
    reader.config_mut().check_comments = true;
    let mut stack: Vec<Frame> = Vec::new();
    let mut root_seen = false;
    let mut declaration_seen = false;
    loop {
        let event = reader.read_event().map_err(|_| "malformed_xml")?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                // Empty events are closed here, without inventing an End token.
                let node = e.name().as_ref().to_owned();
                if let Some(parent) = stack.last_mut() {
                    let allowed = matches!(
                        (parent.node.as_str(), parent.children, node.as_str()),
                        ("COOKED_MISSION", 0, "HistoryText")
                            | ("Steps", 0, "StepDisplayLogText")
                            | ("Objectives", 0, "DisplayLogText")
                            | ("COOKED_MISSION", 1.., "Steps")
                            | ("Steps", 1.., "Objectives")
                            | ("Objectives", 1.., "Tasks")
                    );
                    if !allowed {
                        return Err("invalid_nesting_or_order");
                    }
                    parent.children += 1;
                } else {
                    if root_seen {
                        return Err("multiple_roots");
                    }
                    if node != "COOKED_MISSION" {
                        return Err("wrong_root");
                    }
                    root_seen = true;
                }
                attributes(&e, &node, addition.mission_id, ids, name)?;
                let frame = Frame { node, children: 0 };
                if empty {
                    finish(&frame)?;
                } else {
                    stack.push(frame);
                }
            }
            Event::End(e) => {
                let frame = stack.pop().ok_or("malformed_xml")?;
                if frame.node != e.name().as_ref() {
                    return Err("malformed_xml");
                }
                finish(&frame)?;
            }
            Event::Text(t) => check_text(&stack, &t)?,
            Event::CData(t) => {
                if !stack.last().is_some_and(|f| text_node(&f.node)) {
                    return Err("unexpected_text");
                }
                check_text(&stack, &t)?;
            }
            Event::GeneralRef(reference) => {
                if !stack.last().is_some_and(|f| text_node(&f.node)) {
                    return Err("unexpected_text");
                }
                let escaped = format!("&{};", reference.as_ref());
                let decoded =
                    quick_xml::escape::unescape(&escaped).map_err(|_| "invalid_entity")?;
                check_text(&stack, &decoded)?;
            }
            Event::Decl(decl) => {
                if root_seen || declaration_seen {
                    return Err("invalid_declaration");
                }
                declaration_seen = true;
                if decl.version().map_err(|_| "invalid_declaration")? != "1.0"
                    || decl
                        .encoding()
                        .transpose()
                        .map_err(|_| "invalid_declaration")?
                        .is_some_and(|encoding| !encoding.eq_ignore_ascii_case("UTF-8"))
                {
                    return Err("invalid_declaration");
                }
            }
            Event::DocType(_) => return Err("dtd_forbidden"),
            Event::Comment(_) => {}
            Event::Eof => break,
            _ => return Err("unsupported_xml_event"),
        }
    }
    if !root_seen || !stack.is_empty() {
        return Err("malformed_xml");
    }
    Ok(())
}

fn attributes(
    element: &BytesStart<'_>,
    node: &str,
    root_id: u32,
    ids: &mut HashSet<i32>,
    name: &mut Option<String>,
) -> Result<(), &'static str> {
    let (booleans, integers, texts): (&[&str], &[&str], &[&str]) = match node {
        "COOKED_MISSION" => (
            &[
                "AwardXP",
                "ShowFactionChangeIcon",
                "ShowInstanceIcon",
                "ShowPVPIcon",
                "IsShareable",
                "CanAbandon",
                "CanRepeatOnFail",
                "CanFail",
                "IsHidden",
                "IsOverrideMission",
                "IsEnabled",
                "IsAStory",
            ],
            &["NumRepeats", "Level", "Difficulty", "MissionID"],
            &["MissionLabel", "MissionDefn"],
        ),
        "Steps" => (&["StepEnabled", "AwardXP"], &["StepID", "Difficulty"], &[]),
        "Objectives" => (
            &["IsOptional", "AwardXP", "IsHidden", "IsEnabled"],
            &["ObjectiveID", "Difficulty"],
            &[],
        ),
        "Tasks" => (
            &["AwardXP", "IsEnabled"],
            &["TaskType", "TaskID", "Difficulty"],
            &[],
        ),
        _ => (&[], &[], &[]),
    };
    let mut seen = HashMap::new();
    for attr in element.attributes() {
        let attr = attr.map_err(|_| "invalid_or_duplicate_attribute")?;
        let key = attr.key.as_ref();
        if attr.value.contains('<') {
            return Err("malformed_xml");
        }
        let value = quick_xml::escape::unescape(&attr.value).map_err(|_| "invalid_entity")?;
        if !xml_chars(&value) {
            return Err("invalid_xml_character");
        }
        if node == "COOKED_MISSION" && key.starts_with("xmlns:") {
            let expected = match key {
                "xmlns:SOAP-ENV" => "http://schemas.xmlsoap.org/soap/envelope/",
                "xmlns:SOAP-ENC" => "http://schemas.xmlsoap.org/soap/encoding/",
                "xmlns:xsi" => "http://www.w3.org/2001/XMLSchema-instance",
                "xmlns:xsd" => "http://www.w3.org/2001/XMLSchema",
                "xmlns:CookedData1" => "SGW",
                _ => return Err("unsupported_attribute"),
            };
            if value != expected {
                return Err("invalid_namespace");
            }
        } else if booleans.contains(&key) {
            if !matches!(value.as_ref(), "true" | "false" | "0" | "1") {
                return Err("invalid_boolean");
            }
        } else if integers.contains(&key) {
            let parsed = value.parse::<i32>().map_err(|_| "invalid_integer")?;
            if matches!(key, "MissionID" | "StepID" | "ObjectiveID" | "TaskID") {
                if parsed <= 0 {
                    return Err("nonpositive_id");
                }
                if key == "MissionID" && parsed as u32 != root_id {
                    return Err("mission_id_mismatch");
                }
                if !ids.insert(parsed) {
                    return Err("duplicate_nested_id");
                }
            }
        } else if !texts.contains(&key) {
            return Err("unsupported_attribute");
        }
        if key == "MissionDefn" {
            *name = Some(value.to_string());
        }
        seen.insert(key.to_owned(), value.into_owned());
    }
    for required in booleans.iter().chain(integers).chain(texts) {
        if !seen.contains_key(*required) {
            return Err("missing_required_attribute");
        }
    }
    Ok(())
}
