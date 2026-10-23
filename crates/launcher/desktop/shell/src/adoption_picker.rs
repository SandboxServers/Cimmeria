//! Native SGW.exe selection; only the resolved installation root reaches adoption.
use crate::host::AdoptionError;
use std::path::PathBuf;
use tauri_plugin_dialog::DialogExt;

pub async fn pick(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Option<PathBuf>, AdoptionError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_parent(&window)
            .set_title("Choose SGW.exe in your existing install")
            .set_file_name("SGW.exe")
            .add_filter("SGW.exe (game executable)", &["exe"])
            .blocking_pick_file()
            .map(|file| {
                let selected = file
                    .into_path()
                    .map_err(|_| AdoptionError::InvalidDirectory)?;
                cimmeria_launcher_engine::adoption::resolve_executable(&selected)
                    .map_err(AdoptionError::from)
            })
            .transpose()
    })
    .await
    .map_err(|_| AdoptionError::Io)?
}
