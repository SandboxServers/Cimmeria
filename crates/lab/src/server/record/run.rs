//! `lab_record_start`'s recorder (lab-record LR-07): one sampler task per
//! instance, run under the recorder's own lease, writing `record.jsonl`
//! until a stop, a cap or a lost lease. The stop side is [`super::stop`].
//!
//! The task runs inside a [`permit::scope`] for that lease, so once the
//! lease is taken over every bridge call the client probe makes fails
//! ("lease revoked"), and the loop stops with reason `lease_lost` at its
//! next tick. Everything the tools need is injected through [`Recorder`],
//! so the tests run it on a fake server and a fake client.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::{oneshot, watch};

use crate::lease::run::RECORD_OWNER;
use crate::lease::{permit, LeaseBook, RunLease};
use crate::supervisor::record_slot::{RecordHandle, RecordSlot};
use crate::uat::invoke::ServerInvoker;
use crate::uat::record::log::{write_line, RecordLine};
use crate::uat::record::sampler::{
    check_character, recording_id, ClientProbe, Sampler, SamplerConfig, TapStopGuard,
};
use crate::uat::record::segment::count_anchors;

/// The raw log in a recording's folder (D-LR1).
pub(super) const RECORD_FILE: &str = "record.jsonl";
/// `max_minutes` when not given, and its ceiling.
pub(super) const MAX_MINUTES: u32 = 30;
/// Every tool answer is shorter than this (D-LR16).
pub(super) const BODY_CAP: usize = 400;
/// The stop reason when the recorder's lease is lost.
pub(super) const LEASE_LOST: &str = "lease_lost";
/// The stop reason the stop tool sends.
pub(super) const STOPPED: &str = "stopped";

/// Host clock, ms since the epoch (a fake in tests).
pub(super) type Clock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// A refused or failed call, as D-LR16 prints it: exit 2 when nothing
/// changed (bad arguments, busy, wrong id), 1 when something failed.
#[derive(Debug)]
pub(super) struct Failure {
    pub exit: u8,
    pub error: String,
}

pub(super) fn refused(error: impl Into<String>) -> Failure {
    Failure {
        exit: 2,
        error: error.into(),
    }
}

pub(super) fn failed(error: impl Into<String>) -> Failure {
    Failure {
        exit: 1,
        error: error.into(),
    }
}

impl Failure {
    /// `{"ok":false,"exit":N,"error":"..."}` under [`BODY_CAP`]: the error
    /// is cut, never the shape. Paths count double once JSON escapes their
    /// backslashes, so the cut is measured on the serialized text.
    pub fn body(&self) -> String {
        let mut n = self.error.chars().count();
        loop {
            let error: String = self.error.chars().take(n).collect();
            let s = json!({ "ok": false, "exit": self.exit, "error": error }).to_string();
            if s.len() < BODY_CAP || n == 0 {
                return s;
            }
            n = n.saturating_sub((s.len() + 1 - BODY_CAP).max(1));
        }
    }
}

/// One instance's recorder.
pub(super) struct Recorder {
    pub slot: Arc<RecordSlot>,
    pub book: Arc<LeaseBook>,
    pub server: Arc<dyn ServerInvoker>,
    pub client: Arc<dyn ClientProbe>,
    /// The instance label (`default`, `p2`); it names the recording.
    pub instance: String,
    /// `<uat root>\records`.
    pub records: PathBuf,
    pub clock: Clock,
    /// The tick period (`TICK_MS`; shorter in tests).
    pub tick: Duration,
    /// Tap size and caps; `max_ticks` is set from `max_minutes`.
    pub cfg: SamplerConfig,
}

/// What `lab_record_start` asks for, defaults resolved.
pub(super) struct StartReq {
    pub section: String,
    pub character: String,
    pub specs_dir: Option<PathBuf>,
    pub max_minutes: Option<u32>,
}

/// `max_minutes`: default and at most [`MAX_MINUTES`], at least 1.
pub(super) fn max_minutes_of(m: Option<u32>) -> Result<u32, Failure> {
    match m {
        None => Ok(MAX_MINUTES),
        Some(0) => Err(refused("max_minutes: give 1 to 30")),
        Some(m) => Ok(m.min(MAX_MINUTES)),
    }
}

/// What the task reports once its start line and tick 0 are written.
struct Begun {
    recording_id: String,
    dir: PathBuf,
    started_ms: i64,
}

/// Everything the sampler task owns.
struct Job {
    server: Arc<dyn ServerInvoker>,
    client: Arc<dyn ClientProbe>,
    cfg: SamplerConfig,
    section: String,
    instance: String,
    character: String,
    records: PathBuf,
    clock: Clock,
    tick: Duration,
    minutes: u32,
    ticks: Arc<AtomicU32>,
    anchors: Arc<AtomicU32>,
    stop: watch::Receiver<Option<String>>,
    ready: oneshot::Sender<Result<Begun, String>>,
}

/// Reserve the slot, take the lease, begin (start line and tick 0 on
/// disk), then arm the slot with the running task. Every refusal comes
/// before anything touches the server.
pub(super) async fn start(rec: Recorder, req: StartReq) -> Result<Value, Failure> {
    let minutes = max_minutes_of(req.max_minutes)?;
    if req.section.trim().is_empty() {
        return Err(refused("section: give the section being played"));
    }
    check_character(&req.character).map_err(refused)?;
    recording_id(0, &rec.instance).map_err(refused)?;
    let reservation = rec.slot.reserve().map_err(refused)?;
    let lease = RunLease::acquire_own_as(
        rec.book.clone(),
        RECORD_OWNER,
        format!("recording a human: {}", req.section),
    )
    .map_err(refused)?;

    let ticks = Arc::new(AtomicU32::new(0));
    let anchors = Arc::new(AtomicU32::new(0));
    let (stop_tx, stop_rx) = watch::channel(None);
    let (ready_tx, ready_rx) = oneshot::channel();
    let mut cfg = rec.cfg;
    cfg.max_ticks = minutes.saturating_mul(ticks_per_minute(rec.tick));
    let job = Job {
        server: rec.server,
        client: rec.client,
        cfg,
        section: req.section.clone(),
        instance: rec.instance.clone(),
        character: req.character,
        records: rec.records,
        clock: rec.clock,
        tick: rec.tick,
        minutes,
        ticks: ticks.clone(),
        anchors: anchors.clone(),
        stop: stop_rx,
        ready: ready_tx,
    };
    let done = tokio::spawn(async move {
        let permit = lease.permit();
        // The lease outlives the scope: it is released when this task ends.
        let out = permit::scope(permit, record(job, &lease)).await;
        drop(lease);
        out
    });
    let begun = match ready_rx.await {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => {
            // Its lease is released once it ends: wait for that.
            let _ = done.await;
            return Err(failed(e));
        }
        Err(_) => {
            let why = match done.await {
                Ok(Err(e)) => e,
                Ok(Ok(_)) => "the recorder ended before it began".to_string(),
                Err(e) => format!("the recorder task failed: {e}"),
            };
            return Err(failed(why));
        }
    };
    let out = json!({
        "ok": true,
        "recording_id": begun.recording_id,
        "instance": rec.instance,
        "section": req.section,
        "dir": begun.dir.display().to_string(),
    });
    let handle = RecordHandle {
        recording_id: begun.recording_id,
        dir: begun.dir,
        section: req.section,
        specs_dir: req.specs_dir,
        started_ms: begun.started_ms,
        ticks,
        anchors,
        stop: stop_tx,
        done,
    };
    reservation.arm(handle).map_err(failed)?;
    Ok(out)
}

/// Ticks per minute at `tick` (120 at 500 ms), so `max_ticks` matches
/// `max_minutes` whatever the period.
fn ticks_per_minute(tick: Duration) -> u32 {
    let ms = tick.as_millis().max(1);
    u32::try_from(60_000 / ms).unwrap_or(u32::MAX).max(1)
}

/// The sampler task: begin, tick until a stop, a cap or a lost lease, then
/// finish. Returns the log's path.
async fn record(job: Job, lease: &RunLease) -> Result<PathBuf, String> {
    // Built before begin: an abort or panic from here on stops the tap.
    let guard = TapStopGuard::new(job.server.clone());
    let now = (job.clock)();
    let begun = Sampler::begin(
        &*job.server,
        &*job.client,
        job.cfg,
        guard.cell(),
        &job.section,
        &job.instance,
        &job.character,
        now,
        &local_date(now),
    )
    .await;
    let (mut sampler, start, t0) = match begun {
        Ok(b) => b,
        Err(e) => {
            let _ = job.ready.send(Err(e.clone()));
            return Err(e);
        }
    };
    let dir = job.records.join(&start.recording_id);
    let path = dir.join(RECORD_FILE);
    let opened = open_log(&dir, &path).and_then(|mut f| {
        append(&mut f, &RecordLine::Start(start.clone()))?;
        append(&mut f, &RecordLine::Tick(t0.clone()))?;
        Ok(f)
    });
    let mut file = match opened {
        Ok(f) => f,
        Err(e) => {
            let _ = job.ready.send(Err(e.clone()));
            return Err(e);
        }
    };
    job.ticks.store(1, Ordering::Relaxed);
    job.anchors.store(count_anchors(&t0.tap), Ordering::Relaxed);
    let _ = job.ready.send(Ok(Begun {
        recording_id: start.recording_id.clone(),
        dir,
        started_ms: now,
    }));

    let keep = lease.keep_alive();
    let mut revoked = keep.revoked();
    let mut stop = job.stop;
    let deadline = now.saturating_add(i64::from(job.minutes) * 60_000);
    let mut every = tokio::time::interval(job.tick);
    every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    every.tick().await;
    let mut write_err = None;
    let reason = loop {
        tokio::select! {
            _ = every.tick() => {}
            r = stop.changed() => {
                break match r {
                    Ok(()) => stop.borrow().clone().unwrap_or_else(|| STOPPED.to_string()),
                    Err(_) => STOPPED.to_string(),
                };
            }
            _ = revoked.changed() => break LEASE_LOST.to_string(),
        }
        // The keep-alive notices a takeover too; this catches it at once.
        if lease.touch("lab_record tick").is_err() {
            break LEASE_LOST.to_string();
        }
        // Host time, beside max_ticks: a tick that blocks on a slow read
        // still ends the recording on time.
        let now = (job.clock)();
        if now >= deadline {
            break "max_minutes".to_string();
        }
        let t = sampler.tick(now).await;
        if let Err(e) = append(&mut file, &RecordLine::Tick(t.clone())) {
            write_err = Some(e);
            break "write_failed".to_string();
        }
        job.ticks.fetch_add(1, Ordering::Relaxed);
        job.anchors
            .fetch_add(count_anchors(&t.tap), Ordering::Relaxed);
        if let Some(r) = sampler.full() {
            break r.to_string();
        }
    };
    drop(keep);

    let (t, stop_line) = sampler.finish((job.clock)(), &reason).await;
    let written = append(&mut file, &RecordLine::Tick(t.clone()))
        .and_then(|()| append(&mut file, &RecordLine::Stop(stop_line)));
    job.ticks.fetch_add(1, Ordering::Relaxed);
    job.anchors
        .fetch_add(count_anchors(&t.tap), Ordering::Relaxed);
    tracing::info!(
        target: "lab.record",
        recording_id = %start.recording_id, section = %start.section,
        character = %start.character, instance = %start.instance, %reason,
        ticks = job.ticks.load(Ordering::Relaxed),
        anchors = job.anchors.load(Ordering::Relaxed),
        "recording stopped"
    );
    match write_err.map_or(written, Err) {
        Ok(()) => Ok(path),
        Err(e) => Err(e),
    }
}

/// The folder and its log. A folder that already exists is refused: a
/// recording never writes over another's log.
fn open_log(dir: &Path, path: &Path) -> Result<std::fs::File, String> {
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::create_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    std::fs::OpenOptions::new()
        .append(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// One line, flushed, so a crash keeps every tick before it.
fn append(f: &mut std::fs::File, line: &RecordLine) -> Result<(), String> {
    f.write_all(write_line(line).as_bytes())
        .and_then(|()| f.flush())
        .map_err(|e| format!("{RECORD_FILE}: {e}"))
}

/// The local date for the start line (`2026-10-12`).
fn local_date(now_ms: i64) -> String {
    use chrono::TimeZone as _;
    chrono::Local
        .timestamp_millis_opt(now_ms)
        .single()
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}
