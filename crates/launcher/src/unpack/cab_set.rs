//! MakeCAB installer cabinet sets.
//!
//! The 2009 installer ships its payload as `Data\DATA1.CAB`..`DATA4.CAB`,
//! built by `makecab.exe` from a directive file. MakeCAB also writes an
//! index, `DATA.INF`:
//!
//! ```text
//! [disk list]
//! Disk 1, Disc_1
//! [cabinet list]
//! Disk 1, Cabinet 1, DATA1.CAB
//! Disk 2, Cabinet 2, DATA2.CAB
//! [file list]
//! 84: Cabinet 1, Working\binaries\SGW.exe, 31228448
//! ```
//!
//! The file paths inside the cabinets are the installed layout, so expanding
//! the cabinets in order into the install directory reproduces what the
//! installer would lay down. Files continue across cabinet boundaries (each
//! cabinet is capped at 1 GiB), so the set has to be expanded as a chain;
//! Windows' FDI API in `cabinet.dll` does that, see [`super::fdi`].

use std::path::{Path, PathBuf};

use super::{UnpackError, UnpackSink};

/// A cabinet set found on disk, ready to expand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CabSet {
    /// Directory holding the `.INF` and the cabinets.
    pub dir: PathBuf,
    /// Cabinet file names in chain order.
    pub cabinets: Vec<String>,
    /// Number of entries in the `[file list]`, for progress.
    pub file_count: usize,
}

/// Parse a MakeCAB `.INF`. Returns `None` when the text has no
/// `[cabinet list]`, i.e. it is some other kind of INF.
pub(super) fn parse_inf(text: &str) -> Option<(Vec<String>, usize)> {
    let mut section = "";
    let mut cabinets: Vec<(u32, String)> = Vec::new();
    let mut files = 0;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            section = line;
            continue;
        }
        if line.is_empty() {
            continue;
        }
        match section {
            "[cabinet list]" => {
                // "Disk 1, Cabinet 1, DATA1.CAB"
                let parts: Vec<&str> = line.split(',').map(str::trim).collect();
                if let [_, cab, name] = parts.as_slice() {
                    let n = cab.strip_prefix("Cabinet ")?.parse().ok()?;
                    cabinets.push((n, (*name).to_string()));
                }
            }
            "[file list]" => files += 1,
            _ => {}
        }
    }
    if cabinets.is_empty() {
        return None;
    }
    cabinets.sort_by_key(|(n, _)| *n);
    Some((cabinets.into_iter().map(|(_, name)| name).collect(), files))
}

/// Look for a MakeCAB `.INF` (and its cabinets) up to three directories
/// below `root`. The client RAR keeps it at `Data\DATA.INF`.
pub(super) fn find_installer(root: &Path) -> Result<Option<CabSet>, UnpackError> {
    find_in(root, 0)
}

fn find_in(dir: &Path, depth: usize) -> Result<Option<CabSet>, UnpackError> {
    let mut subdirs = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            subdirs.push(path);
            continue;
        }
        let is_inf = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("inf"));
        if !is_inf {
            continue;
        }
        let text = String::from_utf8_lossy(&std::fs::read(&path)?).into_owned();
        let Some((cabinets, file_count)) = parse_inf(&text) else {
            continue;
        };
        for name in &cabinets {
            // A cabinet name is a bare file name next to the INF.
            if name.contains(['\\', '/', ':']) || !dir.join(name).is_file() {
                return Err(UnpackError::Cab(format!(
                    "{} lists cabinet {name:?}, which is not next to it",
                    path.display()
                )));
            }
        }
        return Ok(Some(CabSet {
            dir: dir.to_path_buf(),
            cabinets,
            file_count,
        }));
    }
    if depth < 3 {
        subdirs.sort();
        for sub in subdirs {
            if let Some(set) = find_in(&sub, depth + 1)? {
                return Ok(Some(set));
            }
        }
    }
    Ok(None)
}

/// Expand every cabinet of `set` into `dest`, in chain order.
#[cfg(windows)]
pub(super) fn expand(set: &CabSet, dest: &Path, sink: &UnpackSink) -> Result<(), UnpackError> {
    super::fdi::expand_chain(&set.dir, &set.cabinets, dest, set.file_count, sink)
}

/// Cabinet sets are expanded with Windows' `cabinet.dll`; the game only runs
/// on Windows, so other platforms report it instead of half-installing.
#[cfg(not(windows))]
pub(super) fn expand(set: &CabSet, _dest: &Path, _sink: &UnpackSink) -> Result<(), UnpackError> {
    Err(UnpackError::Cab(format!(
        "expanding the cabinet set in {} needs Windows",
        set.dir.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The first lines of the real DATA.INF from the archive.org client RAR.
    const REAL_INF_HEAD: &str = "[disk list]\r\nDisk 1, Disc_1\r\nDisk 2, Disc_2\r\n\
        Disk 3, Disc_3\r\nDisk 4, Disc_4\r\n[cabinet list]\r\n\
        Disk 1, Cabinet 1, DATA1.CAB\r\nDisk 2, Cabinet 2, DATA2.CAB\r\n\
        Disk 3, Cabinet 3, DATA3.CAB\r\nDisk 4, Cabinet 4, DATA4.CAB\r\n[file list]\r\n\
        1: Cabinet 1, Common\\res\\entities\\defs\\interfaces\\ClientCache.def, 1473\r\n\
        84: Cabinet 1, Working\\binaries\\SGW.exe, 31228448\r\n";

    #[test]
    fn parses_the_real_installer_index() {
        let (cabs, files) = parse_inf(REAL_INF_HEAD).unwrap();
        assert_eq!(cabs, ["DATA1.CAB", "DATA2.CAB", "DATA3.CAB", "DATA4.CAB"]);
        assert_eq!(files, 2);
    }

    #[test]
    fn orders_cabinets_by_number_not_by_line() {
        let inf = "[cabinet list]\nDisk 2, Cabinet 2, B.CAB\nDisk 1, Cabinet 1, A.CAB\n";
        assert_eq!(parse_inf(inf).unwrap().0, ["A.CAB", "B.CAB"]);
    }

    #[test]
    fn other_inf_files_are_not_cabinet_sets() {
        assert!(parse_inf("[Version]\nSignature=\"$Windows NT$\"\n").is_none());
    }

    #[test]
    fn finds_the_set_under_data() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("Data");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("DATA.INF"), REAL_INF_HEAD).unwrap();
        for n in 1..=4 {
            std::fs::write(data.join(format!("DATA{n}.CAB")), b"MSCF").unwrap();
        }
        std::fs::write(dir.path().join("SetupQA.exe"), b"MZ").unwrap();
        let set = find_installer(dir.path()).unwrap().unwrap();
        assert_eq!(set.dir, data);
        assert_eq!(set.cabinets.len(), 4);
    }

    // A truncated download that lost a cabinet must fail loudly rather
    // than install three quarters of the game.
    #[test]
    fn missing_cabinet_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("DATA.INF"), REAL_INF_HEAD).unwrap();
        std::fs::write(dir.path().join("DATA1.CAB"), b"MSCF").unwrap();
        let err = find_installer(dir.path()).unwrap_err();
        assert!(matches!(err, UnpackError::Cab(_)), "{err:?}");
    }

    #[test]
    fn plain_content_has_no_set() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"hi").unwrap();
        assert!(find_installer(dir.path()).unwrap().is_none());
    }
}
