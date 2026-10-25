//! `cimmeria-launcher-desktop`: the Tauri shell of the desktop launcher (Windows and
//! macOS). It hosts the settings and Play UI and forwards its commands to the native
//! engine.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod adoption_picker;
#[cfg(test)]
#[path = "../build_payload.rs"]
mod build_payload;
mod host;
mod panic_hook;
#[cfg(any(windows, target_os = "linux"))]
mod payload_startup;
#[cfg(test)]
mod release_version;
mod startup_event;
mod telemetry;
// The C9 capture helper; LT-06 onward drive it from the event helpers.
#[cfg(test)]
mod telemetry_tests;
#[cfg(test)]
#[allow(dead_code)]
mod test_capture;
mod updater_restart;
mod webview2;

use cimmeria_launcher_engine::client_changes::{
    ClientChangesCommand, ClientChangesError, ClientChangesStatus,
};
use cimmeria_launcher_engine::client_maintenance::{
    ClientMaintenanceCommand, ClientMaintenanceError, ClientMaintenanceStatus,
};
use cimmeria_launcher_engine::log_upload::{
    LogUploadCommand, LogUploadError, LogUploadStatus, LogUploadStep,
};
use cimmeria_launcher_engine::runtime_setup::windows_physx::{
    PhysxCommand, PhysxError, PhysxStatus,
};
use cimmeria_launcher_engine::{LauncherBuild, NativeCommand, NativeSnapshot, StorageError};
use host::{
    AdoptionCommand, AdoptionError, AdoptionStatus, GameTelemetryCommand, GameTelemetryStatus,
    GameUpdateCommand, GameUpdateStatus, InstallCommand, InstallStatus, JobError, LaunchCommand,
    LaunchStatus, MigrationCommand, MigrationStatus, NativeHost, SeedArchiveCommand,
    SeedArchiveError, SeedArchiveStatus, UpdaterCommand,
};
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
async fn game_update_command(
    request: GameUpdateCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<GameUpdateStatus, JobError> {
    request.validate()?;
    let host = state.inner().clone();
    match request {
        GameUpdateCommand::Inspect { .. } => {
            tauri::async_runtime::spawn_blocking(move || host.game_update_status())
                .await
                .map_err(|_| JobError::Io)?
        }
        request @ (GameUpdateCommand::Maintain { .. } | GameUpdateCommand::Rollback { .. }) => {
            tauri::async_runtime::spawn_blocking(move || host.maintain_game_update(request))
                .await
                .map_err(|_| JobError::Io)?
        }
        request @ (GameUpdateCommand::Apply { .. } | GameUpdateCommand::Cancel { .. }) => {
            tauri::async_runtime::spawn_blocking(move || host.apply_game_update(request))
                .await
                .map_err(|_| JobError::Io)?
        }
        GameUpdateCommand::Check {
            operation_revision, ..
        } => {
            let checking = host.clone();
            let check = tauri::async_runtime::spawn_blocking(move || {
                checking.begin_game_update_check(operation_revision)
            })
            .await
            .map_err(|_| JobError::Io)??;
            let release = cimmeria_launcher_engine::catalog::fetch_release().await?;
            tauri::async_runtime::spawn_blocking(move || {
                host.finish_game_update_check(check, release)
            })
            .await
            .map_err(|_| JobError::Io)?
        }
    }
}

#[tauri::command]
async fn updater_command(
    request: UpdaterCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<cimmeria_launcher_engine::updater::Snapshot, cimmeria_launcher_engine::updater::Error> {
    state.inner().updater_command(request).await
}

#[tauri::command]
async fn launcher_command(
    request: NativeCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<NativeSnapshot, StorageError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || host.dispatch(request))
        .await
        .map_err(|_| StorageError::Io)?
}

#[tauri::command]
async fn choose_install_directory(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Option<String>, StorageError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_parent(&window)
            .set_title("Choose the Cimmeria install folder")
            .blocking_pick_folder()
            .map(|folder| {
                folder
                    .into_path()
                    .map_err(|_| StorageError::InvalidDirectory)?
                    .into_os_string()
                    .into_string()
                    .map_err(|_| StorageError::InvalidDirectory)
            })
            .transpose()
    })
    .await
    .map_err(|_| StorageError::Io)?
}

#[tauri::command]
async fn show_install_directory(
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<(), StorageError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let folder = host.install_folder()?;
        tauri_plugin_opener::reveal_item_in_dir(folder).map_err(|_| StorageError::Io)
    })
    .await
    .map_err(|_| StorageError::Io)?
}

#[tauri::command]
async fn fetch_patch_notes() -> Result<
    cimmeria_launcher_engine::catalog::PatchNotes,
    cimmeria_launcher_engine::catalog::CatalogError,
> {
    cimmeria_launcher_engine::catalog::fetch_patch_notes().await
}

#[tauri::command]
async fn install_command(
    request: InstallCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<InstallStatus, JobError> {
    let host = state.inner().clone();
    let attempt = request.summary_attempt();
    let result = run_install_command(request, host.clone()).await;
    if let Some(error) = result.as_ref().err().copied() {
        // Closed codes for the launcher summary only. Detached: the reply does
        // not wait for the bookkeeping, and the result is returned unchanged.
        drop(tauri::async_runtime::spawn_blocking(move || {
            host.note_command_failure(attempt, error)
        }));
    }
    result
}

async fn run_install_command(
    request: InstallCommand,
    host: Arc<NativeHost>,
) -> Result<InstallStatus, JobError> {
    request.validate()?;
    if let InstallCommand::Reconcile {
        operation_id,
        operation_revision,
        ..
    } = &request
    {
        // Retain stop/wait and durable reconciliation even if the invoking window
        // disappears. Effect timeouts only end frontend observation.
        if let Some(status) = tauri::async_runtime::spawn(
            host.clone()
                .reconcile_runtime(*operation_id, *operation_revision),
        )
        .await
        .map_err(|_| JobError::Io)??
        {
            return Ok(status);
        }
    }
    let release = if request.needs_release() {
        host.require_install_support()?;
        Some(
            select_install_release(
                host.clone(),
                request.clone(),
                cimmeria_launcher_engine::catalog::fetch_release,
            )
            .await?,
        )
    } else {
        None
    };
    tauri::async_runtime::spawn_blocking(move || host.install_command(request, release))
        .await
        .map_err(|_| JobError::Io)?
}

async fn select_install_release<F>(
    host: Arc<NativeHost>,
    request: InstallCommand,
    fetch: impl FnOnce() -> F,
) -> Result<cimmeria_launcher_engine::catalog::VerifiedRelease, JobError>
where
    F: std::future::Future<
        Output = Result<
            cimmeria_launcher_engine::catalog::VerifiedRelease,
            cimmeria_launcher_engine::catalog::CatalogError,
        >,
    >,
{
    let cached = tauri::async_runtime::spawn_blocking(move || host.retry_release(&request))
        .await
        .map_err(|_| JobError::Io)??;
    match cached {
        Some(release) => Ok(release),
        None => fetch().await.map_err(Into::into),
    }
}

#[tauri::command]
async fn launch_command(
    request: LaunchCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<LaunchStatus, JobError> {
    let queued = request.queue_timer();
    let host = state.inner().clone();
    let attempt = request.summary_attempt();
    let worker = host.clone();
    let result = match tauri::async_runtime::spawn_blocking(move || {
        if let Some(timer) = queued {
            timer.finish(cimmeria_launcher_engine::phase_timing::Outcome::Succeeded);
        }
        worker.launch_command(request)
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Err(JobError::Io),
    };
    if let Some(error) = result.as_ref().err().copied() {
        // Closed codes for the launcher summary only. Detached: the reply does
        // not wait for the bookkeeping, and the result is returned unchanged.
        drop(tauri::async_runtime::spawn_blocking(move || {
            host.note_command_failure(attempt, error)
        }));
    }
    result
}

#[tauri::command]
async fn game_telemetry_command(
    request: GameTelemetryCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<GameTelemetryStatus, JobError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || host.game_telemetry_command(request))
        .await
        .map_err(|_| JobError::Io)?
}

#[tauri::command]
async fn log_upload_command(
    request: LogUploadCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<LogUploadStatus, LogUploadError> {
    let host = state.inner().clone();
    let step = tauri::async_runtime::spawn_blocking(move || host.begin_log_upload(request))
        .await
        .map_err(|_| StorageError::Io)??;
    match step {
        LogUploadStep::Finished(status) => Ok(status),
        // The file reads and the paced posts run without the state mutex; the
        // ingest holds the busy flag until it ends.
        LogUploadStep::Ingest(ingest) => ingest.run().await,
    }
}

#[tauri::command]
async fn client_maintenance_command(
    request: ClientMaintenanceCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<ClientMaintenanceStatus, ClientMaintenanceError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || host.client_maintenance_command(request))
        .await
        .map_err(|_| StorageError::Io)?
}

#[tauri::command]
async fn client_changes_command(
    request: ClientChangesCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<ClientChangesStatus, ClientChangesError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || host.client_changes_command(request))
        .await
        .map_err(|_| StorageError::Io)?
}

/// Inspect, or run the elevated PhysX installer (LX-12). Install holds a
/// blocking thread until msiexec exits, which can be minutes of player input.
#[tauri::command]
async fn physx_command(
    request: PhysxCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<PhysxStatus, PhysxError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || host.physx_command(request))
        .await
        .map_err(|_| StorageError::Io)?
}

#[tauri::command]
async fn directx_command(
    request: cimmeria_launcher_engine::runtime_setup::windows_directx::Command,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<host::DirectxStatus, cimmeria_launcher_engine::runtime_setup::windows_directx::Error> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || host.directx_command(request))
        .await
        .map_err(|_| cimmeria_launcher_engine::runtime_setup::windows_directx::Error::Io)?
}

#[tauri::command]
async fn migration_command(
    request: MigrationCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<MigrationStatus, cimmeria_launcher_engine::migration::MigrationError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || host.migration_command(request))
        .await
        .map_err(|_| StorageError::Io)?
}

#[tauri::command]
async fn adoption_command(
    request: AdoptionCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<AdoptionStatus, AdoptionError> {
    request.validate()?;
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || host.adoption_command(request))
        .await
        .map_err(|_| AdoptionError::Io)?
}

/// Picks an existing game folder to adopt: in place, or as a verified copy made
/// into a folder the player picks next. The webview never passes a path; a
/// cancelled dialog returns the status with any earlier refusal cleared.
#[tauri::command]
async fn choose_existing_game_folder(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<NativeHost>>,
    mode: cimmeria_launcher_engine::adoption::AdoptionMode,
) -> Result<AdoptionStatus, AdoptionError> {
    let host = state.inner().clone();
    // Refuse before a dialog or the network when this build cannot offer the mode.
    let checking = host.clone();
    tauri::async_runtime::spawn_blocking(move || checking.adoption_mode_allowed(mode))
        .await
        .map_err(|_| AdoptionError::Io)??;
    // Light adopts the folder where it is, so it counts as in place.
    let in_place = matches!(
        mode,
        cimmeria_launcher_engine::adoption::AdoptionMode::InPlace
            | cimmeria_launcher_engine::adoption::AdoptionMode::Light
    );
    crate::host::adoption_events::mode_chosen(in_place);
    let Some(selected) = adoption_picker::pick(app.clone(), window.clone()).await? else {
        crate::host::adoption_events::folder_chosen(in_place, false);
        return cancelled(host).await;
    };
    // A copy also needs the folder it goes into; in place there is none.
    let location = if mode == cimmeria_launcher_engine::adoption::AdoptionMode::Copy {
        let Some(location) =
            pick_folder(app, window, "Choose where to put the verified copy").await?
        else {
            crate::host::adoption_events::folder_chosen(in_place, false);
            return cancelled(host).await;
        };
        Some(location)
    } else {
        None
    };
    // Only the fixed, signed catalog names the release; the webview cannot.
    let release = cimmeria_launcher_engine::catalog::fetch_release()
        .await
        .map_err(|_| {
            crate::host::adoption_refusal(
                AdoptionError::ReleaseUnavailable,
                "release_unavailable",
                "catalog_unavailable",
            )
        })?;
    crate::host::adoption_events::folder_chosen(in_place, true);
    tauri::async_runtime::spawn_blocking(move || {
        host.begin_folder_adoption(selected, location, mode, release)
    })
    .await
    .map_err(|_| AdoptionError::Io)?
}

/// A folder dialog titled `title`, run off the async runtime. `None` when cancelled.
async fn pick_folder(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    title: &'static str,
) -> Result<Option<std::path::PathBuf>, AdoptionError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_parent(&window)
            .set_title(title)
            .blocking_pick_folder()
            .map(|folder| {
                folder
                    .into_path()
                    .map_err(|_| AdoptionError::InvalidDirectory)
            })
            .transpose()
    })
    .await
    .map_err(|_| AdoptionError::Io)?
}

/// The status after a cancelled dialog: nothing is started.
async fn cancelled(host: Arc<NativeHost>) -> Result<AdoptionStatus, AdoptionError> {
    tauri::async_runtime::spawn_blocking(move || host.adoption_picker_cancelled())
        .await
        .map_err(|_| AdoptionError::Io)?
}

#[tauri::command]
async fn choose_legacy_source(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<MigrationStatus, cimmeria_launcher_engine::migration::MigrationError> {
    let host = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        host.migration_command(MigrationCommand::Dismiss {schema_version: 1})?;
        let Some(launcher) = app.dialog().file().set_parent(&window).set_title("Choose the folder beside your old launcher (launcher-config.json and install.json)").blocking_pick_folder() else {
            return host.migration_command(MigrationCommand::Inspect {schema_version: 1});
        };
        let Some(game) = app.dialog().file().set_parent(&window).set_title("Choose the existing game root (launcher-installed.json)").blocking_pick_folder() else {
            return host.migration_command(MigrationCommand::Inspect {schema_version: 1});
        };
        host.preview_migration(cimmeria_launcher_engine::migration::LegacySource {
            launcher_directory: launcher.into_path().map_err(|_| StorageError::InvalidDirectory)?,
            game_directory: game.into_path().map_err(|_| StorageError::InvalidDirectory)?,
        })
    }).await.map_err(|_| StorageError::Io)?
}

#[tauri::command]
async fn seed_archive_command(
    request: SeedArchiveCommand,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<SeedArchiveStatus, SeedArchiveError> {
    let host = state.inner().clone();
    // Accepting an offer always fetches the signed release to verify it. Inspect
    // fetches only while no seed is cached and a failed fetch has waited a minute,
    // so the 500 ms poll makes no network request. Cancel and Forget never fetch.
    let release = match request {
        SeedArchiveCommand::AcceptOffer { .. } => Some(
            cimmeria_launcher_engine::catalog::fetch_release()
                .await
                .map_err(SeedArchiveError::from)?,
        ),
        SeedArchiveCommand::Inspect { .. } if host.seed_archive_release_wanted() => {
            match cimmeria_launcher_engine::catalog::fetch_release().await {
                Ok(release) => Some(release),
                Err(_) => {
                    host.seed_archive_release_failed();
                    None
                }
            }
        }
        SeedArchiveCommand::Inspect { .. }
        | SeedArchiveCommand::Cancel { .. }
        | SeedArchiveCommand::Forget { .. } => None,
    };
    tauri::async_runtime::spawn_blocking(move || host.seed_archive_command(request, release))
        .await
        .map_err(|_| SeedArchiveError::Io)?
}

#[tauri::command]
async fn choose_seed_archive(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<SeedArchiveStatus, SeedArchiveError> {
    let host = state.inner().clone();
    // Refused before the dialog opens: a check already runs, so nothing is picked.
    if host.seed_archive_busy() {
        return Err(SeedArchiveError::Busy);
    }
    let Some(path) = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_parent(&window)
            .set_title("Choose your downloaded Cimmeria archive")
            .add_filter("RAR archive", &["rar"])
            .blocking_pick_file()
            .map(|file| file.into_path().map_err(|_| SeedArchiveError::Io))
            .transpose()
    })
    .await
    .map_err(|_| SeedArchiveError::Io)??
    else {
        // A cancelled dialog changes nothing and reports the current status.
        return tauri::async_runtime::spawn_blocking(move || host.seed_archive_picker_cancelled())
            .await
            .map_err(|_| SeedArchiveError::Io)?;
    };
    // Only the fixed, signed catalog names the release; the webview cannot.
    let release = cimmeria_launcher_engine::catalog::fetch_release()
        .await
        .map_err(SeedArchiveError::from)?;
    tauri::async_runtime::spawn_blocking(move || host.choose_seed_archive(path, release))
        .await
        .map_err(|_| SeedArchiveError::Io)?
}

/// tauri.conf.json carries the release title, so only a development build
/// (no release tag and stamp) changes it, and a failed change never makes a
/// release say "development".
const DEVELOPMENT_TITLE: &str = "Cimmeria Launcher \u{2014} development";

fn main() {
    #[cfg(any(windows, target_os = "linux"))]
    payload_startup::headless_probe();
    #[cfg(windows)]
    webview2::ensure_runtime_or_exit();
    // An updater restart waits for the old launcher to exit first, so the guard
    // below finds no running instance and this start proceeds (LX-13).
    let startup = updater_restart::prepare(
        std::env::args_os(),
        cimmeria_launcher_engine::updater::process_alive,
        std::thread::sleep,
        updater_restart::OLD_LAUNCHER_WAIT,
    );
    let old_launcher_still_running = matches!(
        startup.old_launcher,
        updater_restart::OldLauncher::StillRunning { .. }
    );
    let builder = tauri::Builder::default();
    // First, so a second launch never reaches setup or the state lock. The plugin
    // exits the second process with status 0 after this callback focuses the first.
    let builder = if startup.single_instance_guard {
        builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
    } else {
        builder
    };
    builder
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            if LauncherBuild::current().tag.is_none() {
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(error) = window.set_title(DEVELOPMENT_TITLE) {
                        tracing::warn!(%error, "development window title not set");
                    }
                }
            }
            let app_data = app.path().app_data_dir()?;
            let redaction = telemetry::Redaction::from_env();
            let telemetry = telemetry::init(&app_data.join("logs"), redaction.clone());
            let log_file_ok = telemetry.guard.is_some();
            let state_dir = app_data.join("state");
            panic_hook::install(state_dir.clone());
            if old_launcher_still_running {
                startup_event::old_launcher_still_running();
            }
            let host = NativeHost::new(state_dir)
                .with_log_file(log_file_ok)
                .with_default_install_directory(
                    app.path().app_local_data_dir()?.join("Stargate Worlds"),
                );
            #[cfg(target_os = "macos")]
            let host = host.with_bundled_helper(app.path().resource_dir()?);
            let app_handle = app.handle().clone();
            let resource_root = app.path().resource_dir()?;
            #[cfg(any(windows, target_os = "linux"))]
            let resource_root = payload_startup::resources(app, resource_root)?;
            #[cfg(target_os = "linux")]
            let host = host.with_prerequisite_worker(&resource_root);
            let host = host
                .with_launch_resources(resource_root)
                .with_compiled_updater_config()
                .with_updater_shutdown(move || app_handle.exit(0));
            let host = Arc::new(host);
            telemetry::spawn_forwarder(Arc::clone(&host), telemetry.records, redaction);
            app.manage(host);
            app.manage(std::sync::Mutex::new(telemetry.guard));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            launcher_command,
            choose_install_directory,
            show_install_directory,
            fetch_patch_notes,
            install_command,
            launch_command,
            game_telemetry_command,
            log_upload_command,
            client_maintenance_command,
            client_changes_command,
            physx_command,
            directx_command,
            migration_command,
            updater_command,
            game_update_command,
            choose_legacy_source,
            adoption_command,
            choose_existing_game_folder,
            seed_archive_command,
            choose_seed_archive,
            host::ui_event::ui_event,
            host::telemetry_status::telemetry_status
        ])
        .run(tauri::generate_context!())
        .expect("desktop launcher could not initialize");
}
