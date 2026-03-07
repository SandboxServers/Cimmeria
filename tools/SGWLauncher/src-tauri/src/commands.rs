use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_shell::ShellExt;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::config::LauncherConfig;
use crate::download::{download_file, DownloadProgress};
use crate::extract::{build_extract_args, count_cab_files, ExtractProgress};
use crate::launch::{check_installation, launch_game, InstallState};
use crate::patch::{needs_patching, patch_exe};
use crate::updater::{check_manifest, fetch_manifest, verify_hash, UpdateCheckResult};

const ARCHIVE_URL: &str = "https://archive.org/download/StargateWorlds_0.8348.1.4046/Stargate%20Worlds%20%280.8348.1.4046%29%20%282009-06-30%29%20%28beta%29.rar";

pub struct AppState {
    pub config_path: PathBuf,
    pub config: Mutex<LauncherConfig>,
    pub cancel_token: Mutex<Option<CancellationToken>>,
}

#[derive(Debug, Clone, Serialize)]
struct UpdateProgress {
    current: usize,
    total: usize,
    filename: String,
}

#[tauri::command]
pub fn cmd_check_installation(state: State<'_, AppState>) -> InstallState {
    let config = state.config.lock().unwrap();
    check_installation(&config.install_path)
}

#[tauri::command]
pub fn cmd_get_default_install_path() -> String {
    if let Ok(pf) = std::env::var("ProgramFiles") {
        format!("{}\\Stargate Worlds", pf)
    } else {
        "C:\\Program Files\\Stargate Worlds".to_string()
    }
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

    let offset = patch_exe(&exe_path, &address).map_err(|e| e.to_string())?;
    Ok(format!(
        "Patched server address to '{}' at offset 0x{:X}",
        address, offset
    ))
}

#[tauri::command]
pub fn cmd_launch_game(state: State<'_, AppState>) -> Result<u32, String> {
    let config = state.config.lock().unwrap();
    launch_game(&config.install_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cmd_cancel_install(state: State<'_, AppState>) {
    let token = state.cancel_token.lock().unwrap();
    if let Some(ref t) = *token {
        info!("Cancelling install");
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
    // Set up cancellation token
    let cancel = CancellationToken::new();
    {
        let mut token = state.cancel_token.lock().unwrap();
        *token = Some(cancel.clone());
    }

    let temp_dir = std::env::temp_dir().join("sgw-launcher");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    let archive_path = temp_dir.join("sgw-installer.rar");

    // Phase 1: Download
    info!("Downloading installer from {}", ARCHIVE_URL);

    let app_dl = app.clone();
    download_file(ARCHIVE_URL, &archive_path, cancel.clone(), move |progress: DownloadProgress| {
        let _ = app_dl.emit("download-progress", &progress);
    })
    .await
    .map_err(|e| {
        let _ = app.emit("install-error", &serde_json::json!({ "message": e.to_string() }));
        e.to_string()
    })?;

    // Phase 2: Extract RAR
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

    // Phase 3: Extract CAB files
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

        let cab_args = build_extract_args(
            cab.to_str().unwrap(),
            &install_path,
        );

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

    // Phase 4: Patch executable
    let exe_path = Path::new(&install_path).join("SGW.exe");
    if exe_path.exists() {
        let data = std::fs::read(&exe_path).map_err(|e| e.to_string())?;
        if needs_patching(&data) {
            patch_exe(&exe_path, &server_address).map_err(|e| e.to_string())?;
            info!("Patched SGW.exe with server address: {}", server_address);
        }
    }

    // Phase 5: Save config with install path
    {
        let mut config = state.config.lock().unwrap();
        config.install_path = install_path;
        config
            .save(&state.config_path)
            .map_err(|e| e.to_string())?;
    }

    // Clean up temp files
    if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
        error!("Failed to clean up temp dir: {}", e);
    }

    let _ = app.emit("install-complete", ());

    info!("Installation complete");
    Ok(())
}

#[tauri::command]
pub async fn cmd_check_for_updates(
    state: State<'_, AppState>,
) -> Result<UpdateCheckResult, String> {
    let (patch_server_url, install_path) = {
        let config = state.config.lock().unwrap();
        (config.patch_server_url.clone(), config.install_path.clone())
    };

    let manifest = fetch_manifest(&patch_server_url)
        .await
        .map_err(|e| e.to_string())?;

    let files_to_update = check_manifest(&manifest, Path::new(&install_path));
    let total_bytes: u64 = files_to_update.iter().map(|f| f.size).sum();

    Ok(UpdateCheckResult {
        updates_available: !files_to_update.is_empty(),
        files_to_update,
        total_bytes,
        server_version: manifest.version,
    })
}

#[tauri::command]
pub async fn cmd_apply_updates(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (patch_server_url, install_path) = {
        let config = state.config.lock().unwrap();
        (config.patch_server_url.clone(), config.install_path.clone())
    };

    let manifest = fetch_manifest(&patch_server_url)
        .await
        .map_err(|e| e.to_string())?;

    let files = check_manifest(&manifest, Path::new(&install_path));
    let total = files.len();
    let cancel = CancellationToken::new();

    for (i, entry) in files.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err("Update cancelled".to_string());
        }

        let local_path = Path::new(&install_path).join(&entry.path);

        let _ = app.emit(
            "update-progress",
            &UpdateProgress {
                current: i + 1,
                total,
                filename: entry.path.clone(),
            },
        );

        // Create parent dirs if needed
        if let Some(parent) = local_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // Download the updated file
        let noop_cancel = CancellationToken::new();
        download_file(&entry.patch_url, &local_path, noop_cancel, |_| {})
            .await
            .map_err(|e| format!("Failed to download {}: {}", entry.path, e))?;

        // Verify hash
        verify_hash(&local_path, &entry.sha256)
            .map_err(|e| format!("Hash verification failed for {}: {}", entry.path, e))?;
    }

    let _ = app.emit("update-complete", ());

    info!("Applied {} updates", total);
    Ok(())
}
