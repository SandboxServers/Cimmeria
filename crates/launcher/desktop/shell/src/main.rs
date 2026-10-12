//! `cimmeria-launcher-desktop`: the Tauri shell of the desktop launcher (Windows and
//! macOS). It hosts the settings and Play UI and forwards its commands to the native
//! engine.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod host;
#[cfg(test)]
mod release_version;
mod updater_restart;
mod webview2;

use cimmeria_launcher_engine::client_changes::{
    ClientChangesCommand, ClientChangesError, ClientChangesStatus,
};
use cimmeria_launcher_engine::client_maintenance::{
    ClientMaintenanceCommand, ClientMaintenanceError, ClientMaintenanceStatus,
};
use cimmeria_launcher_engine::log_upload::{
    CollectedLogs, LogStore, LogUploadCommand, LogUploadError, LogUploadStatus, LogUploadStep,
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
            .set_title("Choose the Stargate Worlds install folder")
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
    let host = state.inner().clone();
    let attempt = request.summary_attempt();
    let worker = host.clone();
    let result =
        match tauri::async_runtime::spawn_blocking(move || worker.launch_command(request)).await {
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
    let beginning = host.clone();
    let step = tauri::async_runtime::spawn_blocking(move || {
        beginning.begin_log_upload(request, LogStore::compiled().as_ref())
    })
    .await
    .map_err(|_| StorageError::Io)??;
    let collection = match step {
        LogUploadStep::Finished(status) => return Ok(status),
        LogUploadStep::Collect(collection) => collection,
    };
    // Reading and zipping up to 64 MiB of logs runs without the state mutex.
    let pending = match tauri::async_runtime::spawn_blocking(move || collection.collect())
        .await
        .map_err(|_| StorageError::Io)??
    {
        CollectedLogs::Finished(status) => return Ok(status),
        CollectedLogs::Send(pending) => pending,
    };
    // `begin_log_upload` only collects when a store is compiled in.
    let store = LogStore::compiled().ok_or(StorageError::Io)?;
    let stored = store.put(pending).await?;
    tauri::async_runtime::spawn_blocking(move || host.finish_log_upload(stored))
        .await
        .map_err(|_| StorageError::Io)?
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

#[tauri::command]
async fn choose_adoption_destination(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<NativeHost>>,
) -> Result<AdoptionStatus, AdoptionError> {
    let host = state.inner().clone();
    // Refuse before a dialog or the network when no preview could be admitted.
    let checking = host.clone();
    tauri::async_runtime::spawn_blocking(move || checking.adoption_choice_allowed())
        .await
        .map_err(|_| AdoptionError::Io)??;
    // In place (D-LX12) the imported game folder is the one adopted, so there
    // is no folder to pick.
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, window);
        let release = cimmeria_launcher_engine::catalog::fetch_release()
            .await
            .map_err(|_| AdoptionError::ReleaseUnavailable)?;
        tauri::async_runtime::spawn_blocking(move || host.begin_adoption_in_place(release))
            .await
            .map_err(|_| AdoptionError::Io)?
    }
    #[cfg(target_os = "macos")]
    choose_copy_destination(app, window, host).await
}

#[cfg(target_os = "macos")]
async fn choose_copy_destination(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    host: Arc<NativeHost>,
) -> Result<AdoptionStatus, AdoptionError> {
    let Some(location) = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_parent(&window)
            .set_title("Choose where to create the verified Stargate Worlds copy")
            .blocking_pick_folder()
            .map(|folder| {
                folder
                    .into_path()
                    .map_err(|_| AdoptionError::InvalidDirectory)
            })
            .transpose()
    })
    .await
    .map_err(|_| AdoptionError::Io)??
    else {
        return tauri::async_runtime::spawn_blocking(move || host.adoption_status())
            .await
            .map_err(|_| AdoptionError::Io)?;
    };
    // Only the fixed, signed catalog names the release; the webview cannot.
    let release = cimmeria_launcher_engine::catalog::fetch_release()
        .await
        .map_err(|_| AdoptionError::ReleaseUnavailable)?;
    tauri::async_runtime::spawn_blocking(move || host.begin_adoption_preview(location, release))
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
    tauri::async_runtime::spawn_blocking(move || host.seed_archive_command(request))
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
    let Some(path) = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_parent(&window)
            .set_title("Choose your downloaded Stargate Worlds archive")
            .add_filter("RAR archive", &["rar"])
            .blocking_pick_file()
            .map(|file| file.into_path().map_err(|_| SeedArchiveError::Io))
            .transpose()
    })
    .await
    .map_err(|_| SeedArchiveError::Io)??
    else {
        // A cancelled dialog changes nothing and reports the current status.
        let inspect = SeedArchiveCommand::Inspect { schema_version: 1 };
        return tauri::async_runtime::spawn_blocking(move || host.seed_archive_command(inspect))
            .await
            .map_err(|_| SeedArchiveError::Io)?;
    };
    // Only the fixed, signed catalog names the release; the webview cannot.
    let release = cimmeria_launcher_engine::catalog::fetch_release()
        .await
        .map_err(|_| SeedArchiveError::ReleaseUnavailable)?;
    tauri::async_runtime::spawn_blocking(move || host.choose_seed_archive(path, release))
        .await
        .map_err(|_| SeedArchiveError::Io)?
}

/// tauri.conf.json carries the release title, so only a development build
/// (no release tag and stamp) changes it, and a failed change never makes a
/// release say "development".
const DEVELOPMENT_TITLE: &str = "Stargate Worlds \u{2014} development";

fn main() {
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
    if let updater_restart::OldLauncher::StillRunning { pid } = startup.old_launcher {
        eprintln!("updated launcher: process {pid} still running after the restart wait");
    }
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
        .setup(|app| {
            if LauncherBuild::current().tag.is_none() {
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(error) = window.set_title(DEVELOPMENT_TITLE) {
                        tracing::warn!(%error, "development window title not set");
                    }
                }
            }
            let app_data = app.path().app_data_dir()?;
            let host = NativeHost::new(app_data.join("state")).with_default_install_directory(
                app.path().app_local_data_dir()?.join("Stargate Worlds"),
            );
            #[cfg(target_os = "macos")]
            let host = host.with_bundled_helper(app.path().resource_dir()?);
            let app_handle = app.handle().clone();
            let host = host
                .with_launch_resources(app.path().resource_dir()?)
                .with_compiled_updater_config()
                .with_updater_shutdown(move || app_handle.exit(0));
            app.manage(Arc::new(host));
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
            migration_command,
            updater_command,
            game_update_command,
            choose_legacy_source,
            adoption_command,
            choose_adoption_destination,
            seed_archive_command,
            choose_seed_archive
        ])
        .run(tauri::generate_context!())
        .expect("desktop launcher could not initialize");
}
