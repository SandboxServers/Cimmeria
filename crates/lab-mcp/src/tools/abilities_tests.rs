//! AB-L1: the `server_ability_state` shaping and the serde contract of the
//! `LabQueryReply` it reads (`kind = "ability_state"`), plus the
//! `LabEntitySnapshot` fields AB-L1 added to `server_entity_get`.

use serde_json::{json, Value};

use cimmeria_services::cell::messages::{LabQueryReply, LabWitnessReport};

use super::shape;

/// A full snapshot as the cell serialises it: every AB-T5 field present.
fn snapshot_json() -> Value {
    json!({
        "entity_id": 7,
        "is_player": true,
        "state_field": 64,
        "pending_cast": {
            "ability_id": 597, "cast_id": 44, "target_id": 9, "wire_target_id": 0,
            "warmup_secs": 2.0, "warmup_remaining_secs": 1.5
        },
        "cooldowns": [{ "ability_id": 592, "remaining_secs": 29.5, "total_secs": 30.0 }],
        "moniker_cooldowns": [{ "moniker_id": 3212632871_i64, "remaining_secs": 9.5, "total_secs": 10.0 }],
        "pulsing": [{
            "effect_id": 5001, "ability_id": 800, "invoker_id": 9, "cast_id": 12,
            "pulses_left": 3, "total_pulses": 5, "pulse_interval_secs": 2.0,
            "next_pulse_in_secs": 0.5
        }],
        "ledger": [{
            "effect_id": 4306, "ability_id": 1013, "invoker_id": 7, "cast_id": 41,
            "stats": [{ "stat_id": 11, "requested": 200, "cur_shift": 200, "min_shift": 0, "max_shift": 200 }],
            "duration_secs": 30.0, "expires_in_secs": 12.25, "held": false,
            "moniker_ids": [], "effect_flags": 4, "state_flags": 0,
            "absorb": [{ "stat_id": 89, "granted": 500, "remaining": 300 }],
            "timer_sent": true
        }],
        "pending_timer_clears": [[700, 7]],
        "state_flag_refcounts": [{ "bit": 6, "mask": 64, "count": 1, "set": true }],
        "stats": [{ "stat_id": 7, "cur": 480, "min": 0, "max": 500 }]
    })
}

/// The reply round-trips through the snapshot's types unchanged: a field the
/// cell sends is a field the tool returns.
#[test]
fn ab_l1_ability_state_reply_round_trips_into_the_tool_json() {
    let wire = json!({ "kind": "ability_state", "state": snapshot_json() });
    let reply: LabQueryReply = serde_json::from_value(wire).expect("the contract deserialises");
    let out = shape(reply.clone()).expect("the matching variant shapes");
    assert_eq!(out, json!({ "state": snapshot_json() }));
    // And back out the way the cell sends it.
    let again = serde_json::to_value(&reply).unwrap();
    assert_eq!(again["kind"], "ability_state");
    assert_eq!(again["state"], snapshot_json());
}

#[test]
fn ab_l1_an_unknown_entity_is_a_null_state_not_an_error() {
    let reply: LabQueryReply =
        serde_json::from_value(json!({ "kind": "ability_state", "state": null })).unwrap();
    assert_eq!(shape(reply).unwrap(), json!({ "state": null }));
}

#[test]
fn ab_l1_a_mismatched_reply_is_a_tool_error() {
    let reply = LabQueryReply::Witnesses {
        report: LabWitnessReport {
            entity_id: 1,
            names: Default::default(),
            space_id: 1,
            world: None,
            witnessed_by: vec![],
            witnesses: vec![],
        },
    };
    assert!(shape(reply).unwrap_err().contains("unexpected"));
}

/// `focus_*` and `stats` are `#[serde(default)]`: a snapshot from a cell that
/// predates AB-L1 still reads, with no focus and no stats.
#[test]
fn ab_l1_entity_snapshot_reads_without_the_new_fields() {
    let old = json!({
        "kind": "entity",
        "entity": {
            "entity_id": 1, "space_id": 1, "world_name": "Agnos",
            "position": [0.0, 0.0, 0.0], "direction": [0.0, 0.0, 0.0],
            "velocity": [0.0, 0.0, 0.0], "is_on_ground": true, "is_player": true,
            "class_id": 2, "faction": 0, "alignment": 0, "level": 1, "name": null,
            "template_id": null, "spawn_id": null, "tag": null, "name_id": null,
            "archetype_id": null, "access_level": 0, "ai_state": "Idle",
            "current_target_id": null, "aoi_radius": 100.0, "state_field": 0,
            "interaction_type_flags": 0, "weapon_holstered": true,
            "has_static_mesh": false, "component_count": 0, "witness_count": 0,
            "health_cur": 10, "health_max": 10
        }
    });
    let LabQueryReply::Entity { entity } = serde_json::from_value(old).unwrap() else {
        panic!("expected Entity");
    };
    let e = entity.unwrap();
    assert_eq!((e.focus_cur, e.focus_max), (None, None));
    assert!(e.stats.is_empty());
}
