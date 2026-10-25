//! Per-launch game logs (PD-28, D-PD17): Wine's stderr and DXVK's logs for one
//! Play, under `state/logs/game/<operation>/`. They contain local paths, so they
//! stay on this computer: the debug log upload reads only `launcher.*.log` in
//! the app-data `logs/` folder (a sibling of `state/`), never this folder.
use std::path::{Path, PathBuf};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

/// The most bytes of the helper's stderr kept per launch; the rest is read and dropped.
pub(crate) const STDERR_CAP: u64 = 4 * 1024 * 1024;
/// How many operation folders `logs/game/` keeps, the new one included.
const KEEP: usize = 5;

/// Creates `state_root/logs/game/<operation>/`, then deletes all but the 4
/// newest other operation folders (by folder mtime). Delete errors are ignored.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn folder(state_root: &Path, operation: Uuid) -> std::io::Result<PathBuf> {
    let games = state_root.join("logs").join("game");
    let folder = games.join(operation.to_string());
    std::fs::create_dir_all(&folder)?;
    let mut older: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&games)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
        .map(|entry| entry.path())
        .filter(|path| *path != folder)
        .map(|path| {
            let modified = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            (modified, path)
        })
        .collect();
    // Newest first; the new folder always stays, whatever the clock says.
    older.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (_, path) in older.into_iter().skip(KEEP - 1) {
        let _ = std::fs::remove_dir_all(path);
    }
    Ok(folder)
}

/// Copies `from` into `to` until EOF, writing at most `cap` bytes. Past the cap
/// (or after a write error) it keeps reading and drops the bytes, so the
/// helper never blocks on a full pipe.
pub(crate) fn copy_capped(
    mut from: impl AsyncRead + Unpin + Send + 'static,
    mut to: tokio::fs::File,
    cap: u64,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 64 * 1024];
        let mut written = 0u64;
        loop {
            let read = match from.read(&mut buffer).await {
                Ok(0) | Err(_) => break,
                Ok(read) => read,
            };
            let keep = cap.saturating_sub(written).min(read as u64) as usize;
            if keep == 0 {
                continue;
            }
            if to.write_all(&buffer[..keep]).await.is_err() || to.flush().await.is_err() {
                written = cap;
                continue;
            }
            written += keep as u64;
        }
    })
}

#[cfg(all(test, unix))]
#[path = "game_logs_tests.rs"]
mod tests;
