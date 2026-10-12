#![allow(dead_code)]
//! A lane harness: a scripted `lab_uat_run`, recording what the lanes ask for.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use cimmeria_lab_cli::batch::{read_records, PlanLane, RunRecord};
use cimmeria_lab_cli::commands::uat::lanes::{no_finish, run_lanes, LaneConfig, LaneEnv};
use cimmeria_lab_cli::daemon::mcp::McpError;
use serde_json::{json, Value};

pub type Reply = Result<Value, McpError>;

/// A reply where every named row passed.
pub fn pass_reply(run_dir: &str, rows: &[&str]) -> Reply {
    let rows: Vec<Value> = rows
        .iter()
        .map(|id| json!({"row": id, "result": "PASS"}))
        .collect();
    Ok(json!({"run_dir": run_dir, "rows": rows}))
}

/// `n` passing replies.
pub fn passes(n: usize, rows: &[&str]) -> Vec<Reply> {
    (0..n).map(|_| pass_reply("C:\\runs\\a", rows)).collect()
}

/// A reply where `row` failed with `reason`.
pub fn fail_reply(row: &str, reason: &str) -> Reply {
    Ok(
        json!({"run_dir": "C:\\runs\\f", "rows": [{"row": row, "result": "FAIL", "reasons": [reason]}]}),
    )
}

/// `n` failing replies on FS-P3.
pub fn fails(n: usize) -> Vec<Reply> {
    (0..n)
        .map(|_| fail_reply("FS-P3", "clause aim failed"))
        .collect()
}

/// The first run fails on FS-P2 (known #1341) after FS-P1 passed.
pub fn frost_reply() -> Reply {
    Ok(json!({"run_dir": "C:\\runs\\f", "rows": [
        {"row": "FS-P1", "result": "PASS"},
        {"row": "FS-P2", "result": "FAIL", "reasons": ["clause movie-over failed"]}]}))
}

/// A `call` closure over a per-instance script, for `Hooks` (no recording).
pub fn scripted(script: Vec<(&str, Vec<Reply>)>) -> impl Fn(&str, Duration) -> Reply + Sync {
    let map: Mutex<HashMap<String, VecDeque<Reply>>> = Mutex::new(
        script
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.into_iter().collect()))
            .collect(),
    );
    move |instance: &str, _timeout: Duration| {
        map.lock()
            .unwrap()
            .get_mut(instance)
            .and_then(VecDeque::pop_front)
            .unwrap_or_else(|| Err(McpError::Tool("no scripted reply".to_string())))
    }
}

pub struct LaneRig {
    pub dir: tempfile::TempDir,
    pub calls: Mutex<Vec<(String, Duration)>>,
    pub waits: Mutex<Vec<(String, Duration)>>,
    pub sleeps: Mutex<Vec<Duration>>,
    pub notices: Mutex<Vec<(String, String)>>,
    pub seen: Mutex<Vec<RunRecord>>,
    pub script: Mutex<HashMap<String, VecDeque<Reply>>>,
    /// What the lease wait answers.
    pub wait_result: bool,
    pub stop: AtomicBool,
    /// Set `stop` inside this (1-based) call, as Ctrl+C would.
    pub stop_on_call: Option<usize>,
    /// Panic inside any call for this instance.
    pub panic_instance: Option<String>,
    /// Sleep this long inside each call.
    pub delay_ms: u64,
    pub in_flight: AtomicUsize,
    pub max_in_flight: AtomicUsize,
}

impl LaneRig {
    pub fn new(script: Vec<(&str, Vec<Reply>)>) -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
            calls: Mutex::default(),
            waits: Mutex::default(),
            sleeps: Mutex::default(),
            notices: Mutex::default(),
            seen: Mutex::default(),
            script: Mutex::new(
                script
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.into_iter().collect()))
                    .collect(),
            ),
            wait_result: true,
            stop: AtomicBool::new(false),
            stop_on_call: None,
            panic_instance: None,
            delay_ms: 0,
            in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
        }
    }

    pub fn jsonl(&self) -> PathBuf {
        self.dir.path().join("batch.jsonl")
    }

    /// The records on disk, in file order.
    pub fn records(&self) -> Vec<RunRecord> {
        read_records(&self.jsonl()).unwrap()
    }

    /// Runs the lanes: `n` runs per lane, a 15 minute run timeout.
    pub fn run(&self, lanes: &[PlanLane], row_ids: &[String], n: u32, stagger: u32, max: u32) {
        let call = |instance: &str, timeout: Duration| -> Reply {
            let count = {
                let mut calls = self.calls.lock().unwrap();
                calls.push((instance.to_string(), timeout));
                calls.len()
            };
            if self.panic_instance.as_deref() == Some(instance) {
                panic!("boom");
            }
            if self.stop_on_call == Some(count) {
                self.stop.store(true, Ordering::SeqCst);
            }
            let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_in_flight.fetch_max(now, Ordering::SeqCst);
            if self.delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(self.delay_ms));
            }
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
            self.script
                .lock()
                .unwrap()
                .get_mut(instance)
                .and_then(VecDeque::pop_front)
                .unwrap_or_else(|| Err(McpError::Tool("no scripted reply".to_string())))
        };
        let wait = |instance: &str, max: Duration| -> bool {
            self.waits.lock().unwrap().push((instance.to_string(), max));
            self.wait_result
        };
        let sleep = |total: Duration| self.sleeps.lock().unwrap().push(total);
        let on_record = |rec: &RunRecord| self.seen.lock().unwrap().push(rec.clone());
        let on_notice = |instance: &str, rest: &str| {
            self.notices
                .lock()
                .unwrap()
                .push((instance.to_string(), rest.to_string()))
        };
        let env = LaneEnv {
            call: &call,
            wait_lease_free: &wait,
            sleep: &sleep,
            on_record: &on_record,
            on_notice: &on_notice,
            finish: &no_finish,
            stop: &self.stop,
        };
        let jsonl = self.jsonl();
        let cfg = LaneConfig {
            row_ids,
            runs_per_lease: n,
            stagger_seconds: stagger,
            run_timeout: Duration::from_secs(15 * 60),
            max_consecutive_failures: max,
            jsonl: &jsonl,
        };
        run_lanes(lanes, &cfg, &env);
    }
}
