#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod install;
mod launch;
mod logs;
mod manifest;
mod patch_rdata;
mod state;
mod worker;

use std::sync::Arc;

use eframe::egui;
use fs4::fs_std::FileExt;
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

use app::LauncherApp;
use config::exe_dir;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Single-instance file lock. A second launcher process opening the same
    // path fails the exclusive try-lock and we exit before constructing
    // any state — the two instances would otherwise race on
    // launcher-installed.json + the .tmp-*.zip files and produce
    // unpredictable results. We keep the `File` alive for the whole
    // process lifetime; OS-level lock release happens at drop.
    let lock_path = exe_dir().join("launcher.lock");
    let lock_file = match std::fs::File::create(&lock_path) {
        Ok(f) => f,
        Err(e) => {
            // Without a usable lock file we have no concurrency protection.
            // Most likely cause: read-only install dir. Surface and bail.
            eprintln!(
                "sgw-launcher: failed to open lock file at {}: {}",
                lock_path.display(),
                e
            );
            std::process::exit(2);
        }
    };
    if FileExt::try_lock_exclusive(&lock_file).is_err() {
        eprintln!(
            "sgw-launcher: another launcher instance appears to be running \
             (lock held at {}). Close the other instance and retry.",
            lock_path.display()
        );
        std::process::exit(3);
    }

    let runtime = Arc::new(Runtime::new().expect("failed to create tokio runtime"));
    let runtime_for_app = runtime.clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 540.0])
            .with_min_inner_size([640.0, 400.0])
            .with_resizable(true)
            .with_title("Stargate Worlds Launcher"),
        ..Default::default()
    };

    let result = eframe::run_native(
        "Stargate Worlds Launcher",
        options,
        Box::new(move |_cc| Ok(Box::new(LauncherApp::new(runtime_for_app)))),
    );

    drop(runtime);
    // Keep `lock_file` alive until here so the OS-level lock survives the
    // entire run. Dropping it explicitly is documentation; the Drop impl
    // would release the lock when the binding goes out of scope anyway.
    drop(lock_file);
    result
}
