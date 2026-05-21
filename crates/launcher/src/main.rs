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
use tokio::runtime::Runtime;
use tracing_subscriber::EnvFilter;

use app::LauncherApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

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
    result
}
