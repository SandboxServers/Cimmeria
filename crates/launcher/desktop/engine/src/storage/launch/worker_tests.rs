//! Native launch preparation tests, separate from the retained worker.
use super::*;

#[test]
fn login_server_refusal_finishes_mint_as_failed_rather_than_interrupted() {
    let (root, mut state, plan) = super::super::tests::fixture();
    state.preferences.login_servers = vec![LoginServer {
        name: "Invalid fixture".into(),
        url: "not a URL".into(),
    }];
    let state = Arc::new(Mutex::new(state));
    let (updates, _) = watch::channel(Observation::Unknown);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (result, events) = crate::test_capture::capture(|| {
        rt.block_on(execute(
            state,
            &plan,
            root.path().into(),
            CancellationToken::new(),
            &updates,
        ))
    });
    assert!(matches!(result, Ok(Observation::NotStarted)));
    assert!(events
        .iter()
        .any(|e| e.field("reason") == Some("login_servers_refused")));
    let finish = events
        .iter()
        .find(|e| {
            e.field("phase") == Some("play_telemetry_mint")
                && e.field("event") == Some("phase_finished")
        })
        .unwrap();
    assert_eq!(finish.field("outcome"), Some("failed"));
    assert!(!events
        .iter()
        .any(|e| e.field("phase") == Some("play_client_setup")));
}

// Inert PE32 header, sufficient for real client preparation, never spawned.
fn inert_client(exe: &Path) {
    let mut bytes = vec![0u8; 0x200];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[60..64].copy_from_slice(&0x128u32.to_le_bytes());
    bytes[0x128..0x12c].copy_from_slice(b"PE\0\0");
    bytes[0x13c..0x13e].copy_from_slice(&0xe0u16.to_le_bytes());
    bytes[0x140..0x142].copy_from_slice(&0x10bu16.to_le_bytes());
    bytes[0x186..0x188].copy_from_slice(&0x8140u16.to_le_bytes());
    std::fs::write(exe, bytes).unwrap();
}

#[test]
fn telemetry_is_injected_after_the_patches_and_only_with_its_session_marker() {
    let (root, _state, mut plan) = super::super::tests::fixture();
    let directory = root.path().canonicalize().unwrap();
    let game = plan.installation.content_root();
    inert_client(&game.join("Working/Binaries/SGW.exe"));
    let patches = super::super::tests::artifact(&directory, "patches.dll");
    let telemetry = super::super::tests::artifact(&directory, "telemetry.dll");
    plan.resources.client_patches = Some(patches.clone());
    plan.resources.client_telemetry = Some(telemetry.clone());
    let marker = game.join("Working/Binaries/sessions/current-session.json");

    // No session (the server gave none): the DLL stays out and nothing is written.
    let (_, request, ownership) =
        prepare_native(&plan, None, &plan.installation.login_servers).unwrap();
    assert_eq!(request.dlls, [patches.path()]);
    assert!(!marker.exists());
    drop(ownership);

    let session = crate::game_telemetry::Session::fixture();
    let (_, request, ownership) =
        prepare_native(&plan, Some(&session), &plan.installation.login_servers).unwrap();
    assert_eq!(request.dlls, [patches.path(), telemetry.path()]);
    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&marker).unwrap()).unwrap();
    assert_eq!(written["telemetry"]["enabled"], true);
    assert_eq!(written["telemetry"]["token"], "payload.sig");
    assert_eq!(written["session_id"], "session-fixture");
    drop(ownership);

    // A plan that does not carry the DLL ignores a session it is handed.
    plan.resources.client_telemetry = None;
    std::fs::remove_file(&marker).unwrap();
    let (_, request, _ownership) =
        prepare_native(&plan, Some(&session), &plan.installation.login_servers).unwrap();
    assert_eq!(request.dlls, [patches.path()]);
    assert!(!marker.exists());
}

#[cfg(unix)]
#[test]
fn a_session_marker_is_never_written_through_a_link_out_of_the_game() {
    let (root, _state, mut plan) = super::super::tests::fixture();
    let directory = root.path().canonicalize().unwrap();
    let game = plan.installation.content_root();
    inert_client(&game.join("Working/Binaries/SGW.exe"));
    plan.resources.client_telemetry =
        Some(super::super::tests::artifact(&directory, "telemetry.dll"));
    let outside = directory.join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, game.join("Working/Binaries/sessions")).unwrap();
    let session = crate::game_telemetry::Session::fixture();
    let (_, request, _ownership) =
        prepare_native(&plan, Some(&session), &plan.installation.login_servers).unwrap();
    assert!(request.dlls.is_empty(), "no marker, so no DLL");
    assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
}

#[test]
fn native_preparation_reads_locked_owner_and_retains_exclusion() {
    let (root, _state, plan) = super::super::tests::fixture();
    let game = plan.installation.content_root();
    let exe = game.join("Working/Binaries/SGW.exe");
    inert_client(&exe);

    // Windows exercises the dispatch preparation entry point; other hosts
    // exercise the same native preparation without enabling native dispatch.
    let prepared = if cfg!(windows) {
        prepare(&plan, root.path(), None, &plan.installation.login_servers)
    } else {
        prepare_native(&plan, None, &plan.installation.login_servers)
    };
    let (command, request, ownership) = prepared.unwrap();
    assert_eq!(request.operation_id, plan.id);
    assert_eq!(request.exe, exe);
    assert_eq!(command.directory, game.join("Working/Binaries"));
    assert_eq!(command.executable, plan.resources.helper.path());
    assert_eq!(&std::fs::read(&exe).unwrap()[0x186..0x188], &[0, 0x81]);
    assert!(crate::client_setup::login_servers::path(&game).is_file());
    let competitor = OpenOptions::new()
        .read(true)
        .write(true)
        .open(plan.installation.destination.join(".cimmeria-install.json"))
        .unwrap();
    assert!(competitor.try_lock().is_err(), "ownership must stay locked");
    drop(ownership);
    competitor.try_lock().unwrap();
}

#[test]
fn play_writes_the_saved_login_servers_and_not_the_default() {
    let (_root, mut state, plan) = super::super::tests::fixture();
    let game = plan.installation.content_root();
    inert_client(&game.join("Working/Binaries/SGW.exe"));
    // The install was admitted with the default list; the player's saved
    // list must win over it at Play.
    assert_eq!(
        plan.installation.login_servers,
        crate::client_setup::login_servers::default_servers()
    );
    let saved = vec![
        LoginServer {
            name: "Home".into(),
            url: "http://home.example:8081".into(),
        },
        LoginServer {
            name: "Lab".into(),
            url: "https://lab.example:8081".into(),
        },
    ];
    let revision = state.preferences().revision;
    state
        .save_client_options(saved.clone(), true, revision)
        .unwrap();
    let servers = state.launch_login_servers(&plan.installation).unwrap();
    assert_eq!(servers, saved);
    // Not adopted, so the settings view is not told of a kept list.
    assert_eq!(state.inspect().adopted_login_servers, None);
    let (_, _, _ownership) = prepare_native(&plan, None, &servers).unwrap();
    let written = std::fs::read_to_string(crate::client_setup::login_servers::path(&game)).unwrap();
    assert!(written.contains(r#"LoginMod.servers["Home"] = "http://home.example:8081""#));
    assert!(written.contains(r#"LoginMod.servers["Lab"] = "https://lab.example:8081""#));
    assert!(!written.contains("play.cimmeria.app"), "{written}");
}

#[tokio::test]
async fn the_telemetry_session_is_minted_from_the_saved_servers_not_the_install_list() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "session_id": "saved", "token": "payload.sig", "expires_at_ms": 1_700_028_800_000i64,
            "upload_endpoint": format!("{}/api/telemetry", server.uri()),
            "chunk_max_bytes": 1_048_576, "flush_interval_ms": 2_000,
        })))
        .expect(1)
        .mount(&server)
        .await;
    let (root, mut state, mut plan) = super::super::tests::fixture();
    let directory = root.path().canonicalize().unwrap();
    plan.resources.client_telemetry =
        Some(super::super::tests::artifact(&directory, "telemetry.dll"));
    state.set_game_telemetry(true).unwrap();
    // Installed with the default list; the player saved the mock server.
    assert_ne!(plan.installation.login_servers[0].url, server.uri());
    let saved = vec![LoginServer {
        name: "Home".into(),
        url: server.uri(),
    }];
    let revision = state.preferences().revision;
    state
        .save_client_options(saved.clone(), true, revision)
        .unwrap();
    let state = Arc::new(Mutex::new(state));

    let (servers, session) = servers_and_session(&state, &plan).await.unwrap().unwrap();
    assert_eq!(servers, saved);
    assert_eq!(
        session
            .expect("minted by the saved server")
            .marker()
            .session_id,
        "saved"
    );
}

/// The game-telemetry session records the code and pids the helper
/// reported, and claims none for an ending it did not observe.
#[test]
fn the_session_takes_the_exit_the_supervisor_observed() {
    let exited = Observation::ProcessExited {
        host_pid: 10,
        guest_pid: 20,
        code: -1073741819,
        early: true,
    };
    assert_eq!(
        telemetry::game_exit(exited),
        Some(crate::game_telemetry::session_loop::GameExit {
            host_pid: 10,
            guest_pid: 20,
            code: -1073741819,
            early: true,
        })
    );
    for other in [
        Observation::Unknown,
        Observation::NotStarted,
        Observation::Cancelled,
        Observation::ProcessStarted {
            host_pid: 10,
            guest_pid: 20,
        },
    ] {
        assert_eq!(telemetry::game_exit(other), None, "{other:?}");
    }
}

#[test]
fn an_empty_list_means_the_defaults_and_an_invalid_one_never_reaches_the_client() {
    let (_root, mut state, plan) = super::super::tests::fixture();
    assert_eq!(
        state.launch_login_servers(&plan.installation),
        Ok(crate::client_setup::login_servers::default_servers())
    );
    // preferences.json is not checked for a valid list on load, so Play
    // checks again before it writes anything.
    state.preferences.login_servers = vec![LoginServer {
        name: "Old".into(),
        url: "ftp://old.example".into(),
    }];
    assert_eq!(
        state.launch_login_servers(&plan.installation),
        Err(IntentError::Storage(StorageError::InvalidServerList))
    );
}
