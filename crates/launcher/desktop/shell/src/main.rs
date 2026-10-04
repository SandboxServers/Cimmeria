#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod host;

use cimmeria_launcher_engine::{NativeCommand, NativeSnapshot, StorageError};
use host::{
    InstallCommand, InstallStatus, JobError, LaunchCommand, LaunchStatus, MigrationCommand,
    MigrationStatus, NativeHost, UpdaterCommand,
};
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            let host = NativeHost::new(app_data.join("state")).with_default_install_directory(
                app.path().app_local_data_dir()?.join("Stargate Worlds"),
            );
            #[cfg(target_os = "macos")]
            let host = host.with_bundled_helper(app.path().resource_dir()?);
            let host = host.with_launch_resources(app.path().resource_dir()?);
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
            migration_command,
            updater_command,
            choose_legacy_source
        ])
        .run(tauri::generate_context!())
        .expect("desktop launcher could not initialize");
}
