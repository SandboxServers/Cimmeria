//! SHA-256 of staged files, optionally reporting `Progress::Checking` (C-7).
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::{InstallError, Progress};
use crate::install_progress::{ProgressReporter, ProgressSink};

/// How many bytes pass between two `Checking` reports.
const REPORT_EVERY: u64 = 8 * 1024 * 1024;

pub(super) fn verify_sha256(path: &Path, expected: &str, what: &str) -> Result<(), InstallError> {
    verify_sha256_reporting(path, expected, what, None)
}

/// [`verify_sha256`] that reports `Checking` to `progress` while it reads.
pub(super) fn verify_sha256_reporting(
    path: &Path,
    expected: &str,
    what: &str,
    progress: Option<&ProgressSink>,
) -> Result<(), InstallError> {
    let actual = hash_file_reporting(path, progress)?;
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(InstallError::HashMismatch {
            what: what.to_string(),
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

/// Lower-case hex SHA-256 of the file at `path`.
pub fn hash_file(path: &Path) -> std::io::Result<String> {
    hash_file_reporting(path, None)
}

/// [`hash_file`] that reports `Checking { read, total }` every 8 MiB and once
/// at the end, when `progress` is set.
pub(super) fn hash_file_reporting(
    path: &Path,
    progress: Option<&ProgressSink>,
) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let total = f.metadata()?.len();
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 64];
    let mut read = 0u64;
    let mut next_report = REPORT_EVERY;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        read += n as u64;
        if read >= next_report {
            if let Some(sink) = progress {
                sink.report(Progress::Checking { read, total });
            }
            next_report += REPORT_EVERY;
        }
    }
    if let Some(sink) = progress {
        sink.report(Progress::Checking { read, total });
    }
    // digest 0.11's `Array` output no longer implements `LowerHex`, so the old
    // `{:x}` no longer compiles: hex-encode the bytes explicitly.
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
