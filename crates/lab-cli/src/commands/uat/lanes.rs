//! The lanes of a batch: one thread per lane, each running the rows
//! `runs_per_lease` times, one after another.
//!
//! A lane starts `(lane - 1) * stagger` seconds after the batch. Each run is one
//! `call` (the caller makes the `lab_uat_run` call, with its own MCP session), its
//! record is appended to `batch.jsonl` as soon as it ends, then the brake and the
//! timeout wait decide whether the lane goes on. A lane that dies outside a run
//! (a panic, an append that fails) leaves an `Index` 0 record.

use std::any::Any;
use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use chrono::Utc;
use serde_json::Value;

use crate::batch::{append_record, PlanLane, RunRecord};
use crate::daemon::mcp::McpError;

use super::record::{apply_error, apply_reply, lane_death, new_record, stop_lane, utc_stamp};

/// How long a lane waits for its instance's lease to clear after a timed-out run.
pub const LEASE_CLEAR_WAIT: Duration = Duration::from_secs(600);

/// One run: `(instance, timeout) -> the lab_uat_run reply`.
pub type CallFn<'a> = dyn Fn(&str, Duration) -> Result<Value, McpError> + Sync + 'a;
/// `(instance, max) -> whether the instance's lease cleared within max`.
pub type WaitFn<'a> = dyn Fn(&str, Duration) -> bool + Sync + 'a;
/// Sleeps for the stagger; a real one wakes early when the stop flag is set.
pub type SleepFn<'a> = dyn Fn(Duration) + Sync + 'a;
/// Called with each record after it is appended.
pub type RecordFn<'a> = dyn Fn(&RunRecord) + Sync + 'a;
/// Called with `(instance, text)` when a lane stops (the text follows the
/// instance in `[<instance>] <text>`).
pub type NoticeFn<'a> = dyn Fn(&str, &str) + Sync + 'a;

/// Called with each run's record (after its reply or error is applied, before
/// the brake and the append) and the reply when the call returned one. LX-17b
/// uses it for the run's chaos header and the golden diff.
pub type FinishFn<'a> = dyn Fn(&mut RunRecord, Option<&Value>) + Sync + 'a;

/// The `FinishFn` of a batch with nothing to add (LX-16b passes it).
pub fn no_finish(_rec: &mut RunRecord, _reply: Option<&Value>) {}

/// What a lane does, as numbers and names.
pub struct LaneConfig<'a> {
    /// The planned row ids the verdict checks the reply against.
    pub row_ids: &'a [String],
    pub runs_per_lease: u32,
    pub stagger_seconds: u32,
    pub run_timeout: Duration,
    /// Failed runs in a row that stop a lane (0 never).
    pub max_consecutive_failures: u32,
    /// `batch.jsonl`.
    pub jsonl: &'a Path,
}

/// What a lane needs from outside.
pub struct LaneEnv<'a> {
    pub call: &'a CallFn<'a>,
    pub wait_lease_free: &'a WaitFn<'a>,
    pub sleep: &'a SleepFn<'a>,
    pub on_record: &'a RecordFn<'a>,
    pub on_notice: &'a NoticeFn<'a>,
    /// The per-run seam LX-17b fills ([`no_finish`] until then).
    pub finish: &'a FinishFn<'a>,
    /// Set by Ctrl+C: no new run starts once it is set.
    pub stop: &'a AtomicBool,
}

/// Sleeps `total`, in short steps, returning at once when `stop` is set.
pub fn sleep_unless_stopped(total: Duration, stop: &AtomicBool) {
    let deadline = Instant::now() + total;
    while !stop.load(Ordering::SeqCst) {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        std::thread::sleep(left.min(Duration::from_millis(200)));
    }
}

fn panic_text(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "the lane panicked".to_string()
    }
}

fn lane_body(lane: &PlanLane, cfg: &LaneConfig, env: &LaneEnv) -> io::Result<()> {
    if lane.lane > 1 {
        (env.sleep)(Duration::from_secs(
            u64::from(lane.lane - 1) * u64::from(cfg.stagger_seconds),
        ));
    }
    let total = cfg.runs_per_lease;
    let mut outcomes: Vec<bool> = Vec::new();
    for index in 1..=total {
        if env.stop.load(Ordering::SeqCst) {
            break;
        }
        let started = Instant::now();
        let mut rec = new_record(lane.lane, &lane.instance, index, Utc::now());
        let mut timed_out = false;
        let mut reply: Option<Value> = None;
        match (env.call)(&lane.instance, cfg.run_timeout) {
            Ok(r) => {
                apply_reply(&mut rec, &r, cfg.row_ids);
                reply = Some(r);
            }
            Err(error) => timed_out = apply_error(&mut rec, &error),
        }
        rec.seconds = started.elapsed().as_secs();
        // LX-17 hook: per-run finish (chaos header, golden diff); a no-op until LX-17b.
        (env.finish)(&mut rec, reply.as_ref());
        outcomes.push(rec.ok);
        // A run cut short may still hold its lease on the daemon: wait for it,
        // or the next run collides with it.
        let stuck = timed_out && !(env.wait_lease_free)(&lane.instance, LEASE_CLEAR_WAIT);
        let brake = stop_lane(index, total, &outcomes, cfg.max_consecutive_failures);
        rec.braked = brake || stuck;
        rec.ended = Some(utc_stamp(Utc::now()));
        append_record(cfg.jsonl, &rec)?;
        (env.on_record)(&rec);
        if stuck {
            (env.on_notice)(
                &lane.instance,
                "its lease did not clear after the timeout; lane stopped",
            );
        } else if brake {
            (env.on_notice)(
                &lane.instance,
                &format!(
                    "stopped after {} failed runs in a row",
                    cfg.max_consecutive_failures
                ),
            );
        }
        if rec.braked {
            break;
        }
    }
    Ok(())
}

/// Runs one lane. Never panics and never returns an error: a failure outside a
/// run becomes an `Index` 0 record (best effort).
pub fn run_lane(lane: &PlanLane, cfg: &LaneConfig, env: &LaneEnv) {
    let message = match catch_unwind(AssertUnwindSafe(|| lane_body(lane, cfg, env))) {
        Ok(Ok(())) => return,
        Ok(Err(error)) => error.to_string(),
        Err(payload) => panic_text(payload.as_ref()),
    };
    let death = lane_death(lane.lane, &lane.instance, &message, Utc::now());
    let _ = append_record(cfg.jsonl, &death);
}

/// Runs every lane at once, one thread each, and returns when all are done.
pub fn run_lanes(lanes: &[PlanLane], cfg: &LaneConfig, env: &LaneEnv) {
    std::thread::scope(|scope| {
        for lane in lanes {
            scope.spawn(move || run_lane(lane, cfg, env));
        }
    });
}
