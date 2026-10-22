//! Looks for a game archive the player already downloaded, so the seed need not
//! be fetched again. Only the top level of the Downloads folder is listed, and
//! only a name and size match is looked at: nothing is read or hashed here.

use cimmeria_launcher_core::manifest::SeedEntry;
use std::path::{Path, PathBuf};

/// The player's Downloads folder: the known folder on Windows, `~/Downloads`
/// elsewhere. `None` when the location cannot be found.
#[cfg(windows)]
pub fn downloads_folder() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_Downloads, SHGetKnownFolderPath},
    };
    let mut path = std::ptr::null_mut();
    // SAFETY: the folder id is a constant and the out pointer is valid. On success
    // the string is NUL-terminated, read once here and then freed exactly once.
    unsafe {
        if SHGetKnownFolderPath(&FOLDERID_Downloads, 0, std::ptr::null_mut(), &mut path) != 0
            || path.is_null()
        {
            return None;
        }
        let mut len = 0;
        while *path.add(len) != 0 {
            len += 1;
        }
        let folder = PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(
            path, len,
        )));
        CoTaskMemFree(path.cast());
        Some(folder)
    }
}

/// The player's Downloads folder: `~/Downloads`.
#[cfg(not(windows))]
pub fn downloads_folder() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Downloads"))
}

/// The first regular file directly in `dir` whose name is the seed's archive
/// file name and whose size is exactly the seed's size. Subfolders, links and
/// other files are never offered.
pub fn find_downloads_offer(dir: &Path, seed: &SeedEntry) -> Option<PathBuf> {
    let wanted = archive_file_name(&seed.blob)?;
    let entries = std::fs::read_dir(dir)
        .map_err(|error| log_offer_unreadable(&error))
        .ok()?;
    entries.flatten().find_map(|entry| {
        if entry.file_name().to_str() != Some(wanted.as_str()) {
            return None;
        }
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)
            .map_err(|error| log_offer_unreadable(&error))
            .ok()?;
        (meta.is_file() && meta.len() == seed.size).then_some(path)
    })
}

/// LT-15: an offer the launcher could not look at is logged as a closed
/// `seed_archive_failed` row, and the archive is fetched instead.
fn log_offer_unreadable(error: &std::io::Error) {
    tracing::warn!(
        target: "launcher.install",
        event = "seed_archive_failed",
        reason = "offer_unreadable",
        error_kind = crate::game_telemetry::launcher_event::kind(error),
        "seed archive: the Downloads folder could not be read; the archive is fetched again"
    );
}

/// The archive.org file name: the last path segment of the blob URL, percent-decoded.
pub fn archive_file_name(blob: &str) -> Option<String> {
    let without_query = blob.split(['?', '#']).next()?;
    let segment = without_query.rsplit('/').next()?;
    let name = percent_decode(segment)?;
    (!name.is_empty() && !name.contains(['/', '\\'])).then_some(name)
}

fn percent_decode(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.bytes();
    while let Some(byte) = rest.next() {
        if byte == b'%' {
            let high = hex_digit(rest.next()?)?;
            let low = hex_digit(rest.next()?)?;
            bytes.push(high * 16 + low);
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).ok()
}

fn hex_digit(byte: u8) -> Option<u8> {
    (byte as char).to_digit(16).map(|digit| digit as u8)
}
