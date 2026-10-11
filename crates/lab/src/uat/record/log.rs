//! The raw recording log: one JSON object per line, a `start`, then one
//! `tick` per [`TICK_MS`], then a `stop`. Absent optional fields are left out.

use serde::{Deserialize, Serialize};

/// The log format version written by `start` and required by [`read_log`].
pub const RECORD_SCHEMA: u32 = 1;
/// The sampling period of the ticks, in milliseconds.
pub const TICK_MS: i64 = 500;

/// One line of the log, tagged by its `kind` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
// A tick is the common line and is large; boxing it would change the shape
// the later packets build against.
#[allow(clippy::large_enum_variant)]
pub enum RecordLine {
    Start(RecordStart),
    Tick(Tick),
    Stop(RecordStop),
}

/// The first line: who is recording what, and when.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordStart {
    pub schema: u32,
    /// For example `"20261012-143000-p2"`.
    pub recording_id: String,
    /// The section the person says they are playing.
    pub section: String,
    /// `"default"`, `"p2"`, ...
    pub instance: String,
    /// The character being played.
    pub character: String,
    /// `sgw_player.player_id`.
    pub player_id: i32,
    pub player_entity: u32,
    pub host_ms: i64,
    /// The local date at start, for example `"2026-10-12"`.
    pub date: String,
    /// The rolling window length in minutes (LR-14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rolling_min: Option<u32>,
}

/// One sample, taken every [`TICK_MS`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Tick {
    pub seq: u32,
    pub host_ms: i64,
    /// Drained tap rows, verbatim from `server_packet_tap_read` `messages`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tap: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub tap_dropped: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<CameraSample>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<EntitySample>,
    /// Visible top-level windows, sorted; `None` when the UI read failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialog: Option<DialogSample>,
    /// Entities named by an inbound call for the first time this tick.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<EntitySample>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<DbSnapshot>,
    /// `KNOWN_SERVER_EVENTS` keys.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub server_events: Vec<String>,
    /// UI clicks made during the tick (LR-10).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clicks: Vec<UiClick>,
    /// At most 4, each cut to 120 characters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// The camera readout and the camera actor's pose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraSample {
    /// The readout `pitch_offset_deg`.
    pub pitch_deg: f64,
    /// The readout `zoom`.
    pub zoom: f64,
    /// The readout gain, negated when the yaw-invert flag is set.
    pub yaw_gain: f64,
    /// The camera actor's world yaw (`pose.yaw_deg`).
    pub yaw_deg: f64,
    /// The camera actor's server position (`pose.server`).
    pub pos: [f64; 3],
}

/// An entity's position and identity, as the client or server knows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntitySample {
    pub entity_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Server metres.
    pub pos: [f64; 3],
    pub space_id: u32,
}

/// The dialog window's title and button labels.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DialogSample {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buttons: Vec<String>,
}

/// A database read of the player's state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DbSnapshot {
    pub missions: Vec<SnapMission>,
    pub player: Option<SnapPlayer>,
    pub items: Vec<SnapItem>,
}

/// One mission row of a [`DbSnapshot`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapMission {
    pub mission_id: i32,
    pub status: i32,
    pub step: Option<i32>,
    pub repeats: i32,
}

/// The player row of a [`DbSnapshot`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapPlayer {
    pub level: i32,
    pub exp: i64,
    pub naquadah: i64,
    pub world_location: String,
}

/// One inventory stack of a [`DbSnapshot`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapItem {
    pub type_id: i32,
    pub container_id: i32,
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// A UI button press (LR-10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiClick {
    pub window: String,
    pub button: String,
}

/// The last line: why the recording stopped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordStop {
    pub host_ms: i64,
    pub ticks: u32,
    /// `stop`, `max_minutes`, `size_cap`, `lease_lost` or `bug_spec:<id>`.
    pub reason: String,
}

/// A parsed log. `stop` is `None` for a crashed recording.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordLog {
    pub start: RecordStart,
    pub ticks: Vec<Tick>,
    pub stop: Option<RecordStop>,
}

impl RecordLog {
    /// False for a crashed recording: the log has no `stop` line.
    pub fn has_stop(&self) -> bool {
        self.stop.is_some()
    }
}

/// Parse a whole log. Errors name the 1-based line. The first line must be
/// `start` with `schema == RECORD_SCHEMA`; a missing `stop` is allowed (a
/// crashed recording) and reported by `has_stop`.
pub fn read_log(text: &str) -> Result<RecordLog, String> {
    let mut start: Option<RecordStart> = None;
    let mut ticks = Vec::new();
    let mut stop = None;
    for (i, raw) in text.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let n = i + 1;
        let line: RecordLine = serde_json::from_str(raw).map_err(|e| format!("line {n}: {e}"))?;
        if stop.is_some() {
            return Err(format!("line {n}: a line after stop"));
        }
        match (start.is_some(), line) {
            (false, RecordLine::Start(s)) => {
                if s.schema != RECORD_SCHEMA {
                    return Err(format!(
                        "line {n}: schema {} is not {RECORD_SCHEMA}",
                        s.schema
                    ));
                }
                start = Some(s);
            }
            (false, _) => return Err(format!("line {n}: the first line must be start")),
            (true, RecordLine::Start(_)) => return Err(format!("line {n}: a second start")),
            (true, RecordLine::Tick(t)) => ticks.push(t),
            (true, RecordLine::Stop(s)) => stop = Some(s),
        }
    }
    let start = start.ok_or_else(|| "line 1: the log is empty, expected start".to_string())?;
    Ok(RecordLog { start, ticks, stop })
}

/// One line, `\n`-terminated.
pub fn write_line(line: &RecordLine) -> String {
    let mut text = serde_json::to_string(line).expect("a record line always serializes");
    text.push('\n');
    text
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("fixtures/fs_p3_record.jsonl");

    fn start(schema: u32) -> RecordStart {
        RecordStart {
            schema,
            recording_id: "20261012-143000-p2".into(),
            section: "first-session".into(),
            instance: "p2".into(),
            character: "Pxfixture".into(),
            player_id: 4242,
            player_entity: 8,
            host_ms: 0,
            date: "2026-10-12".into(),
            rolling_min: None,
        }
    }

    /// The FS-P3 capture reads back with its 42 ticks, the `interact` row and
    /// the Frost entity on tick 32, and its stop line. Fails if the fixture
    /// or the parser drifts from the contract.
    #[test]
    fn fs_p3_fixture_reads_with_42_ticks_and_a_stop() {
        let log = read_log(FIXTURE).unwrap();
        assert_eq!(log.ticks.len(), 42);
        assert!(log
            .ticks
            .windows(2)
            .all(|w| w[1].host_ms - w[0].host_ms == TICK_MS));
        assert_eq!(log.stop.as_ref().map(|s| s.reason.as_str()), Some("stop"));
        let t32 = &log.ticks[32];
        assert!(t32.tap.iter().any(|r| r["msg_name"] == "interact"));
        assert_eq!(
            t32.entities[0].tag.as_deref(),
            Some("ArmYourself_FrostBody")
        );
    }

    /// Absent optional fields are left out of the line. Fails if a field
    /// loses its `skip_serializing_if`.
    #[test]
    fn a_line_round_trips_and_leaves_absent_fields_out() {
        let line = RecordLine::Tick(Tick {
            seq: 3,
            host_ms: 10,
            ..Default::default()
        });
        let text = write_line(&line);
        assert_eq!(text, "{\"kind\":\"tick\",\"seq\":3,\"host_ms\":10}\n");
        let back: RecordLine = serde_json::from_str(text.trim_end()).unwrap();
        assert_eq!(back, line);
    }

    /// A log that does not open with `start`, or whose schema is not
    /// `RECORD_SCHEMA`, is rejected with the line named.
    #[test]
    fn a_bad_first_line_or_schema_is_an_error_naming_the_line() {
        let tick = write_line(&RecordLine::Tick(Tick::default()));
        let err = read_log(&tick).unwrap_err();
        assert!(err.starts_with("line 1"), "{err}");

        let bad = write_line(&RecordLine::Start(start(2)));
        let err = read_log(&bad).unwrap_err();
        assert!(err.starts_with("line 1"), "{err}");
    }

    /// Dropping the stop line (a crashed recording) still reads; `stop` is
    /// `None` and `has_stop` is false.
    #[test]
    fn a_log_without_stop_still_reads() {
        let cut = FIXTURE.trim_end().rsplit_once('\n').unwrap().0;
        let log = read_log(cut).unwrap();
        assert!(log.stop.is_none());
        assert!(!log.has_stop());
        assert_eq!(log.ticks.len(), 42);
    }
}
