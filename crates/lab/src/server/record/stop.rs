//! `lab_record_status` and `lab_record_stop`'s side of the recorder
//! (lab-record LR-07): read the slot, stop the task, and turn its log into
//! a draft (D-LR1, D-LR16). The raw log is always kept.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::run::{failed, refused, Failure, RECORD_FILE, STOPPED};
use crate::supervisor::record_slot::RecordSlot;
use crate::uat::record::emit::{transform, Draft, TransformOptions};
use crate::uat::record::log::read_log;

/// The full transform error, beside the log, when the draft could not be
/// written: the tool's answer is cut to [`super::run::BODY_CAP`].
pub(super) const TRANSFORM_ERROR_FILE: &str = "transform-error.txt";

/// What `lab_record_stop` asks for.
pub(super) struct StopReq {
    pub recording_id: String,
    pub character_expr: Option<String>,
    pub profile: Option<String>,
    pub no_draft: bool,
    /// Used when the start named no spec directory.
    pub specs_dir: Option<PathBuf>,
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
    let handle = slot.take_if(&req.recording_id).map_err(refused)?;
    // Already ended (a cap or a lost lease) when nobody is listening.
    let _ = handle.stop.send(Some(STOPPED.to_string()));
    let raw = handle.dir.clone();
    let ended = match handle.done.await {
        Ok(Ok(_)) => None,
        Ok(Err(e)) => Some(e),
        Err(e) => Some(e.to_string()),
    };
    if let Some(e) = ended {
        tracing::warn!(target: "lab.record", recording_id = %req.recording_id, error = %e,
            "recording: the recorder ended with an error; reading what it wrote");
    }
    let text = std::fs::read_to_string(raw.join(RECORD_FILE))
        .map_err(|e| failed(format!("{RECORD_FILE}: {e}; raw: {}", raw.display())))?;
    let log = read_log(&text)
        .map_err(|e| failed(format!("{RECORD_FILE}: {e}; raw: {}", raw.display())))?;
    if req.no_draft {
        return Ok(json!({
            "ok": true,
            "recording_id": req.recording_id,
            "ticks": log.ticks.len(),
            "raw": raw.display().to_string(),
        }));
    }
    let specs = handle.specs_dir.or(req.specs_dir);
    let opts = TransformOptions {
        base: specs
            .as_deref()
            .and_then(|d| section_meta(d, &log.start.section)),
        character_expr: req.character_expr,
        profile: req.profile,
        ..Default::default()
    };
    draft_outcome(&raw, specs.as_deref(), transform(&log, &opts))
}

/// The named section's meta from `specs`, when it loads and holds it.
fn section_meta(specs: &Path, section: &str) -> Option<crate::uat::spec::SectionMeta> {
    crate::uat::load_sections(specs, Some(&[section.to_string()]))
        .ok()?
        .pop()
        .map(|s| s.spec.section)
}

/// Write the draft under `<specs>/drafts/`, never over an existing file.
/// A failed transform keeps the raw log, writes its whole error beside it,
/// and answers D-LR16's failure shape.
pub(super) fn draft_outcome(
    raw: &Path,
    specs: Option<&Path>,
    result: Result<Draft, String>,
) -> Result<Value, Failure> {
    let kept = format!("raw log kept in {}", raw.display());
    let d = match result {
        Ok(d) => d,
        Err(e) => {
            let _ = std::fs::write(raw.join(TRANSFORM_ERROR_FILE), &e);
            return Err(failed(format!("transform: {e}; {kept}")));
        }
    };
    let specs = specs.ok_or_else(|| {
        failed(format!(
            "no spec directory for the draft (start with specs_dir or set CIMMERIA_LAB_UAT_SPECS); {kept}"
        ))
    })?;
    let drafts = specs.join("drafts");
    std::fs::create_dir_all(&drafts)
        .map_err(|e| failed(format!("{}: {e}; {kept}", drafts.display())))?;
    let path = drafts.join(&d.file_name);
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => failed(format!(
                "draft {} exists, not overwritten; {kept}",
                path.display()
            )),
            _ => failed(format!("{}: {e}; {kept}", path.display())),
        })?;
    f.write_all(d.text.as_bytes())
        .map_err(|e| failed(format!("{}: {e}; {kept}", path.display())))?;
    Ok(json!({
        "ok": true,
        "draft": path.display().to_string(),
        "rows": d.rows,
        "steps": d.actions,
        "todo": d.todos,
        "dropped": d.dropped_rows,
        "raw": raw.display().to_string(),
    }))
}
