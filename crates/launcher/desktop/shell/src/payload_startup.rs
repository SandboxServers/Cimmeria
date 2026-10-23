//! Resolve embedded Windows resources, with an explicit headless packaging probe.
use cimmeria_launcher_engine::embedded_payload::{prepare, Lease};
use std::{io, path::PathBuf};
use tauri::Manager;
include!(concat!(env!("OUT_DIR"), "/embedded_payload.rs"));

pub fn resources(
    app: &tauri::App,
    fallback: PathBuf,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if ENTRIES.is_empty() {
        return Ok(fallback);
    }
    let lease = prepare(&app.path().app_local_data_dir()?.join("payloads"), ENTRIES)
        .inspect_err(|_| show_failure())?;
    let path = lease.root().to_owned();
    app.manage(lease);
    Ok(path)
}
/// Runs before WebView2/Tauri initialization and never starts a helper or game.
/// Only the compiled payload is used; this cannot select executable input.
pub fn headless_probe() {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--verify-embedded-payload")) {
        return;
    }
    let result = (|| -> io::Result<Lease> {
        let path = args
            .next()
            .ok_or_else(|| io::Error::other("payload cache path required"))?;
        if args.next().is_some() || ENTRIES.len() != 3 {
            return Err(io::Error::other(
                "probe requires a complete embedded payload and one cache path",
            ));
        }
        let lease = prepare(&PathBuf::from(path), ENTRIES)?;
        if !crate::host::NativeHost::verify_embedded_launch_bundle(lease.root()) {
            return Err(io::Error::other(
                "native resource resolution refused embedded payload",
            ));
        }
        Ok(lease)
    })();
    match result {
        Ok(lease) => {
            println!("embedded payload verified: {} files", ENTRIES.len());
            drop(lease);
            std::process::exit(0);
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
fn show_failure() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let text: Vec<u16> = "Cimmeria Launcher could not prepare its bundled files. Close other launcher instances and try again. If this continues, share the launcher debug logs with support.".encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "Cimmeria Launcher".encode_utf16().chain(Some(0)).collect();
    // SAFETY: NUL-terminated strings live for the call; no owner window.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
