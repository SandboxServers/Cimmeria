use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::io;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_shell::ShellExt;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::config::LauncherConfig;
use crate::download::{download_file, DownloadProgress};
use crate::extract::{build_extract_args, count_cab_files, extract_zip_to_dir, ExtractProgress};
use crate::launch::{check_installation, launch_game, InstallState};
use crate::patch::{needs_patching, patch_exe};
use crate::updater::{
    check_manifest, fetch_manifest, verify_hash, load_applied_patches, save_applied_patches,
    AppliedPatches, UpdateCheckResult,
};

pub struct AppState {
    pub config_path: PathBuf,
    pub config: Mutex<LauncherConfig>,
    pub install_cancel: Mutex<Option<CancellationToken>>,
    pub update_cancel: Mutex<Option<CancellationToken>>,
}

#[derive(Debug, Clone, Serialize)]
struct UpdateProgress {
    id: String,
    current: usize,
    total: usize,
}

#[tauri::command]
pub fn cmd_check_installation(state: State<'_, AppState>) -> InstallState {
    let config = state.config.lock().unwrap();
    check_installation(&config.install_path)
}

#[tauri::command]
pub fn cmd_get_default_install_path(app: AppHandle) -> String {
    let cwd = std::env::current_dir().unwrap_or_default();
    if cwd.join("Cargo.toml").exists() && cwd.join("crates").exists() {
        return cwd
            .join("game")
            .join("sgw")
            .to_string_lossy()
            .to_string();
    }
    app.path()
        .home_dir()
        .map(|h| {
            h.join("Games")
                .join("Stargate Worlds")
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|_| "C:\\Games\\Stargate Worlds".to_string())
}

#[tauri::command]
pub fn cmd_load_config(state: State<'_, AppState>) -> LauncherConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn cmd_save_config(
    state: State<'_, AppState>,
    config: LauncherConfig,
) -> Result<(), String> {
    config
        .save(&state.config_path)
        .map_err(|e| e.to_string())?;
    *state.config.lock().unwrap() = config;
    Ok(())
}

#[tauri::command]
pub fn cmd_patch_server_address(
    state: State<'_, AppState>,
    address: String,
) -> Result<String, String> {
    let config = state.config.lock().unwrap();
    let exe_path = Path::new(&config.install_path).join("SGW.exe");
    if !exe_path.exists() {
        return Err(format!("SGW.exe not found at {}", exe_path.display()));
    }

    let data = std::fs::read(&exe_path).map_err(|e| e.to_string())?;
    if !needs_patching(&data) {
        return Ok("Already patched".to_string());
    }

    patch_exe(&exe_path, &address).map_err(|e| e.to_string())?;
    Ok(format!("Patched server address to '{}'", address))
}

#[tauri::command]
pub fn cmd_launch_game(state: State<'_, AppState>) -> Result<u32, String> {
    let config = state.config.lock().unwrap();
    launch_game(&config.install_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cmd_cancel_install(state: State<'_, AppState>) {
    if let Some(t) = state.install_cancel.lock().unwrap().take() {
        info!("Cancelling install");
        t.cancel();
    }
    if let Some(t) = state.update_cancel.lock().unwrap().take() {
        info!("Cancelling updates");
        t.cancel();
    }
}

#[tauri::command]
pub async fn cmd_download_and_install(
    app: AppHandle,
    state: State<'_, AppState>,
    install_path: String,
    server_address: String,
) -> Result<(), String> {
    if install_path.is_empty() {
        return Err("Install path must be selected before downloading".to_string());
    }

    let cancel = CancellationToken::new();
    {
        let mut token = state.install_cancel.lock().unwrap();
        *token = Some(cancel.clone());
    }

    let config = state.config.lock().unwrap();
    let manifest_url = config.manifest_url.clone();
    drop(config);

    let manifest = fetch_manifest(&manifest_url)
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

    let base_entry = manifest
        .patches
        .iter()
        .find(|p| p.id == "base-client")
        .ok_or_else(|| "Base client entry not found in manifest".to_string())?
        .clone();

    let temp_dir = std::env::temp_dir().join("sgw-launcher");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    let archive_path = temp_dir.join("sgw-installer.rar");

    info!("Downloading base client from {}", base_entry.url);
    let app_dl = app.clone();
    download_file(
        &base_entry.url,
        &archive_path,
        cancel.clone(),
        move |progress: DownloadProgress| {
            let _ = app_dl.emit("download-progress", &progress);
        },
    )
    .await
    .map_err(|e| {
        let _ = app.emit(
            "install-error",
            &serde_json::json!({ "message": e.to_string() }),
        );
        e.to_string()
    })?;

    verify_hash(&archive_path, &base_entry.sha256)
        .map_err(|e| format!("Download integrity check failed: {}", e))?;

    info!("Extracting RAR archive");
    let _ = app.emit(
        "extract-progress",
        &ExtractProgress {
            phase: "rar".to_string(),
            current: 0,
            total: 1,
            filename: "sgw-installer.rar".to_string(),
        },
    );

    let extract_dir = temp_dir.join("extracted");
    std::fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;

    let rar_args = build_extract_args(
        archive_path.to_str().unwrap(),
        extract_dir.to_str().unwrap(),
    );

    let output = app
        .shell()
        .sidecar("binaries/7za")
        .unwrap()
        .args(&rar_args)
        .output()
        .await
        .map_err(|e| format!("Failed to run 7za: {}", e))?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("7za RAR extraction failed (code {}): {}", code, stderr));
    }

    let _ = app.emit(
        "extract-progress",
        &ExtractProgress {
            phase: "rar".to_string(),
            current: 1,
            total: 1,
            filename: "sgw-installer.rar".to_string(),
        },
    );

    let cabs = count_cab_files(&extract_dir).map_err(|e| e.to_string())?;
    let total_cabs = cabs.len() as u32;
    info!("Found {} CAB files to extract", total_cabs);

    std::fs::create_dir_all(&install_path).map_err(|e| e.to_string())?;

    for (i, cab) in cabs.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err("Installation cancelled".to_string());
        }

        let cab_name = cab
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let current = (i + 1) as u32;

        let _ = app.emit(
            "extract-progress",
            &ExtractProgress {
                phase: "cab".to_string(),
                current,
                total: total_cabs,
                filename: cab_name.clone(),
            },
        );

        let cab_args = build_extract_args(cab.to_str().unwrap(), &install_path);

        let cab_output = app
            .shell()
            .sidecar("binaries/7za")
            .unwrap()
            .args(&cab_args)
            .output()
            .await
            .map_err(|e| format!("Failed to run 7za on {}: {}", cab_name, e))?;

        if !cab_output.status.success() {
            let code = cab_output.status.code().unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&cab_output.stderr);
            return Err(format!(
                "7za CAB extraction failed for {} (code {}): {}",
                cab_name, code, stderr
            ));
        }
    }

    let exe_path = Path::new(&install_path).join("SGW.exe");
    if exe_path.exists() {
        let data = std::fs::read(&exe_path).map_err(|e| e.to_string())?;
        if needs_patching(&data) {
            patch_exe(&exe_path, &server_address).map_err(|e| e.to_string())?;
            info!("Patched SGW.exe with server address: {}", server_address);
        }
    }

    {
        let mut config = state.config.lock().unwrap();
        config.install_path = install_path;
        config
            .save(&state.config_path)
            .map_err(|e| e.to_string())?;
    }

    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let mut applied = load_applied_patches(&config_dir);
    applied
        .applied
        .insert("base-client".to_string(), base_entry.sha256.clone());
    save_applied_patches(&config_dir, &applied).map_err(|e| e.to_string())?;

    if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
        error!("Failed to clean up temp dir: {}", e);
    }

    let _ = app.emit("install-complete", ());
    info!("Installation complete");
    Ok(())
}

#[tauri::command]
pub async fn cmd_check_for_updates(app: AppHandle, state: State<'_, AppState>) -> Result<UpdateCheckResult, String> {
    let (manifest_url, config_dir) = {
        let config = state.config.lock().unwrap();
        (config.manifest_url.clone(), app.path().app_config_dir()
            .map_err(|e| e.to_string())?)
    };

    let manifest = fetch_manifest(&manifest_url)
        .await
        .map_err(|e| e.to_string())?;

    let applied = load_applied_patches(&config_dir);
    let patches_to_apply = check_manifest(&manifest, &applied);
    let total_bytes: u64 = patches_to_apply.iter().map(|p| p.size).sum();

    Ok(UpdateCheckResult {
        updates_available: !patches_to_apply.is_empty(),
        patches_to_apply,
        total_bytes,
        manifest_version: manifest.manifest_version,
    })
}

#[tauri::command]
pub async fn cmd_apply_updates(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let cancel = CancellationToken::new();
    {
        let mut token = state.update_cancel.lock().unwrap();
        *token = Some(cancel.clone());
    }

    let (manifest_url, install_path, config_dir) = {
        let config = state.config.lock().unwrap();
        (
            config.manifest_url.clone(),
            config.install_path.clone(),
            app.path().app_config_dir().map_err(|e| e.to_string())?,
        )
    };

    let manifest = fetch_manifest(&manifest_url)
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

    let applied = load_applied_patches(&config_dir);
    let patches = check_manifest(&manifest, &applied);
    let total = patches.len() as u32;

    if patches.is_empty() {
        let _ = app.emit("update-complete", ());
        return Ok(());
    }

    let staging_dir = std::env::temp_dir().join("sgw-launcher/updates");
    std::fs::create_dir_all(&staging_dir).map_err(|e| e.to_string())?;

    for (i, patch) in patches.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err("Update cancelled".to_string());
        }

        let staging_zip = staging_dir.join(format!("{}-{}.zip", patch.id, patch.version));

        info!("Downloading patch {}", patch.id);
        let app_dl = app.clone();
        download_file(
            &patch.url,
            &staging_zip,
            cancel.clone(),
            move |progress: DownloadProgress| {
                let _ = app_dl.emit("download-progress", &progress);
            },
        )
        .await
        .map_err(|e| format!("Failed to download patch {}: {}", patch.id, e))?;

        verify_hash(&staging_zip, &patch.sha256).map_err(|e| {
            let _ = std::fs::remove_file(&staging_zip);
            format!("Hash verification failed for patch {}: {}", patch.id, e)
        })?;

        let extracted_dir = staging_dir.join("extracted");
        std::fs::create_dir_all(&extracted_dir).map_err(|e| e.to_string())?;

        info!("Extracting patch {}", patch.id);
        extract_zip_to_dir(&staging_zip, &extracted_dir)
            .map_err(|e| format!("Failed to extract patch {}: {}", patch.id, e))?;

        let mut extracted_files = Vec::new();
        if extracted_dir.exists() {
            walk_dir(&extracted_dir, &extracted_dir, &mut extracted_files)
                .map_err(|e| e.to_string())?;
        }

        for extracted_file in extracted_files {
            if cancel.is_cancelled() {
                return Err("Update cancelled".to_string());
            }

            let relative_path = extracted_file
                .strip_prefix(&extracted_dir)
                .map_err(|e| e.to_string())?;
            let final_path = Path::new(&install_path).join(relative_path);

            if let Some(parent) = final_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }

            let temp_path = final_path.with_file_name(format!(
                ".sgw-{}-{}",
                std::process::id(),
                final_path.file_name().unwrap_or_default().to_string_lossy()
            ));

            std::fs::copy(&extracted_file, &temp_path)
                .map_err(|e| format!("Failed to stage file: {}", e))?;

            std::fs::rename(&temp_path, &final_path).map_err(|e| {
                let _ = std::fs::remove_file(&temp_path);
                format!("Failed to commit file {}: {}", relative_path.display(), e)
            })?;
        }

        let mut applied = load_applied_patches(&config_dir);
        applied
            .applied
            .insert(patch.id.clone(), patch.sha256.clone());
        save_applied_patches(&config_dir, &applied).map_err(|e| e.to_string())?;

        let _ = app.emit(
            "update-progress",
            &UpdateProgress {
                id: patch.id.clone(),
                current: (i + 1),
                total: (total as usize),
            },
        );
    }

    if let Err(e) = std::fs::remove_dir_all(&staging_dir) {
        error!("Failed to clean up staging dir: {}", e);
    }

    let _ = app.emit("update-complete", ());
    info!("Applied {} patches", total);
    Ok(())
}

fn walk_dir(dir: &Path, base: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            walk_dir(&path, base, files)?;
        }
    }
    Ok(())
}
