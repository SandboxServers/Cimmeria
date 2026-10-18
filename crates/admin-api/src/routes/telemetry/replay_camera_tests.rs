//! Camera rows replay in their own shape, with no names and the chunk's
//! server time (PC-17). Driven through `replay_event`, so the routing in
//! `replay.rs` is covered too.

use serde_json::json;

use super::dto::{ClientNativeEvent, TelemetryEvent};
use super::entity_labels::ChunkClock;
use super::replay::replay_event;
use super::replay_camera::{CameraLift, SENT_SEQ_TARGET};
use super::replay_native::ReplayNames;
use super::replay_tests::{capture, claims};

/// The entity the labels name; a camera row must not carry its name.
const ENTITY: i64 = 4242;

fn camera(target: &str, level: &str, fields: serde_json::Value) -> ClientNativeEvent {
    ClientNativeEvent {
        ts_ms: 1_000,
        seq: 1,
        target: target.into(),
        level: level.into(),
        fields: fields.as_object().cloned().unwrap_or_default(),
    }
}

/// Replay one row as a player's, with `clock` as the chunk's clock.
fn replay(e: ClientNativeEvent, clock: Option<ChunkClock>) -> Vec<super::replay_tests::Row> {
    let book = cimmeria_names::book();
    let label = |id: u32| (i64::from(id) == ENTITY).then_some("Jaffa Guard");
    let names = ReplayNames {
        book: &book,
        entity_label: &label,
        client_pid: None,
        clock,
    };
    capture(|| {
        replay_event(
            &claims(Some("player")),
            TelemetryEvent::ClientNative(e),
            &names,
        )
    })
}

/// A camera pose row carries its lifted numbers and the entity ID, and no
/// `entity_name`, even though the labels name that entity.
#[test]
fn camera_row_is_replayed_without_names() {
    let rows = replay(
        camera(
            "client.camera.at_interact",
            "debug",
            json!({
                "method": "interact",
                "entity_id": ENTITY,
                "send_id": 9,
                "zoom": 400.0,
                "yaw_offset_deg": 90.0,
                "camera_pose_zero": false,
                "camera_x": 1.5,
            }),
        ),
        None,
    );
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.target, "client.native");
    assert_eq!(r.level, tracing::Level::INFO);
    assert_eq!(r.fields["client_target"], "client.camera.at_interact");
    assert_eq!(r.fields["zoom"], "400.0");
    assert_eq!(r.fields["send_id"], "9");
    assert_eq!(r.fields["camera_x"], "1.5");
    assert_eq!(r.fields["entity_id"], "4242");
    assert!(!r.fields.contains_key("entity_name"));
}

/// A camera row is INFO whatever level the DLL sent.
#[test]
fn camera_row_stays_info_whatever_the_client_level() {
    let rows = replay(
        camera("client.camera.at_interact", "warn", json!({ "zoom": 1.0 })),
        None,
    );
    assert_eq!(rows[0].level, tracing::Level::INFO);
}

/// A non-camera target keeps the game shape (`method_name`), which proves
/// the camera guard is scoped to `client.camera.`.
#[test]
fn non_camera_target_keeps_the_game_shape() {
    let rows = replay(
        camera("client.net.out", "info", json!({ "method_index": 14 })),
        None,
    );
    assert_eq!(rows[0].fields["method_name"], "onEffectResults");
    assert!(rows[0].fields.contains_key("names_source"));
}

/// The chunk's clock puts the row on the server clock, and without a clock
/// neither clock key is written.
#[test]
fn server_ts_comes_from_the_chunk_clock() {
    let clock = ChunkClock {
        offset_ms: 100,
        recv_ms: 5_100,
    };
    let rows = replay(
        camera("client.camera.at_interact", "info", json!({ "zoom": 1.0 })),
        Some(clock),
    );
    assert_eq!(rows[0].fields["server_ts_ms"], "1100");
    assert_eq!(rows[0].fields["clock_offset_ms"], "100");

    let rows = replay(
        camera("client.camera.at_interact", "info", json!({ "zoom": 1.0 })),
        None,
    );
    assert!(!rows[0].fields.contains_key("server_ts_ms"));
    assert!(!rows[0].fields.contains_key("clock_offset_ms"));
}

/// The `sent_seq` row lifts the Mercury range and carries no pose keys.
#[test]
fn sent_seq_row_lifts_the_range() {
    let rows = replay(
        camera(
            SENT_SEQ_TARGET,
            "info",
            json!({
                "send_id": 9,
                "method": "interact",
                "mercury_seq_first": 100,
                "mercury_seq_last": 101,
                "packets": 2,
            }),
        ),
        None,
    );
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.level, tracing::Level::INFO);
    assert_eq!(r.fields["mercury_seq_first"], "100");
    assert_eq!(r.fields["mercury_seq_last"], "101");
    assert_eq!(r.fields["packets"], "2");
    assert!(!r.fields.contains_key("zoom"));
}

/// A number sent as a string is lifted like a number.
#[test]
fn lift_reads_numeric_strings() {
    let bag = json!({ "zoom": "400" });
    let lift = CameraLift::from_fields(bag.as_object().unwrap());
    assert_eq!(lift.zoom, Some(400.0));
}

/// A string that parses as a non-finite float reads as absent, so no `NaN`
/// or infinity reaches the log attributes.
#[test]
fn lift_drops_non_finite_numbers() {
    let bag = json!({ "zoom": "NaN", "camera_x": "inf", "camera_y": "-inf", "camera_z": "1.5" });
    let lift = CameraLift::from_fields(bag.as_object().unwrap());
    assert_eq!(lift.zoom, None);
    assert_eq!(lift.camera_x, None);
    assert_eq!(lift.camera_y, None);
    assert_eq!(lift.camera_z, Some(1.5));
}
