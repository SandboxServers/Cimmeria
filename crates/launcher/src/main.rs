#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod client_paths;
mod config;
mod identity;
mod install;
mod launch;
mod logs;
mod manifest;
mod patch_rdata;
mod state;
mod telemetry;
mod worker;

use std::sync::Arc;

use eframe::egui;
use fs4::fs_std::FileExt;
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

use app::LauncherApp;
use config::{exe_dir, telemetry_state_path};
use identity::{identity_path, LauncherIdentity};
use state::TelemetryState;

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
            // Most likely cause: read-only install dir. Surface via a
            // message box on Windows (release builds detach stderr) and
            // also via stderr for dev builds and Linux/macOS.
            let msg = format!(
                "Failed to open lock file at {}:\n{}",
                lock_path.display(),
                e
            );
            fatal_startup_error("Stargate Worlds Launcher", &msg);
            std::process::exit(2);
        }
    };
    if FileExt::try_lock_exclusive(&lock_file).is_err() {
        let msg = format!(
            "Another Stargate Worlds Launcher instance appears to be running \
             (lock held at {}). Close the other instance and retry.",
            lock_path.display()
        );
        fatal_startup_error("Stargate Worlds Launcher", &msg);
        std::process::exit(3);
    }

    let runtime = Arc::new(Runtime::new().expect("failed to create tokio runtime"));
    let runtime_for_app = runtime.clone();

    // Mint install_id on first launch; subsequent launches reuse it.
    // Logging before the eframe handoff puts the correlator in
    // early-init output where startup failures still land.
    log_telemetry_foundation();

    // Re-enqueue any telemetry left on disk from a previous run that
    // crashed before flushing. Best-effort.
    let recovered = telemetry::recover_pending_on_startup();
    if recovered > 0 {
        tracing::info!(recovered, "telemetry pending events recovered");
    }

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

/// Best-effort identity load — a mint failure must not block the
/// game from launching.
fn log_telemetry_foundation() {
    let id_path = identity_path();
    match LauncherIdentity::load_or_mint(&id_path) {
        Ok(id) => {
            // Persistent identifiers stay at debug-level so they don't
            // leak into log uploads.
            tracing::info!(
                install_short = %short_correlator(&id.install_id.to_string()),
                launcher_version = %id.created_by_launcher_version,
                first_seen_ms = id.first_seen_ms,
                "Loaded launcher identity"
            );
            tracing::debug!(
                install_id = %id.install_id,
                machine_id = %id.machine_id,
                "Launcher identity (debug-only)"
            );
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                path = %id_path.display(),
                "Failed to load/mint launcher identity — telemetry will be inert this session"
            );
        }
    }
    let tel_state = TelemetryState::load(&telemetry_state_path());
    tracing::info!(
        last_upload_ms = ?tel_state.last_upload_ms,
        kill_switch_active = tel_state.kill_switch_active,
        last_upload_endpoint = ?tel_state.last_upload_endpoint,
        "Loaded telemetry runtime state"
    );
}

fn short_correlator(id: &str) -> String {
    id.chars().take(8).collect()
}

/// Surface a fatal startup error to the user before exiting. On Windows
/// release builds (`windows_subsystem = "windows"`) `stderr` is not
/// attached to anything when the user double-clicks the binary, so an
/// `eprintln!` becomes a silent crash. A `MessageBoxW` always shows.
/// Always emits to stderr too so dev / CI runs still get a log line.
fn fatal_startup_error(title: &str, message: &str) {
    eprintln!("sgw-launcher: {message}");
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        // Wide-encode with a trailing NUL for the W APIs.
        let to_wide =
            |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
        let title_w = to_wide(title);
        let body_w = to_wide(message);
        // SAFETY: pointers are valid for the duration of the call, both
        // strings are NUL-terminated, hWnd=null is allowed.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                body_w.as_ptr(),
                title_w.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}
