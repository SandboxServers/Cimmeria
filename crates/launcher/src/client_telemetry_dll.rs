//! The `cimmeria-client-telemetry` DLL, injected into `SGW.exe` only when
//! the player opted in to telemetry (owner decision 2026-09-29).
//!
//! It goes in after the client-patches DLL
//! ([`crate::client_patches::injection_order`]) and reads the
//! `current-session.json` the launcher writes before the game starts, so
//! its uploads carry the same player session token as the launcher's.
//!
//! Where it comes from, in order:
//!
//! 1. The copy bundled into release launchers, written to
//!    `<launcher dir>/client-telemetry/<sha256 prefix>/` (see
//!    [`crate::bundled`]).
//! 2. `cimmeria-client-telemetry.dll` beside the launcher, for dev builds,
//!    which embed nothing.
//!
//! Whichever it is, a build compiled with the `lab-bridge` feature is
//! refused: that build opens an inbound TCP command channel, which has no
//! place on a player's machine. A DLL that is missing or refused never
//! blocks play; the game starts without it and the status log says why.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::bundled::{self, Found};

/// The DLL's file name wherever the launcher puts it.
pub const DLL_FILE_NAME: &str = "cimmeria-client-telemetry.dll";

/// Subdirectory of the launcher's directory for the written-out copy.
const BUNDLE_DIR: &str = "client-telemetry";

/// The bundled DLL; empty when this build embeds none.
static EMBEDDED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cimmeria-client-telemetry.dll"));

/// Text only a `lab-bridge` build carries in its image. The same literal
/// is `cimmeria_client_telemetry::LAB_BRIDGE_MARKER` (the DLL logs it at
/// boot), the check in this crate's `build.rs`, and the `verify` stage of
/// `tools/launcher-release/build.sh`; a test below pins that all four agree.
pub const LAB_BRIDGE_MARKER: &str = "cimmeria-client-telemetry build flavour: lab-bridge";

#[derive(Debug, Error)]
pub enum DllUnavailable {
    #[error("this launcher build does not bundle {DLL_FILE_NAME}, and there is none in {0}")]
    NotBundled(PathBuf),
    #[error(
        "{0} is a lab-bridge build of the telemetry DLL (it opens a command port); \
         refusing to load it into the game"
    )]
    LabBuild(PathBuf),
    #[error("could not write or read {DLL_FILE_NAME}: {0}")]
    Io(#[from] std::io::Error),
}

/// Resolve the DLL for this launch. `launcher_dir` is where the
/// launcher's own files live ([`crate::config::exe_dir`]).
pub fn resolve(launcher_dir: &Path) -> Result<PathBuf, DllUnavailable> {
    resolve_with(launcher_dir, EMBEDDED)
}

fn resolve_with(launcher_dir: &Path, embedded: &[u8]) -> Result<PathBuf, DllUnavailable> {
    let path = match bundled::find(embedded, BUNDLE_DIR, DLL_FILE_NAME, launcher_dir)? {
        Some(Found::Bundled(p) | Found::BesideLauncher(p)) => p,
        None => return Err(DllUnavailable::NotBundled(launcher_dir.to_path_buf())),
    };
    // build.rs already refuses to embed a lab build, but a DLL dropped
    // beside a dev launcher is whatever someone built last.
    if is_lab_bridge_build(&std::fs::read(&path)?) {
        return Err(DllUnavailable::LabBuild(path));
    }
    Ok(path)
}

/// Whether a DLL image is a `lab-bridge` build.
pub fn is_lab_bridge_build(image: &[u8]) -> bool {
    let marker = LAB_BRIDGE_MARKER.as_bytes();
    image.windows(marker.len()).any(|w| w == marker)
}

/// What a player launch does with the telemetry DLL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelemetryDll {
    /// Telemetry is off: the launch is exactly the non-telemetry launch.
    NotOptedIn,
    /// Opted in, but no session could be started, so the DLL would have
    /// no token to upload with. The reason is for the status log.
    NoSession(String),
    /// Inject this DLL after the client patches.
    Inject(PathBuf),
    /// Opted in with a session, but there is no DLL to load.
    Unavailable(String),
}

impl TelemetryDll {
    /// Decide from whether a session started and, only then, the lookup.
    /// No session means the DLL is never looked for (no disk writes).
    pub fn decide(
        session: Result<(), String>,
        resolve: impl FnOnce() -> Result<PathBuf, DllUnavailable>,
    ) -> Self {
        if let Err(why) = session {
            return Self::NoSession(why);
        }
        match resolve() {
            Ok(path) => Self::Inject(path),
            Err(e) => Self::Unavailable(e.to_string()),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Inject(p) => Some(p),
            _ => None,
        }
    }
}

/// How the telemetry DLL fared on one launch: the `outcome` field of the
/// launcher's `client.telemetry_dll.launch` event. SigNoz queries filter
/// on these strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DllOutcome {
    Injected,
    Unavailable,
    /// Found, but the injection failed; the game started without it.
    InjectFailed,
}

impl DllOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Injected => "injected",
            Self::Unavailable => "unavailable",
            Self::InjectFailed => "inject_failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAB_IMAGE: &[u8] = b"MZ....cimmeria-client-telemetry build flavour: lab-bridge....";
    const PLAYER_IMAGE: &[u8] = b"MZ....cimmeria-client-telemetry build flavour: player....";

    /// build.rs embeds either nothing or a checked, non-lab PE image.
    #[test]
    fn embedded_dll_is_empty_or_a_player_pe_image() {
        assert!(EMBEDDED.is_empty() || EMBEDDED.starts_with(b"MZ"));
        assert!(!is_lab_bridge_build(EMBEDDED));
    }

    #[test]
    fn bundled_wins_over_beside() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(DLL_FILE_NAME), b"MZbeside").unwrap();
        let path = resolve_with(dir.path(), PLAYER_IMAGE).unwrap();
        assert!(path.starts_with(dir.path().join(BUNDLE_DIR)), "{path:?}");
        assert_eq!(std::fs::read(&path).unwrap(), PLAYER_IMAGE);
    }

    #[test]
    fn dev_build_uses_the_dll_beside_the_launcher() {
        let dir = tempfile::tempdir().unwrap();
        let beside = dir.path().join(DLL_FILE_NAME);
        std::fs::write(&beside, PLAYER_IMAGE).unwrap();
        assert_eq!(resolve_with(dir.path(), b"").unwrap(), beside);
    }

    /// The missing-DLL case the launch falls back from.
    #[test]
    fn nothing_bundled_and_nothing_beside_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_with(dir.path(), b"").unwrap_err();
        assert!(matches!(err, DllUnavailable::NotBundled(_)), "{err}");
    }

    /// The no-lab-bridge guard: a lab build beside a dev launcher (the
    /// likeliest way one reaches a player) is refused, not injected.
    #[test]
    fn a_lab_bridge_build_beside_the_launcher_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(DLL_FILE_NAME), LAB_IMAGE).unwrap();
        let err = resolve_with(dir.path(), b"").unwrap_err();
        assert!(matches!(err, DllUnavailable::LabBuild(_)), "{err}");
        assert!(err.to_string().contains("lab-bridge"), "{err}");
    }

    #[test]
    fn a_lab_bridge_build_is_refused_even_when_bundled() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_with(dir.path(), LAB_IMAGE).unwrap_err();
        assert!(matches!(err, DllUnavailable::LabBuild(_)), "{err}");
    }

    /// Every copy of the marker text must match, or one check would pass
    /// a lab build the others refuse.
    #[test]
    fn every_copy_of_the_lab_bridge_marker_agrees() {
        let quoted = format!("\"{LAB_BRIDGE_MARKER}\"");
        let copies = [
            ("launcher build.rs", include_str!("../build.rs")),
            (
                "cimmeria-client-telemetry lib.rs",
                include_str!("../../client-telemetry/src/lib.rs"),
            ),
            (
                "tools/launcher-release/build.sh",
                include_str!("../../../tools/launcher-release/build.sh"),
            ),
        ];
        for (name, text) in copies {
            assert!(text.contains(&quoted), "{name} lacks {quoted}");
        }
    }

    #[test]
    fn no_session_never_resolves() {
        let d = TelemetryDll::decide(Err("auth down".into()), || {
            panic!("a launch with no session must not look for the DLL")
        });
        assert_eq!(d, TelemetryDll::NoSession("auth down".into()));
        assert_eq!(d.path(), None);
    }

    #[test]
    fn a_session_and_a_dll_injects_it() {
        let d = TelemetryDll::decide(Ok(()), || Ok(PathBuf::from("t.dll")));
        assert_eq!(d.path(), Some(Path::new("t.dll")));
    }

    #[test]
    fn a_session_without_a_dll_is_unavailable_with_the_reason() {
        let d = TelemetryDll::decide(Ok(()), || {
            Err(DllUnavailable::NotBundled(PathBuf::from("L")))
        });
        match d {
            TelemetryDll::Unavailable(why) => assert!(why.contains("does not bundle"), "{why}"),
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn outcome_labels_are_stable() {
        assert_eq!(DllOutcome::Injected.as_str(), "injected");
        assert_eq!(DllOutcome::Unavailable.as_str(), "unavailable");
        assert_eq!(DllOutcome::InjectFailed.as_str(), "inject_failed");
    }
}
