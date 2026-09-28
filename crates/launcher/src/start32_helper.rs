//! Where the launcher gets the 32-bit `sgw-start32.exe` helper.
//!
//! The launcher is 64-bit and `SGW.exe` is 32-bit, so the launcher cannot
//! inject into it itself (see `cimmeria_client_launch::start32`). It runs
//! this helper instead, for every launch that loads a DLL.
//!
//! A release launcher embeds the helper and keeps it at one stable path,
//! `<launcher dir>/sgw-start32.exe`, rewritten only when this launcher
//! carries different bytes. An unsigned exe that starts a process suspended
//! and writes into it is what antivirus heuristics look for, so it gets a
//! stable name, a version resource, and a place beside the launcher an
//! exclusion can name; never `%TEMP%`, never a per-version directory. A dev
//! build embeds nothing and uses a helper already beside it.

use std::path::{Path, PathBuf};

use cimmeria_client_launch::start32::HELPER_EXE_NAME;
use thiserror::Error;

use crate::bundled;

/// The bundled helper; empty when this build embeds none.
static EMBEDDED: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sgw-start32.exe"));

#[derive(Debug, Error)]
pub enum HelperUnavailable {
    #[error("this launcher build does not bundle {HELPER_EXE_NAME}, and there is none in {0}")]
    NotBundled(PathBuf),
    #[error("could not write {HELPER_EXE_NAME} beside the launcher: {0}")]
    Write(#[from] std::io::Error),
}

/// The helper for this launch.
pub fn resolve(launcher_dir: &Path) -> Result<PathBuf, HelperUnavailable> {
    resolve_with(launcher_dir, EMBEDDED)
}

fn resolve_with(launcher_dir: &Path, embedded: &[u8]) -> Result<PathBuf, HelperUnavailable> {
    let path = launcher_dir.join(HELPER_EXE_NAME);
    if !embedded.is_empty() {
        bundled::write_stable(embedded, &path)?;
        return Ok(path);
    }
    if path.is_file() {
        Ok(path)
    } else {
        Err(HelperUnavailable::NotBundled(launcher_dir.to_path_buf()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// build.rs embeds either nothing or a checked PE image.
    #[test]
    fn embedded_helper_is_empty_or_a_pe_image() {
        assert!(EMBEDDED.is_empty() || EMBEDDED.starts_with(b"MZ"));
    }

    /// One stable path beside the launcher, whatever the version, so an
    /// antivirus exclusion for it survives launcher updates.
    #[test]
    fn bundled_helper_lives_at_one_stable_path_beside_the_launcher() {
        let dir = tempfile::tempdir().unwrap();
        let v1 = resolve_with(dir.path(), b"MZv1").unwrap();
        assert_eq!(v1, dir.path().join(HELPER_EXE_NAME));
        let v2 = resolve_with(dir.path(), b"MZv2").unwrap();
        assert_eq!(v1, v2, "a new launcher version keeps the same path");
        assert_eq!(std::fs::read(&v2).unwrap(), b"MZv2");
    }

    /// Written into the launcher's own directory, nowhere else (never a
    /// temporary directory).
    #[test]
    fn bundled_helper_is_written_into_the_launcher_dir() {
        let dir = tempfile::tempdir().unwrap();
        let path = resolve_with(dir.path(), b"MZ").unwrap();
        assert_eq!(path.parent().unwrap(), dir.path());
    }

    #[test]
    fn dev_build_uses_the_helper_beside_the_launcher() {
        let dir = tempfile::tempdir().unwrap();
        let beside = dir.path().join(HELPER_EXE_NAME);
        std::fs::write(&beside, b"MZ").unwrap();
        assert_eq!(resolve_with(dir.path(), b"").unwrap(), beside);
    }

    #[test]
    fn no_helper_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            resolve_with(dir.path(), b""),
            Err(HelperUnavailable::NotBundled(_))
        ));
    }
}
