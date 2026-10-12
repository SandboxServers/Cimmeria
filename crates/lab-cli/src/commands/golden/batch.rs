//! The child `lab uat` batch: its arguments, how it is started, its result
//! and the run folders of its `batch.jsonl`. Ports `Get-GoldenBatchArgs`,
//! `Get-GoldenRunDirs`, `ConvertFrom-GoldenBatchOutput` (the removed PowerShell golden library).

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::commands::uat::ctrlc;

use super::reply::text_of;

/// What the child printed on stdout and how it exited (`-1` when it had no
/// exit code). Its stderr is not captured: it reaches our stderr live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildOutput {
    /// Everything the child wrote to stdout.
    pub stdout: String,
    /// The child's exit code, or -1 when it had none.
    pub code: i32,
}

/// The child `lab uat` arguments for a golden batch: fingerprint runs, `runs`
/// split evenly over `leases`, a lane brake of one failed run, progress on
/// stderr and one JSON line on stdout. They are spelled as the PowerShell
/// `uat` takes them (`argv::rewrite` maps them for the native one). Each value
/// is its own element, so a path with spaces stays one argument.
pub fn batch_args(
    section: &str,
    rows: &[String],
    runs: u32,
    leases: u32,
    server_version: &str,
    specs_dir: Option<&str>,
) -> Result<Vec<String>, String> {
    if leases < 1 || !runs.is_multiple_of(leases) {
        return Err(format!(
            "--runs {runs} does not split evenly over --leases {leases}"
        ));
    }
    let mut args = vec![section.to_string()];
    let rows: Vec<&str> = rows
        .iter()
        .map(String::as_str)
        .filter(|r| !r.is_empty())
        .collect();
    if !rows.is_empty() {
        args.push("-Rows".to_string());
        args.push(rows.join(","));
    }
    args.push("-Leases".to_string());
    args.push(leases.to_string());
    args.push("-RunsPerLease".to_string());
    args.push((runs / leases).to_string());
    args.push("-ServerVersion".to_string());
    args.push(server_version.to_string());
    if let Some(dir) = specs_dir.filter(|d| !d.is_empty()) {
        args.push("-SpecsDir".to_string());
        args.push(dir.to_string());
    }
    for fixed in [
        "-Fingerprint",
        "-MaxConsecutiveFailures",
        "1",
        "-ProgressToStderr",
        "-Json",
    ] {
        args.push(fixed.to_string());
    }
    Ok(args)
}

/// Runs `<this exe> uat <args>`: stdout captured, stderr inherited (so the
/// per-run progress lines show live), stdin closed.
pub fn spawn_uat(args: &[String]) -> io::Result<ChildOutput> {
    let exe = std::env::current_exe()?;
    let mut command = Command::new(exe);
    command.arg("uat").args(args);
    wait_for_child(command)
}

/// Starts `command` with stdin closed and stderr inherited, and waits for it.
///
/// Ctrl+C reaches this process and the child alike (one console). This process
/// handles it too (`ctrlc::install_silent`), so it does not die and leave the
/// child running as an orphan: the child's own handler decides (first press: the
/// run under way finishes; second: it writes its summary and exits 130; a third
/// press ends both).
pub fn wait_for_child(mut command: Command) -> io::Result<ChildOutput> {
    ctrlc::install_silent();
    let output = command
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output();
    // The child is gone: Ctrl+C ends this process again (the daemon call that
    // follows has no child to hand the press to).
    ctrlc::uninstall();
    let output = output?;
    Ok(ChildOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        code: output.status.code().unwrap_or(-1),
    })
}

/// The records of a batch's `batch.jsonl` (none when the file is missing).
/// One compact JSON object per non-blank line. LX-13 adds a shared reader in
/// `batch/mod.rs`; switch to it then.
pub fn read_records(path: &Path) -> Result<Vec<Value>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    // `Get-Content` drops a byte-order mark; so do we.
    let text = text.trim_start_matches('\u{feff}');
    let mut records = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record: Value =
            serde_json::from_str(line).map_err(|e| format!("line {}: {e}", index + 1))?;
        records.push(record);
    }
    Ok(records)
}

/// The `RunDir` of every record, in order; `None` when there are no records,
/// or any record did not pass (`Ok` is not `true`) or has no `RunDir` (every
/// run must pass). `null` records are skipped.
pub fn run_dirs(records: &[Value]) -> Option<Vec<String>> {
    let mut dirs = Vec::new();
    for record in records {
        if record.is_null() {
            continue;
        }
        let dir = record.get("RunDir").and_then(Value::as_str).unwrap_or("");
        let passed = record.get("Ok").and_then(Value::as_bool) == Some(true);
        if !passed || dir.is_empty() {
            return None;
        }
        dirs.push(dir.to_string());
    }
    if dirs.is_empty() {
        None
    } else {
        Some(dirs)
    }
}

/// The child's result: the exit code `golden` ends with, the message, and the
/// batch folder when there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchOutcome {
    pub exit: u8,
    pub error: Option<String>,
    pub batch: Option<String>,
}

/// The child `lab uat --json` result. Its exit 0 gives the batch folder; 2 and
/// 3 pass through with its error; 1 and 4 are exit 1 with the first failure
/// group, because every run must pass (D-GD6). `lines` are the child's
/// stdout; its JSON object is the last line that starts with `{` and parses.
pub fn parse_batch_output(lines: &[String], exit_code: i32) -> BatchOutcome {
    let mut obj: Option<Value> = None;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with('{') {
            if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                obj = Some(parsed);
            }
        }
    }
    let Some(obj) = obj else {
        if exit_code == 130 {
            return BatchOutcome {
                exit: 130,
                error: Some("interrupted by Ctrl+C; no golden recorded".to_string()),
                batch: None,
            };
        }
        return BatchOutcome {
            exit: 3,
            error: Some(format!("lab uat exited {exit_code} without a result")),
            batch: None,
        };
    };
    let batch = text_of(obj.get("batch"));
    let batch_opt = if batch.is_empty() {
        None
    } else {
        Some(batch.clone())
    };
    match exit_code {
        0 => {
            if batch.is_empty() {
                BatchOutcome {
                    exit: 3,
                    error: Some("lab uat passed but named no batch folder".to_string()),
                    batch: None,
                }
            } else {
                BatchOutcome {
                    exit: 0,
                    error: None,
                    batch: batch_opt,
                }
            }
        }
        2 | 3 => BatchOutcome {
            exit: u8::try_from(exit_code).unwrap_or(3),
            error: Some(text_of(obj.get("error"))),
            batch: None,
        },
        1 | 4 => {
            let first = obj
                .get("failures")
                .and_then(Value::as_array)
                .and_then(|failures| failures.first())
                .filter(|f| !f.is_null());
            let why = match first {
                Some(failure) => {
                    let runs = failure
                        .get("runs")
                        .and_then(Value::as_array)
                        .map(|runs| {
                            runs.iter()
                                .map(|r| text_of(Some(r)))
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    let mut why = text_of(failure.get("why"));
                    if !runs.is_empty() {
                        why.push_str(&format!(" (runs: {runs})"));
                    }
                    why
                }
                None => text_of(obj.get("error")),
            };
            let passed = format!(
                "{}/{}",
                text_of(obj.get("passed")),
                text_of(obj.get("runs"))
            );
            let mut message = format!(
                "every run must pass to record a golden; {passed} passed, first failure: {why}"
            );
            if !batch.is_empty() {
                message.push_str("; ");
                message.push_str(&batch);
            }
            BatchOutcome {
                exit: 1,
                error: Some(message),
                batch: batch_opt,
            }
        }
        // The child ended on a second Ctrl+C after writing its summary.
        130 => BatchOutcome {
            exit: 130,
            error: Some(if batch.is_empty() {
                "interrupted by Ctrl+C; no golden recorded".to_string()
            } else {
                format!("interrupted by Ctrl+C; no golden recorded; {batch}")
            }),
            batch: batch_opt,
        },
        other => BatchOutcome {
            exit: 3,
            error: Some(format!("lab uat exited {other}")),
            batch: batch_opt,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SHA: &str = "1a2b3c4";
    const BATCH_DIR: &str =
        r"C:\Users\someone\AppData\Local\cimmeria-lab\uat-runs\batch-20261011-101500";

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// The element after `name` (PS `ArgAfter`).
    fn arg_after<'a>(list: &'a [String], name: &str) -> Option<&'a str> {
        let i = list.iter().position(|a| a == name)?;
        list.get(i + 1).map(String::as_str)
    }

    fn ten_runs_two_lanes() -> Vec<String> {
        batch_args(
            "first-session",
            &strings(&["FS-01", "FS-P1"]),
            10,
            2,
            SHA,
            Some(r"C:\My Specs\uat specs"),
        )
        .unwrap()
    }

    // PS L54
    #[test]
    fn the_section_comes_first() {
        assert_eq!(ten_runs_two_lanes()[0], "first-session");
    }

    // PS L55
    #[test]
    fn the_runs_are_fingerprint_runs() {
        assert!(ten_runs_two_lanes().contains(&"-Fingerprint".to_string()));
    }

    // PS L56
    #[test]
    fn the_lane_brake_is_one_failed_run() {
        assert_eq!(
            arg_after(&ten_runs_two_lanes(), "-MaxConsecutiveFailures"),
            Some("1")
        );
    }

    // PS L57
    #[test]
    fn runs_per_lease_is_runs_over_leases() {
        let a = ten_runs_two_lanes();
        assert_eq!(arg_after(&a, "-RunsPerLease"), Some("5"));
        assert_eq!(arg_after(&a, "-Leases"), Some("2"));
    }

    // PS L58
    #[test]
    fn a_specs_dir_with_spaces_stays_one_element() {
        assert_eq!(
            arg_after(&ten_runs_two_lanes(), "-SpecsDir"),
            Some(r"C:\My Specs\uat specs")
        );
    }

    // PS L59
    #[test]
    fn the_build_goes_to_the_runs() {
        assert_eq!(
            arg_after(&ten_runs_two_lanes(), "-ServerVersion"),
            Some(SHA)
        );
    }

    // PS L60
    #[test]
    fn the_rows_go_as_one_comma_list() {
        assert_eq!(
            arg_after(&ten_runs_two_lanes(), "-Rows"),
            Some("FS-01,FS-P1")
        );
    }

    // PS L61
    #[test]
    fn progress_goes_to_stderr_and_the_result_is_json() {
        let a = ten_runs_two_lanes();
        assert!(a.contains(&"-ProgressToStderr".to_string()));
        assert_eq!(a.last().map(String::as_str), Some("-Json"));
    }

    // PS L63
    #[test]
    fn no_rows_and_no_spec_folder_add_no_arguments() {
        let a = batch_args("first-session", &[], 5, 1, SHA, None).unwrap();
        assert!(!a.contains(&"-Rows".to_string()));
        assert!(!a.contains(&"-SpecsDir".to_string()));
        assert_eq!(arg_after(&a, "-RunsPerLease"), Some("5"));
        let empty = batch_args("first-session", &[], 5, 1, SHA, Some("")).unwrap();
        assert!(!empty.contains(&"-SpecsDir".to_string()));
    }

    // PS L64 (PowerShell's $null rows: here blank rows)
    #[test]
    fn blank_rows_add_no_rows_argument() {
        let a = batch_args("first-session", &strings(&["", ""]), 5, 1, SHA, None).unwrap();
        assert!(!a.contains(&"-Rows".to_string()));
    }

    // PS L66
    #[test]
    fn an_uneven_split_is_refused() {
        let err = batch_args("first-session", &[], 5, 2, SHA, None).unwrap_err();
        assert!(err.contains("does not split evenly"), "{err}");
        assert!(batch_args("first-session", &[], 5, 0, SHA, None).is_err());
    }

    fn rec(ok: bool, dir: Value) -> Value {
        json!({"Lane": 1, "Instance": "default", "Ok": ok, "RunDir": dir})
    }

    // PS L71
    #[test]
    fn every_records_run_dir_in_order() {
        let records = [
            rec(true, json!(r"C:\r\a")),
            rec(true, json!(r"C:\r\b")),
            rec(true, json!(r"C:\r\c")),
        ];
        assert_eq!(
            run_dirs(&records),
            Some(strings(&[r"C:\r\a", r"C:\r\b", r"C:\r\c"]))
        );
    }

    // PS L72
    #[test]
    fn a_failed_run_gives_no_run_folders() {
        let records = [rec(true, json!(r"C:\r\a")), rec(false, json!(r"C:\r\b"))];
        assert_eq!(run_dirs(&records), None);
    }

    // PS L73
    #[test]
    fn a_run_without_a_run_dir_gives_no_run_folders() {
        let records = [rec(true, json!(r"C:\r\a")), rec(true, Value::Null)];
        assert_eq!(run_dirs(&records), None);
    }

    // PS L74
    #[test]
    fn a_run_with_an_empty_run_dir_gives_no_run_folders() {
        let records = [rec(true, json!(r"C:\r\a")), rec(true, json!(""))];
        assert_eq!(run_dirs(&records), None);
    }

    // PS L75
    #[test]
    fn no_records_give_no_run_folders() {
        assert_eq!(run_dirs(&[]), None);
    }

    // PS L76
    #[test]
    fn a_missing_batch_gives_no_run_folders() {
        assert_eq!(run_dirs(&[Value::Null]), None);
    }

    // PS L78
    #[test]
    fn one_record_gives_one_folder() {
        assert_eq!(
            run_dirs(&[rec(true, json!(r"C:\r\only"))]),
            Some(strings(&[r"C:\r\only"]))
        );
    }

    fn write_jsonl(path: &Path, records: &[Value]) {
        let text: String = records.iter().map(|r| format!("{r}\n")).collect();
        std::fs::write(path, text).unwrap();
    }

    // PS L86 (the real file writer and reader)
    #[test]
    fn five_passing_batch_jsonl_records_give_five_run_folders() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("batch.jsonl");
        let records: Vec<Value> = (1..=5)
            .map(|i| {
                json!({"Lane": 1, "Instance": "default", "Index": i, "Ok": true,
                       "Rows": [], "RunDir": format!(r"C:\runs\run-{i}"), "Error": null})
            })
            .collect();
        write_jsonl(&path, &records);
        let back = run_dirs(&read_records(&path).unwrap()).unwrap();
        assert_eq!(
            back.join(","),
            r"C:\runs\run-1,C:\runs\run-2,C:\runs\run-3,C:\runs\run-4,C:\runs\run-5"
        );
        assert_eq!(
            read_records(&dir.path().join("missing.jsonl"))
                .unwrap()
                .len(),
            0
        );
    }

    // New: a batch.jsonl with a byte-order mark reads like one without.
    #[test]
    fn a_byte_order_mark_on_batch_jsonl_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("batch.jsonl");
        std::fs::write(&path, "\u{feff}{\"Ok\":true,\"RunDir\":\"C:\\\\r\\\\a\"}\n").unwrap();
        assert_eq!(
            run_dirs(&read_records(&path).unwrap()),
            Some(strings(&[r"C:\r\a"]))
        );
    }

    // PS L88
    #[test]
    fn a_lane_error_record_in_batch_jsonl_gives_no_run_folders() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("batch.jsonl");
        let mut records: Vec<Value> = (1..=5)
            .map(|i| json!({"Ok": true, "RunDir": format!(r"C:\runs\run-{i}")}))
            .collect();
        records.push(
            json!({"Lane": 1, "Instance": "default", "Index": 0, "Ok": false,
                            "Rows": [], "RunDir": null, "Error": "lane: boom"}),
        );
        write_jsonl(&path, &records);
        assert_eq!(run_dirs(&read_records(&path).unwrap()), None);
    }

    fn ok_line() -> String {
        json!({"ok": true, "runs": 5, "passed": 5, "failed": 0, "rows": {"FS-01": "5/5"},
               "lanes": [{"instance": "default", "passed": "5/5"}], "batch": BATCH_DIR})
        .to_string()
    }

    fn fail_line() -> String {
        json!({"ok": false, "runs": 2, "passed": 1, "failed": 1, "failed_known": 1, "failed_new": 0,
               "rows": {}, "lanes": [{"instance": "default", "passed": "1/2", "braked": true}],
               "failures": [{"n": 1, "why": "FS-P2 FAIL: clause movie-over failed",
                             "issue": "#1341", "runs": ["default#2"]}],
               "batch": r"C:\b"})
        .to_string()
    }

    // PS L95
    #[test]
    fn a_passing_batch_gives_its_folder() {
        let r = parse_batch_output(&[ok_line()], 0);
        assert_eq!((r.exit, r.batch.as_deref()), (0, Some(BATCH_DIR)));
        assert_eq!(r.error, None);
    }

    // PS L97
    #[test]
    fn the_json_line_is_found_among_other_output() {
        let r = parse_batch_output(
            &strings(&["stray host text"])
                .into_iter()
                .chain([ok_line()])
                .collect::<Vec<_>>(),
            0,
        );
        assert_eq!((r.exit, r.batch.as_deref()), (0, Some(BATCH_DIR)));
    }

    // PS L99
    #[test]
    fn a_usage_error_passes_through_as_2() {
        let line = r#"{"ok":false,"exit":2,"error":"section first-session has no row(s) FS-99"}"#;
        let r = parse_batch_output(&[line.to_string()], 2);
        assert_eq!(r.exit, 2);
        assert!(r.error.unwrap().contains("FS-99"));
    }

    // PS L101
    #[test]
    fn a_preflight_failure_passes_through_as_3() {
        let line = r#"{"ok":false,"exit":3,"error":"the lab daemon is not answering"}"#;
        let r = parse_batch_output(&[line.to_string()], 3);
        assert_eq!(r.exit, 3);
        assert!(r.error.unwrap().contains("not answering"));
    }

    // PS L105
    #[test]
    fn a_braked_lane_is_exit_1_with_its_first_failure() {
        let r = parse_batch_output(&[fail_line()], 4);
        assert_eq!(r.exit, 1);
        assert_eq!(
            r.error.as_deref(),
            Some(
                r"every run must pass to record a golden; 1/2 passed, first failure: FS-P2 FAIL: clause movie-over failed (runs: default#2); C:\b"
            )
        );
    }

    // PS L106
    #[test]
    fn a_failed_batch_still_names_its_folder() {
        assert_eq!(
            parse_batch_output(&[fail_line()], 4).batch.as_deref(),
            Some(r"C:\b")
        );
    }

    // PS L108
    #[test]
    fn a_failed_run_is_exit_1_every_run_must_pass() {
        let r = parse_batch_output(&[fail_line()], 1);
        assert_eq!(r.exit, 1);
        assert!(r.error.unwrap().starts_with("every run must pass"));
    }

    // PS L110
    #[test]
    fn a_child_with_no_json_is_a_preflight_failure_not_a_pass() {
        let r = parse_batch_output(&[], 1);
        assert_eq!(r.exit, 3);
        assert_eq!(
            r.error.as_deref(),
            Some("lab uat exited 1 without a result")
        );
    }

    // PS L112
    #[test]
    fn a_pass_that_names_no_batch_folder_cannot_go_on() {
        let r = parse_batch_output(&[r#"{"ok":true,"runs":5}"#.to_string()], 0);
        assert_eq!(r.exit, 3);
        assert_eq!(
            r.error.as_deref(),
            Some("lab uat passed but named no batch folder")
        );
    }

    // a second Ctrl+C ends the child with 130: golden ends with 130, not 3
    #[test]
    fn a_child_ended_by_ctrl_c_is_exit_130_with_its_folder() {
        let line =
            r#"{"ok":false,"runs":2,"passed":1,"failed":1,"batch":"C:\\b","interrupted":true}"#;
        let r = parse_batch_output(&[line.to_string()], 130);
        assert_eq!(r.exit, 130);
        assert_eq!(r.batch.as_deref(), Some(r"C:\b"));
        assert!(r.error.unwrap().starts_with("interrupted by Ctrl+C"));
        let r = parse_batch_output(&[], 130);
        assert_eq!((r.exit, r.batch), (130, None));
    }
}
