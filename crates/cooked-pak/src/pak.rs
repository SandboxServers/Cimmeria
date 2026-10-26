//! Reading and writing cooked PAK archives.
//!
//! The writer's layout is fixed because the presync depends on it: the
//! `_<key>` entries come first in ascending key order, deflated, and
//! `MetaData` comes last, stored, exactly 4 bytes LE. Every entry carries the
//! default zip timestamp, so the same input always gives the same bytes.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

/// The entry that holds the category version.
pub const METADATA_ENTRY: &str = "MetaData";

/// A PAK read into memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pak {
    /// The `MetaData` version, or `None` when the entry is absent or short.
    pub version: Option<u32>,
    /// Record bytes by key, from the `_<key>` entries.
    pub entries: BTreeMap<u32, Vec<u8>>,
}

/// An error reading or writing a PAK.
#[derive(Debug, thiserror::Error)]
pub enum PakError {
    /// The file could not be opened, read or written.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// The ZIP container is malformed or an entry could not be read.
    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),
}

/// The ZIP entry name for a record key: `_{key}`.
pub fn entry_name(key: u32) -> String {
    format!("_{key}")
}

/// The record key of a ZIP entry name, or `None` for any other name.
///
/// Accepts `_<u32>` only. It uses the same `u32` parse as the server's
/// `ResourceCache::load_pak`, so the two read the same entries.
pub fn parse_entry_name(name: &str) -> Option<u32> {
    name.strip_prefix('_')?.parse::<u32>().ok()
}

/// Reads only the version from a PAK's `MetaData` entry.
///
/// A PAK without `MetaData`, or with fewer than 4 bytes in it, gives `None`.
pub fn read_version(path: &Path) -> Result<Option<u32>, PakError> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let mut entry = match archive.by_name(METADATA_ENTRY) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    Ok(read_metadata(&mut entry))
}

/// Reads a whole PAK. Entries whose names are not `_<u32>` or `MetaData`
/// are ignored.
pub fn read_pak(path: &Path) -> Result<Pak, PakError> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let mut version = None;
    let mut entries = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name()?.into_owned();
        if name == METADATA_ENTRY {
            version = read_metadata(&mut entry);
        } else if let Some(key) = parse_entry_name(&name) {
            let mut bytes = Vec::with_capacity(usize::try_from(entry.size()).unwrap_or(0));
            entry.read_to_end(&mut bytes)?;
            entries.insert(key, bytes);
        }
    }
    Ok(Pak { version, entries })
}

/// Writes a PAK: the `_<key>` entries in ascending key order, deflated, then
/// `MetaData` last, stored, holding `version` as 4 bytes LE.
pub fn write_pak<W: Write + Seek>(
    out: W,
    version: u32,
    entries: &BTreeMap<u32, Vec<u8>>,
) -> Result<(), PakError> {
    let mut zip = ZipWriter::new(out);
    let deflated = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(DateTime::default());
    for (key, bytes) in entries {
        zip.start_file(entry_name(*key), deflated)?;
        zip.write_all(bytes)?;
    }
    let stored = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(DateTime::default());
    zip.start_file(METADATA_ENTRY, stored)?;
    zip.write_all(&version.to_le_bytes())?;
    zip.finish()?;
    Ok(())
}

/// The first 4 bytes of a `MetaData` entry as LE, or `None` when it is short.
fn read_metadata<R: Read>(entry: &mut R) -> Option<u32> {
    let mut buf = [0u8; 4];
    entry.read_exact(&mut buf).ok()?;
    Some(u32::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipArchive, ZipWriter};

    use super::*;

    fn sample_entries() -> BTreeMap<u32, Vec<u8>> {
        BTreeMap::from([
            (1, b"alpha".to_vec()),
            (7, b"bravo bravo".to_vec()),
            (300, vec![0u8, 1, 2, 255]),
        ])
    }

    fn write_to_temp(dir: &Path, name: &str, version: u32) -> std::path::PathBuf {
        let path = dir.join(name);
        let file = File::create(&path).unwrap();
        write_pak(file, version, &sample_entries()).unwrap();
        path
    }

    #[test]
    fn write_then_read_round_trips_entries_and_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_to_temp(dir.path(), "cat.pak", 7);
        let pak = read_pak(&path).unwrap();
        assert_eq!(pak.entries, sample_entries());
        assert_eq!(pak.version, Some(7));
        assert_eq!(read_version(&path).unwrap(), Some(7));
    }

    #[test]
    fn metadata_is_last_stored_and_four_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_to_temp(dir.path(), "cat.pak", 9);
        let mut archive = ZipArchive::new(File::open(&path).unwrap()).unwrap();
        let count = archive.len();
        let last = archive.by_index(count - 1).unwrap();
        assert_eq!(last.name().unwrap(), METADATA_ENTRY);
        assert_eq!(last.compression(), CompressionMethod::Stored);
        assert_eq!(last.size(), 4);
        drop(last);
        let names: Vec<String> = (0..count - 1)
            .map(|i| archive.by_index(i).unwrap().name().unwrap().into_owned())
            .collect();
        assert_eq!(names, vec!["_1", "_7", "_300"]);
    }

    #[test]
    fn a_pak_without_metadata_reads_version_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bare.pak");
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("_5", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"five").unwrap();
        std::fs::write(&path, zip.finish().unwrap().into_inner()).unwrap();
        assert_eq!(read_version(&path).unwrap(), None);
        let pak = read_pak(&path).unwrap();
        assert_eq!(pak.version, None);
        assert_eq!(pak.entries, BTreeMap::from([(5, b"five".to_vec())]));
    }

    #[test]
    fn unknown_entry_names_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed.pak");
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for name in ["foo", "_x", "_2"] {
            zip.start_file(name, SimpleFileOptions::default()).unwrap();
            zip.write_all(b"data").unwrap();
        }
        std::fs::write(&path, zip.finish().unwrap().into_inner()).unwrap();
        let pak = read_pak(&path).unwrap();
        assert_eq!(pak.entries.keys().copied().collect::<Vec<_>>(), vec![2]);
    }

    #[test]
    fn parse_entry_name_accepts_only_underscore_u32() {
        assert_eq!(parse_entry_name("_12"), Some(12));
        assert_eq!(parse_entry_name("12"), None);
        assert_eq!(parse_entry_name("_"), None);
        assert_eq!(parse_entry_name("_-1"), None);
        assert_eq!(parse_entry_name("_4294967296"), None);
        assert_eq!(entry_name(12), "_12");
    }
}
