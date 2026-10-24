//! A harmless protocol helper proves startup timing ends before game lifetime.
use super::*;
use crate::phase_timing::Context;
use crate::test_capture::capture;
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;

fn run_helper(starts: bool) -> (Observation, Vec<crate::test_capture::Captured>) {
    let root = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let environment: BTreeMap<_, _> = ["SystemRoot", "WINDIR", "PATH", "TEMP", "TMP"]
        .into_iter()
        .filter_map(|key| std::env::var_os(key).map(|value| (key.into(), value)))
        .collect();
    #[cfg(windows)]
    let (executable, arguments) = {
        let script = root.path().join("helper.ps1");
        let reply = if starts {
            r#"
@{schema_version=1;operation_id=$r.operation_id;observation=@{event='process_started';guest_pid=987}} | ConvertTo-Json -Depth 5 -Compress
@{schema_version=1;operation_id=$r.operation_id;observation=@{event='process_exited';guest_pid=987;code=0}} | ConvertTo-Json -Depth 5 -Compress
"#
        } else {
            r#"
@{schema_version=1;operation_id=$r.operation_id;observation=@{event='not_started'}} | ConvertTo-Json -Depth 5 -Compress
"#
        };
        std::fs::write(
            &script,
            format!("$r = [Console]::ReadLine() | ConvertFrom-Json\n{reply}"),
        )
        .unwrap();
        let exe = std::path::PathBuf::from(std::env::var_os("ProgramFiles").unwrap())
            .join("PowerShell/7/pwsh.exe");
        assert!(
            exe.is_file(),
            "PowerShell 7 is the validated Windows test runtime"
        );
        (
            exe,
            vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-File".into(),
                script.into_os_string(),
            ],
        )
    };
    #[cfg(not(windows))]
    let (executable, arguments) = {
        let script = root.path().join("helper.py");
        let replies = if starts {
            "[dict(event='process_started',guest_pid=987),dict(event='process_exited',guest_pid=987,code=0)]"
        } else {
            "[dict(event='not_started')]"
        };
        std::fs::write(&script, format!("import sys,json\nr=json.load(sys.stdin)\nfor e in {replies}:\n print(json.dumps(dict(schema_version=1,operation_id=r['operation_id'],observation=e)),flush=True)\n")).unwrap();
        (
            std::path::PathBuf::from("/usr/bin/python3"),
            vec![script.into_os_string()],
        )
    };
    let spec = crate::helper_supervisor::HelperCommand {
        executable,
        arguments,
        directory: root.path().into(),
        environment,
        stderr_log: None,
    };
    let request = cimmeria_runtime_probe::game_launch::Request {
        schema_version: 1,
        operation_id: id,
        exe: root.path().join("never-executed.exe"),
        directory: root.path().into(),
        dlls: vec![],
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (observation, records) = capture(|| {
        rt.block_on(async {
            Context::for_operation(id);
            supervisor::run(spec, request, CancellationToken::new(), |_| Ok(())).await
        })
    });
    assert!(!root.path().join("never-executed.exe").exists());
    (observation, records)
}

#[test]
fn helper_handshake_emits_spawn_and_startup_times_without_starting_a_game() {
    let (observation, records) = run_helper(true);
    assert!(matches!(
        observation,
        Observation::ProcessExited { code: 0, .. }
    ));
    for phase in [
        "play_helper_start",
        "play_helper_spawn",
        "play_helper_handshake",
    ] {
        let pair: Vec<_> = records
            .iter()
            .filter(|e| e.field("phase") == Some(phase))
            .collect();
        assert_eq!(pair.len(), 2, "{phase}");
        assert_eq!(pair[1].field("outcome"), Some("succeeded"));
        assert!(pair[1].field("duration_ms").unwrap().parse::<u64>().is_ok());
    }
    let started = records
        .iter()
        .find(|e| e.field("event") == Some("game_started"))
        .unwrap();
    assert!(started.field("startup_ms").unwrap().parse::<u64>().is_ok());
    // Cross-clock inequalities depend on the global bounded correlation cache.
    // Event ordering proves startup finishes before the guest lifetime instead.
    let at = |event: &str| {
        records
            .iter()
            .position(|e| e.field("event") == Some(event))
            .unwrap()
    };
    let startup_finish = records
        .iter()
        .position(|e| {
            e.field("phase") == Some("play_helper_start")
                && e.field("event") == Some("phase_finished")
        })
        .unwrap();
    assert!(startup_finish < at("game_started"));
    assert!(at("game_started") < at("game_exited"));
}

#[test]
fn helper_refusal_finishes_startup_as_failed_rather_than_interrupted() {
    let (observation, records) = run_helper(false);
    assert!(matches!(observation, Observation::NotStarted));
    for phase in ["play_helper_start", "play_helper_handshake"] {
        let finish = records
            .iter()
            .find(|e| e.field("phase") == Some(phase) && e.field("event") == Some("phase_finished"))
            .unwrap();
        assert_eq!(finish.field("outcome"), Some("failed"), "{phase}");
    }
    assert!(!records
        .iter()
        .any(|e| e.field("event") == Some("game_started")));
}
