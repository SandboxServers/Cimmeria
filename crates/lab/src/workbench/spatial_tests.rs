//! Regression guards for world-qualified joins and local bookmark storage.

use serde_json::json;

use super::{
    bookmarks::{action_allowed, BookmarkDraft, BookmarkStore},
    spatial::{
        assemble_spatial_frame, current_frame, definition_query, definition_rows, fresh_frame,
        join_entity, observed_frame_consistent, SpatialEntity, SpatialFrameReads,
    },
};

#[test]
fn production_spatial_assembly_never_promotes_same_world_relog_numbers_to_current() {
    use crate::supervisor::combat::player::PlayerState;

    let player = PlayerState {
        world_id: Some(7),
        world: Some("same-world".into()),
        pos: Some([1.0, 2.0, 3.0]),
        ..PlayerState::default()
    };
    let frame = assemble_spatial_frame(SpatialFrameReads {
        instance: "default".into(),
        started_ms: 900,
        frame_at_ms: 1_000,
        now_ms: 1_100,
        before_pid: Some(100),
        after_pid: Some(100),
        before: Ok(player.clone()),
        after: Ok(player),
        found: Ok(
            json!({"player":{"id":42},"matches":[{"id":42,"position":{"server":[1.0,2.0,3.0]}}]}),
        ),
        after_player_id: Some(42),
        camera: Some(json!({"position":[1.0,2.0,3.0]})),
        regions: Ok(json!({"rows":[]})),
        spawns: Ok(json!({"rows":[]})),
    });
    assert_eq!(frame["context"]["observed_consistent"], true);
    assert_eq!(frame["context"]["current"], false);
    assert_eq!(frame["context"]["identity_verified"], false);
    assert_eq!(
        frame["data"]["client_entities"]["value"]["player"]["id"],
        42
    );
    assert_eq!(frame["data"]["server_entities"], serde_json::Value::Null);
    assert!(frame["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|gap| gap["code"] == "client_identity_unbound"));
}

#[test]
fn production_spatial_assembly_classifies_db_and_projection_without_raw_errors() {
    use crate::supervisor::combat::player::PlayerState;

    let player = PlayerState {
        world_id: Some(7),
        world: Some("same-world".into()),
        ..PlayerState::default()
    };
    let frame = assemble_spatial_frame(SpatialFrameReads {
        instance: "default".into(),
        started_ms: 900,
        frame_at_ms: 1_000,
        now_ms: 1_100,
        before_pid: Some(100),
        after_pid: Some(100),
        before: Ok(player.clone()),
        after: Ok(player),
        found: Ok(json!({"player":{"id":42},"projection_error":"private-marker"})),
        after_player_id: Some(42),
        camera: None,
        regions: Err("db_query_failed"),
        spawns: Err("db_result_truncated"),
    });
    let gaps = frame["gaps"].as_array().unwrap();
    assert!(gaps
        .iter()
        .any(|gap| gap["source"] == "client_entity_find.projection"
            && gap["category"] == "projection_failed"));
    assert!(gaps
        .iter()
        .any(|gap| gap["source"] == "server_db_query.regions"
            && gap["category"] == "db_query_failed"));
    assert!(gaps
        .iter()
        .any(|gap| gap["source"] == "server_db_query.spawns"
            && gap["category"] == "db_result_truncated"));
    assert_eq!(
        frame["data"]["client_entities"]["value"]["projection_error"],
        "unavailable"
    );
    assert!(!frame.to_string().contains("private-marker"));
}

fn entity(world_id: i64, space_id: u64, at_ms: i64) -> SpatialEntity {
    SpatialEntity {
        instance: "default".into(),
        world_id,
        space_id: Some(space_id),
        entity_id: 42,
        at_ms,
        value: json!({"position":[1,2,3]}),
    }
}

#[test]
fn same_entity_number_in_another_world_or_space_never_joins() {
    let client = entity(7, 101, 1000);
    assert!(join_entity(&client, &entity(8, 101, 1000), 1100).is_none());
    assert!(join_entity(&client, &entity(7, 102, 1000), 1100).is_none());
    assert!(join_entity(&client, &entity(7, 101, 1000), 4000).is_none());
    let mut other_instance = entity(7, 101, 1000);
    other_instance.instance = "p2".into();
    assert!(join_entity(&client, &other_instance, 1100).is_none());
    assert!(join_entity(&client, &entity(7, 101, 1000), 1100).is_some());
}

#[test]
fn frame_expires_from_first_read_and_rejects_future_clock() {
    assert!(fresh_frame(1000, 3000));
    assert!(!fresh_frame(1000, 3001));
    assert!(!fresh_frame(3001, 3000));
}

#[test]
fn same_world_relog_or_process_change_cannot_make_a_current_frame() {
    assert!(!observed_frame_consistent(
        Some(10),
        Some(11),
        Some(42),
        Some(42)
    ));
    assert!(!observed_frame_consistent(
        Some(10),
        Some(10),
        Some(42),
        Some(43)
    ));
    assert!(!observed_frame_consistent(None, None, Some(42), Some(42)));
    // Equal PID and player number can recur after a relog. Without a trusted
    // login generation this is an observation, not a current joined frame.
    assert!(observed_frame_consistent(
        Some(10),
        Some(10),
        Some(42),
        Some(42)
    ));
    assert!(!current_frame(true, false));
    assert!(current_frame(true, true));
}

#[test]
fn authored_regions_keep_all_corners_flags_and_metre_dimensions() {
    let sql = definition_query(7, "regions").unwrap();
    for column in [
        "flags", "x1", "y1", "z1", "x2", "y2", "z2", "x3", "y3", "z3", "x4", "y4", "z4", "radius",
        "height",
    ] {
        assert!(sql.contains(column), "missing {column}");
    }
    let row = json!({"region_id":5,"flags":3,"x1":1.0,"y1":2.0,"z1":3.0,"x2":4.0,"y2":5.0,"z2":6.0,"x3":7.0,"y3":8.0,"z3":9.0,"x4":10.0,"y4":11.0,"z4":12.0,"radius":13.0,"height":14.0,"world_id":7});
    let rows = definition_rows(
        &json!({"rows":[row.clone()],"truncated":false}),
        7,
        "regions",
    )
    .unwrap();
    assert_eq!(rows["rows"][0], row);
    assert_eq!(rows["position_unit"], "BigWorld m");
    assert_eq!(rows["radius_unit"], "m");
    assert_eq!(rows["height_unit"], "m");
    assert_eq!(rows["heading_unit"], serde_json::Value::Null);
}

#[test]
fn spawn_heading_is_radians_and_not_a_metre_dimension() {
    let row = json!({"spawn_id":9,"x":1.0,"y":2.0,"z":3.0,"heading":1.57,"world_id":7});
    let rows = definition_rows(&json!({"rows":[row.clone()]}), 7, "spawns").unwrap();
    assert!(definition_query(7, "spawns").unwrap().contains("heading"));
    assert_eq!(rows["rows"][0], row);
    assert_eq!(rows["position_unit"], "BigWorld m");
    assert_eq!(rows["heading_unit"], "rad");
    assert_eq!(rows["radius_unit"], serde_json::Value::Null);
}

#[test]
fn bookmark_round_trip_update_delete_and_cross_world_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let store = BookmarkStore::new(dir.path().to_path_buf());
    let draft = BookmarkDraft {
        world_id: 7,
        space_id: Some(101),
        position_m: [1.0, 2.0, 3.0],
        facing_deg: Some(90.0),
        label: "Gate pad".into(),
        source: "client_actor".into(),
        screenshot_path: Some("captures/pad.png".into()),
        observed_at_ms: 1000,
    };
    let saved = store.save("default", None, draft.clone()).unwrap();
    let reopened = BookmarkStore::new(dir.path().to_path_buf());
    assert_eq!(reopened.list("default").unwrap(), vec![saved.clone()]);
    assert_eq!(reopened.list("p2").unwrap(), Vec::new());
    assert_eq!(
        action_allowed(&saved, "default", Some(8), Some(101)),
        Err("cross_world_or_unknown")
    );
    assert_eq!(
        action_allowed(&saved, "default", Some(7), None),
        Err("cross_space_or_unknown")
    );
    assert_eq!(
        action_allowed(&saved, "p2", Some(7), Some(101)),
        Err("instance_mismatch")
    );
    assert!(action_allowed(&saved, "default", Some(7), Some(101)).is_ok());
    let updated = reopened
        .save(
            "default",
            Some(saved.id.clone()),
            BookmarkDraft {
                label: "Gate".into(),
                ..draft
            },
        )
        .unwrap();
    assert_eq!(updated.id, saved.id);
    assert_eq!(reopened.list("default").unwrap()[0].label, "Gate");
    reopened.delete("default", &saved.id).unwrap();
    assert!(reopened.list("default").unwrap().is_empty());
}
