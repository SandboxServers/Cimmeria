//! Status-log lines: the pure formatting that turns worker events and
//! client-setup results into the lines of Settings › Advanced › Activity
//! log. Kept out of egui so every arm is unit-tested.

use super::update_banner;
use crate::worker::Event;

/// Status lines for a client-setup result, and whether launching may go
/// ahead. Extracted so the launch gate is testable without an egui frame.
pub(super) fn setup_status_lines(
    result: &std::io::Result<crate::client_setup::SetupReport>,
) -> (bool, Vec<String>) {
    match result {
        Ok(report) => {
            let mut lines = Vec::new();
            for r in &report.restored_names {
                lines.push(format!(
                    "Renamed {} back to its stock name {} (the game can't find it otherwise).",
                    r.from.display(),
                    r.to.file_name().unwrap_or_default().to_string_lossy()
                ));
            }
            if report.login_servers_written {
                lines.push("Wrote the login server list (LoginInternal.lua).".into());
            }
            if report.aslr == crate::client_setup::AslrOutcome::Disabled {
                lines.push("Switched ASLR off in SGW.exe.".into());
            }
            (true, lines)
        }
        Err(e) => (
            false,
            vec![format!("Not launching: client setup failed: {e}")],
        ),
    }
}

/// Render a worker [`Event`] into the human-readable status-log line
/// the UI appends to its scrollback. Pure formatting — extracted from
/// `drain_events` so each Event arm has at least minimal coverage
/// without needing an egui context. Returns `None` for events that
/// don't translate to a status line on their own (manifest updates,
/// progress ticks).
pub(super) fn status_line_for(event: &Event) -> Option<String> {
    Some(match event {
        Event::AdoptComplete => {
            "Adopted existing install — patches will apply on top (seed bytes not verified).".into()
        }
        Event::AdoptError(e) => format!("Adopt failed: {e}"),
        Event::Wiped { kind, report } => format!(
            "Wiped {kind}: {} item(s), {} freed",
            report.entries_removed,
            human_bytes(report.bytes_freed)
        ),
        Event::WipeError(e) => format!("Wipe failed: {e}"),
        Event::InstallStarted => "Install started.".into(),
        Event::InstallCancelled => "Install cancelled; finished steps are kept.".into(),
        Event::Refused { reason, .. } => format!("Not started: {reason}."),
        Event::GameExited { pid, exit_code } => match exit_code {
            Some(code) => format!("Game exited (pid {pid}, exit code {code})."),
            None => format!("Game exited (pid {pid})."),
        },
        Event::GameUntracked { pid } => format!(
            "Game started (pid {pid}) but cannot be followed; watching for SGW.exe instead."
        ),
        Event::OpenFolderError(e) => format!("Could not open the folder: {e}"),
        Event::InstallComplete => "Install complete.".into(),
        Event::InstallError(e) => format!("Install failed: {e}"),
        Event::Launched(name, pid) => format!("Launched {name} (pid {pid})"),
        Event::LaunchError(e) => format!("Launch failed: {e}"),
        Event::ClientPatchesNote(n) => format!("Client patches: {n}"),
        Event::ClientTelemetryNote(n) => format!("In-game telemetry: {n}"),
        Event::UploadStarted => "Uploading logs…".into(),
        Event::UploadSkipped(why) => format!("Log upload skipped: {why}"),
        Event::UploadComplete { blob, bytes } => format!("Uploaded {bytes} bytes to {blob}"),
        Event::UploadError(e) => format!("Log upload failed: {e}"),
        Event::TelemetrySessionComplete(o) => {
            let sha_short: String = o.bundle_sha256.chars().take(12).collect();
            format!(
                "Telemetry session complete — {} events, {} dropped, bundle {} (sha {})",
                o.event_count,
                o.dropped_lines,
                human_bytes(o.bundle_bytes),
                if sha_short.is_empty() {
                    "n/a"
                } else {
                    sha_short.as_str()
                }
            )
        }
        Event::TelemetrySessionError(e) => format!("Telemetry session error: {e}"),
        Event::Update(u) => return update_banner::update_status_line(u),
        // Progress + manifest events drive other UI state, not the
        // status log. Returning None makes that explicit.
        Event::ManifestFetched { .. } | Event::ManifestError { .. } | Event::Progress(_) => {
            return None
        }
    })
}

/// The download progress label, e.g. `seed: 1.27 GB / 3.85 GB`.
/// [`human_bytes`] already carries the unit, so none is appended.
pub(super) fn download_progress_line(label: &str, downloaded: u64, total: u64) -> String {
    format!(
        "{label}: {} / {}",
        human_bytes(downloaded),
        human_bytes(total)
    )
}

pub(super) fn human_bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut f = n as f64;
    let mut i = 0;
    while f >= 1024.0 && i + 1 < UNITS.len() {
        f /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{f:.2} {}", UNITS[i])
    }
}

#[cfg(test)]
mod tests {
    use super::super::{should_show_adopt_button, MAX_STATUS_LINES};
    use super::{download_progress_line, human_bytes, setup_status_lines, status_line_for};
    use crate::client_setup::{AslrOutcome, SetupReport};

    // Bug shape: a failed client setup (SGW.exe locked, ASLR still on) used
    // to be reported and then launched anyway.
    #[test]
    fn a_failed_client_setup_blocks_the_launch() {
        let err: std::io::Result<SetupReport> = Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "SGW.exe is locked",
        ));
        let (ok, lines) = setup_status_lines(&err);
        assert!(!ok);
        assert!(lines[0].starts_with("Not launching") && lines[0].contains("locked"));

        let done: std::io::Result<SetupReport> = Ok(SetupReport {
            restored_names: Vec::new(),
            login_servers_written: true,
            aslr: AslrOutcome::Disabled,
        });
        let (ok, lines) = setup_status_lines(&done);
        assert!(ok);
        assert_eq!(lines.len(), 2);
    }

    /// The EULA repair is visible in the status log, naming both spellings.
    #[test]
    fn a_restored_file_name_is_reported() {
        let eula = std::path::Path::new("Working/SGWGame/Content/UI/Startup/EULA");
        let done: std::io::Result<SetupReport> = Ok(SetupReport {
            restored_names: vec![crate::client_setup::Restored {
                from: eula.join("eula.lua"),
                to: eula.join("EULA.lua"),
            }],
            login_servers_written: false,
            aslr: AslrOutcome::AlreadyOff,
        });
        let (ok, lines) = setup_status_lines(&done);
        assert!(ok);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            lines[0].contains("eula.lua") && lines[0].contains("stock name EULA.lua"),
            "{}",
            lines[0]
        );
    }
    use crate::client_paths::WipeReport;
    use crate::worker::Event;

    #[test]
    fn human_bytes_formats_units() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(2048), "2.00 KB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.00 MB");
    }

    // Bug shape: the line read "seed: 1.27 GB / 3.85 GB bytes", the unit
    // printed twice.
    #[test]
    fn download_progress_line_prints_the_unit_once() {
        const GIB: u64 = 1024 * 1024 * 1024;
        assert_eq!(
            download_progress_line("seed", GIB * 127 / 100, GIB * 385 / 100),
            "seed: 1.27 GB / 3.85 GB"
        );
        assert_eq!(
            download_progress_line("patch x", 10, 20),
            "patch x: 10 B / 20 B"
        );
    }

    // Drives the same drain-on-overflow shape that `push_status` uses, with
    // a Vec we can inspect directly. Keeps the test free of the full
    // LauncherApp construction (which requires a tokio runtime).
    fn push_capped(buf: &mut Vec<String>, line: String) {
        buf.push(line);
        if buf.len() > MAX_STATUS_LINES {
            let overflow = buf.len() - MAX_STATUS_LINES;
            buf.drain(0..overflow);
        }
    }

    #[test]
    fn push_status_caps_at_max_lines() {
        let mut buf = Vec::new();
        for i in 0..(MAX_STATUS_LINES + 25) {
            push_capped(&mut buf, format!("line {i}"));
        }
        assert_eq!(buf.len(), MAX_STATUS_LINES);
        // Oldest 25 should have been dropped.
        assert_eq!(buf.first().unwrap(), "line 25");
        assert_eq!(
            buf.last().unwrap(),
            &format!("line {}", MAX_STATUS_LINES + 24)
        );
    }

    #[test]
    fn push_status_under_cap_does_not_drain() {
        let mut buf = Vec::new();
        for i in 0..10 {
            push_capped(&mut buf, format!("{i}"));
        }
        assert_eq!(buf.len(), 10);
        assert_eq!(buf.first().unwrap(), "0");
    }

    // Empty install dir: no SGW.exe + no marker → the Install panel
    // should NOT surface the Adopt affordance (nothing to adopt).
    #[test]
    fn should_show_adopt_button_false_on_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!should_show_adopt_button(dir.path()));
    }

    // SGW.exe present + no marker → adopt is the user's least-destructive
    // path forward. This is the trigger condition.
    #[test]
    fn should_show_adopt_button_true_when_unmanaged_install_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"").unwrap();
        assert!(should_show_adopt_button(dir.path()));
    }

    // Marker file already present → install is launcher-managed; adopt
    // is a no-op (and would refuse with AlreadyManaged anyway). Hiding
    // the button keeps the UI honest.
    #[test]
    fn should_show_adopt_button_false_when_already_managed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"").unwrap();
        std::fs::write(
            crate::state::InstalledState::path(dir.path()),
            r#"{"applied_patches":[],"seed_sha256":"h"}"#,
        )
        .unwrap();
        assert!(!should_show_adopt_button(dir.path()));
    }

    // status_line_for covers every Event variant that produces a
    // status entry. Wiped is the only one with non-trivial formatting
    // (bytes-freed → human_bytes) — pin its exact shape against a
    // realistic report.
    #[test]
    fn status_line_for_formats_adopt_complete() {
        let line = status_line_for(&Event::AdoptComplete).unwrap();
        assert!(line.contains("Adopted"), "got: {line}");
        assert!(
            line.contains("not verified"),
            "must surface the trust trade-off, got: {line}"
        );
    }

    #[test]
    fn status_line_for_formats_client_patches_note() {
        let line =
            status_line_for(&Event::ClientPatchesNote("off (launcher setting).".into())).unwrap();
        assert_eq!(line, "Client patches: off (launcher setting).");
    }

    #[test]
    fn status_line_for_formats_client_telemetry_note() {
        let line = status_line_for(&Event::ClientTelemetryNote("unavailable: x.".into())).unwrap();
        assert_eq!(line, "In-game telemetry: unavailable: x.");
    }

    #[test]
    fn status_line_for_formats_adopt_error() {
        let line = status_line_for(&Event::AdoptError("boom".into())).unwrap();
        assert_eq!(line, "Adopt failed: boom");
    }

    #[test]
    fn status_line_for_formats_wiped_with_human_bytes() {
        let line = status_line_for(&Event::Wiped {
            kind: "Cache.en-US".into(),
            report: WipeReport {
                entries_removed: 3,
                bytes_freed: 5 * 1024 * 1024,
            },
        })
        .unwrap();
        // Pin both the item count and the human-bytes rendering so a
        // future change to either thread shows up as a test diff.
        assert_eq!(line, "Wiped Cache.en-US: 3 item(s), 5.00 MB freed");
    }

    #[test]
    fn status_line_for_formats_wipe_error() {
        let line = status_line_for(&Event::WipeError("permission denied".into())).unwrap();
        assert_eq!(line, "Wipe failed: permission denied");
    }

    #[test]
    fn status_line_for_returns_none_for_progress_and_manifest_events() {
        // These drive UI state directly (progress bars, manifest
        // summary panel) — they don't belong in the scrolling status
        // log. Returning None enforces that at the type level.
        assert!(status_line_for(&Event::ManifestError {
            url: String::new(),
            message: "x".into(),
        })
        .is_none());
        assert!(
            status_line_for(&Event::Progress(crate::install::Progress::Downloading {
                label: "seed".into(),
                downloaded: 0,
                total: 0,
            },))
            .is_none()
        );
    }
}
