//! `resume_from` (AX-16, contract C10): continue an appended run at a row
//! instead of re-running the rows that already have evidence.

use serde_json::Value;

use super::{RunManifest, RunRequest};

/// The request-level errors, before any row runs.
pub(super) fn validate(req: &RunRequest) -> Result<(), String> {
    if req.resume_from.is_none() {
        return Ok(());
    }
    if req.run_dir.is_none() {
        return Err("resume_from needs run_dir".into());
    }
    if req.rows.is_some() {
        return Err("resume_from and rows are exclusive".into());
    }
    Ok(())
}

/// Index into `order` (selected rows as `"<section>/<row>"`, spec order) at
/// which the run starts. `auto` is the first row with no manifest entry
/// (`order.len()` when all have one).
pub(super) fn start_index(
    order: &[String],
    done: &[String],
    resume: &str,
) -> Result<usize, String> {
    if resume == "auto" {
        return Ok(order
            .iter()
            .position(|k| !done.contains(k))
            .unwrap_or(order.len()));
    }
    order
        .iter()
        .position(|k| k == resume)
        .ok_or_else(|| format!("resume_from: no row {resume} in the selected sections"))
}

/// The keys of the rows the run selected, in spec order, with the same
/// row filter `run_all` applies.
pub(super) fn order(req: &RunRequest) -> Vec<String> {
    let mut keys = Vec::new();
    for loaded in &req.sections {
        for row in &loaded.spec.rows {
            if let Some(only) = &req.rows {
                if !only.iter().any(|r| r == &row.id) {
                    continue;
                }
            }
            keys.push(format!("{}/{}", loaded.spec.section.id, row.id));
        }
    }
    keys
}

/// How many leading rows of the selection to skip; `0` without
/// `resume_from`.
pub(super) fn skip_count(req: &RunRequest, manifest: &RunManifest) -> Result<usize, String> {
    let Some(resume) = &req.resume_from else {
        return Ok(0);
    };
    let done: Vec<String> = manifest
        .rows
        .iter()
        .filter_map(|r| r.get("row").and_then(Value::as_str).map(str::to_string))
        .collect();
    start_index(&order(req), &done, resume)
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod tests;
