//! The macOS probes preflight reads (C-PD4, PD-08): Rosetta 2, the cached Wine
//! runtime and App Translocation.
//!
//! The classification and path checks are pure so their tests run everywhere;
//! only [`rosetta`] and [`wine_runtime_cached`] touch the system.

use std::path::{Component, Path};

use super::RosettaProbe;

/// How long the Rosetta probe may run before it counts as timed out.
#[cfg(target_os = "macos")]
const ROSETTA_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Map the Rosetta probe's outcome to a [`RosettaProbe`].
///
/// `None` means the probe timed out, `Some(Err(_))` that spawning
/// `/usr/bin/arch` failed, and `Some(Ok(success))` whether it exited 0.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn classify_rosetta(outcome: Option<std::io::Result<bool>>) -> RosettaProbe {
    match outcome {
        None => RosettaProbe::TimedOut,
        Some(Err(_)) => RosettaProbe::Error,
        Some(Ok(true)) => RosettaProbe::Present,
        Some(Ok(false)) => RosettaProbe::Missing,
    }
}

/// Whether Rosetta 2 can run x86_64 code: runs `/usr/bin/arch -x86_64
/// /usr/bin/true` with a cleared environment and null stdio, killed after
/// five seconds. Off macOS it returns `Present` without spawning (the caller
/// leaves the check out there).
pub async fn rosetta() -> RosettaProbe {
    #[cfg(target_os = "macos")]
    {
        use std::process::Stdio;

        let status = tokio::process::Command::new("/usr/bin/arch")
            .args(["-x86_64", "/usr/bin/true"])
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .status();
        let outcome = tokio::time::timeout(ROSETTA_TIMEOUT, status)
            .await
            .ok()
            .map(|result| result.map(|status| status.success()));
        classify_rosetta(outcome)
    }
    #[cfg(not(target_os = "macos"))]
    {
        RosettaProbe::Present
    }
}

/// Whether `exe` runs from an App Translocation mount (Gatekeeper's random
/// read-only copy of an app that was opened in place from a download).
pub fn translocated(exe: &Path) -> bool {
    exe.components()
        .any(|component| component == Component::Normal("AppTranslocation".as_ref()))
}

/// Whether the Wine runtime is cached and verifies under
/// `<state_root>/runtimes`. This verifies the whole tree and can take about a
/// second, so call it from `spawn_blocking`. The cache lock is dropped at once.
#[cfg(target_os = "macos")]
pub fn wine_runtime_cached(state_root: &Path) -> bool {
    crate::mac_runtime::verified_cached(&state_root.join("runtimes")).is_ok()
}

#[cfg(test)]
#[path = "mac_tests.rs"]
mod tests;
