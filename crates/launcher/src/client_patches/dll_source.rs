//! Where the launcher gets `cimmeria-client-patches.dll`.
//!
//! In order:
//!
//! 1. `client_patches.dll_override` in the launcher config, for testing a
//!    local build. A missing file there is an error, not a fall-through:
//!    the tester asked for that DLL.
//! 2. The copy bundled into the launcher, written to
//!    `<launcher dir>/client-patches/<sha256 prefix>/` (see
//!    [`crate::bundled`]).
//! 3. `cimmeria-client-patches.dll` beside the launcher, for dev builds,
//!    which embed nothing.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::bundled::{self, Found};
use crate::config::ClientPatchesSettings;

/// The DLL's file name wherever the launcher puts it.
pub const DLL_FILE_NAME: &str = "cimmeria-client-patches.dll";

/// Subdirectory of the launcher's directory for the written-out copy.
const BUNDLE_DIR: &str = "client-patches";

/// The bundled DLL; empty when this build embeds none.
static EMBEDDED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cimmeria-client-patches.dll"));

/// Where the DLL to inject came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DllSource {
    Override(PathBuf),
    Bundled(PathBuf),
    BesideLauncher(PathBuf),
}

impl DllSource {
    pub fn path(&self) -> &Path {
        match self {
            Self::Override(p) | Self::Bundled(p) | Self::BesideLauncher(p) => p,
        }
    }
}

#[derive(Debug, Error)]
pub enum DllUnavailable {
    #[error("the configured client_patches.dll_override {0} does not exist")]
    OverrideMissing(PathBuf),
    #[error("this launcher build does not bundle {DLL_FILE_NAME}, and there is none in {0}")]
    NotBundled(PathBuf),
    #[error("could not write the bundled {DLL_FILE_NAME}: {0}")]
    Write(#[from] std::io::Error),
}

/// Resolve the DLL for this launch. `launcher_dir` is where the
/// launcher's own files live ([`crate::config::exe_dir`]).
pub fn resolve(
    settings: &ClientPatchesSettings,
    launcher_dir: &Path,
) -> Result<DllSource, DllUnavailable> {
    resolve_with(settings, launcher_dir, EMBEDDED)
}

fn resolve_with(
    settings: &ClientPatchesSettings,
    launcher_dir: &Path,
    embedded: &[u8],
) -> Result<DllSource, DllUnavailable> {
    if let Some(path) = &settings.dll_override {
        return if path.is_file() {
            // LoadLibraryW runs in SGW.exe, whose working directory is the
            // install dir, so a relative path would name a different file.
            Ok(DllSource::Override(std::path::absolute(path)?))
        } else {
            Err(DllUnavailable::OverrideMissing(path.clone()))
        };
    }
    match bundled::find(embedded, BUNDLE_DIR, DLL_FILE_NAME, launcher_dir)? {
        Some(Found::Bundled(p)) => Ok(DllSource::Bundled(p)),
        Some(Found::BesideLauncher(p)) => Ok(DllSource::BesideLauncher(p)),
        None => Err(DllUnavailable::NotBundled(launcher_dir.to_path_buf())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(override_path: Option<PathBuf>) -> ClientPatchesSettings {
        ClientPatchesSettings {
            enabled: true,
            dll_override: override_path,
        }
    }

    /// build.rs embeds either nothing or a checked PE image.
    #[test]
    fn embedded_dll_is_empty_or_a_pe_image() {
        assert!(EMBEDDED.is_empty() || EMBEDDED.starts_with(b"MZ"));
    }

    #[test]
    fn override_wins_over_bundled_and_beside() {
        let dir = tempfile::tempdir().unwrap();
        let custom = dir.path().join("custom.dll");
        std::fs::write(&custom, b"MZcustom").unwrap();
        std::fs::write(dir.path().join(DLL_FILE_NAME), b"MZbeside").unwrap();
        let src = resolve_with(&settings(Some(custom.clone())), dir.path(), b"MZbundled").unwrap();
        assert_eq!(src, DllSource::Override(custom));
    }

    /// A tester who named a DLL must not silently get a different one.
    #[test]
    fn missing_override_is_an_error_not_a_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(DLL_FILE_NAME), b"MZbeside").unwrap();
        let err = resolve_with(
            &settings(Some(dir.path().join("gone.dll"))),
            dir.path(),
            b"MZbundled",
        )
        .unwrap_err();
        assert!(matches!(err, DllUnavailable::OverrideMissing(_)), "{err}");
    }

    #[test]
    fn bundled_wins_over_beside() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(DLL_FILE_NAME), b"MZbeside").unwrap();
        let src = resolve_with(&settings(None), dir.path(), b"MZbundled").unwrap();
        assert!(matches!(src, DllSource::Bundled(_)), "{src:?}");
        assert_eq!(std::fs::read(src.path()).unwrap(), b"MZbundled");
    }

    #[test]
    fn dev_build_uses_the_dll_beside_the_launcher() {
        let dir = tempfile::tempdir().unwrap();
        let beside = dir.path().join(DLL_FILE_NAME);
        std::fs::write(&beside, b"MZbeside").unwrap();
        assert_eq!(
            resolve_with(&settings(None), dir.path(), b"").unwrap(),
            DllSource::BesideLauncher(beside)
        );
    }

    #[test]
    fn nothing_bundled_and_nothing_beside_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_with(&settings(None), dir.path(), b"").unwrap_err();
        assert!(matches!(err, DllUnavailable::NotBundled(_)), "{err}");
    }
}
