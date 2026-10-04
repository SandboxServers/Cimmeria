//! "Open in Explorer" for the install folder.
//!
//! Opening a folder only shows it: a missing folder is reported, never
//! created, so the button cannot make directories at a path the player
//! has not confirmed.

use std::path::{Path, PathBuf};

use super::{Event, Worker};

/// Why a folder cannot be opened, or `None` when it can.
pub(super) fn open_refusal(dir: &Path) -> Option<String> {
    if dir.as_os_str().is_empty() {
        return Some("no install folder is set".into());
    }
    if !dir.is_dir() {
        return Some(format!(
            "{} does not exist yet; it is created when the game installs",
            dir.display()
        ));
    }
    None
}

impl Worker {
    pub(super) fn spawn_open_folder(&self, dir: PathBuf) {
        let events_tx = self.events_tx.clone();
        self.runtime.spawn(async move {
            if let Some(why) = open_refusal(&dir) {
                let _ = events_tx.send(Event::OpenFolderError(why));
                return;
            }
            if let Err(e) = open_in_file_manager(&dir) {
                tracing::warn!(dir = %dir.display(), error = %e, "could not open the folder");
                let _ = events_tx.send(Event::OpenFolderError(format!(
                    "could not open {}: {e}",
                    dir.display()
                )));
            }
        });
    }
}

#[cfg(windows)]
fn open_in_file_manager(dir: &Path) -> std::io::Result<()> {
    // explorer.exe exits non-zero even when it opens the window, so only
    // a failure to start it is an error.
    std::process::Command::new("explorer.exe")
        .arg(dir)
        .spawn()
        .map(|_| ())
}

#[cfg(not(windows))]
fn open_in_file_manager(dir: &Path) -> std::io::Result<()> {
    std::process::Command::new("xdg-open")
        .arg(dir)
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::open_refusal;

    #[test]
    fn a_missing_folder_is_reported_not_created() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("Stargate Worlds");
        let why = open_refusal(&missing).expect("refused");
        assert!(why.contains("does not exist"), "{why}");
        assert!(!missing.exists(), "opening must not create the folder");
        assert!(open_refusal(tmp.path()).is_none());
        assert!(open_refusal(std::path::Path::new("")).is_some());
    }
}
