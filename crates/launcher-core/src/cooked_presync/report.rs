//! The presync run's report: the progress snapshot, the outcome, the failure
//! with the fields the failure table gives, and the per-category lines the
//! engine reads for its status and events (D-LP26).

use serde::Serialize;

/// The step a run is in, as the progress callback reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Checking the cache folder and the index.
    Checking,
    /// Fetching a category body.
    Downloading,
    /// Composing and writing a category's temp file.
    Writing,
    /// Renaming the temps over their PAKs.
    Committing,
}

/// A snapshot of the run for the progress callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// The step the run is in.
    pub stage: Stage,
    /// Categories staged so far.
    pub categories_done: u32,
    /// Categories the run writes.
    pub categories_total: u32,
    /// Body bytes read so far.
    pub bytes_done: u64,
    /// Body bytes the run reads, in all.
    pub bytes_total: u64,
}

/// How a run ended when it did not fail, stop or skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Every category already matches the server.
    UpToDate,
    /// Every stale category was written.
    Synced,
    /// The server or the settings say no presync runs.
    Skipped,
    /// A step failed; the report's `failure` says where.
    Failed,
    /// The cancel token stopped the run.
    Cancelled,
}

/// The step a failure happened in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    /// Before the index: the cache folder checks.
    Prepare,
    /// Fetching the index.
    Index,
    /// Comparing versions against the index.
    Plan,
    /// Fetching a category body.
    Download,
    /// Composing and writing a temp file.
    Stage,
    /// Renaming the temps over their PAKs.
    Commit,
}

/// Why a run failed: the step, the reason and the fields the failure table gives.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Failure {
    /// The step the failure happened in.
    pub step: Step,
    /// The closed reason code, such as `catalog_unavailable`.
    pub reason: &'static str,
    /// The category in progress, when the step has one.
    pub category_id: Option<u32>,
    /// That category's PAK file name.
    pub pak: Option<String>,
    /// The HTTP status, for a refusal.
    pub http_status: Option<u16>,
    /// The I/O kind, when an I/O error caused it.
    pub io_kind: Option<&'static str>,
    /// The OS error code, when there is one.
    pub os_code: Option<i32>,
    /// The server's `Retry-After`, when it sent one.
    pub retry_after_s: Option<u32>,
    /// The source PAK's version, for `source_newer`.
    pub source_version: Option<u32>,
    /// The served version, for `source_newer`.
    pub served_version: Option<u32>,
    /// The whole run's limit in seconds, for `deadline` only.
    pub deadline_s: Option<u32>,
}

impl Failure {
    /// A failure with a step and a reason, and no other fields.
    pub(super) fn new(step: Step, reason: &'static str) -> Self {
        Self {
            step,
            reason,
            category_id: None,
            pak: None,
            http_status: None,
            io_kind: None,
            os_code: None,
            retry_after_s: None,
            source_version: None,
            served_version: None,
            deadline_s: None,
        }
    }
}

/// What the run did with one category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryAction {
    /// The cache already matched the server.
    UpToDate,
    /// The category was committed to the cache.
    Written,
    /// The category was the one that failed.
    Failed,
}

/// One index entry's line in the report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CategoryReport {
    /// The category id.
    pub category_id: u32,
    /// The PAK file name.
    pub pak: String,
    /// What the run did with it.
    pub action: CategoryAction,
    /// The cache PAK's version before the run; None when missing or unreadable.
    pub cache_version: Option<u32>,
    /// The source PAK's version; None when missing or unreadable.
    pub source_version: Option<u32>,
    /// The version the server serves.
    pub served_version: u32,
    /// How many records of the category differ from the stock PAK.
    pub changed: u32,
    /// The body bytes read for the category.
    pub bytes: u64,
}

/// The outcome of one run, for the engine's status and events.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    /// How the run ended.
    pub outcome: Outcome,
    /// The reason code: `versions_match`, `synced`, a skip reason, or the failure's reason.
    pub reason: &'static str,
    /// The failure, when the run failed or was cancelled.
    pub failure: Option<Failure>,
    /// Every category the run decided on, ascending by id.
    pub categories: Vec<CategoryReport>,
    /// Body bytes read, in all.
    pub bytes_downloaded: u64,
    /// The run's length in milliseconds.
    pub duration_ms: u64,
    /// Index fetches made, including the restart's.
    pub attempts: u32,
}

impl Report {
    /// The report of a run that failed at `failure`, with no categories.
    pub fn failed(failure: Failure) -> Self {
        Self::bare(Outcome::Failed, failure.reason, Some(failure))
    }

    /// The report of a run that skipped for `reason`, with no categories.
    pub fn skipped(reason: &'static str) -> Self {
        Self::bare(Outcome::Skipped, reason, None)
    }

    /// A report with no categories and no totals yet.
    pub(super) fn bare(outcome: Outcome, reason: &'static str, failure: Option<Failure>) -> Self {
        Self {
            outcome,
            reason,
            failure,
            categories: Vec::new(),
            bytes_downloaded: 0,
            duration_ms: 0,
            attempts: 0,
        }
    }
}
