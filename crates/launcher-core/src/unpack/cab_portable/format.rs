//! The Microsoft cabinet format, as far as expanding a MakeCAB set needs it.
//!
//! Layout (all integers little-endian), from Microsoft's cabinet format
//! specification: a 36-byte `CFHEADER`, optional reserve fields, optional
//! previous/next cabinet names, `cFolders` `CFFOLDER` records, then
//! `cFiles` `CFFILE` records at `coffFiles`. Each folder's compressed data
//! is `cCFData` `CFDATA` blocks starting at its `coffCabStart`.

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

use super::super::UnpackError;

pub(super) const SIGNATURE: &[u8; 4] = b"MSCF";
const FLAG_PREV_CABINET: u16 = 0x0001;
const FLAG_NEXT_CABINET: u16 = 0x0002;
const FLAG_RESERVE_PRESENT: u16 = 0x0004;

/// `iFolder` values for a file that continues across a cabinet boundary.
pub(super) const CONTINUED_FROM_PREV: u16 = 0xFFFD;
pub(super) const CONTINUED_TO_NEXT: u16 = 0xFFFE;
pub(super) const CONTINUED_PREV_AND_NEXT: u16 = 0xFFFF;

/// `attribs` bit: the name is UTF-8 rather than the ANSI code page.
const A_NAME_IS_UTF: u16 = 0x80;

/// Low nibble of `typeCompress`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Compression {
    None,
    Mszip,
    Quantum,
    Lzx,
    Unknown(u16),
}

impl Compression {
    fn from_type(type_compress: u16) -> Self {
        match type_compress & 0x000F {
            0 => Self::None,
            1 => Self::Mszip,
            2 => Self::Quantum,
            3 => Self::Lzx,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct Folder {
    pub data_offset: u32,
    pub blocks: u16,
    pub compression: Compression,
    /// `typeCompress` as written; the LZX window size lives in its high bits.
    pub raw_type: u16,
}

#[derive(Debug, Clone)]
pub(super) struct FileEntry {
    pub name: String,
    pub size: u32,
    pub folder_offset: u32,
    pub folder: u16,
    pub date: u16,
    pub time: u16,
}

impl FileEntry {
    /// The entry continues a file an earlier cabinet already listed.
    pub fn is_continuation(&self) -> bool {
        matches!(self.folder, CONTINUED_FROM_PREV | CONTINUED_PREV_AND_NEXT)
    }

    /// The entry's data runs on into the next cabinet.
    pub fn continues_to_next(&self) -> bool {
        matches!(self.folder, CONTINUED_TO_NEXT | CONTINUED_PREV_AND_NEXT)
    }
}

#[derive(Debug, Clone)]
pub(super) struct Cabinet {
    pub has_prev: bool,
    pub has_next: bool,
    pub set_id: u16,
    pub index: u16,
    pub data_reserve: u8,
    pub folders: Vec<Folder>,
    pub files: Vec<FileEntry>,
}

fn corrupt(what: impl std::fmt::Display) -> UnpackError {
    UnpackError::Cab(format!("corrupt cabinet: {what}"))
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn read_exact<R: Read>(r: &mut R, buf: &mut [u8], what: &str) -> Result<(), UnpackError> {
    r.read_exact(buf).map_err(|error| match error.kind() {
        std::io::ErrorKind::UnexpectedEof => corrupt(format!("it ends inside the {what}")),
        _ => UnpackError::Io(error),
    })
}

/// A NUL-terminated string of at most 256 bytes, as the format bounds them.
fn read_cstr<R: BufRead>(r: &mut R, what: &str) -> Result<Vec<u8>, UnpackError> {
    let mut out = Vec::new();
    r.take(257).read_until(0, &mut out)?;
    if out.pop() != Some(0) {
        return Err(corrupt(format!("unterminated {what}")));
    }
    Ok(out)
}

/// Read the header, folder table and file table of one cabinet.
pub(super) fn parse<R: Read + Seek>(source: R) -> Result<Cabinet, UnpackError> {
    let mut r = BufReader::new(source);
    r.seek(SeekFrom::Start(0))?;
    let mut h = [0u8; 36];
    read_exact(&mut r, &mut h, "header")?;
    if &h[..4] != SIGNATURE {
        return Err(UnpackError::Cab("not a cabinet (no MSCF signature)".into()));
    }
    let (minor, major) = (h[24], h[25]);
    if major != 1 {
        return Err(UnpackError::Cab(format!(
            "unsupported cabinet version {major}.{minor}"
        )));
    }
    let files_offset = u32_at(&h, 16);
    let folder_count = u16_at(&h, 26);
    let file_count = u16_at(&h, 28);
    let flags = u16_at(&h, 30);

    let (mut folder_reserve, mut data_reserve) = (0u8, 0u8);
    if flags & FLAG_RESERVE_PRESENT != 0 {
        let mut res = [0u8; 4];
        read_exact(&mut r, &mut res, "reserve sizes")?;
        let header_reserve = u16_at(&res, 0);
        folder_reserve = res[2];
        data_reserve = res[3];
        r.seek_relative(i64::from(header_reserve))?;
    }
    if flags & FLAG_PREV_CABINET != 0 {
        read_cstr(&mut r, "previous cabinet name")?;
        read_cstr(&mut r, "previous disk name")?;
    }
    if flags & FLAG_NEXT_CABINET != 0 {
        read_cstr(&mut r, "next cabinet name")?;
        read_cstr(&mut r, "next disk name")?;
    }

    let mut folders = Vec::with_capacity(usize::from(folder_count));
    for _ in 0..folder_count {
        let mut f = [0u8; 8];
        read_exact(&mut r, &mut f, "folder table")?;
        r.seek_relative(i64::from(folder_reserve))?;
        let raw_type = u16_at(&f, 6);
        folders.push(Folder {
            data_offset: u32_at(&f, 0),
            blocks: u16_at(&f, 4),
            compression: Compression::from_type(raw_type),
            raw_type,
        });
    }

    r.seek(SeekFrom::Start(u64::from(files_offset)))?;
    let mut files = Vec::with_capacity(usize::from(file_count));
    for _ in 0..file_count {
        let mut f = [0u8; 16];
        read_exact(&mut r, &mut f, "file table")?;
        let attribs = u16_at(&f, 14);
        let raw = read_cstr(&mut r, "file name")?;
        let name = if attribs & A_NAME_IS_UTF != 0 {
            String::from_utf8(raw).map_err(|_| corrupt("an entry name is not valid UTF-8"))?
        } else {
            // ANSI; the client's names are ASCII, and Latin-1 is the closest
            // total mapping for anything else (as the FDI path does).
            raw.iter().map(|&b| char::from(b)).collect()
        };
        let folder = u16_at(&f, 8);
        if folder < CONTINUED_FROM_PREV && usize::from(folder) >= folders.len() {
            return Err(corrupt(format!("{name:?} names folder {folder}")));
        }
        files.push(FileEntry {
            name,
            size: u32_at(&f, 0),
            folder_offset: u32_at(&f, 4),
            folder,
            date: u16_at(&f, 10),
            time: u16_at(&f, 12),
        });
    }

    Ok(Cabinet {
        has_prev: flags & FLAG_PREV_CABINET != 0,
        has_next: flags & FLAG_NEXT_CABINET != 0,
        set_id: u16_at(&h, 32),
        index: u16_at(&h, 34),
        data_reserve,
        folders,
        files,
    })
}

/// One `CFDATA` block header, read in sequence from a folder's data.
#[derive(Debug, Clone, Copy)]
pub(super) struct BlockHeader {
    pub compressed: u16,
    pub uncompressed: u16,
}

pub(super) fn read_block_header<R: Read + Seek>(
    r: &mut R,
    data_reserve: u8,
) -> Result<BlockHeader, UnpackError> {
    let mut b = [0u8; 8];
    read_exact(r, &mut b, "data block header")?;
    r.seek_relative(i64::from(data_reserve))?;
    Ok(BlockHeader {
        compressed: u16_at(&b, 4),
        uncompressed: u16_at(&b, 6),
    })
}

pub(super) fn read_block_data<R: Read>(
    r: &mut R,
    header: BlockHeader,
    out: &mut Vec<u8>,
) -> Result<(), UnpackError> {
    let start = out.len();
    out.resize(start + usize::from(header.compressed), 0);
    read_exact(r, &mut out[start..], "data block")
}
