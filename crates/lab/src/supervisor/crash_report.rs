//! `lab_crash_report` assembly: the last minidump, the last N bridge
//! commands (with the quarantined one flagged), and the DLL's crash
//! marker.
//!
//! Evidence comes from two sources that should agree: the supervisor's
//! own [`super::recovery::CommandJournal`] (authoritative on what was
//! sent and in flight) and the DLL-flushed files in the session dir
//! (the minidump + `lab-crash-marker.json`). The minidump-picking and
//! the report shape are pure and tested; the filesystem scan is a thin
//! wrapper.

use std::path::Path;

use serde_json::{json, Value};

use super::recovery::CommandJournal;

/// Pick the newest minidump from a list of filenames of the form
/// `lab-minidump-<ts_ms>.dmp`. Newest = largest embedded timestamp.
/// Pure so the ordering rule is testable.
pub fn pick_latest_minidump(names: &[String]) -> Option<String> {
    names
        .iter()
        .filter_map(|n| parse_minidump_ts(n).map(|ts| (ts, n)))
        .max_by_key(|(ts, _)| *ts)
        .map(|(_, n)| n.clone())
}

/// Extract the `<ts_ms>` from `lab-minidump-<ts_ms>.dmp`.
fn parse_minidump_ts(name: &str) -> Option<i64> {
    name.strip_prefix("lab-minidump-")
        .and_then(|s| s.strip_suffix(".dmp"))
        .and_then(|s| s.parse::<i64>().ok())
}

/// Scan a directory for the newest minidump filename.
pub fn latest_minidump_in(dir: &Path) -> Option<String> {
    let names: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    pick_latest_minidump(&names)
}

/// Read the DLL crash marker (`lab-crash-marker.json`) if present.
pub fn read_marker(dir: &Path) -> Option<Value> {
    let path = dir.join("lab-crash-marker.json");
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Build the `lab_crash_report` payload from the journal + the session
/// dir. `last_n` bounds the command history returned.
pub fn build_report(journal: &CommandJournal, sessions_dir: &Path, last_n: usize) -> Value {
    let minidump = latest_minidump_in(sessions_dir);
    let marker = read_marker(sessions_dir);
    json!({
        "minidump": minidump.map(|m| sessions_dir.join(m).display().to_string()),
        "marker": marker,
        "recent_commands": journal.recent(last_n),
        "quarantined": journal.quarantined(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supervisor::recovery::CommandJournal;

    #[test]
    fn latest_minidump_picks_newest_timestamp() {
        let names = vec![
            "lab-minidump-1000.dmp".to_string(),
            "lab-minidump-3000.dmp".to_string(),
            "lab-minidump-2000.dmp".to_string(),
            "unrelated.txt".to_string(),
            "lab-crash-marker.json".to_string(),
        ];
        assert_eq!(
            pick_latest_minidump(&names),
            Some("lab-minidump-3000.dmp".to_string())
        );
    }

    #[test]
    fn latest_minidump_none_when_absent() {
        assert_eq!(pick_latest_minidump(&["foo.txt".to_string()]), None);
        assert_eq!(pick_latest_minidump(&[]), None);
    }

    /// The report carries the quarantined command and the minidump path
    /// read off disk.
    #[test]
    fn report_includes_quarantined_command_and_minidump() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("lab-minidump-1700000000000.dmp"), b"MDMP").unwrap();
        std::fs::write(
            dir.path().join("lab-crash-marker.json"),
            br#"{"in_flight":true,"exception_code":3221225477}"#,
        )
        .unwrap();

        let mut j = CommandJournal::new(8);
        let a = j.record("lua_eval", 1);
        j.complete(a, true);
        j.record("mem_write", 2); // in flight → quarantined
        j.quarantine_in_flight();

        let report = build_report(&j, dir.path(), 10);
        assert!(report["minidump"]
            .as_str()
            .unwrap()
            .contains("lab-minidump-1700000000000.dmp"));
        assert_eq!(report["marker"]["in_flight"], true);
        let q = report["quarantined"].as_array().unwrap();
        assert_eq!(q.len(), 1);
        assert_eq!(q[0]["method"], "mem_write");
        // Sanity: both commands are in the recent history.
        let recent = report["recent_commands"].as_array().unwrap();
        assert_eq!(recent.len(), 2);
    }
}
