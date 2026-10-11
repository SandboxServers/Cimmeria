//! Byte-exact shapes of the events LX-08c emits. They come from core's
//! builders (the Windows launcher's), so SigNoz queries written for that
//! launcher keep matching; these fixtures fail if the desktop's inputs, or
//! the builders, change what goes on the wire.
use super::install_event::{report_for, Ending};
use super::patch_events::{injection, queue_at_exit, PatchEvents, PatchInjection};
use super::transport::TelemetryEvent;
use super::*;
use crate::install_report::{
    HashEvidence, InstallReport, PatchOutcome, PatchOutcomeKind, RunResult,
};
use cimmeria_launcher_core::telemetry::install_result::install_result_event;
use cimmeria_launcher_core::telemetry::patch_log::LOG_FILE_NAME;
use cimmeria_launcher_core::telemetry::queue::DiskQueue;
use std::sync::Mutex;
use std::time::Instant;

fn json(event: &TelemetryEvent) -> String {
    serde_json::to_string(event).unwrap()
}

/// A Repair whose patch set refused a modified stock file.
fn patches_failed() -> InstallReport {
    let mut r = InstallReport {
        seed_applied: true,
        ..InstallReport::default()
    };
    r.push("001-base", PatchOutcomeKind::Applied, None);
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
    r.result = RunResult::PatchesFailed;
    r
}

#[test]
fn install_result_with_a_failed_patch_is_byte_exact() {
    let report = report_for(Some(patches_failed()), Ending::Failed("repair_failed"));
    assert_eq!(
        json(&install_result_event(&report, 42)),
        concat!(
            r#"{"type":"client_native","ts_ms":42,"seq":0,"#,
            r#""target":"client.launcher.install_result","level":"warn","fields":{"#,
            r#""already":0,"applied":1,"failed":1,"#,
            r#""patch.001-base":"applied","#,
            r#""patch.008-dialog-portraits":"failed","#,
            r#""patch.008-dialog-portraits.actual_sha256":"bb","#,
            r#""patch.008-dialog-portraits.expected_sha256":"aa","#,
            r#""patch.008-dialog-portraits.mismatch":"source_mismatch","#,
            r#""patch.008-dialog-portraits.path":"SGWGame/CookedPC/x.upk","#,
            r#""patch.008-dialog-portraits.reason":"Patch set error: x.upk does not match the stock client","#,
            r#""patch.009":"skipped_dependency","#,
            r#""patch.009.reason":"skipped, it builds on 008-dialog-portraits, which did not apply","#,
            r#""patch_count":3,"result":"patches_failed","seed_applied":true,"skipped_dependency":1}}"#,
        )
    );
}

#[test]
fn a_stage_that_fails_after_every_patch_applied_is_an_error_with_the_desktop_code() {
    let mut ok = InstallReport::default();
    ok.push("001", PatchOutcomeKind::Applied, None);
    let report = report_for(Some(ok), Ending::Failed("content_invalid"));
    assert_eq!(
        json(&install_result_event(&report, 7)),
        concat!(
            r#"{"type":"client_native","ts_ms":7,"seq":0,"#,
            r#""target":"client.launcher.install_result","level":"warn","fields":{"#,
            r#""already":0,"applied":1,"error":"content_invalid","failed":0,"#,
            r#""patch.001":"applied","patch_count":1,"result":"error","#,
            r#""seed_applied":false,"skipped_dependency":0}}"#,
        )
    );
}

#[test]
fn endings_without_a_report_keep_the_cores_result_names() {
    let result = |ending| report_for(None, ending);
    assert_eq!(result(Ending::Succeeded).result, RunResult::Ok);
    assert_eq!(result(Ending::Cancelled).result, RunResult::Cancelled);
    let lost = result(Ending::Uncertain);
    assert_eq!(lost.result, RunResult::Error);
    assert_eq!(lost.error.as_deref(), Some("reconciliation_required"));
    // A report's own error text wins over the desktop's code.
    let mut failed = InstallReport {
        result: RunResult::Error,
        error: Some("seed download failed".into()),
        ..InstallReport::default()
    };
    failed = report_for(Some(failed), Ending::Failed("install_failed"));
    assert_eq!(failed.error.as_deref(), Some("seed download failed"));
}

const BOOT_LOG: &str = "\
[cimmeria-client-patches +0ms] attached, version 0.1.0, host SGW.exe\r\n\
[cimmeria-client-patches +4ms] Client_NetIn_EntityMethodDispatch at 0x00c6f8f0: stock\r\n\
[cimmeria-client-patches +4ms] FEngineLoop::Tick at 0x00416ec0: already hooked (jump to 0x10002000), chaining\r\n\
[cimmeria-client-patches +5ms] Black Market installed: received calls go to the Lua table CimmeriaBM\r\n";

const COUNTS_LOG: &str = "\
[cimmeria-client-patches +900ms] claimed onBMOpen (#1 received)\r\n\
[cimmeria-client-patches +901ms] onBMOpen dropped: the global CimmeriaBM is not defined, so the UI overlay is not installed (#1)\r\n\
[cimmeria-client-patches +980ms] delivered onBMListings to CimmeriaBM.onBMListings (#1)\r\n";

#[test]
fn boot_verdict_then_final_counts_are_byte_exact_and_each_sent_once() {
    let dir = tempfile::tempdir().unwrap();
    let mut events = PatchEvents::new(dir.path(), PatchInjection::Injected);
    assert!(events.poll(Instant::now()).is_empty(), "no log yet");
    std::fs::write(dir.path().join(LOG_FILE_NAME), BOOT_LOG).unwrap();
    let boot = events.poll(Instant::now());
    assert_eq!(
        boot.len(),
        1,
        "the boot verdict, and no counts before a call"
    );
    assert_eq!(
        json(&boot[0]),
        concat!(
            r#"{"type":"client_native","ts_ms":0,"seq":0,"#,
            r#""target":"client.patches.boot","level":"info","fields":{"#,
            r#""dll_version":"0.1.0","#,
            r#""fingerprint.Client_NetIn_EntityMethodDispatch":"stock","#,
            r#""fingerprint.FEngineLoop::Tick":"chained","#,
            r#""fingerprint_ok":true,"injection":"injected","log_found":true,"verdict":"installed"}}"#,
        )
    );
    std::fs::write(
        dir.path().join(LOG_FILE_NAME),
        format!("{BOOT_LOG}{COUNTS_LOG}"),
    )
    .unwrap();
    let exit = events.finish();
    assert_eq!(exit.len(), 1, "the boot verdict is not sent twice");
    assert_eq!(
        json(&exit[0]),
        concat!(
            r#"{"type":"client_native","ts_ms":0,"seq":0,"#,
            r#""target":"client.patches.counts","level":"warn","fields":{"#,
            r#""claimed":1,"claimed.methods":"onBMOpen","counts_are_lower_bounds":true,"#,
            r#""decode_failed":0,"delivered":1,"delivered.methods":"onBMListings","#,
            r#""dropped_no_handler":0,"dropped_no_overlay":1,"#,
            r#""dropped_no_overlay.last_reason":"the global CimmeriaBM is not defined, so the UI overlay is not installed","#,
            r#""dropped_no_overlay.methods":"onBMOpen","dropped_queue_full":0,"dropped_total":1,"#,
            r#""final":true,"handler_failed":0,"lines":7,"log_capped":false,"#,
            r#""natives_registered":0,"not_local_player":0,"register_failed":0,"#,
            r#""send_refused":0,"sent":0,"tech_competency_nil":0}}"#,
        )
    );
    assert!(events.finish().is_empty());
}

#[test]
fn patches_left_out_report_at_once_without_reading_a_log() {
    assert_eq!(injection(true, false), PatchInjection::Injected);
    assert_eq!(injection(false, false), PatchInjection::OptedOut);
    assert_eq!(injection(false, true), PatchInjection::Unavailable);
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(LOG_FILE_NAME), BOOT_LOG).unwrap();
    let mut events = PatchEvents::new(dir.path(), PatchInjection::OptedOut);
    let boot = events.poll(Instant::now());
    assert_eq!(
        json(&boot[0]),
        concat!(
            r#"{"type":"client_native","ts_ms":0,"seq":0,"#,
            r#""target":"client.patches.boot","level":"warn","fields":{"#,
            r#""injection":"opted_out","log_found":false}}"#,
        )
    );
    assert!(events.finish().is_empty());
}

#[test]
fn exit_events_are_queued_stamped_only_for_an_opted_in_player() {
    let root = tempfile::tempdir().unwrap();
    let game = tempfile::tempdir().unwrap();
    let state = Mutex::new(DesktopState::open(root.path()).unwrap());
    let queue = DiskQueue::new(state.lock().unwrap().state_root());

    let mut events = PatchEvents::new(game.path(), PatchInjection::Unavailable);
    assert_eq!(queue_at_exit(&state, &mut events), 0);
    assert!(queue.drain::<TelemetryEvent>().unwrap().is_empty());

    state.lock().unwrap().set_game_telemetry(true).unwrap();
    let mut events = PatchEvents::new(game.path(), PatchInjection::Unavailable);
    assert_eq!(queue_at_exit(&state, &mut events), 1);
    let queued: Vec<TelemetryEvent> = queue.drain().unwrap();
    let TelemetryEvent::ClientNative(boot) = &queued[0] else {
        panic!("expected ClientNative, got {queued:?}");
    };
    assert_eq!(boot.target, "client.patches.boot");
    assert!(boot.ts_ms > 0, "stamped: no session stamps it later");
    assert_eq!(boot.fields["injection"], "unavailable");
}
