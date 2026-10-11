//! `lab_record_status` and `lab_record_stop`'s side of the recorder
//! (lab-record LR-07): read the slot, stop the task, and turn its log into
//! a draft (D-LR1, D-LR16). The raw log is always kept.
//!
//! A stop is bounded and can be retried: it signals the task and waits at
//! most [`STOP_WAIT`] with the recording still in the slot, takes it out
//! only once the task has ended, and puts it back when anything before the
//! draft fails (a wedged finish, a log read, the transform, the write). A
//! second `lab_record_stop`, perhaps with another `character_expr`, retries.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};

use super::checks::check_character_expr;
use super::run::{cut, failed, refused, Failure, RECORD_FILE, STOPPED};
use crate::supervisor::record_slot::{RecordHandle, RecordSlot};
use crate::uat::record::emit::{Draft, TransformOptions};
use crate::uat::record::log::{read_log, RecordLog};

/// The full transform error, beside the log, when the draft could not be
/// written: the tool's answer is cut to [`super::run::BODY_CAP`].
pub(super) const TRANSFORM_ERROR_FILE: &str = "transform-error.txt";
/// How long a stop waits for the task to write its stop line and release
/// its lease. A finish is one tick and a tap stop, a few seconds at worst.
pub(super) const STOP_WAIT: Duration = Duration::from_secs(60);
const STOP_POLL: Duration = Duration::from_millis(25);
/// The `ended` reason in a stop's answer is cut to this many characters.
const ENDED_CAP: usize = 80;

/// `emit::transform`, injected so a test can fail it.
pub(super) type TransformFn = fn(&RecordLog, &TransformOptions) -> Result<Draft, String>;

/// What `lab_record_stop` asks for.
pub(super) struct StopReq {
    pub recording_id: String,
    pub character_expr: Option<String>,
    pub profile: Option<String>,
    pub no_draft: bool,
    /// Used when the start named no spec directory.
    pub specs_dir: Option<PathBuf>,
    /// The instance's label, for the refusals.
    pub instance: String,
    /// [`STOP_WAIT`]; shorter in tests.
    pub wait: Duration,
    pub transform: TransformFn,
}

/// `lab_record_status` for one slot: what is armed, and whether its task
/// still runs.
pub(super) fn status(slot: &RecordSlot, instance: &str, now_ms: i64) -> Value {
    match slot.status() {
        None => json!({ "ok": true, "recording": false, "instance": instance }),
        Some(s) => json!({
            "ok": true,
            "recording": s.running,
            "recording_id": s.recording_id,
            "instance": instance,
            "section": s.section,
            "age_s": (now_ms - s.started_ms).max(0) / 1000,
            "ticks": s.ticks,
            "anchors": s.anchors,
        }),
    }
}

/// Signal the recording `req.recording_id`, wait for its task (which
/// writes the stop line and releases the lease), then transform its log
/// into a draft. A wrong id is refused and the recording keeps running.
pub(super) async fn stop(slot: &RecordSlot, req: StopReq) -> Result<Value, Failure> {
    check_character_expr(req.character_expr.as_deref())?;
    let id = req.recording_id.as_str();
    let raw = slot
        .signal_stop(id, &req.instance, STOPPED)
        .map_err(refused)?;
    let waited = tokio::time::timeout(req.wait, async {
        while slot.ended(id) == Some(false) {
            tokio::time::sleep(STOP_POLL).await;
        }
    })
    .await;
    if waited.is_err() {
        tracing::warn!(
            target: "lab.record",
            recording_id = %id, instance = %req.instance, wait_s = req.wait.as_secs(),
            "recording: the recorder did not finish in time; it stays for a retry"
        );
        return Err(failed(format!(
            "the recorder did not finish within {} s; the recording stays on instance {}: call lab_record_stop again",
            req.wait.as_secs(),
            req.instance
        ))
        .keeping(raw.display().to_string()));
    }
    let mut handle = slot.take_ended(id, &req.instance).map_err(refused)?;
    let ended = handle.outcome().await.err();
    if let Some(e) = &ended {
        tracing::warn!(target: "lab.record", recording_id = %id, error = %e,
            "recording: the recorder ended with an error; reading what it wrote");
    }
    match collect(&handle, &req, ended.as_deref()) {
        Ok(v) => {
            tracing::info!(
                target: "lab.record",
                recording_id = %id, instance = %req.instance, no_draft = req.no_draft,
                "recording collected"
            );
            Ok(v)
        }
        Err(f) => {
            slot.restore(handle);
            Err(f)
        }
    }
}

/// Read the ended recording's log and write its draft (or, `no_draft`,
/// only count it).
fn collect(h: &RecordHandle, req: &StopReq, ended: Option<&str>) -> Result<Value, Failure> {
    let raw = &h.dir;
    let text = std::fs::read_to_string(raw.join(RECORD_FILE))
        .map_err(|e| failed_at(raw, format!("{RECORD_FILE}: {e}")))?;
    let log = read_log(&text).map_err(|e| failed_at(raw, format!("{RECORD_FILE}: {e}")))?;
    let mut out = if req.no_draft {
        json!({
            "ok": true,
            "recording_id": h.recording_id,
            "ticks": log.ticks.len(),
            "raw": h.recording_id,
        })
    } else {
        let specs = h.specs_dir.clone().or_else(|| req.specs_dir.clone());
        let opts = TransformOptions {
            base: specs
                .as_deref()
                .and_then(|d| section_meta(d, &log.start.section)),
            character_expr: req.character_expr.clone(),
            profile: req.profile.clone(),
            ..Default::default()
        };
        draft_outcome(raw, specs.as_deref(), (req.transform)(&log, &opts))?
    };
    if let Some(e) = ended {
        out["ended"] = json!(cut(e, ENDED_CAP));
    }
    Ok(out)
}

/// The named section's meta from `specs`, when it loads and holds it.
fn section_meta(specs: &Path, section: &str) -> Option<crate::uat::spec::SectionMeta> {
    crate::uat::load_sections(specs, Some(&[section.to_string()]))
        .ok()?
        .pop()
        .map(|s| s.spec.section)
}

/// A failure that leads with the raw log's folder, logged.
fn failed_at(raw: &Path, error: String) -> Failure {
    tracing::warn!(target: "lab.record", raw = %raw.display(), %error,
        "recording: no draft; the raw log is kept");
    failed(error).keeping(raw.display().to_string())
}

/// Write the draft under `<specs>/drafts/`, never over an existing file.
/// A failed transform keeps the raw log, writes its whole error beside it,
/// and answers D-LR16's failure shape. The answer names the draft by its
/// file name and the raw log by its recording id (the folder's name), so a
/// deep path cannot push it past the cap.
pub(super) fn draft_outcome(
    raw: &Path,
    specs: Option<&Path>,
    result: Result<Draft, String>,
) -> Result<Value, Failure> {
    let d = match result {
        Ok(d) => d,
        Err(e) => {
            let _ = std::fs::write(raw.join(TRANSFORM_ERROR_FILE), &e);
            let f = failed_at(raw, format!("transform: {e}"));
            return Err(f.keeping(format!("{}; see {TRANSFORM_ERROR_FILE}", raw.display())));
        }
    };
    let specs = specs.ok_or_else(|| {
        failed_at(
            raw,
            "no spec directory for the draft (start with specs_dir or set CIMMERIA_LAB_UAT_SPECS)"
                .into(),
        )
    })?;
    let drafts = specs.join("drafts");
    std::fs::create_dir_all(&drafts)
        .map_err(|e| failed_at(raw, format!("{}: {e}", drafts.display())))?;
    let path = drafts.join(&d.file_name);
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => failed_at(
                raw,
                format!("draft {} exists, not overwritten", d.file_name),
            ),
            _ => failed_at(raw, format!("{}: {e}", path.display())),
        })?;
    if let Err(e) = f.write_all(d.text.as_bytes()).and_then(|()| f.flush()) {
        // A half-written draft would block the retry's `create_new`.
        drop(f);
        let _ = std::fs::remove_file(&path);
        return Err(failed_at(raw, format!("{}: {e}", path.display())));
    }
    let id = raw
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(json!({
        "ok": true,
        "draft": d.file_name,
        "rows": d.rows,
        "steps": d.actions,
        "todo": d.todos,
        "dropped": d.dropped_rows,
        "raw": id,
    }))
}
