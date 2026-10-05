//! NT-41: `server_entity_get` / `server_entity_query` pair every id with its
//! name (Rule 6), and leave a name that does not resolve out of the JSON.

use serde_json::{json, Value};

use cimmeria_names::{NameBook, Table};
use cimmeria_services::cell::messages::{LabEntitySnapshot, LabQueryReply};

use super::{shape_entity_get, shape_entity_query};

/// A book naming template 7001 ("Jaffa_Guard_T1", shown as "Jaffa Guard").
fn book() -> NameBook {
    let mut b = NameBook::empty();
    b.insert(Table::Templates, 7001, "Jaffa_Guard_T1");
    b.insert(Table::Texts, 555, "Jaffa Guard");
    b.insert_template_name_id(7001, 555);
    b
}

/// A snapshot as the cell builds it, with the given overrides.
fn snapshot(overrides: Value) -> LabEntitySnapshot {
    let mut v = json!({
        "entity_id": 900, "space_id": 3, "world": "Castle_CellBlock",
        "world_name": "Castle_CellBlock",
        "position": [0.0, 0.0, 0.0], "direction": [0.0, 0.0, 0.0],
        "velocity": [0.0, 0.0, 0.0], "is_on_ground": true, "is_player": false,
        "class_id": 4, "faction": 0, "alignment": 0, "level": 5, "name": null,
        "template_id": 7001, "spawn_id": null, "tag": null, "name_id": null,
        "archetype_id": null, "access_level": 0, "ai_state": "Idle",
        "current_target_id": null, "aoi_radius": 100.0, "state_field": 0,
        "interaction_type_flags": 0, "weapon_holstered": true,
        "has_static_mesh": false, "component_count": 0, "witness_count": 0,
        "health_cur": 10, "health_max": 10
    });
    v.as_object_mut()
        .unwrap()
        .extend(overrides.as_object().unwrap().clone());
    serde_json::from_value(v).unwrap()
}

fn get(snap: LabEntitySnapshot, book: &NameBook) -> Value {
    let out = shape_entity_get(LabQueryReply::Entity { entity: Some(snap) }, book).unwrap();
    out["entity"].clone()
}

#[test]
fn nt41_entity_get_names_an_npc_from_the_namebook() {
    let e = get(snapshot(json!({})), &book());
    assert_eq!(e["entity_id"], 900);
    assert_eq!(e["entity_name"], "Jaffa Guard", "template display text");
    assert_eq!(e["template_id"], 7001);
    assert_eq!(e["template_name"], "Jaffa_Guard_T1");
    assert_eq!(e["space_id"], 3);
    assert_eq!(e["world"], "Castle_CellBlock");
}

#[test]
fn nt41_entity_get_keeps_the_cells_live_name_and_target_name() {
    let e = get(
        snapshot(json!({
            "entity_name": "Captain Ka'lel", "name": "Captain Ka'lel",
            "current_target_id": 100, "current_target_name": "Tealc"
        })),
        &book(),
    );
    assert_eq!(e["entity_name"], "Captain Ka'lel", "npc_name wins");
    assert_eq!(e["current_target_id"], 100);
    assert_eq!(e["current_target_name"], "Tealc");
}

#[test]
fn nt41_entity_get_names_a_player_archetype() {
    let e = get(
        snapshot(json!({
            "entity_id": 100, "is_player": true, "class_id": 2,
            "entity_name": "Tealc", "template_id": null, "archetype_id": 1
        })),
        &book(),
    );
    assert_eq!(e["entity_name"], "Tealc");
    assert_eq!(e["archetype_id"], 1);
    assert_eq!(
        e["archetype_name"],
        cimmeria_names::archetype_name(1).unwrap()
    );
    assert!(e.get("template_name").is_none(), "a player has no template");
}

/// A template the book does not name, and a space with no world: every name
/// key is absent, never `null`, `""` or `"unknown"`.
#[test]
fn nt41_entity_get_omits_names_that_do_not_resolve() {
    let mut snap = snapshot(json!({ "template_id": 4242, "archetype_id": 99 }));
    snap.world = None;
    let e = get(snap, &book());
    assert_eq!(e["template_id"], 4242);
    for key in [
        "entity_name",
        "template_name",
        "archetype_name",
        "world",
        "current_target_name",
    ] {
        assert!(e.get(key).is_none(), "{key} must be absent, got {e}");
    }
}

#[test]
fn nt41_entity_query_names_every_snapshot() {
    let out = shape_entity_query(
        LabQueryReply::Entities {
            entities: vec![
                snapshot(json!({})),
                snapshot(json!({ "entity_id": 901, "template_id": 4242 })),
            ],
            total_matched: 2,
            capped: false,
        },
        &book(),
    )
    .unwrap();
    assert_eq!(out["returned"], 2);
    let named = &out["entities"][0];
    assert_eq!(named["entity_name"], "Jaffa Guard");
    assert_eq!(named["template_name"], "Jaffa_Guard_T1");
    let unnamed = &out["entities"][1];
    assert_eq!(unnamed["entity_id"], 901);
    assert!(unnamed.get("entity_name").is_none());
    assert!(unnamed.get("template_name").is_none());
}
