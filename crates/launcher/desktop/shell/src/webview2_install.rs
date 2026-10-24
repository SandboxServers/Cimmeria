//! Installs the WebView2 runtime for the player (PD-41, owner D1). The
//! Evergreen Bootstrapper runs with `/silent /install` and no elevation, which
//! installs the runtime for the current user (Microsoft Learn, WebView2
//! distribution). This runs before the window and before the tracing pipeline,
//! so it logs nothing; the decisions are pure and tested on every host.

/// The question the missing-runtime dialog asks. Yes runs the install.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const INSTALL_PROMPT: &str = "Install the Microsoft Edge WebView2 Runtime now? It is about 2 MB and installs for your user account.";

/// The line the download-page dialog leads with after an install that did
/// not leave a runtime behind.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const INSTALL_DID_NOT_FINISH: &str = "The automatic install didn't finish.";

/// The bootstrapper's arguments, in this order: it ignores `/silent` unless it
/// comes first.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const BOOTSTRAPPER_ARGS: [&str; 2] = ["/silent", "/install"];

/// How long the bootstrapper may run before the launcher stops waiting.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const INSTALL_WAIT: std::time::Duration = std::time::Duration::from_secs(5 * 60);

/// The largest download accepted. The bootstrapper is about 2 MB; anything
/// far larger is not it.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) const MAX_DOWNLOAD_BYTES: usize = 16 * 1024 * 1024;

/// How a bootstrapper run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) enum InstallOutcome {
    /// The bootstrapper exited, with any exit code. Only the re-read
    /// registry decides whether it installed anything.
    Exited,
    /// It was still running when [`INSTALL_WAIT`] ran out.
    TimedOut,
    /// The download failed, the file was not an executable, or it could not
    /// be written or started.
    Failed,
}

/// What startup does after an install attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) enum Next {
    /// The runtime is there now: open the launcher.
    Continue,
    /// Show the download page and exit, as before PD-41.
    FallBack,
}

/// Whether downloaded bytes are a Windows executable. The download link can
/// serve an HTML page (a captive portal, an error page), which must never run.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn is_executable(bytes: &[u8]) -> bool {
    bytes.starts_with(b"MZ")
}

/// Startup continues only when the bootstrapper exited and the registry now
/// names a runtime. A timeout falls back even if a runtime appears, because
/// the bootstrapper may still be changing it.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn after_install(present: bool, outcome: InstallOutcome) -> Next {
    match outcome {
        InstallOutcome::Exited if present => Next::Continue,
        _ => Next::FallBack,
    }
}

/// One install attempt, never repeated: download, refuse anything that is
/// not an executable, run it, then re-read the registry. The steps are
/// parameters so the order and the refusal are tested off Windows.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn install_with(
    download: impl FnOnce() -> Option<Vec<u8>>,
    run: impl FnOnce(&[u8]) -> InstallOutcome,
    present: impl FnOnce() -> bool,
) -> Next {
    let Some(bytes) = download() else {
        return after_install(false, InstallOutcome::Failed);
    };
    if !is_executable(&bytes) {
        return after_install(false, InstallOutcome::Failed);
    }
    let outcome = run(&bytes);
    after_install(present(), outcome)
}

/// Downloads and runs the bootstrapper, then asks `present` whether the
/// runtime is installed.
#[cfg(windows)]
pub(crate) fn install(download_url: &str, present: impl FnOnce() -> bool) -> Next {
    install_with(|| download(download_url), run, present)
}

/// Fetches the bootstrapper over HTTPS on a runtime of its own (Tauri has not
/// started one yet). `None` on any network error, a non-success status or an
/// oversized body.
#[cfg(windows)]
fn download(url: &str) -> Option<Vec<u8>> {
    use std::time::Duration;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .build()
            .ok()?;
        let mut response = client.get(url).send().await.ok()?;
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|len| len > MAX_DOWNLOAD_BYTES as u64)
        {
            return None;
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.ok()? {
            if chunk.len() > MAX_DOWNLOAD_BYTES.saturating_sub(bytes.len()) {
                return None;
            }
            bytes.extend_from_slice(&chunk);
        }
        Some(bytes)
    })
}

/// Writes the bootstrapper to a new temp file and runs it unelevated, waiting
/// at most [`INSTALL_WAIT`]. A bootstrapper still running at the deadline is
/// left to finish on its own; its file stays behind because Windows will not
/// delete a running executable.
#[cfg(windows)]
fn run(bytes: &[u8]) -> InstallOutcome {
    use std::io::Write;
    use std::time::{Duration, Instant};

    let path = std::env::temp_dir().join(format!(
        "cimmeria-webview2-setup-{}.exe",
        uuid::Uuid::new_v4()
    ));
    // create_new refuses a file planted at the name beforehand.
    let written = std::fs::File::create_new(&path).and_then(|mut file| file.write_all(bytes));
    if written.is_err() {
        let _ = std::fs::remove_file(&path);
        return InstallOutcome::Failed;
    }
    let Ok(mut child) = std::process::Command::new(&path)
        .args(BOOTSTRAPPER_ARGS)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        let _ = std::fs::remove_file(&path);
        return InstallOutcome::Failed;
    };
    let deadline = Instant::now() + INSTALL_WAIT;
    let outcome = loop {
        match child.try_wait() {
            Ok(Some(_)) => break InstallOutcome::Exited,
            Ok(None) if Instant::now() >= deadline => break InstallOutcome::TimedOut,
            Ok(None) => std::thread::sleep(Duration::from_millis(500)),
            Err(_) => break InstallOutcome::Failed,
        }
    };
    if outcome != InstallOutcome::TimedOut {
        let _ = std::fs::remove_file(&path);
    }
    outcome
}

#[cfg(test)]
#[path = "webview2_install_tests.rs"]
mod tests;
