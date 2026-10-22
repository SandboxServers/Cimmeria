//! One launcher log line, as `ingest::rows` takes it (contract C-LF7).
//!
//! LF-13 owns this file: it adds `Collected`, `MAX_LINES` and `collect` around
//! this struct. LF-14 carries only the struct so the sender compiles on its own
//! branch; take LF-13's file whole when the two meet.

/// One complete log line. `end` is the byte offset just past its `\n`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogLine {
    pub file: String,
    pub line_no: u64,
    pub end: u64,
    pub ts_ms: Option<i64>,
    pub level: Option<String>,
    pub target: Option<String>,
    pub message: String,
    pub truncated: bool,
}
