//! Read one named member out of a single MakeCAB cabinet.
//!
//! The install path expands a whole set and refuses LZX. The native DirectX
//! prerequisites need one file from each of two LZX cabinets, so this decodes
//! only the folder that holds the member, and only as far as the member's end.
//! A folder that spans into another cabinet cannot be finished from one
//! cabinet, so it is refused.

use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::Path;

use lzxd::{Lzxd, WindowSize};

use super::super::UnpackError;
use super::format::{self, Compression, Folder};
use super::mszip::Mszip;

/// The uncompressed bytes of `member` in the single cabinet `cab`.
///
/// The name match ignores ASCII case. A member larger than `max_bytes` is
/// refused before anything is decoded.
pub(in crate::unpack) fn extract_member(
    cab: &Path,
    member: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, UnpackError> {
    let file = File::open(cab)?;
    let cabinet = format::parse(&file)?;

    let matches: Vec<&format::FileEntry> = cabinet
        .files
        .iter()
        .filter(|f| !f.is_continuation() && f.name.eq_ignore_ascii_case(member))
        .collect();
    let entry = match matches.as_slice() {
        [entry] => *entry,
        _ => {
            return Err(UnpackError::Cab(format!(
                "{} matches for {member:?}, not one",
                matches.len()
            )))
        }
    };
    if u64::from(entry.size) > max_bytes {
        return Err(UnpackError::Cab(format!(
            "{member:?} is {} bytes, over the {max_bytes} allowed",
            entry.size
        )));
    }

    // A folder that continues to or from another cabinet is not whole here.
    let slot = if entry.continues_to_next() {
        cabinet.folders.len().wrapping_sub(1)
    } else {
        usize::from(entry.folder)
    };
    let folder = cabinet
        .folders
        .get(slot)
        .ok_or_else(|| UnpackError::Cab(format!("{member:?} names no folder")))?;
    let continued_in = cabinet.files.iter().any(|f| f.is_continuation());
    let spans_out = cabinet.files.iter().any(|f| f.continues_to_next());
    if (slot == 0 && continued_in) || (slot + 1 == cabinet.folders.len() && spans_out) {
        return Err(UnpackError::Cab(format!(
            "{member:?} is in a folder that spans cabinets"
        )));
    }

    decode_member(&file, &cabinet, folder, entry)
}

/// Decode `folder`'s blocks in order until the member's last byte is out.
fn decode_member(
    file: &File,
    cabinet: &format::Cabinet,
    folder: &Folder,
    entry: &format::FileEntry,
) -> Result<Vec<u8>, UnpackError> {
    let start = u64::from(entry.folder_offset);
    let end = start + u64::from(entry.size);
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(u64::from(folder.data_offset)))?;

    let mut decoder = Decoder::for_folder(folder)?;
    // Only the member's own bytes are kept; the folder's earlier members are decoded and dropped.
    let mut out: Vec<u8> = Vec::new();
    let mut produced: u64 = 0;
    let mut block = Vec::new();
    for _ in 0..folder.blocks {
        if produced >= end {
            break;
        }
        let header = format::read_block_header(&mut reader, cabinet.data_reserve)?;
        format::read_block_data(&mut reader, header, &mut block)?;
        let size = usize::from(header.uncompressed);
        if size == 0 {
            return Err(UnpackError::Cab(
                "an empty data block sits inside a folder".into(),
            ));
        }
        let bytes = match &mut decoder {
            Decoder::Stored if block.len() == size => &block[..],
            Decoder::Stored => {
                return Err(UnpackError::Cab(
                    "a stored data block's sizes disagree".into(),
                ))
            }
            Decoder::Mszip(mszip) => mszip.block(&block, size).map_err(UnpackError::Cab)?,
            Decoder::Lzx(lzx) => lzx
                .decompress_next(&block, size)
                .map_err(|error| UnpackError::Cab(format!("LZX data does not decode: {error}")))?,
        };
        let len = bytes.len() as u64;
        let lo = start.saturating_sub(produced).min(len) as usize;
        let hi = end.saturating_sub(produced).min(len) as usize;
        out.extend_from_slice(&bytes[lo..hi]);
        produced += len;
        block.clear();
    }

    if produced < end {
        return Err(UnpackError::Cab(
            "the folder ends before the member does".into(),
        ));
    }
    Ok(out)
}

/// The decoder one folder's blocks need, chosen by its `typeCompress`.
enum Decoder {
    Stored,
    Mszip(Mszip),
    /// One LZX stream per folder, so the window carries across blocks.
    Lzx(Box<Lzxd>),
}

impl Decoder {
    fn for_folder(folder: &Folder) -> Result<Self, UnpackError> {
        match folder.compression {
            Compression::None => Ok(Self::Stored),
            Compression::Mszip => Ok(Self::Mszip(Mszip::new())),
            Compression::Lzx => Ok(Self::Lzx(Box::new(Lzxd::new(lzx_window(folder.raw_type)?)))),
            Compression::Quantum | Compression::Unknown(_) => Err(UnpackError::Cab(format!(
                "a folder uses {:?} compression, which is not read",
                folder.compression
            ))),
        }
    }
}

/// The LZX window a folder's `typeCompress` names: bits 8..=12 hold 15..=21
/// for 32 KiB to 2 MiB.
fn lzx_window(raw_type: u16) -> Result<WindowSize, UnpackError> {
    Ok(match (raw_type >> 8) & 0x1F {
        15 => WindowSize::KB32,
        16 => WindowSize::KB64,
        17 => WindowSize::KB128,
        18 => WindowSize::KB256,
        19 => WindowSize::KB512,
        20 => WindowSize::MB1,
        21 => WindowSize::MB2,
        other => {
            return Err(UnpackError::Cab(format!(
                "an LZX folder names window size {other}"
            )))
        }
    })
}

#[cfg(test)]
#[path = "single_tests.rs"]
mod tests;
