//! NT-41: `server_witnesses` pairs the entity and every listed witness with
//! its names (Rule 6), and leaves a name that does not resolve out.

use serde_json::json;

use cimmeria_names::{NameBook, Table};
use cimmeria_services::cell::messages::{
    LabEntityNames, LabEntityRef, LabQueryReply, LabWitnessReport,
};

use super::shape;

fn book() -> NameBook {
    let mut b = NameBook::empty();
    b.insert(Table::Templates, 7001, "Jaffa_Guard_T1");
    b.insert(Table::Texts, 555, "Jaffa Guard");
    b.insert_template_name_id(7001, 555);
    b
}

fn npc(entity_id: u32, template_id: i32) -> LabEntityRef {
    LabEntityRef {
        entity_id,
        names: LabEntityNames {
            template_id: Some(template_id),
            ..Default::default()
        },
    }
}

#[test]
fn nt41_witnesses_name_the_entity_and_every_listed_one() {
    let report = LabWitnessReport {
        entity_id: 100,
        names: LabEntityNames {
            entity_name: Some("Tealc".to_string()),
            ..Default::default()
        },
        space_id: 3,
        world: Some("Castle_CellBlock".to_string()),
        witnessed_by: vec![],
        // 900: named by the NameBook. 901: an unnamed template.
        witnesses: vec![npc(900, 7001), npc(901, 4242)],
    };
    let out = shape(LabQueryReply::Witnesses { report }, &book()).unwrap();

    assert_eq!(out["entity_id"], 100);
    assert_eq!(out["entity_name"], "Tealc");
    assert!(out.get("template_id").is_none(), "a player has no template");
    assert_eq!(out["space_id"], 3);
    assert_eq!(out["world"], "Castle_CellBlock");
    assert_eq!(out["witnesses_count"], 2);
    assert_eq!(
        out["witnesses"][0],
        json!({
            "entity_id": 900,
            "entity_name": "Jaffa Guard",
            "template_id": 7001,
            "template_name": "Jaffa_Guard_T1"
        })
    );
    assert_eq!(
        out["witnesses"][1],
        json!({ "entity_id": 901, "template_id": 4242 }),
        "unresolved names are left out"
    );
}

#[test]
fn nt41_witnesses_omit_an_unresolved_entity_name_and_world() {
    let report = LabWitnessReport {
        entity_id: 901,
        names: LabEntityNames {
            template_id: Some(4242),
            ..Default::default()
        },
        space_id: 3,
        world: None,
        witnessed_by: vec![LabEntityRef {
            entity_id: 100,
            names: LabEntityNames {
                entity_name: Some("Tealc".to_string()),
                ..Default::default()
            },
        }],
        witnesses: vec![],
    };
    let out = shape(LabQueryReply::Witnesses { report }, &book()).unwrap();
    for key in ["entity_name", "template_name", "world"] {
        assert!(out.get(key).is_none(), "{key} must be absent, got {out}");
    }
    assert_eq!(out["template_id"], 4242);
    assert_eq!(
        out["witnessed_by"],
        json!([{ "entity_id": 100, "entity_name": "Tealc" }])
    );
}
