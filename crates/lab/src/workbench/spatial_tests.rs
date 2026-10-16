//! Regression guards for world-qualified joins and local bookmark storage.

use serde_json::json;

use super::{
    bookmarks::{action_allowed, BookmarkDraft, BookmarkStore},
    spatial::{fresh_frame, join_entity, SpatialEntity},
};

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
