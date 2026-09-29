//! One `client.launcher.install_result` event per Install / Update run.
//!
//! The 2026-09-29 colo playtest had to ask the player for the launcher's
//! local log to learn why a patch had not applied. This event carries
//! what [`crate::install::install_all`] did to every manifest patch
//! (`applied`, `already`, `failed`, `skipped_dependency`), the reason for
//! each failure, and, for a hash check that failed (a patch set's stock
//! source or rebuilt result, or a download), the file and both sha256s.
//!
//! An install runs before any game session exists, so there is no
//! telemetry token to upload with yet. The event goes into the
//! launcher's on-disk telemetry queue (the same `DiskQueue` a session
//! drains), stamped with the install time, and ships with the first
//! flush of the next telemetry session. Nothing is queued unless the
//! player opted in to telemetry.

use std::path::Path;

use serde_json::{Map, Value};

use super::events::{ClientNativeEvent, TelemetryEvent};
use super::queue::DiskQueue;
use crate::install_report::{HashEvidence, InstallReport, PatchOutcomeKind, RunResult};

/// Target of the event (routed under `client.native`, in `client_target`).
pub const EVENT_TARGET: &str = "client.launcher.install_result";

/// Longest reason or error text carried, in characters. A patch-set
/// error names one file; anything longer is a runaway message.
const MAX_TEXT_CHARS: usize = 512;

/// Build the event. `ts_ms` is when the run ended.
pub fn install_result_event(report: &InstallReport, ts_ms: i64) -> TelemetryEvent {
    let mut f = Map::new();
    f.insert("result".into(), report.result.as_str().into());
    f.insert("seed_applied".into(), report.seed_applied.into());
    f.insert("patch_count".into(), report.patches.len().into());
    for kind in [
        PatchOutcomeKind::Applied,
        PatchOutcomeKind::Already,
        PatchOutcomeKind::Failed,
        PatchOutcomeKind::SkippedDependency,
    ] {
        f.insert(kind.as_str().into(), report.count(kind).into());
    }
    if let Some(e) = &report.error {
        f.insert("error".into(), bounded(e).into());
    }
    if let Some(ev) = &report.error_evidence {
        insert_evidence(&mut f, "error", ev);
    }
    for p in &report.patches {
        let key = format!("patch.{}", safe_key(&p.id));
        f.insert(key.clone(), p.outcome.as_str().into());
        if let Some(reason) = &p.reason {
            f.insert(format!("{key}.reason"), bounded(reason).into());
        }
        if let Some(ev) = &p.evidence {
            insert_evidence(&mut f, &key, ev);
        }
    }
    let healthy = matches!(report.result, RunResult::Ok | RunResult::Cancelled);
    TelemetryEvent::ClientNative(ClientNativeEvent {
        ts_ms,
        seq: 0,
        target: EVENT_TARGET.into(),
        level: if healthy { "info" } else { "warn" }.into(),
        fields: f,
    })
}

/// Queue the event for the next telemetry session, if the player opted
/// in. Returns whether it was queued. `state_dir` is the directory of
/// the launcher's telemetry queue (`crate::config::exe_dir()`).
pub fn queue_install_result(report: &InstallReport, opted_in: bool, state_dir: &Path) -> bool {
    if !opted_in {
        return false;
    }
    let event = install_result_event(report, chrono::Utc::now().timestamp_millis());
    match DiskQueue::new(state_dir).enqueue(&event) {
        Ok(()) => {
            tracing::info!(
                result = report.result.as_str(),
                patches = report.patches.len(),
                "install result queued for the next telemetry session"
            );
            true
        }
        Err(e) => {
            tracing::warn!(error = %e, reason = "queue_write_failed", "install result not queued");
            false
        }
    }
}

fn insert_evidence(f: &mut Map<String, Value>, prefix: &str, ev: &HashEvidence) {
    f.insert(format!("{prefix}.mismatch"), ev.kind.into());
    f.insert(format!("{prefix}.path"), bounded(&ev.path).into());
    f.insert(
        format!("{prefix}.expected_sha256"),
        bounded(&ev.expected_sha256).into(),
    );
    f.insert(
        format!("{prefix}.actual_sha256"),
        bounded(&ev.actual_sha256).into(),
    );
}

fn bounded(s: &str) -> String {
    s.chars().take(MAX_TEXT_CHARS).collect()
}

/// A patch id as a field-key segment. Ids come from the signed manifest,
/// but a key is kept to `[A-Za-z0-9_-]` all the same.
fn safe_key(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::install_report::PatchOutcome;

    fn report() -> InstallReport {
        let mut r = InstallReport::default();
        r.push("001-base", PatchOutcomeKind::Already, None);
        r.patches.push(PatchOutcome {
            id: "008-dialog-portraits".into(),
            outcome: PatchOutcomeKind::Failed,
            reason: Some("Patch set error: x.upk does not match the stock client".into()),
            evidence: Some(HashEvidence {
                kind: "source_mismatch",
                path: "SGWGame/CookedPC/x.upk".into(),
                expected_sha256: "aa".into(),
                actual_sha256: "bb".into(),
            }),
        });
        r.push(
            "009",
            PatchOutcomeKind::SkippedDependency,
            Some("skipped, it builds on 008-dialog-portraits, which did not apply".into()),
        );
        r.push("bm ui/overlay", PatchOutcomeKind::Applied, None);
        r.result = RunResult::PatchesFailed;
        r
    }

    fn parts(ev: &TelemetryEvent) -> (&ClientNativeEvent, &Map<String, Value>) {
        match ev {
            TelemetryEvent::ClientNative(e) => (e, &e.fields),
            other => panic!("expected ClientNative, got {other:?}"),
        }
    }

    #[test]
    fn carries_every_outcome_and_the_source_mismatch_hashes() {
        let ev = install_result_event(&report(), 42);
        let (e, f) = parts(&ev);
        assert_eq!(e.target, EVENT_TARGET);
        assert_eq!(e.level, "warn");
        assert_eq!(e.ts_ms, 42);
        assert_eq!(f["result"], "patches_failed");
        assert_eq!(f["patch_count"], 4);
        assert_eq!(
            (
                &f["applied"],
                &f["already"],
                &f["failed"],
                &f["skipped_dependency"]
            ),
            (&1.into(), &1.into(), &1.into(), &1.into())
        );
        assert_eq!(f["patch.001-base"], "already");
        assert!(!f.contains_key("patch.001-base.reason"));
        let p = "patch.008-dialog-portraits";
        assert_eq!(f[p], "failed");
        assert_eq!(f[&format!("{p}.mismatch")], "source_mismatch");
        assert_eq!(f[&format!("{p}.path")], "SGWGame/CookedPC/x.upk");
        assert_eq!(f[&format!("{p}.expected_sha256")], "aa");
        assert_eq!(f[&format!("{p}.actual_sha256")], "bb");
        assert_eq!(f["patch.009"], "skipped_dependency");
        assert!(f["patch.009.reason"]
            .as_str()
            .unwrap()
            .contains("builds on 008"));
        assert_eq!(f["patch.bm_ui_overlay"], "applied", "key is sanitized");
    }

    #[test]
    fn a_clean_run_is_info_and_a_long_error_is_bounded() {
        let mut ok = InstallReport::default();
        ok.push("001", PatchOutcomeKind::Applied, None);
        let ev = install_result_event(&ok, 1);
        let (e, _) = parts(&ev);
        assert_eq!(e.level, "info");

        let failed = InstallReport {
            result: RunResult::Error,
            error: Some("x".repeat(5_000)),
            ..InstallReport::default()
        };
        let ev = install_result_event(&failed, 1);
        let (e, f) = parts(&ev);
        assert_eq!(e.level, "warn");
        assert_eq!(f["error"].as_str().unwrap().len(), MAX_TEXT_CHARS);
    }

    #[test]
    fn queues_only_when_opted_in() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!queue_install_result(&report(), false, dir.path()));
        let q = DiskQueue::new(dir.path());
        assert!(q.drain::<TelemetryEvent>().unwrap().is_empty());

        assert!(queue_install_result(&report(), true, dir.path()));
        let queued: Vec<TelemetryEvent> = q.drain().unwrap();
        assert_eq!(queued.len(), 1);
        let (e, _) = parts(&queued[0]);
        assert_eq!(e.target, EVENT_TARGET);
        assert!(e.ts_ms > 0, "stamped with the install time");
    }
}
