//! The normalised event and the fingerprint records a golden stores.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Bumped when the rendered form changes, so an old golden is refused
/// rather than compared against a different rendering.
pub const FINGERPRINT_SCHEMA: u32 = 1;

/// Which way a message travelled, from the lab client's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    C2s,
    S2c,
}

/// One normalised message. Rendered:
/// `c2s interact target=tag:ArmYourself_FrostBody`
/// `s2c onDialogDisplay@self dialog_id=3995 entity_id=tag:ArmYourself_FrostBody mission_id=0`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub dir: Dir,
    /// The tap's msg_name; `unknown#<method_index>` when the tap says `unknown`.
    pub name: String,
    /// S2C only: the label of `target_entity_id`.
    pub target: Option<String>,
    /// Key arguments in table order, values already rendered.
    pub args: Vec<(String, String)>,
}

impl Event {
    /// One line per event, so a golden diff reads as a list of lines.
    pub fn render(&self) -> String {
        let dir = match self.dir {
            Dir::C2s => "c2s",
            Dir::S2c => "s2c",
        };
        let mut out = format!("{dir} {}", self.name);
        if let Some(target) = &self.target {
            out += &format!("@{target}");
        }
        for (key, value) in &self.args {
            out += &format!(" {key}={value}");
        }
        out
    }
}

/// The resolved variance one row was fingerprinted with (stored in the golden).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variance {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_unordered: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collapse: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore_windows: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub key_args: BTreeMap<String, Vec<String>>,
    pub late_bind_settle_ms: u64,
}

/// One row's fingerprint: its verdicts, transitions, windows and events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowFingerprint {
    pub row: String,
    /// `RowResult::as_str()`: PASS, FAIL, ...
    pub result: String,
    /// `full`, `late:<after>`, `none` or `unavailable` (the packet-tap
    /// body's `coverage`; `none` when the row has no packet-tap attachment).
    pub tap: String,
    /// `dropped <n>`, `replaced`, `not stopped`, `read failed`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tap_faults: Vec<String>,
    /// `<clause id>=<verdict>` for required clauses whose source is not
    /// timing, signoz or human, in spec order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clauses: Vec<String>,
    /// `mission 1360 -> 0`, `step 2113 -> 1`, `objective 3238 -> 1`, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transitions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows_opened: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows_closed: Vec<String>,
    /// Rendered ordered events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<String>,
    /// Rendered `allow_unordered` events, sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unordered: Vec<String>,
    pub variance: Variance,
}

/// A whole run's fingerprint: one record per row of the section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunFingerprint {
    pub schema: u32,
    pub section: String,
    pub rows: Vec<RowFingerprint>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two rendered forms in the contract: a C2S event with no target
    /// and one argument, and an S2C event with a target and three arguments.
    #[test]
    fn events_render_in_one_line() {
        let interact = Event {
            dir: Dir::C2s,
            name: "interact".to_string(),
            target: None,
            args: vec![(
                "target".to_string(),
                "tag:ArmYourself_FrostBody".to_string(),
            )],
        };
        assert_eq!(
            interact.render(),
            "c2s interact target=tag:ArmYourself_FrostBody"
        );

        let dialog = Event {
            dir: Dir::S2c,
            name: "onDialogDisplay".to_string(),
            target: Some("self".to_string()),
            args: vec![
                ("dialog_id".to_string(), "3995".to_string()),
                (
                    "entity_id".to_string(),
                    "tag:ArmYourself_FrostBody".to_string(),
                ),
                ("mission_id".to_string(), "0".to_string()),
            ],
        };
        assert_eq!(
            dialog.render(),
            "s2c onDialogDisplay@self dialog_id=3995 entity_id=tag:ArmYourself_FrostBody mission_id=0"
        );
    }
}
