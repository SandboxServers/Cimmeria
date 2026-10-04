use super::super::tests::{make_worker, recv_matching};
use super::super::{Command, LaunchTelemetryConfig};
use super::*;
use crate::client_patches::dll_source::DllSource;
use crate::telemetry::session::current_session_path;

fn opted_out() -> ClientPatchesSettings {
    ClientPatchesSettings {
        enabled: false,
        dll_override: None,
    }
}

fn args(req: &Request) -> Vec<String> {
    req.to_args()
        .into_iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

/// The request of a launch's first attempt, as `start_game` builds it.
fn first_request(patches: &InjectDecision, telemetry: &TelemetryDll) -> Request {
    let patches = match patches {
        InjectDecision::Inject(src) => Some(src.path()),
        _ => None,
    };
    let attempts = launch_attempts(patches, telemetry.path());
    helper_request(
        PathBuf::from("C:/SGW/Working/Binaries"),
        PathBuf::from("C:/SGW/Working/Binaries/SGW.exe"),
        &attempts[0],
    )
}

fn patches_on() -> InjectDecision {
    InjectDecision::Inject(DllSource::Bundled(PathBuf::from(
        "C:/L/client-patches/ab/cimmeria-client-patches.dll",
    )))
}

/// Opt-in off: the helper gets exactly today's command line, the client
/// patches and nothing else.
#[test]
fn the_launch_request_without_the_opt_in_injects_only_the_client_patches() {
    assert_eq!(
        args(&first_request(&patches_on(), &TelemetryDll::NotOptedIn)),
        vec![
            "spawn",
            "C:/SGW/Working/Binaries/SGW.exe",
            "--cwd",
            "C:/SGW/Working/Binaries",
            "--dll",
            "C:/L/client-patches/ab/cimmeria-client-patches.dll",
        ]
    );
}

/// Opt-in on: the client patches first, then the telemetry DLL, the
/// order the lab's `launch_request` uses.
#[test]
fn the_launch_request_with_the_opt_in_injects_patches_then_telemetry() {
    let tel = TelemetryDll::Inject(PathBuf::from(
        "C:/L/client-telemetry/cd/cimmeria-client-telemetry.dll",
    ));
    assert_eq!(
        args(&first_request(&patches_on(), &tel)),
        vec![
            "spawn",
            "C:/SGW/Working/Binaries/SGW.exe",
            "--cwd",
            "C:/SGW/Working/Binaries",
            "--dll",
            "C:/L/client-patches/ab/cimmeria-client-patches.dll",
            "--dll",
            "C:/L/client-telemetry/cd/cimmeria-client-telemetry.dll",
        ]
    );
}

/// Opted in but no DLL (or no session): the same command line as without
/// the opt-in.
#[test]
fn an_opted_in_launch_without_the_dll_is_the_plain_patches_launch() {
    let without = args(&first_request(&patches_on(), &TelemetryDll::NotOptedIn));
    for tel in [
        TelemetryDll::Unavailable("missing".into()),
        TelemetryDll::NoSession("auth down".into()),
    ] {
        assert_eq!(
            args(&first_request(&patches_on(), &tel)),
            without,
            "{tel:?}"
        );
    }
}

/// A failed injection of both DLLs retries with the patches alone, so a
/// telemetry failure never costs the player the Black Market window.
#[test]
fn a_failed_two_dll_launch_retries_with_the_patches_alone() {
    let p = Path::new("p.dll");
    let t = Path::new("t.dll");
    assert_eq!(
        launch_attempts(Some(p), Some(t)),
        vec![
            vec![p.to_path_buf(), t.to_path_buf()],
            vec![p.to_path_buf()]
        ]
    );
    assert_eq!(launch_attempts(Some(p), None), vec![vec![p.to_path_buf()]]);
    assert_eq!(launch_attempts(None, Some(t)), vec![vec![t.to_path_buf()]]);
    // No DLL at all: straight to the plain launch.
    assert!(launch_attempts(None, None).is_empty());
}

#[test]
fn the_launched_label_names_what_went_in() {
    assert_eq!(
        launched_label(true, true),
        "SGW.exe with client patches and telemetry"
    );
    assert_eq!(launched_label(true, false), "SGW.exe with client patches");
    assert_eq!(launched_label(false, true), "SGW.exe with telemetry");
    assert_eq!(launched_label(false, false), "SGW.exe");
}

/// With no SGW.exe the launch fails once, with the game's error, and
/// the opt-out still gets its status line first.
#[test]
fn launch_sgw_opted_out_reports_the_opt_out_then_the_missing_game() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, rt) = make_worker();
    worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
        prep: None,
        install_dir: dir.path().to_path_buf(),
        client_patches: opted_out(),
        telemetry: None,
    }));
    let (note, err) = rt.block_on(async {
        let note = recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::ClientPatchesNote(_))
        })
        .await;
        let err = recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::Launched(..) | Event::LaunchError(_))
        })
        .await;
        (note, err)
    });
    match note {
        Event::ClientPatchesNote(n) => assert!(n.contains("launcher setting"), "{n}"),
        other => panic!("expected ClientPatchesNote, got {other:?}"),
    }
    match err {
        Event::LaunchError(msg) => assert!(msg.contains("SGW.exe"), "{msg}"),
        other => panic!("expected LaunchError, got {other:?}"),
    }
}

/// A configured DLL that is missing is reported, and the launch
/// carries on without it.
#[test]
fn launch_sgw_reports_a_missing_override_dll() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, rt) = make_worker();
    worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
        prep: None,
        install_dir: dir.path().to_path_buf(),
        client_patches: ClientPatchesSettings {
            enabled: true,
            dll_override: Some(dir.path().join("missing.dll")),
        },
        telemetry: None,
    }));
    let note = rt.block_on(recv_matching(&mut worker.events_rx, |e| {
        matches!(e, Event::ClientPatchesNote(_))
    }));
    match note {
        Event::ClientPatchesNote(n) => assert!(n.contains("missing.dll"), "{n}"),
        other => panic!("expected ClientPatchesNote, got {other:?}"),
    }
}

/// With an injectable DLL but no SGW.exe, the error is the game's,
/// reported once, and the patches are not blamed.
#[test]
fn launch_sgw_missing_game_is_not_blamed_on_the_patches() {
    let dir = tempfile::tempdir().unwrap();
    let dll = dir.path().join("p.dll");
    std::fs::write(&dll, b"MZ").unwrap();
    let (mut worker, rt) = make_worker();
    worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
        prep: None,
        install_dir: dir.path().to_path_buf(),
        client_patches: ClientPatchesSettings {
            enabled: true,
            dll_override: Some(dll),
        },
        telemetry: None,
    }));
    let ev = rt.block_on(recv_matching(&mut worker.events_rx, |e| {
        matches!(
            e,
            Event::Launched(..) | Event::LaunchError(_) | Event::ClientPatchesNote(_)
        )
    }));
    match ev {
        Event::LaunchError(msg) => assert!(msg.contains("SGW.exe"), "{msg}"),
        other => panic!("expected only a LaunchError, got {other:?}"),
    }
}

/// A build with the patches DLL but no `sgw-start32.exe` (a dev build:
/// nothing is embedded in tests and none sits beside the test binary)
/// says so, then still starts the game plainly.
#[cfg(windows)]
#[test]
fn launch_sgw_without_the_helper_reports_it_and_launches_plainly() {
    let dir = tempfile::tempdir().unwrap();
    copy_a_stand_in_game(dir.path());
    let dll = dir.path().join("p.dll");
    std::fs::write(&dll, b"MZ").unwrap();
    let (mut worker, rt) = make_worker();
    worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
        prep: None,
        install_dir: dir.path().to_path_buf(),
        client_patches: ClientPatchesSettings {
            enabled: true,
            dll_override: Some(dll),
        },
        telemetry: None,
    }));
    let (note, launched) = rt.block_on(async {
        let note = recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::ClientPatchesNote(_))
        })
        .await;
        let launched = recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::Launched(..) | Event::LaunchError(_))
        })
        .await;
        (note, launched)
    });
    match note {
        Event::ClientPatchesNote(n) => {
            assert!(n.contains("sgw-start32.exe"), "{n}");
            assert!(n.contains("not loaded"), "{n}");
        }
        other => panic!("expected ClientPatchesNote, got {other:?}"),
    }
    match launched {
        Event::Launched(name, _) => assert_eq!(name, "SGW.exe"),
        other => panic!("expected a plain launch, got {other:?}"),
    }
}

#[cfg(windows)]
fn copy_a_stand_in_game(dir: &Path) {
    let sys = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32");
    std::fs::copy(sys.join("HOSTNAME.EXE"), dir.join("SGW.exe")).unwrap();
}

fn telemetry_config(auth_base_url: String, state_dir: &Path) -> LaunchTelemetryConfig {
    LaunchTelemetryConfig {
        auth_base_url,
        install_id: "i".into(),
        machine_id: "m".into(),
        branch: "main".into(),
        git_sha: "abc".into(),
        launcher_version: "0.1.0".into(),
        state_dir: state_dir.to_path_buf(),
        tags: vec![],
        login_server_urls: vec![],
    }
}

/// A mock admin API that mints a player session.
async fn mock_auth() -> wiremock::MockServer {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth/dev-session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "session_id": "sid-1",
            "token": "tok",
            "expires_at_ms": 1_700_028_800_000_i64,
            "upload_endpoint": format!("{}/api/telemetry", server.uri()),
            "chunk_max_bytes": 1_048_576,
            "flush_interval_ms": 2_000,
        })))
        .mount(&server)
        .await;
    server
}

/// Opted in, session minted, but no telemetry DLL anywhere (a dev build
/// with nothing beside the test binary): the session file is written
/// before the game starts, the player is told the DLL is missing, and the
/// launch carries on without it (to the missing-game error here: a game
/// that ran would start a real session, whose disk queue beside the test
/// binary other tests share).
#[test]
fn an_opted_in_launch_with_no_telemetry_dll_warns_and_launches_without_it() {
    let dir = tempfile::tempdir().unwrap();
    let (mut worker, rt) = make_worker();
    let server = rt.block_on(mock_auth());
    worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
        prep: None,
        install_dir: dir.path().to_path_buf(),
        client_patches: opted_out(),
        telemetry: Some(telemetry_config(server.uri(), dir.path())),
    }));
    let (note, launched) = rt.block_on(async {
        let note = recv_matching(&mut worker.events_rx, |e| {
            matches!(
                e,
                Event::ClientTelemetryNote(_) | Event::Launched(..) | Event::LaunchError(_)
            )
        })
        .await;
        let launched = recv_matching(&mut worker.events_rx, |e| {
            matches!(e, Event::Launched(..) | Event::LaunchError(_))
        })
        .await;
        (note, launched)
    });
    match note {
        Event::ClientTelemetryNote(n) => {
            assert!(n.contains("unavailable"), "{n}");
            assert!(n.contains(client_telemetry_dll::DLL_FILE_NAME), "{n}");
        }
        other => panic!("expected the telemetry note before the launch, got {other:?}"),
    }
    assert!(
        current_session_path(dir.path()).is_file(),
        "the session file must exist before the game starts"
    );
    match launched {
        Event::LaunchError(msg) => assert!(msg.contains("SGW.exe"), "{msg}"),
        other => panic!("expected the missing game's error, got {other:?}"),
    }
}

/// Opted in, but the telemetry server refuses: the session error is
/// reported, the DLL is not even looked for, and the game still launches.
#[test]
fn an_opted_in_launch_whose_session_fails_still_launches() {
    let dir = tempfile::tempdir().unwrap();
    #[cfg(windows)]
    copy_a_stand_in_game(dir.path());
    let (mut worker, rt) = make_worker();
    // No mocks mounted: the handshake gets a 404.
    let server = rt.block_on(wiremock::MockServer::start());
    worker.dispatch(Command::LaunchSgw(LaunchSgwRequest {
        prep: None,
        install_dir: dir.path().to_path_buf(),
        client_patches: opted_out(),
        telemetry: Some(telemetry_config(server.uri(), dir.path())),
    }));
    let (first, launched) = rt.block_on(async {
        let first = recv_matching(&mut worker.events_rx, |e| {
            matches!(
                e,
                Event::TelemetrySessionError(_)
                    | Event::ClientTelemetryNote(_)
                    | Event::Launched(..)
                    | Event::LaunchError(_)
            )
        })
        .await;
        let launched = recv_matching(&mut worker.events_rx, |e| {
            matches!(
                e,
                Event::ClientTelemetryNote(_) | Event::Launched(..) | Event::LaunchError(_)
            )
        })
        .await;
        (first, launched)
    });
    match first {
        Event::TelemetrySessionError(e) => assert!(e.contains("auth handshake"), "{e}"),
        other => panic!("expected the session error first, got {other:?}"),
    }
    assert!(!current_session_path(dir.path()).exists());
    #[cfg(windows)]
    match launched {
        Event::Launched(name, _) => assert_eq!(name, "SGW.exe"),
        other => panic!("expected a plain launch and no DLL note, got {other:?}"),
    }
    #[cfg(not(windows))]
    assert!(matches!(launched, Event::LaunchError(_)), "{launched:?}");
}
