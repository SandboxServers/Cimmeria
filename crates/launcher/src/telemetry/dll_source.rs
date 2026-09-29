//! Where the launcher gets `cimmeria-client-telemetry.dll`, the in-game
//! half of telemetry. It goes into `SGW.exe` only when the player opted in
//! and the session handshake succeeded (it reads that session's file when
//! it starts).
//!
//! In order: `telemetry.dll_override` (a tester's build; missing is an
//! error), the copy bundled into release launchers, written to
//! `<launcher dir>/client-telemetry/<sha256 prefix>/`, then a file beside
//! the launcher (dev builds, which embed nothing).

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::bundled::{self, Found};

/// The DLL's file name wherever the launcher puts it.
pub const DLL_FILE_NAME: &str = "cimmeria-client-telemetry.dll";

/// Subdirectory of the launcher's directory for the written-out copy.
const BUNDLE_DIR: &str = "client-telemetry";

/// The bundled DLL; empty when this build embeds none.
static EMBEDDED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cimmeria-client-telemetry.dll"));

#[derive(Debug, Error)]
pub enum DllUnavailable {
    #[error("the configured telemetry.dll_override {0} does not exist")]
    OverrideMissing(PathBuf),
    #[error("this launcher build does not bundle {DLL_FILE_NAME}, and there is none in {0}")]
    NotBundled(PathBuf),
    #[error("could not write the bundled {DLL_FILE_NAME}: {0}")]
    Write(#[from] std::io::Error),
}

/// Resolve the DLL for this launch.
/// `dll_override` is `telemetry.dll_override` from the launcher config.
pub fn resolve(
    dll_override: Option<&Path>,
    launcher_dir: &Path,
) -> Result<PathBuf, DllUnavailable> {
    resolve_with(dll_override, launcher_dir, EMBEDDED)
}

fn resolve_with(
    dll_override: Option<&Path>,
    launcher_dir: &Path,
    embedded: &[u8],
) -> Result<PathBuf, DllUnavailable> {
    if let Some(path) = dll_override {
        return if path.is_file() {
            // LoadLibraryW runs in SGW.exe, whose working directory differs.
            Ok(std::path::absolute(path)?)
        } else {
            Err(DllUnavailable::OverrideMissing(path.to_path_buf()))
        };
    }
    match bundled::find(embedded, BUNDLE_DIR, DLL_FILE_NAME, launcher_dir)? {
        Some(Found::Bundled(p) | Found::BesideLauncher(p)) => Ok(p),
        None => Err(DllUnavailable::NotBundled(launcher_dir.to_path_buf())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_dll_is_empty_or_a_pe_image() {
        assert!(EMBEDDED.is_empty() || EMBEDDED.starts_with(b"MZ"));
    }

    #[test]
    fn override_wins_and_a_missing_one_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let custom = dir.path().join("t.dll");
        std::fs::write(&custom, b"MZ").unwrap();
        std::fs::write(dir.path().join(DLL_FILE_NAME), b"MZbeside").unwrap();
        assert_eq!(
            resolve_with(Some(&custom), dir.path(), b"MZb").unwrap(),
            std::path::absolute(&custom).unwrap()
        );
        let err = resolve_with(Some(&dir.path().join("gone.dll")), dir.path(), b"MZb").unwrap_err();
        assert!(matches!(err, DllUnavailable::OverrideMissing(_)), "{err}");
    }

    #[test]
    fn bundled_copy_is_written_under_its_own_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = resolve_with(None, dir.path(), b"MZbundled").unwrap();
        assert!(
            path.starts_with(dir.path().join(BUNDLE_DIR)),
            "{}",
            path.display()
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"MZbundled");
    }

    #[test]
    fn dev_build_uses_the_copy_beside_the_launcher_or_says_why_not() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_with(None, dir.path(), b"").unwrap_err();
        assert!(matches!(err, DllUnavailable::NotBundled(_)), "{err}");
        std::fs::write(dir.path().join(DLL_FILE_NAME), b"MZ").unwrap();
        assert_eq!(
            resolve_with(None, dir.path(), b"").unwrap(),
            dir.path().join(DLL_FILE_NAME)
        );
    }
}
