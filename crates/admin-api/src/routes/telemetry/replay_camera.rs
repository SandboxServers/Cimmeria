//! Replaying the DLL's camera rows (`client.camera.*`) in their own shape
//! (PC-17, D-PC14).
//!
//! The game shape ([`super::replay_native`]) names an `entity_id` with an
//! `entity_name`, which is a character name, and it sits at `tracing`'s
//! 32-field cap. A camera row needs neither: it carries the numbers lifted
//! out of the DLL's `fields` bag, the entity ID with no name, and the row's
//! time on the server clock ([`super::entity_labels::ChunkClock`]), so it
//! lines up with the pose row PC-26 writes.
//!
//! Two shapes, by target: a camera pose row (every `client.camera.*` target
//! but one) and the `client.camera.sent_seq` row, which maps a camera send
//! to the Mercury sequence range of the packets that carried it. Both are
//! always INFO, whatever level the DLL sent.

use serde_json::{Map, Value};

use crate::routes::dev_session::TokenClaims;

use super::dto::ClientNativeEvent;
use super::replay_native::ReplayNames;

/// Every camera row's target starts with this.
pub(super) const CAMERA_TARGET_PREFIX: &str = "client.camera.";

/// The send-to-sequence row; every other camera target is a pose row.
pub(super) const SENT_SEQ_TARGET: &str = "client.camera.sent_seq";

/// Whether DLL event `target` is a camera row, replayed in its own shape.
pub(super) fn is_camera_target(target: &str) -> bool {
    target.starts_with(CAMERA_TARGET_PREFIX)
}

/// The camera keys lifted out of a row's `fields` bag. A key that is absent,
/// of the wrong type, or not a number (a numeric string is one) is `None`.
#[derive(Debug, Default, Clone, PartialEq)]
pub(super) struct CameraLift {
    /// The camera method that produced the row (`interact`, `useItem`).
    pub method: Option<String>,
    /// The entity the camera was for. Lifted as an ID only; never named.
    pub entity_id: Option<i64>,
    /// The DLL's send counter, which joins a pose row to its `sent_seq` row.
    pub send_id: Option<u32>,
    pub zoom: Option<f64>,
    pub yaw_offset_deg: Option<f64>,
    pub pitch_offset_deg: Option<f64>,
    pub camera_flags: Option<u32>,
    pub camera_gain: Option<f64>,
    pub camera_pose_zero: Option<bool>,
    pub camera_x: Option<f64>,
    pub camera_y: Option<f64>,
    pub camera_z: Option<f64>,
    pub camera_yaw_deg: Option<f64>,
    pub pawn_x: Option<f64>,
    pub pawn_y: Option<f64>,
    pub pawn_z: Option<f64>,
    pub pawn_yaw_deg: Option<f64>,
    pub pawn_pitch_deg: Option<f64>,
    /// The first Mercury sequence of the packets that carried a send.
    pub mercury_seq_first: Option<u32>,
    /// The last Mercury sequence of those packets.
    pub mercury_seq_last: Option<u32>,
    /// How many packets carried the send.
    pub packets: Option<u32>,
}

/// A JSON number, or a string holding one.
fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// A whole JSON number, or a string holding one.
fn whole(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// A JSON boolean, or a string holding one.
fn flag(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

impl CameraLift {
    /// Lift every camera key the bag carries; the rest stay `None`.
    pub(super) fn from_fields(f: &Map<String, Value>) -> Self {
        let num = |k: &str| f.get(k).and_then(number);
        let int = |k: &str| f.get(k).and_then(whole);
        let uint = |k: &str| int(k).and_then(|v| u32::try_from(v).ok());
        Self {
            method: f.get("method").and_then(Value::as_str).map(str::to_string),
            entity_id: int("entity_id"),
            send_id: uint("send_id"),
            zoom: num("zoom"),
            yaw_offset_deg: num("yaw_offset_deg"),
            pitch_offset_deg: num("pitch_offset_deg"),
            camera_flags: uint("camera_flags"),
            camera_gain: num("camera_gain"),
            camera_pose_zero: f.get("camera_pose_zero").and_then(flag),
            camera_x: num("camera_x"),
            camera_y: num("camera_y"),
            camera_z: num("camera_z"),
            camera_yaw_deg: num("camera_yaw_deg"),
            pawn_x: num("pawn_x"),
            pawn_y: num("pawn_y"),
            pawn_z: num("pawn_z"),
            pawn_yaw_deg: num("pawn_yaw_deg"),
            pawn_pitch_deg: num("pawn_pitch_deg"),
            mercury_seq_first: uint("mercury_seq_first"),
            mercury_seq_last: uint("mercury_seq_last"),
            packets: uint("packets"),
        }
    }
}

/// Replay one camera row through `tracing`, in the camera shape for its
/// target. `server_ts_ms` and `clock_offset_ms` come from the chunk's clock
/// and are absent when the chunk had none.
pub(super) fn replay_camera(claims: &TokenClaims, e: ClientNativeEvent, names: &ReplayNames<'_>) {
    let l = CameraLift::from_fields(&e.fields);
    let server_ts_ms = names.clock.map(|c| c.server_ts_ms(e.ts_ms));
    let clock_offset_ms = names.clock.map(|c| c.offset_ms);
    let kind = claims.session_kind();
    let lab = claims.is_lab();
    let client_target = e.target.as_str();
    let client_pid = names.client_pid;
    let fields = Value::Object(e.fields);

    if client_target == SENT_SEQ_TARGET {
        tracing::info!(
            target: "client.native",
            session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
            install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
            cimmeria.session_kind = kind,
            lab,
            ts_ms = e.ts_ms,
            seq = e.seq,
            client_pid = client_pid,
            client_target = client_target,
            server_ts_ms = server_ts_ms,
            clock_offset_ms = clock_offset_ms,
            method = l.method.as_deref(),
            send_id = l.send_id, // nt:id-only DLL send counter, names nothing
            mercury_seq_first = l.mercury_seq_first,
            mercury_seq_last = l.mercury_seq_last,
            packets = l.packets,
            fields = %fields,
            "{client_target}"
        );
    } else {
        // No character name rides on a camera row (D-PC2): the entity is
        // its ID, and the names the game shape would add are left out.
        tracing::info!(
            target: "client.native",
            session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
            install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
            cimmeria.session_kind = kind,
            lab,
            ts_ms = e.ts_ms,
            seq = e.seq,
            client_pid = client_pid,
            client_target = client_target,
            server_ts_ms = server_ts_ms,
            clock_offset_ms = clock_offset_ms,
            method = l.method.as_deref(),
            entity_id = l.entity_id, // nt:id-only privacy: no character name on camera rows (D-PC2, D-PC14)
            send_id = l.send_id, // nt:id-only DLL send counter, names nothing
            zoom = l.zoom,
            yaw_offset_deg = l.yaw_offset_deg,
            pitch_offset_deg = l.pitch_offset_deg,
            camera_flags = l.camera_flags,
            camera_gain = l.camera_gain,
            camera_pose_zero = l.camera_pose_zero,
            camera_x = l.camera_x,
            camera_y = l.camera_y,
            camera_z = l.camera_z,
            camera_yaw_deg = l.camera_yaw_deg,
            pawn_x = l.pawn_x,
            pawn_y = l.pawn_y,
            pawn_z = l.pawn_z,
            pawn_yaw_deg = l.pawn_yaw_deg,
            pawn_pitch_deg = l.pawn_pitch_deg,
            fields = %fields,
            "{client_target}"
        );
    }
}
