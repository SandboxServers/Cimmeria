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
        },
    )
}
#[tokio::test]
async fn real_host_and_guest_namespace_and_early_exit() {
    let (_root, spec, request) = fixture("event('process_started',guest_pid=987)\ntime.sleep(.05)\nevent('process_exited',guest_pid=987,code=7)");
    let observations = Arc::new(Mutex::new(Vec::new()));
    let seen = observations.clone();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        run(spec, request, CancellationToken::new(), move |event| {
            seen.lock().unwrap().push(event);
            Ok(())
        }),
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
                run(spec, request, CancellationToken::new(), |_| Ok(()))
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
        run(spec, request, cancel, |_| panic!("no host")).await,
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
    };
    assert_eq!(
        run(spec, request, CancellationToken::new(), |_| panic!(
            "no host"
        ))
        .await,
        Observation::NotStarted
    );
}
