//! `labd.log` with size rotation: past [`MAX_BYTES`] the file becomes
//! `labd.1.log` (older ones shift to `.2` .. `.KEEP`, the oldest is deleted)
//! and a fresh `labd.log` starts. The live log keeps one stable name, which is
//! what `Get-Content -Wait` and the runbook point at.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Rotate past 10 MiB.
pub const MAX_BYTES: u64 = 10 * 1024 * 1024;
/// Rotated files kept beside the live one.
pub const KEEP: u32 = 5;

/// An append-only log file that rotates itself by size.
#[derive(Debug)]
pub struct RotatingFile {
    path: PathBuf,
    file: File,
    written: u64,
    max_bytes: u64,
    keep: u32,
}

impl RotatingFile {
    pub fn open(path: &Path, max_bytes: u64, keep: u32) -> io::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let written = file.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(Self {
            path: path.to_path_buf(),
            file,
            written,
            max_bytes,
            keep,
        })
    }

    /// `labd.log` -> `labd.<n>.log`.
    fn numbered(&self, n: u32) -> PathBuf {
        let stem = self
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "labd".into());
        let name = match self.path.extension() {
            Some(ext) => format!("{stem}.{n}.{}", ext.to_string_lossy()),
            None => format!("{stem}.{n}"),
        };
        self.path.with_file_name(name)
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.file.flush()?;
        if self.keep == 0 {
            self.file = File::create(&self.path)?;
            self.written = 0;
            return Ok(());
        }
        let _ = std::fs::remove_file(self.numbered(self.keep));
        for n in (1..self.keep).rev() {
            let from = self.numbered(n);
            if from.exists() {
                let _ = std::fs::rename(&from, self.numbered(n + 1));
            }
        }
        // Windows refuses to rename an open file only when it was opened
        // without FILE_SHARE_DELETE; std opens with it, so this works with
        // the handle still open. Reopen the fresh file afterwards.
        std::fs::rename(&self.path, self.numbered(1))?;
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        self.written = 0;
        Ok(())
    }
}

impl Write for RotatingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.written > 0 && self.written + buf.len() as u64 > self.max_bytes {
            // A failed rotation must not lose the line: keep writing to the
            // current file and try again on the next write.
            if let Err(e) = self.rotate() {
                eprintln!("labd log rotation failed: {e}");
            }
        }
        let n = self.file.write(buf)?;
        self.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_shifts_files_and_keeps_the_cap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("labd.log");
        let mut f = RotatingFile::open(&path, 10, 2).unwrap();
        for line in ["aaaaaaaa\n", "bbbbbbbb\n", "cccccccc\n", "dddddddd\n"] {
            f.write_all(line.as_bytes()).unwrap();
        }
        f.flush().unwrap();
        let read = |p: &str| std::fs::read_to_string(dir.path().join(p)).unwrap();
        assert_eq!(read("labd.log"), "dddddddd\n");
        assert_eq!(read("labd.1.log"), "cccccccc\n");
        assert_eq!(read("labd.2.log"), "bbbbbbbb\n");
        assert!(!dir.path().join("labd.3.log").exists(), "keep = 2");
    }

    #[test]
    fn reopening_appends_and_counts_the_existing_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("labd.log");
        std::fs::write(&path, "123456789\n").unwrap();
        let mut f = RotatingFile::open(&path, 12, 1).unwrap();
        f.write_all(b"next\n").unwrap();
        f.flush().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "next\n");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("labd.1.log")).unwrap(),
            "123456789\n"
        );
    }
}
