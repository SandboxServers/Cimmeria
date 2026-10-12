//! The files of a `lab uat` batch folder: `plan.json` (written once, before any
//! run), `batch.jsonl` (one line per run, appended as each run ends), then
//! `batch.json` and `summary.md` (written at the end). Shared by `lab watch`
//! and `lab uat` (LX-15 to LX-17).

pub mod progress;
pub mod read;
pub mod write;

pub use progress::format_progress_line;
pub use read::{find_latest_batch, read_plan, read_records};
pub use write::{append_record, write_plan};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

/// File name of the plan.
pub const PLAN_JSON: &str = "plan.json";
/// File name of the run records.
pub const BATCH_JSONL: &str = "batch.jsonl";
/// File name of the finished summary; its presence means the batch is done.
pub const BATCH_JSON: &str = "batch.json";

/// `plan.json`. Keys, in this order: `schema`, `section`, `rows`, `lanes`,
/// `runs_per_lease`, `started` (UTC, round-trip format `...Z`), `pid` (the
/// process that runs the batch).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Plan {
    pub schema: u32,
    pub section: String,
    pub rows: Vec<String>,
    pub lanes: Vec<PlanLane>,
    pub runs_per_lease: u32,
    pub started: String,
    pub pid: Option<u32>,
}

/// One entry of `Plan::lanes`: keys `lane` (1-based) and `instance`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanLane {
    pub lane: u32,
    pub instance: String,
}

/// One line of `batch.jsonl`: a finished run, or (`Index` 0) a lane that died
/// outside a run. Keys in PascalCase, in the order below; `Chaos` and `Golden`
/// are written only when set. A missing key reads as its default, as
/// PowerShell's `Get-UatField` reads a missing property.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct RunRecord {
    pub lane: u32,
    pub instance: String,
    /// 1-based run number in the lane; 0 for a lane death.
    pub index: u32,
    pub ok: bool,
    #[serde(deserialize_with = "lenient_rows")]
    pub rows: Vec<RowResult>,
    pub verdict: Option<Verdict>,
    pub run_dir: Option<String>,
    pub error: Option<String>,
    pub seconds: u64,
    pub braked: bool,
    pub started: Option<String>,
    pub ended: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chaos: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub golden: Option<String>,
}

/// A run's verdict: keys `Ok`, `Row`, `Result`, `Reason`, `Issue` (the first
/// failing row and why; `Issue` the known-issue tag the writer found, if any).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Verdict {
    pub ok: bool,
    pub row: Option<String>,
    pub result: Option<String>,
    pub reason: Option<String>,
    pub issue: Option<String>,
}

/// One row of `lab_uat_run`'s reply, as stored in `Rows`. Keys the daemon adds
/// that are not named here are kept in `extra` and written back unchanged.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RowResult {
    pub row: String,
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    #[serde(
        deserialize_with = "null_as_default",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub reasons: Vec<String>,
    /// The heal hint (`{pose, step}`) of a failed row; LX-17 reads it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heal: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A JSON `null` reads as the type's default.
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// `Rows`: `null` is no rows, and a `null` element (PowerShell wraps a missing
/// reply as `[null]`) is dropped.
fn lenient_rows<'de, D>(deserializer: D) -> Result<Vec<RowResult>, D::Error>
where
    D: Deserializer<'de>,
{
    let items: Option<Vec<Option<RowResult>>> = Option::deserialize(deserializer)?;
    Ok(items.unwrap_or_default().into_iter().flatten().collect())
}
