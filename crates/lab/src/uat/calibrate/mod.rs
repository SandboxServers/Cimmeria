//! Calibration core (lab-spec-vocab SV-09): find a stand-off and camera pose
//! that hover-verifies its target, and write it into the spec for review.
//!
//! - **Spec edits (D-SV2).** A pose is one single-line inline table, so a
//!   calibrated pose replaces exactly that line and nothing else. The whole
//!   file is re-parsed to prove it, and the change is shown as a line diff
//!   built here, with no new dependency.
//! - **Scoring (D-SV6).** A candidate is judged by hover only, never a click:
//!   eight jitter points around the aim pixel must agree, and margin from the
//!   avatar and the screen edge breaks ties. The winner is confirmed from a
//!   fresh teleport before it is proposed.
//!
//! Nothing in this module clicks or commits. [`drive`] (SV-10,
//! `lab_uat_calibrate`) moves the camera, stands and hovers; a person
//! reviews the diff it writes into the run folder.

mod candidate;
pub mod drive;
pub mod edit;
pub mod grid;
pub mod score;

use serde_json::Value;

/// One pose's calibration result, as `lab calibrate -Json` reports it (D-SV12).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CalibrateOutcome {
    /// A pose was found and confirmed.
    pub ok: bool,
    /// The section id the pose belongs to.
    pub section: String,
    /// The row id the pose belongs to.
    pub row: String,
    /// The pose id.
    pub pose: String,
    /// The winner's D-SV6 score; absent when no candidate scored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<u32>,
    /// How many of the `CONFIRM_RUNS` re-probes passed.
    pub confirmed: u32,
    /// Only the fields that changed: `{ "bearing_deg": [322.5, 330.0] }`.
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub changed: serde_json::Map<String, Value>,
    /// The proposed diff file, when there is a change to propose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_path: Option<String>,
    /// The spec file the diff applies to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_path: Option<String>,
    /// Hover probes spent, confirmations included.
    pub probes: u32,
    /// Why the pose was not calibrated, when `ok` is false; with `ok`, why
    /// the sweep stopped early (`budget: ...`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
