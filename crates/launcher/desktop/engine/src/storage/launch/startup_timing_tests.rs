//! A harmless protocol helper proves startup timing ends before game lifetime.
use super::*;
use crate::phase_timing::Context;
use crate::test_capture::capture;
use std::collections::BTreeMap;
use tokio_util::sync::CancellationToken;

#[test]
fn helper_handshake_emits_spawn_and_startup_times_without_starting_a_game() {
    let root = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let environment: BTreeMap<_, _> = ["SystemRoot", "WINDIR", "PATH", "TEMP", "TMP"]
        .into_iter()
        .filter_map(|key| std::env::var_os(key).map(|value| (key.into(), value)))
        .collect();
    #[cfg(windows)]
    let (executable, arguments) = {
        let script = root.path().join("helper.ps1");
        std::fs::write(&script, r#"$r = [Console]::ReadLine() | ConvertFrom-Json
@{schema_version=1;operation_id=$r.operation_id;observation=@{event='process_started';guest_pid=987}} | ConvertTo-Json -Depth 5 -Compress
@{schema_version=1;operation_id=$r.operation_id;observation=@{event='process_exited';guest_pid=987;code=0}} | ConvertTo-Json -Depth 5 -Compress
"#).unwrap();
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
        std::fs::write(&script, "import sys,json\nr=json.load(sys.stdin)\nfor e in [dict(event='process_started',guest_pid=987),dict(event='process_exited',guest_pid=987,code=0)]:\n print(json.dumps(dict(schema_version=1,operation_id=r['operation_id'],observation=e)),flush=True)\n").unwrap();
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
    let startup = started.field("startup_ms").unwrap().parse::<u64>().unwrap();
    let handshake = records
        .iter()
        .find(|e| {
            e.field("phase") == Some("play_helper_handshake")
                && e.field("event") == Some("phase_finished")
        })
        .unwrap();
    assert!(
        startup
            >= handshake
                .field("duration_ms")
                .unwrap()
                .parse::<u64>()
                .unwrap()
    );
    assert!(!root.path().join("never-executed.exe").exists());
}
