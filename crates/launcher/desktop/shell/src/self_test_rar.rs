//! `--self-test-rar`: extracts a small stored RAR 4 archive through the bundled
//! UnRAR, the way a seed is unpacked. `stage_verify` runs it on the built
//! AppImage, because a Linux build linked with mold 1.0.3 shipped a UnRAR that
//! rejected every header (PP-U-FIX). The archive is built in code, so the
//! binary carries no game data and no fixture file.

use std::ffi::OsString;
use std::path::Path;

use cimmeria_launcher_engine::unpack::{self, UnpackSink};
use tokio_util::sync::CancellationToken;

const FLAG: &str = "--self-test-rar";
const ENTRY_NAME: &str = "SELF-TEST.TXT";
const ENTRY_DATA: &[u8] = b"self-test\r\n";
/// 2009-06-30 12:00:00, the DOS stamp the 2009 client's archives carry.
const DOS_DATE: u16 = ((2009 - 1980) << 9) | (6 << 5) | 30;
const DOS_TIME: u16 = 12 << 11;

/// Whether the process was started with the hidden flag (`argv[0]` skipped).
pub fn requested(args: impl Iterator<Item = OsString>) -> bool {
    args.skip(1).any(|arg| arg == FLAG)
}

/// Extracts the self-test archive into a scratch directory and checks the one
/// file it holds. The error text is the UnRAR error, for the shell's stderr.
pub fn run() -> Result<(), String> {
    let dir = std::env::temp_dir().join(format!("cimmeria-self-test-rar-{}", std::process::id()));
    let result = run_in(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn run_in(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| format!("scratch directory: {error}"))?;
    let archive = dir.join("self-test.rar");
    std::fs::write(&archive, stored_rar4(ENTRY_NAME, ENTRY_DATA))
        .map_err(|error| format!("archive write: {error}"))?;
    let dest = dir.join("out");
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let sink = UnpackSink {
        progress: tx.into(),
        label: "self-test".into(),
        cancel: CancellationToken::new(),
    };
    unpack::unpack(&archive, &dest, &sink).map_err(|error| error.to_string())?;
    let extracted =
        std::fs::read(dest.join(ENTRY_NAME)).map_err(|error| format!("{ENTRY_NAME}: {error}"))?;
    if extracted != ENTRY_DATA {
        return Err(format!("{ENTRY_NAME} came out with the wrong contents"));
    }
    Ok(())
}

/// The bitwise CRC-32 (IEEE) that RAR 4 headers and file data use.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// RAR 4 header CRC: the low 16 bits of CRC-32 over the header from `HEAD_TYPE`.
fn head_crc(header_after_crc: &[u8]) -> [u8; 2] {
    ((crc32(header_after_crc) & 0xffff) as u16).to_le_bytes()
}

/// One stored (method 0x30) RAR 4 file, laid out as the launcher's own unpack
/// tests lay theirs out: marker, main header, one file header and data, end
/// block. No archive comment and no lock flag.
fn stored_rar4(name: &str, data: &[u8]) -> Vec<u8> {
    let mut out = b"Rar!\x1a\x07\x00".to_vec();
    // Main archive header: type 0x73, no flags, size 13, 6 reserved bytes.
    let main = [0x73, 0x00, 0x00, 13, 0x00, 0, 0, 0, 0, 0, 0];
    out.extend_from_slice(&head_crc(&main));
    out.extend_from_slice(&main);

    let name = name.as_bytes();
    let size = u32::try_from(data.len()).expect("the self-test entry is small");
    let mut header = vec![0x74]; // HEAD_TYPE: file header
    header.extend_from_slice(&0x8000u16.to_le_bytes()); // LONG_BLOCK
    header.extend_from_slice(&((32 + name.len()) as u16).to_le_bytes()); // HEAD_SIZE
    header.extend_from_slice(&size.to_le_bytes()); // PACK_SIZE
    header.extend_from_slice(&size.to_le_bytes()); // UNP_SIZE
    header.push(2); // HOST_OS: Win32
    header.extend_from_slice(&crc32(data).to_le_bytes()); // FILE_CRC
    header.extend_from_slice(&((u32::from(DOS_DATE) << 16) | u32::from(DOS_TIME)).to_le_bytes());
    header.push(20); // UNP_VER 2.0
    header.push(0x30); // METHOD: store
    header.extend_from_slice(&(name.len() as u16).to_le_bytes()); // NAME_SIZE
    header.extend_from_slice(&0x20u32.to_le_bytes()); // ATTR: archive
    header.extend_from_slice(name);
    out.extend_from_slice(&head_crc(&header));
    out.extend_from_slice(&header);
    out.extend_from_slice(data);

    // End-of-archive block.
    let end = [0x7b, 0x00, 0x40, 7, 0x00];
    out.extend_from_slice(&head_crc(&end));
    out.extend_from_slice(&end);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_is_read_from_the_arguments_only() {
        let args = |list: &[&str]| list.iter().map(OsString::from).collect::<Vec<_>>();
        assert!(requested(args(&["launcher", FLAG]).into_iter()));
        assert!(!requested(args(&["launcher"]).into_iter()));
        // argv[0] is never the flag.
        assert!(!requested(args(&[FLAG]).into_iter()));
    }

    #[test]
    fn the_crc_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn the_self_test_archive_extracts_through_unrar() {
        run().expect("UnRAR extracts the self-test archive");
    }
}
