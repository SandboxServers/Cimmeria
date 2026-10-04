//! Test fixtures for the unpack pipeline.
//!
//! No RAR writer is available as a crate, and 7-Zip cannot create RAR, so
//! [`write_stored_rar4`] builds an uncompressed RAR 4.x archive by hand. That
//! is the same shape as the archive.org client RAR (method `m0`, store), so
//! the tests drive the real UnRAR library over a realistic input.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use super::UnpackSink;
use crate::install::Progress;

pub(crate) fn sink() -> (UnpackSink, tokio::sync::mpsc::UnboundedReceiver<Progress>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (
        UnpackSink {
            progress: tx.into(),
            label: "test".into(),
            cancel: CancellationToken::new(),
        },
        rx,
    )
}

/// 2009-06-30 12:00:00 as a DOS date: the date the 2009 client's
/// cabinets stamp on every file. Both archive fixtures carry it.
pub(crate) const FIXTURE_DOS_DATE: u16 = ((2009 - 1980) << 9) | (6 << 5) | 30;
/// 12:00:00 as a DOS time.
pub(crate) const FIXTURE_DOS_TIME: u16 = 12 << 11;

/// The modified time an extracted fixture file must end up with.
pub(crate) fn fixture_mtime() -> std::time::SystemTime {
    super::dos_time::to_system_time(FIXTURE_DOS_DATE, FIXTURE_DOS_TIME).expect("valid stamp")
}

/// Poorly compressible bytes (xorshift), so a small `MaxDiskSize` really
/// makes a cabinet set span several cabinets.
#[cfg(windows)]
pub(crate) fn incompressible(seed: u8, len: usize) -> Vec<u8> {
    let mut x = u32::from(seed) | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

/// RAR 4 header CRC: the low 16 bits of CRC-32 over the header from
/// `HEAD_TYPE` onwards.
fn head_crc(header_after_crc: &[u8]) -> [u8; 2] {
    ((crc32fast::hash(header_after_crc) & 0xffff) as u16).to_le_bytes()
}

/// Write a stored (method 0x30) RAR 4 archive holding `entries`, each a
/// `(name, contents)` pair. Names use `\` as the RAR 4 format does.
pub(crate) fn write_stored_rar4(path: &Path, entries: &[(&str, &[u8])]) {
    let mut out = Vec::new();
    // Marker block.
    out.extend_from_slice(b"Rar!\x1a\x07\x00");
    // Main archive header: type 0x73, no flags, size 13, 6 reserved bytes.
    let main = [0x73, 0x00, 0x00, 13, 0x00, 0, 0, 0, 0, 0, 0];
    out.extend_from_slice(&head_crc(&main));
    out.extend_from_slice(&main);

    // DOS date in the high half, time in the low half.
    let dos_time: u32 = (u32::from(FIXTURE_DOS_DATE) << 16) | u32::from(FIXTURE_DOS_TIME);
    for (name, data) in entries {
        let name = name.as_bytes();
        let size = u32::try_from(data.len()).expect("fixture entries are small");
        let mut h = Vec::new();
        h.push(0x74); // HEAD_TYPE: file header
        h.extend_from_slice(&0x8000u16.to_le_bytes()); // LONG_BLOCK
        h.extend_from_slice(&((32 + name.len()) as u16).to_le_bytes()); // HEAD_SIZE
        h.extend_from_slice(&size.to_le_bytes()); // PACK_SIZE
        h.extend_from_slice(&size.to_le_bytes()); // UNP_SIZE
        h.push(2); // HOST_OS: Win32
        h.extend_from_slice(&crc32fast::hash(data).to_le_bytes()); // FILE_CRC
        h.extend_from_slice(&dos_time.to_le_bytes()); // FTIME
        h.push(20); // UNP_VER 2.0
        h.push(0x30); // METHOD: store
        h.extend_from_slice(&(name.len() as u16).to_le_bytes()); // NAME_SIZE
        h.extend_from_slice(&0x20u32.to_le_bytes()); // ATTR: archive
        h.extend_from_slice(name);
        out.extend_from_slice(&head_crc(&h));
        out.extend_from_slice(&h);
        out.extend_from_slice(data);
    }

    // End-of-archive block.
    let end = [0x7b, 0x00, 0x40, 7, 0x00];
    out.extend_from_slice(&head_crc(&end));
    out.extend_from_slice(&end);
    std::fs::write(path, out).unwrap();
}

/// Build a MakeCAB cabinet set from `files` in `dir`, the way the 2009
/// installer's `DATA*.CAB` + `DATA.INF` were made. `max_cab_bytes` small
/// enough forces the set to span several cabinets with files continued
/// across the boundary. Returns the cabinet file names in order.
#[cfg(windows)]
pub(crate) fn make_cab_set(
    dir: &Path,
    files: &[(&str, Vec<u8>)],
    max_cab_bytes: u32,
) -> Vec<String> {
    let src = dir.join("src");
    let mut ddf = String::new();
    ddf.push_str(".OPTION EXPLICIT\r\n");
    ddf.push_str(".Set CabinetNameTemplate=DATA*.CAB\r\n");
    ddf.push_str(".Set DiskDirectoryTemplate=\r\n");
    ddf.push_str(".Set InfFileName=DATA.INF\r\n");
    ddf.push_str(".Set RptFileName=DATA.RPT\r\n");
    // The INF layout directives, copied from the real client's DATA.DDF.
    ddf.push_str(".Set DiskLabelTemplate=Disc_*\r\n");
    ddf.push_str(".Set InfHeader=\r\n");
    ddf.push_str(".Set InfFooter=\r\n");
    ddf.push_str(".Set InfDiskHeader=\"[disk list]\"\r\n");
    ddf.push_str(".Set InfDiskLineFormat=\"Disk *disk#*, *label*\"\r\n");
    ddf.push_str(".Set InfCabinetHeader=\"[cabinet list]\"\r\n");
    ddf.push_str(".Set InfCabinetLineFormat=\"Disk *disk#*, Cabinet *cab#*, *cabfile*\"\r\n");
    ddf.push_str(".Set InfFileHeader=\"[file list]\"\r\n");
    ddf.push_str(".Set InfFileLineFormat=\"*file#*: Cabinet *cab#*, *file*, *size*\"\r\n");
    ddf.push_str(".Set Cabinet=on\r\n");
    ddf.push_str(".Set Compress=on\r\n");
    ddf.push_str(".Set CompressionType=MSZIP\r\n");
    // MakeCAB insists on a multiple of its 512-byte cluster size.
    let max_cab_bytes = max_cab_bytes.div_ceil(512) * 512;
    ddf.push_str(&format!(".Set MaxDiskSize={max_cab_bytes}\r\n"));
    ddf.push_str(".Set FolderSizeThreshold=1\r\n");
    for (name, data) in files {
        let p = src.join(name.replace('\\', std::path::MAIN_SEPARATOR_STR));
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, data).unwrap();
        // MakeCAB stamps each file with its source mtime (as local time),
        // so this gives every cabinet entry the fixture's DOS date/time.
        std::fs::File::options()
            .write(true)
            .open(&p)
            .unwrap()
            .set_modified(fixture_mtime())
            .unwrap();
        ddf.push_str(&format!("\"{}\" \"{}\"\r\n", p.display(), name));
    }
    let ddf_path = dir.join("DATA.DDF");
    std::fs::write(&ddf_path, ddf).unwrap();
    let status = std::process::Command::new("makecab.exe")
        .arg("/F")
        .arg(&ddf_path)
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .status()
        .expect("makecab.exe ships with Windows");
    assert!(status.success(), "makecab failed");
    std::fs::remove_dir_all(&src).unwrap();
    let mut cabs: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_ascii_uppercase().ends_with(".CAB"))
        .collect();
    cabs.sort();
    cabs
}
