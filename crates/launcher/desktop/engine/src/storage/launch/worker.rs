use super::*;
use crate::game_telemetry::patch_events;
use futures_util::FutureExt;
use std::{panic::AssertUnwindSafe, sync::Mutex};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
pub struct Worker {
    id: Uuid,
    state: Arc<Mutex<DesktopState>>,
    cancel: CancellationToken,
    pub observation: watch::Receiver<Observation>,
}
impl Worker {
    pub fn operation_id(&self) -> Uuid {
        self.id
    }
    /// Cancellation is supported only before native dispatch commits Running.
    pub fn request_cancel(&self) -> Result<(), IntentError> {
        let mut owner = self.state.lock().map_err(|_| StorageError::Io)?;
        let operation = owner
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .ok_or(ContractError::UnknownOperation)?;
        if operation.id != self.id {
            return Err(ContractError::IdentityConflict.into());
        }
        if operation.state != OperationState::Starting {
            return Err(ContractError::InvalidTransition.into());
        }
        owner.operations_mut()?.request_cancel(self.id)?;
        self.cancel.cancel();
        Ok(())
    }
}
/// Retained task: dropping the view/Worker stops neither launch nor observation.
pub fn dispatch(state: Arc<Mutex<DesktopState>>, id: Uuid) -> Result<Worker, IntentError> {
    let handle = tokio::runtime::Handle::try_current().map_err(|_| StorageError::Io)?;
    let (plan, root) = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        let plan = owner
            .launch_plan()?
            .ok_or(ContractError::UnknownOperation)?;
        if plan.id != id {
            return Err(ContractError::IdentityConflict.into());
        }
        if owner
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state
            != OperationState::Starting
        {
            return Err(ContractError::InvalidTransition.into());
        }
        // Durable dispatch marker closes the gap before the task is polled.
        if owner.launch_observation()?.is_some() {
            return Err(ContractError::InvalidTransition.into());
        }
        owner.record_launch(&plan, Observation::Preparing)?;
        (plan, owner.state_root().to_path_buf())
    };
    let cancel = CancellationToken::new();
    let (updates, observation) = watch::channel(Observation::Preparing);
    let worker = Worker {
        id,
        state: state.clone(),
        cancel: cancel.clone(),
        observation,
    };
    handle.spawn(async move {
        let result = AssertUnwindSafe(execute(state.clone(), &plan, root, cancel, &updates))
            .catch_unwind()
            .await;
        let result = match result {
            Ok(Ok(result)) => result,
            _ => Observation::Unknown,
        };
        let commit = (|| {
            let mut owner = state.lock().map_err(|_| StorageError::Io)?;
            owner.record_launch(&plan, result)?;
            match result {
                Observation::ProcessExited { code: 0, .. } => {
                    owner
                        .operations_mut()?
                        .observe(id, OperationState::Succeeded)?;
                }
                Observation::ProcessExited { .. } | Observation::NotStarted => {
                    owner
                        .operations_mut()?
                        .observe(id, OperationState::Failed)?;
                }
                Observation::Cancelled => {
                    owner
                        .operations_mut()?
                        .observe(id, OperationState::Cancelled)?;
                }
                _ => {
                    owner.operations_mut()?.mark_uncertain(id)?;
                }
            }
            Ok::<_, IntentError>(())
        })();
        if commit.is_err() {
            if let Ok(mut owner) = state.lock() {
                if let Ok(operations) = owner.operations_mut() {
                    let _ = operations.mark_uncertain(id);
                }
            }
        }
        updates.send_replace(if commit.is_ok() {
            result
        } else {
            Observation::Unknown
        });
    });
    Ok(worker)
}
async fn execute(
    state: Arc<Mutex<DesktopState>>,
    plan: &Plan,
    root: PathBuf,
    cancel: CancellationToken,
    updates: &watch::Sender<Observation>,
) -> Result<Observation, IntentError> {
    if cancel.is_cancelled() {
        return Ok(Observation::Cancelled);
    }
    // All fallible preparation before a host spawn is a known NotStarted result.
    let resources = plan.resources.clone();
    if tokio::task::spawn_blocking(move || resources.verify())
        .await
        .map_err(|_| StorageError::Io)?
        .is_err()
    {
        return Ok(Observation::NotStarted);
    }
    let Some((servers, session)) = servers_and_session(&state, plan).await? else {
        return Ok(Observation::NotStarted);
    };
    let minted = session.is_some();
    let prepared = plan.clone();
    let setup =
        tokio::task::spawn_blocking(move || prepare(&prepared, &root, session.as_ref(), &servers))
            .await
            .map_err(|_| StorageError::Io)?;
    let Ok((spec, request, _ownership)) = setup else {
        return Ok(Observation::NotStarted);
    };
    if minted {
        let attached = request.dlls.len() > plan.resources.client_patches.iter().count();
        telemetry::record(
            &state,
            plan,
            if attached {
                crate::game_telemetry::Outcome::Attached
            } else {
                crate::game_telemetry::Outcome::SessionNotWritten
            },
        );
    }
    let binaries = spec.directory.clone();
    // Lazy: nothing is spawned until `run_game` awaits it, after Running.
    let supervised = supervisor::run(spec, request, cancel.clone(), |observation| {
        state
            .lock()
            .map_err(|_| StorageError::Io)?
            .record_launch(plan, observation)?;
        updates.send_replace(observation);
        Ok(())
    });
    run_game(&state, plan, &binaries, &cancel, supervised).await
}
/// Commits Running, awaits the supervised game, then reports the
/// client-patches events. `binaries` holds `SGW.exe` and the DLL's log.
pub(super) async fn run_game(
    state: &Arc<Mutex<DesktopState>>,
    plan: &Plan,
    binaries: &Path,
    cancel: &CancellationToken,
    supervised: impl std::future::Future<Output = Observation>,
) -> Result<Observation, IntentError> {
    let injection = {
        let mut owner = state.lock().map_err(|_| StorageError::Io)?;
        if cancel.is_cancelled() {
            return Ok(Observation::Cancelled);
        }
        owner
            .operations_mut()?
            .observe(plan.id, OperationState::Running)?;
        // The plan's resources are what Play resolved, so they say whether
        // the DLL went in; the policy says why it did not.
        owner.patch_injection(plan.resources.client_patches.is_some())
    };
    // TODO(LX-08b): poll these from the session loop; until then the boot
    // verdict and the final counts are queued once the game exits.
    let mut patches = patch_events::PatchEvents::new(binaries, injection);
    let outcome = supervised.await;
    // A game that never started has no client-patches log to report on.
    if matches!(
        outcome,
        Observation::ProcessExited { .. } | Observation::Unknown
    ) {
        // The log read and the queue append are file I/O: off the runtime.
        let state = state.clone();
        let _ =
            tokio::task::spawn_blocking(move || patch_events::queue_at_exit(&state, &mut patches))
                .await;
    }
    Ok(outcome)
}
/// The login servers this Play writes, resolved before a telemetry session is
/// minted so both use the same list. `None` is a refused list: nothing starts.
async fn servers_and_session(
    state: &Arc<Mutex<DesktopState>>,
    plan: &Plan,
) -> Result<Option<(Vec<LoginServer>, Option<crate::game_telemetry::Session>)>, IntentError> {
    let resolved = state
        .lock()
        .map_err(|_| StorageError::Io)?
        .launch_login_servers(&plan.installation);
    let servers = match resolved {
        Ok(servers) => servers,
        Err(error) => {
            tracing::warn!(operation = %plan.id, ?error, "launch login servers refused; Play not started");
            return Ok(None);
        }
    };
    // A session that cannot be had costs a bounded wait, never the launch.
    let session = telemetry::session(state, plan, &servers).await;
    Ok(Some((servers, session)))
}
// Guards live through the helper's full guest lifetime, preventing Repair/uninstall.
enum Ownership {
    Native {
        _owner: File,
    },
    #[cfg(target_os = "macos")]
    Wine {
        _resources: crate::mac_wine::prerequisites::prefix::Resources,
    },
}
fn prepare(
    plan: &Plan,
    root: &Path,
    session: Option<&crate::game_telemetry::Session>,
    servers: &[LoginServer],
) -> Result<
    (
        crate::helper_supervisor::HelperCommand,
        cimmeria_runtime_probe::game_launch::Request,
        Ownership,
    ),
    IntentError,
> {
    if plan.runtime.is_some() {
        #[cfg(target_os = "macos")]
        {
            let (spec, request, resources) = wine::prepare(plan, root, session, servers)?;
            return Ok((
                spec,
                request,
                Ownership::Wine {
                    _resources: resources,
                },
            ));
        }
        #[cfg(not(target_os = "macos"))]
        return Err(StorageError::Corrupt.into());
    }
    let _ = root;
    if !cfg!(windows) {
        return Err(StorageError::Corrupt.into());
    }
    prepare_native(plan, session, servers)
}
fn prepare_native(
    plan: &Plan,
    session: Option<&crate::game_telemetry::Session>,
    servers: &[LoginServer],
) -> Result<
    (
        crate::helper_supervisor::HelperCommand,
        cimmeria_runtime_probe::game_launch::Request,
        Ownership,
    ),
    IntentError,
> {
    let owner_path = plan.installation.destination.join(".cimmeria-install.json");
    let owner = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&owner_path)
        .map_err(|_| StorageError::Io)?;
    owner.try_lock().map_err(|_| StorageError::InUse)?;
    // Windows byte-range locks also exclude reads through a second handle.
    let saved: InstallIntent = read_open(&owner)?;
    if saved != plan.installation {
        return Err(StorageError::Corrupt.into());
    }
    let directory = preparation::prepare(plan, servers)?;
    let telemetry = telemetry::injected(plan, session);
    let request = cimmeria_runtime_probe::game_launch::Request {
        schema_version: 1,
        operation_id: plan.id,
        exe: directory.join("SGW.exe"),
        directory: directory.clone(),
        // Patches first: the telemetry DLL shares their install lock.
        dlls: plan
            .resources
            .client_patches
            .iter()
            .chain(telemetry)
            .map(|p| p.path().to_path_buf())
            .collect(),
    };
    let environment: std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString> = [
        "SystemRoot",
        "WINDIR",
        "PATH",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
    ]
    .into_iter()
    .filter_map(|key| std::env::var_os(key).map(|value| (key.into(), value)))
    .chain(
        telemetry
            .iter()
            .flat_map(|_| crate::game_telemetry::passthrough_environment())
            .map(|(key, value)| (key.into(), value.into())),
    )
    .collect();
    Ok((
        crate::helper_supervisor::HelperCommand {
            executable: plan.resources.helper.path().into(),
            arguments: vec![],
            directory,
            environment,
        },
        request,
        Ownership::Native { _owner: owner },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let game = plan.installation.destination.join("game");
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
        let game = plan.installation.destination.join("game");
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
        let game = plan.installation.destination.join("game");
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
        let game = plan.installation.destination.join("game");
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
        let written =
            std::fs::read_to_string(crate::client_setup::login_servers::path(&game)).unwrap();
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
}
