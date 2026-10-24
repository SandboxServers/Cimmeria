use super::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
fn fixture(script: &str) -> (tempfile::TempDir, HelperCommand, Request) {
    let root = tempfile::tempdir().unwrap();
    let script_path = root.path().join("host.py");
    std::fs::write(&script_path, format!("import sys,json,time,os\nr=json.load(sys.stdin)\ndef event(e,**kw):\n print(json.dumps(dict(schema_version=1,operation_id=r['operation_id'],observation=dict(event=e,**kw))),flush=True)\n{script}\n")).unwrap();
    (
        root,
        HelperCommand {
            executable: "/usr/bin/python3".into(),
            arguments: vec![script_path.into_os_string()],
            directory: "/".into(),
            environment: BTreeMap::new(),
        },
        Request {
            schema_version: 1,
            operation_id: Uuid::new_v4(),
            exe: "Z:\\SGW.exe".into(),
            directory: "Z:\\".into(),
            dlls: vec![],
            args: vec![],
        },
    )
}
// These two start a python3 fixture and give it five seconds. The hosted
// macOS runner sometimes takes longer to start it (2026-10-05).
#[tokio::test]
#[cfg_attr(
    target_os = "macos",
    ignore = "flaky on the hosted macOS runner (timing); run with --ignored"
)]
async fn real_host_and_guest_namespace_and_early_exit() {
    let (_root, spec, request) = fixture("event('process_started',guest_pid=987)\ntime.sleep(.05)\nevent('process_exited',guest_pid=987,code=7)");
    let observations = Arc::new(Mutex::new(Vec::new()));
    let seen = observations.clone();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        run(
            spec,
            request,
            CancellationToken::new(),
            move |event| {
                seen.lock().unwrap().push(event);
                Ok(())
            },
            |_| {},
        ),
    )
    .await
    .unwrap();
    let Observation::ProcessExited {
        host_pid,
        guest_pid,
        code,
        early,
    } = result
    else {
        panic!("{result:?}");
    };
    assert_ne!(host_pid, guest_pid);
    assert_eq!(guest_pid, 987);
    assert_eq!(code, 7);
    assert!(early);
    assert_eq!(
        observations.lock().unwrap().as_slice(),
        &[
            Observation::HostStarted { host_pid },
            Observation::ProcessStarted {
                host_pid,
                guest_pid
            }
        ]
    );
}
#[tokio::test]
#[cfg_attr(
    target_os = "macos",
    ignore = "flaky on the hosted macOS runner (timing); run with --ignored"
)]
async fn lost_helper_wrong_guest_and_injection_uncertainty_are_not_success() {
    for script in [
        "event('process_started',guest_pid=987)",
        "event('process_started',guest_pid=987)\nevent('process_exited',guest_pid=988,code=0)",
        "event('unknown')",
        "print('x'*17000,flush=True)",
        "event('process_exited',guest_pid=987,code=0)",
    ] {
        let (_root, spec, request) = fixture(script);
        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(5),
                run(spec, request, CancellationToken::new(), |_| Ok(()), |_| {})
            )
            .await
            .unwrap(),
            Observation::Unknown
        );
    }
}
#[tokio::test]
async fn cancellation_before_spawn_does_not_invoke_helper_and_spawn_failure_is_known() {
    let (_root, spec, request) = fixture("raise Exception('must not run')");
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        run(spec, request, cancel, |_| panic!("no host"), |_| {}).await,
        Observation::Cancelled
    );
    let (_second, mut spec, _) = fixture("pass");
    spec.executable = "/missing-launch-fixture".into();
    let request = Request {
        schema_version: 1,
        operation_id: Uuid::new_v4(),
        exe: "x".into(),
        directory: "y".into(),
        dlls: vec![],
        args: vec![],
    };
    assert_eq!(
        run(
            spec,
            request,
            CancellationToken::new(),
            |_| panic!("no host"),
            |_| {}
        )
        .await,
        Observation::NotStarted
    );
}
#[test]
fn a_missing_executable_reports_spawn_not_found() {
    // A second live Dispatch keeps tracing's callsite interest per dispatcher.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (_root, mut spec, request) = fixture("pass");
    spec.executable = "/missing-launch-fixture".into();
    let reported = Arc::new(Mutex::new(Vec::new()));
    let seen = reported.clone();
    let (observation, events) = crate::test_capture::capture(|| {
        tracing::callsite::rebuild_interest_cache();
        rt.block_on(run(
            spec,
            request,
            CancellationToken::new(),
            |_| panic!("no host"),
            move |code| seen.lock().unwrap().push(code),
        ))
    });
    assert_eq!(observation, Observation::NotStarted);
    assert_eq!(
        reported.lock().unwrap().as_slice(),
        &[FailureCode::SpawnNotFound]
    );
    let logged: Vec<_> = events
        .iter()
        .filter(|e| e.target == "launcher.launch" && e.field("event") == Some("launch_not_started"))
        .collect();
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].level, tracing::Level::WARN);
    assert_eq!(logged[0].field("reason"), Some("spawn_failed"));
    assert_eq!(logged[0].field("failure_code"), Some("spawn_not_found"));
    assert_eq!(logged[0].field("os_error"), Some("2"));
}
#[test]
fn an_oversized_request_reports_spawn_failed() {
    // A second live Dispatch keeps tracing's callsite interest per dispatcher.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (_root, spec, mut request) = fixture("pass");
    request.dlls = vec!["x".repeat(MAX_MESSAGE).into()];
    let reported = Arc::new(Mutex::new(Vec::new()));
    let seen = reported.clone();
    let (observation, events) = crate::test_capture::capture(|| {
        tracing::callsite::rebuild_interest_cache();
        rt.block_on(run(
            spec,
            request,
            CancellationToken::new(),
            |_| panic!("no host"),
            move |code| seen.lock().unwrap().push(code),
        ))
    });
    assert_eq!(observation, Observation::NotStarted);
    assert_eq!(
        reported.lock().unwrap().as_slice(),
        &[FailureCode::SpawnFailed]
    );
    let logged: Vec<_> = events
        .iter()
        .filter(|e| e.target == "launcher.launch" && e.field("event") == Some("launch_not_started"))
        .collect();
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].level, tracing::Level::WARN);
    assert_eq!(logged[0].field("reason"), Some("request_too_large"));
    assert_eq!(logged[0].field("failure_code"), Some("spawn_failed"));
}
#[test]
fn a_spawn_failure_is_noted_on_the_launch_record() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (_r, mut state, plan) = super::super::tests::fixture();
    state.record_launch(&plan, Observation::Preparing).unwrap();
    let (_root, mut spec, request) = fixture("pass");
    spec.executable = "/missing-launch-fixture".into();
    let state = Mutex::new(state);
    let (updates, _) = watch::channel(Observation::Unknown);
    let observation = rt.block_on(run_recorded(
        &state,
        &plan,
        spec,
        request,
        CancellationToken::new(),
        &updates,
    ));
    assert_eq!(observation, Observation::NotStarted);
    assert_eq!(
        state.lock().unwrap().launch_failure(),
        Ok(Some(FailureCode::SpawnNotFound))
    );
}
#[test]
fn a_helper_not_started_is_noted_with_its_failure_code() {
    // A second live Dispatch keeps tracing's callsite interest per dispatcher.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (_root, spec, request) = fixture("event('not_started',stage='create_process',win32=5)");
    let reported = Arc::new(Mutex::new(None));
    let seen = reported.clone();
    let (observation, events) = crate::test_capture::capture(|| {
        tracing::callsite::rebuild_interest_cache();
        rt.block_on(async {
            tokio::time::timeout(
                Duration::from_secs(30),
                run(
                    spec,
                    request,
                    CancellationToken::new(),
                    |_| Ok(()),
                    move |code| *seen.lock().unwrap() = Some(code),
                ),
            )
            .await
            .unwrap()
        })
    });
    assert_eq!(observation, Observation::NotStarted);
    assert_eq!(
        *reported.lock().unwrap(),
        Some(FailureCode::SpawnPermissionDenied)
    );
    let logged: Vec<_> = events
        .iter()
        .filter(|e| e.target == "launcher.launch" && e.field("event") == Some("launch_not_started"))
        .collect();
    assert_eq!(logged.len(), 1);
    assert_eq!(logged[0].level, tracing::Level::WARN);
    assert_eq!(logged[0].field("reason"), Some("create_process_failed"));
    assert_eq!(
        logged[0].field("failure_code"),
        Some("spawn_permission_denied")
    );
    assert_eq!(logged[0].field("win32"), Some("5"));
}
#[test]
fn a_helper_not_started_is_noted_on_the_launch_record() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (_r, mut state, plan) = super::super::tests::fixture();
    state.record_launch(&plan, Observation::Preparing).unwrap();
    let (_root, spec, request) = fixture("event('not_started',stage='create_process',win32=2)");
    let state = Mutex::new(state);
    let (updates, _) = watch::channel(Observation::Unknown);
    let observation = rt.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(30),
            run_recorded(
                &state,
                &plan,
                spec,
                request,
                CancellationToken::new(),
                &updates,
            ),
        )
        .await
        .unwrap()
    });
    assert_eq!(observation, Observation::NotStarted);
    // The worker records the terminal observation; the noted failure survives it.
    state
        .lock()
        .unwrap()
        .record_launch(&plan, observation)
        .unwrap();
    assert_eq!(
        state.lock().unwrap().launch_failure(),
        Ok(Some(FailureCode::SpawnNotFound))
    );
}
