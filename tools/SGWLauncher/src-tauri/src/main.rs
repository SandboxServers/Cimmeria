#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod commands;
mod config;
mod download;
mod extract;
mod launch;
mod patch;
mod updater;

use std::sync::Mutex;

use tauri::Manager;

use commands::AppState;
use config::LauncherConfig;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            let config_dir = app.path().app_config_dir()?;
            let config_path = config_dir.join("config.json");

            let config = LauncherConfig::load(&config_path).unwrap_or_default();

            app.manage(AppState {
                config_path,
                config: Mutex::new(config),
                cancel_token: Mutex::new(None),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::cmd_check_installation,
            commands::cmd_get_default_install_path,
            commands::cmd_load_config,
            commands::cmd_save_config,
            commands::cmd_patch_server_address,
            commands::cmd_launch_game,
            commands::cmd_cancel_install,
            commands::cmd_download_and_install,
            commands::cmd_check_for_updates,
            commands::cmd_apply_updates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
