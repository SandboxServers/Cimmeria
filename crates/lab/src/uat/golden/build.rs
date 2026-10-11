//! A run directory as a fingerprint (README D-GD1): each row's evidence
//! and its fingerprint-mode attachments, normalised by pure code, so an old
//! bundle can be fingerprinted again when the normaliser changes.

use serde_json::Value;

use super::event::{RowFingerprint, RunFingerprint, Variance, FINGERPRINT_SCHEMA};
use super::labels::EntityLabels;
use super::normalise::{normalise_tap, resolve, windows, RUN_VARS};
use crate::uat::evidence::{RowEvidence, RunDir};
use crate::uat::spec::{SectionSpec, Source};

/// One row's evidence and the attachments the fingerprint reads.
#[derive(Debug, Clone)]
pub struct RowInput {
    pub evidence: RowEvidence,
    /// The `packet_tap` attachment's body.
    pub tap: Option<Value>,
    /// The `entities` attachment's body.
    pub entities: Option<Value>,
    /// The `windows` attachment's body.
    pub windows: Option<Value>,
}

/// What a golden checks before it compares two runs: the same spec, the
/// same server build and the same SGW.exe (README D-GD6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunMeta {
    pub run_id: String,
    /// The run directory's folder name.
    pub run_dir: String,
    pub spec_sha256: Option<String>,
    pub server_build: Option<String>,
    pub sgw_sha256: Option<String>,
}

/// One row's fingerprint under the variance `v`.
pub fn build_row(input: &RowInput, v: &Variance) -> RowFingerprint {
    let ev = &input.evidence;
    let body = input.tap.as_ref();
    let tap = match body {
        // A bundle from before fingerprint mode has no `coverage`; its tap
        // was started with the row, so it covered the whole row.
        Some(b) => b
            .get("coverage")
            .and_then(Value::as_str)
            .unwrap_or("full")
            .to_string(),
        None => "none".to_string(),
    };
    let player = body
        .and_then(|b| b.get("entity_id"))
        .and_then(Value::as_u64)
        .and_then(|id| u32::try_from(id).ok())
        .unwrap_or(0);
    let mut labels = EntityLabels::new(player, None, input.entities.as_ref());
    let messages: &[Value] = body
        .and_then(|b| b.pointer("/tap/messages"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice);
    let subst: Vec<(String, String)> = RUN_VARS
        .iter()
        .filter_map(|name| {
            let value = ev.vars.get(*name)?.as_str()?;
            Some((name.to_string(), value.to_string()))
        })
        .collect();
    let n = normalise_tap(messages, tap.starts_with("late:"), &mut labels, v, &subst);
    let (windows_opened, windows_closed) = windows(input.windows.as_ref(), v);
    RowFingerprint {
        row: ev.row_id.clone(),
        result: ev.result.as_str().to_string(),
        tap,
        tap_faults: body.map(tap_faults).unwrap_or_default(),
        clauses: clauses(ev),
        transitions: n.transitions,
        windows_opened,
        windows_closed,
        events: n.events.iter().map(|e| e.render()).collect(),
        unordered: n.unordered,
        variance: v.clone(),
    }
}

/// Why a tap cannot be trusted to hold every message (README D-GD8).
fn tap_faults(body: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let dropped = body
        .pointer("/tap/dropped")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if dropped > 0 {
        out.push(format!("dropped {dropped}"));
    }
    if body.get("replaced_existing").and_then(Value::as_bool) == Some(true) {
        out.push("replaced".to_string());
    }
    if body.get("stopped").and_then(Value::as_bool) == Some(false) {
        out.push("not stopped".to_string());
    }
    if body.get("read_ok").and_then(Value::as_bool) == Some(false) {
        out.push("read failed".to_string());
    }
    out
}

/// Required clauses whose verdict should repeat run to run. Timing, SigNoz
/// and human clauses do not (F10).
fn clauses(ev: &RowEvidence) -> Vec<String> {
    ev.clauses
        .iter()
        .filter(|c| {
            c.required && !matches!(c.source, Source::Timing | Source::Signoz | Source::Human)
        })
        .map(|c| {
            let verdict = serde_json::to_value(c.verdict)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            format!("{}={verdict}", c.id)
        })
        .collect()
}

/// The row JSON and its `packet_tap`, `entities` and `windows` attachments.
/// An attachment the row does not list (or lists without a file, as an
/// unrouted tool does) is None; one listed but unreadable is an error.
pub fn load_row_input(run: &RunDir, section: &str, row: &str) -> Result<RowInput, String> {
    let evidence: RowEvidence = run.read_json(&run.row_json(section, row))?;
    let attachment = |name: &str| -> Result<Option<Value>, String> {
        match evidence
            .attachments
            .iter()
            .find(|a| a.name == name && !a.path.is_empty())
        {
            Some(a) => run.read_json(&run.root.join(&a.path)).map(Some),
            None => Ok(None),
        }
    };
    let tap = attachment("packet_tap")?;
    let entities = attachment("entities")?;
    let windows = attachment("windows")?;
    Ok(RowInput {
        evidence,
        tap,
        entities,
        windows,
    })
}

/// The run's identity from `run.json`, with the spec hash of `section`.
fn run_meta(run: &RunDir, section: &str) -> Result<RunMeta, String> {
    let manifest: Value = run.read_json(&run.run_json())?;
    let text = |v: Option<&Value>| v.and_then(Value::as_str).map(str::to_string);
    let spec_sha256 = manifest
        .get("specs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|s| s.get("section").and_then(Value::as_str) == Some(section))
        .and_then(|s| text(s.get("sha256")));
    Ok(RunMeta {
        run_id: text(manifest.get("run_id")).unwrap_or_default(),
        run_dir: run
            .root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        spec_sha256,
        server_build: text(manifest.pointer("/server/service_version")),
        sgw_sha256: text(manifest.pointer("/client/sgw_exe/sha256")),
    })
}

/// The run's fingerprint for `spec`'s section: every row (or only those
/// listed, in spec order) whose `golden.skip` is not set. Without a list, a
/// row the run did not reach is left out; a listed row the run lacks, or
/// one the spec does not have, is an error.
pub fn fingerprint_run(
    run: &RunDir,
    spec: &SectionSpec,
    rows: Option<&[String]>,
) -> Result<(RunFingerprint, RunMeta), String> {
    let section = spec.section.id.as_str();
    if let Some(listed) = rows {
        if let Some(missing) = listed
            .iter()
            .find(|id| !spec.rows.iter().any(|r| &r.id == *id))
        {
            return Err(format!("row {missing} is not in section {section}"));
        }
    }
    let mut out = Vec::new();
    for row in &spec.rows {
        if row.golden.as_ref().is_some_and(|g| g.skip) {
            continue;
        }
        match rows {
            Some(listed) if !listed.contains(&row.id) => continue,
            Some(_) => {}
            None if !run.row_json(section, &row.id).is_file() => continue,
            None => {}
        }
        let input =
            load_row_input(run, section, &row.id).map_err(|e| format!("row {}: {e}", row.id))?;
        let v = resolve(spec.section.golden.as_ref(), row.golden.as_ref());
        out.push(build_row(&input, &v));
    }
    Ok((
        RunFingerprint {
            schema: FINGERPRINT_SCHEMA,
            section: section.to_string(),
            rows: out,
        },
        run_meta(run, section)?,
    ))
}

/// The same as [`fingerprint_run`] with a given variance per row and no
/// spec: a golden's stored variance, so a diff applies the golden's model
/// rather than the spec's (README D-GD5).
pub fn fingerprint_run_with(
    run: &RunDir,
    section: &str,
    rows: &[(String, Variance)],
) -> Result<(RunFingerprint, RunMeta), String> {
    let mut out = Vec::new();
    for (row, v) in rows {
        let input = load_row_input(run, section, row).map_err(|e| format!("row {row}: {e}"))?;
        out.push(build_row(&input, v));
    }
    Ok((
        RunFingerprint {
            schema: FINGERPRINT_SCHEMA,
            section: section.to_string(),
            rows: out,
        },
        run_meta(run, section)?,
    ))
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;
