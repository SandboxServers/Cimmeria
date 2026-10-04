#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod host;

use cimmeria_launcher_engine::{NativeCommand, NativeSnapshot, StorageError};
use host::{InstallCommand, InstallStatus, JobError, NativeHost};
use std::sync::Arc;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

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
    request.validate()?;
    let host = state.inner().clone();
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let host = NativeHost::new(app.path().app_data_dir()?.join("state"));
            #[cfg(target_os = "macos")]
            let host = host.with_bundled_helper(app.path().resource_dir()?);
            app.manage(Arc::new(host));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            launcher_command,
            choose_install_directory,
            show_install_directory,
            fetch_patch_notes,
            install_command
        ])
        .run(tauri::generate_context!())
        .expect("desktop launcher could not initialize");
}
