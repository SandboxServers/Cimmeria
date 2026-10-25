use super::sign_in_play;
use super::*;
use crate::failure::FailureCode;
use crate::game_telemetry::patch_events;
use crate::storage::display_options::{DisplayOptions, Placement};
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
        let (adoption_check, patches_incomplete) = super::adoption_tags::adoption_tags(&owner);
        events::launch_started(
            plan.id,
            plan.resources.client_telemetry.is_some(),
            adoption_check,
            patches_incomplete,
        );
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
            Ok(Err(error)) => {
                events::launch_failed(plan.id, error.code());
                Observation::Unknown
            }
            Err(_) => {
                events::launch_failed(plan.id, "panicked");
                Observation::Unknown
            }
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
            events::launch_failed(plan.id, "commit_failed");
            if let Ok(mut owner) = state.lock() {
                if let Ok(operations) = owner.operations_mut() {
                    if let Err(error) = operations.mark_uncertain(id) {
                        events::mark_uncertain_failed(error);
                    }
                }
            }
        }
        events::launch_finished(
            plan.id,
            if commit.is_ok() {
                result
            } else {
                Observation::Unknown
            },
        );
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
    // Every Play start clears the last refusal first (C9).
    state
        .lock()
        .map_err(|_| StorageError::Io)?
        .set_play_refusal(None);
    if cancel.is_cancelled() {
        return Ok(Observation::Cancelled);
    }
    // All fallible preparation before a host spawn is a known NotStarted result.
    let resources = plan.resources.clone();
    let context = crate::phase_timing::Context::for_operation(plan.id);
    let resource_timer = context.start(crate::phase_timing::Phase::PlayResources);
    let verified = tokio::task::spawn_blocking(move || resources.verify())
        .await
        .map_err(|_| StorageError::Io)?;
    if resource_timer.result(verified).is_err() {
        failure::not_started(&state, plan, "verify_failed", FailureCode::GameFilesInvalid);
        return Ok(Observation::NotStarted);
    }
    let mint_timer = context.start(crate::phase_timing::Phase::PlayTelemetryMint);
    let prepared = match servers_and_session(&state, plan).await {
        Ok(prepared) => prepared,
        Err(error) => {
            mint_timer.finish(crate::phase_timing::Outcome::Failed);
            return Err(error);
        }
    };
    let Some((servers, session)) = prepared else {
        mint_timer.finish(crate::phase_timing::Outcome::Failed);
        return Ok(Observation::NotStarted);
    };
    mint_timer.finish(if session.is_some() {
        crate::phase_timing::Outcome::Succeeded
    } else {
        crate::phase_timing::Outcome::Unavailable
    });
    let minted = session.is_some();
    let display = display::options(&state);
    // The monitor per display setup: its refresh rate feeds the Mac frame cap (PD-35),
    // and in windowed mode the helper opens the game on it (PD-33).
    let placements = state
        .lock()
        .map(|owner| owner.preferences.display_placements.clone())
        .unwrap_or_default();
    let place_on = window_target(&placements, display.effective().mode);
    // Sign-in (C9): the relay or ticket is started here and owned by `login` until `finish`.
    let discovery = sign_in_play::discover(&servers).await;
    let sign_in = state.lock().map_err(|_| StorageError::Io)?.sign_in();
    let planned = sign_in_play::plan(sign_in, plan, &servers, discovery).await;
    let (client_servers, login) = match planned {
        Ok(planned) => planned,
        Err(refusal) => {
            state
                .lock()
                .map_err(|_| StorageError::Io)?
                .set_play_refusal(Some(refusal.reason()));
            let code = match refusal {
                sign_in_play::TicketRefusal::Unavailable => FailureCode::TicketUnavailable,
                sign_in_play::TicketRefusal::SignInEnded => FailureCode::SignInEnded,
            };
            failure::not_started(&state, plan, refusal.reason(), code);
            return Ok(Observation::NotStarted);
        }
    };
    let (telemetry_servers, client_servers) = sign_in_play::lists_for(&servers, client_servers);
    let telemetry_session = session
        .clone()
        .map(|session| (session, telemetry_servers.to_vec()));
    let prepared = plan.clone();
    // The ticket file lives in the game install, where `prepare` writes the plain list.
    let ticket_root = plan.installation.content_root();
    let setup_timer = context.start(crate::phase_timing::Phase::PlayClientSetup);
    let setup_result = tokio::task::spawn_blocking(move || {
        prepare(
            &prepared,
            &root,
            session.as_ref(),
            &client_servers,
            display,
            &placements,
        )
    })
    .await;
    // A panicked `prepare` still ends the ticket and stops the relay.
    let setup = match setup_result {
        Ok(setup) => setup,
        Err(_) => {
            sign_in_play::finish(&ticket_root, &servers, login).await;
            return Err(StorageError::Io.into());
        }
    };
    let Ok((spec, mut request, _ownership)) = setup_timer.result(setup) else {
        sign_in_play::finish(&ticket_root, &servers, login).await;
        failure::not_started(
            &state,
            plan,
            "prepare_failed",
            FailureCode::LaunchPrepareFailed,
        );
        return Ok(Observation::NotStarted);
    };
    if sign_in_play::write(&ticket_root, &login).is_err() {
        sign_in_play::finish(&ticket_root, &servers, login).await;
        failure::not_started(
            &state,
            plan,
            "ticket_write_failed",
            FailureCode::TicketUnavailable,
        );
        return Ok(Observation::NotStarted);
    }
    request.place_on = place_on;
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
    let settled = match telemetry_session {
        Some((session, servers)) => {
            let timer = context.start(crate::phase_timing::Phase::PlayPreviousSession);
            let settled = telemetry::open_session(&state, plan, &session, &servers).await;
            timer.finish(if settled.is_some() {
                crate::phase_timing::Outcome::Succeeded
            } else {
                crate::phase_timing::Outcome::Unavailable
            });
            settled
        }
        None => None,
    };
    let binaries = spec.directory.clone();
    // Lazy: nothing is spawned until `run_game` awaits it, after Running.
    let supervised = supervisor::run_recorded(&state, plan, spec, request, cancel.clone(), updates);
    // The heartbeat never returns; dropping it when the game ends is its stop.
    let observed = tokio::select! {
        result = run_game(&state, plan, &binaries, &cancel, settled, supervised) => result,
        () = sign_in_play::heartbeat_loop(&login) => unreachable!("heartbeat_loop pends"),
    };
    sign_in_play::finish(&ticket_root, &servers, login).await;
    observed
}
/// Commits Running, awaits the supervised game, and reports the
/// client-patches events: live through the telemetry session when one is
/// `settled`, else queued once the game exits. `binaries` holds `SGW.exe`
/// and the DLL's log.
pub(super) async fn run_game(
    state: &Arc<Mutex<DesktopState>>,
    plan: &Plan,
    binaries: &Path,
    cancel: &CancellationToken,
    settled: Option<crate::game_telemetry::session_loop::Settled>,
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
    let patches = patch_events::PatchEvents::new(binaries, injection);
    // Before the spawn, so the tail's baseline holds only earlier sessions'
    // lines. With no session, the patches come back to be queued at exit.
    let session = telemetry::begin_session(plan, settled, patches);
    let outcome = supervised.await;
    // A game that never started has no client-patches log to report on.
    let ran = matches!(
        outcome,
        Observation::ProcessExited { .. } | Observation::Unknown
    );
    match session {
        // The supervisor is the one watcher of the game; the session takes
        // its exit from here and finishes on its own, so the launch result
        // is never held up by the bundle upload.
        Ok(session) if ran => drop(session.finish(telemetry::game_exit(outcome))),
        // Dropped: the session ends as a game that never ran.
        Ok(session) => drop(session),
        Err(mut patches) if ran => {
            // The log read and the queue append are file I/O: off the runtime.
            let state = state.clone();
            if tokio::task::spawn_blocking(move || {
                patch_events::queue_at_exit(&state, &mut patches)
            })
            .await
            .is_err()
            {
                events::patch_events_queue_failed(plan.id);
            }
        }
        Err(_) => {}
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
        Err(_error) => {
            failure::not_started(
                state,
                plan,
                "login_servers_refused",
                FailureCode::LoginServersInvalid,
            );
            return Ok(None);
        }
    };
    // A session that cannot be had costs a bounded wait, never the launch.
    let session = telemetry::session(state, plan, &servers).await;
    Ok(Some((servers, session)))
}
/// The monitor the helper opens the game on (PD-33): the one the current
/// display setup remembers, as `[x, y, width, height]` in the units the game
/// sees. Only a window is placed: exclusive fullscreen owns its adapter, so
/// moving it could only lose the device. The main display, fullscreen, an
/// unknown setup or no displays give `None`, which sends the same request as
/// before.
pub(super) fn place_on(
    mode: crate::storage::display_options::DisplayMode,
    placements: &[crate::storage::display_options::Placement],
    displays: &[crate::display_targets::DisplayInfo],
) -> Option<[i32; 4]> {
    if mode != crate::storage::display_options::DisplayMode::Windowed {
        return None;
    }
    let d = crate::storage::display_options::placement_for(placements, displays)?;
    if d.main {
        return None;
    }
    Some([
        d.x,
        d.y,
        i32::try_from(d.width).ok()?,
        i32::try_from(d.height).ok()?,
    ])
}
/// `place_on` for the saved placements and the displays connected now. The
/// displays are listed only when a window is to be placed somewhere saved.
fn window_target(
    placements: &[crate::storage::display_options::Placement],
    mode: crate::storage::display_options::DisplayMode,
) -> Option<[i32; 4]> {
    if placements.is_empty() || mode != crate::storage::display_options::DisplayMode::Windowed {
        return None;
    }
    place_on(mode, placements, &crate::display_targets::list())
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
    display: DisplayOptions,
    placements: &[Placement],
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
            let (spec, request, resources) =
                wine::prepare(plan, root, session, servers, display, placements)?;
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
    let _ = (root, placements);
    if !cfg!(windows) {
        return Err(StorageError::Corrupt.into());
    }
    prepare_native(plan, session, servers, display)
}
fn prepare_native(
    plan: &Plan,
    session: Option<&crate::game_telemetry::Session>,
    servers: &[LoginServer],
    display: DisplayOptions,
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
        args: display::arguments(display),
        // Filled by `execute` from the remembered monitor (PD-33).
        place_on: None,
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
    display::apply(plan.id, display::native_profile().as_deref(), display);
    Ok((
        crate::helper_supervisor::HelperCommand {
            executable: plan.resources.helper.path().into(),
            arguments: vec![],
            directory,
            environment,
            stderr_log: None,
        },
        request,
        Ownership::Native { _owner: owner },
    ))
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
