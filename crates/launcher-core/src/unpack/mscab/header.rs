//! MS-CAB header parsing: the cabinet header, its folder and file tables, and
//! the `CFDATA` blocks that hold the compressed bytes.
//!
//! This reads what a MakeCAB cabinet says about itself and nothing more. It
//! does not decompress anything, and it does not check that folders and files
//! agree across a spanning set (that is the plan's job). Every error names the
//! cabinet file, never a full path, and nothing here panics on input bytes.

use std::io::{Read, Seek, SeekFrom};

use super::mszip::BLOCK_MAX;
use crate::unpack::UnpackError;

/// `typeCompress & 0x000F` for a folder stored without compression.
pub(in crate::unpack) const TYPE_NONE: u16 = 0;
/// `typeCompress & 0x000F` for a folder of MSZIP blocks.
pub(in crate::unpack) const TYPE_MSZIP: u16 = 1;
/// `iFolder`: the file continues from the previous cabinet's last folder.
pub(in crate::unpack) const CONTINUED_FROM_PREV: u16 = 0xFFFD;
/// `iFolder`: the file starts here and continues in the next cabinet.
pub(in crate::unpack) const CONTINUED_TO_NEXT: u16 = 0xFFFE;
/// `iFolder`: the file continues from the previous cabinet and into the next.
pub(in crate::unpack) const PREV_AND_NEXT: u16 = 0xFFFF;

/// `flags`: the cabinet has a previous cabinet.
const FLAG_PREV: u16 = 0x1;
/// `flags`: the cabinet has a next cabinet.
const FLAG_NEXT: u16 = 0x2;
/// `flags`: the header carries reserved-area sizes.
const FLAG_RESERVE: u16 = 0x4;
/// `attribs`: the file name is UTF-8 rather than Latin-1.
const ATTR_UTF8: u16 = 0x80;

/// Longest NUL-terminated string read from a header, without the NUL.
const MAX_STRING: usize = 256;
/// Largest `cbData` a block may claim: a full uncompressed block plus the
/// worst-case MSZIP expansion of deflate's stored blocks.
const MAX_DATA_BLOCK: usize = BLOCK_MAX + 6144;

/// One cabinet of a set, as its header describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::unpack) struct Cabinet {
    pub set_id: u16,
    pub index: u16,
    pub has_prev: bool,
    pub has_next: bool,
    /// `cbCFData`: bytes of reserved area at the start of each data block.
    pub reserve_data: u8,
    pub folders: Vec<Folder>,
    pub files: Vec<FileEntry>,
}

/// One `CFFOLDER` record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::unpack) struct Folder {
    /// `coffCabStart`: where this folder's first data block is.
    pub data_offset: u32,
    /// `cCFData`: how many data blocks the folder has in this cabinet.
    pub block_count: u16,
    /// `typeCompress & 0x000F`.
    pub compression: u16,
}

/// One `CFFILE` record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::unpack) struct FileEntry {
    pub size: u32,
    /// `uoffFolderStart`: where the file starts in its folder.
    pub folder_offset: u32,
    /// `iFolder`: a folder index, or one of the continuation markers.
    pub folder: u16,
    pub date: u16,
    pub time: u16,
    pub attribs: u16,
    pub name: String,
}

/// One `CFDATA` record: its stored bytes and its uncompressed size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::unpack) struct DataBlock {
    pub bytes: Vec<u8>,
    /// `cbUncomp`. Zero marks a split block's first part.
    pub uncompressed_len: u16,
}

/// Read a cabinet's header, folder table and file table.
///
/// The reader should be buffered: the file table is read a few bytes at a time.
pub(in crate::unpack) fn read_cabinet<R: Read + Seek>(
    r: &mut R,
    name: &str,
) -> Result<Cabinet, UnpackError> {
    let signature: [u8; 4] = read_array(r, name)?;
    if &signature != b"MSCF" {
        return Err(cab_error(name, "not a cabinet"));
    }
    // reserved1, cbCabinet, reserved2: not needed.
    read_u32(r, name)?;
    read_u32(r, name)?;
    read_u32(r, name)?;
    let files_offset = read_u32(r, name)?;
    // reserved3, versionMinor, versionMajor: not needed.
    read_u32(r, name)?;
    read_u8(r, name)?;
    read_u8(r, name)?;
    let folder_count = read_u16(r, name)?;
    let file_count = read_u16(r, name)?;
    let flags = read_u16(r, name)?;
    let set_id = read_u16(r, name)?;
    let index = read_u16(r, name)?;

    let mut reserve_data = 0;
    let mut folder_reserve = 0;
    if flags & FLAG_RESERVE != 0 {
        let header_reserve = read_u16(r, name)?;
        folder_reserve = read_u8(r, name)?;
        reserve_data = read_u8(r, name)?;
        skip(r, header_reserve)?;
    }
    if flags & FLAG_PREV != 0 {
        // szCabinetPrev and szDiskPrev: the names are not needed.
        read_cstring(r, name)?;
        read_cstring(r, name)?;
    }
    if flags & FLAG_NEXT != 0 {
        // szCabinetNext and szDiskNext: the names are not needed.
        read_cstring(r, name)?;
        read_cstring(r, name)?;
    }

    let mut folders = Vec::new();
    for _ in 0..folder_count {
        let data_offset = read_u32(r, name)?;
        let block_count = read_u16(r, name)?;
        let type_compress = read_u16(r, name)?;
        skip(r, u16::from(folder_reserve))?;
        folders.push(Folder {
            data_offset,
            block_count,
            compression: type_compress & 0x000F,
        });
    }

    r.seek(SeekFrom::Start(u64::from(files_offset)))?;
    let mut files = Vec::new();
    for _ in 0..file_count {
        let size = read_u32(r, name)?;
        let folder_offset = read_u32(r, name)?;
        let folder = read_u16(r, name)?;
        let date = read_u16(r, name)?;
        let time = read_u16(r, name)?;
        let attribs = read_u16(r, name)?;
        let raw_name = read_cstring(r, name)?;
        files.push(FileEntry {
            size,
            folder_offset,
            folder,
            date,
            time,
            attribs,
            name: decode_name(raw_name, attribs, name)?,
        });
    }

    Ok(Cabinet {
        set_id,
        index,
        has_prev: flags & FLAG_PREV != 0,
        has_next: flags & FLAG_NEXT != 0,
        reserve_data,
        folders,
        files,
    })
}

/// Read one `CFDATA` record. The checksum is read and ignored (D-LI5).
pub(in crate::unpack) fn read_data_block<R: Read>(
    r: &mut R,
    reserve_data: u8,
    name: &str,
) -> Result<DataBlock, UnpackError> {
    let _checksum: [u8; 4] = read_array_or(r, name, "data block")?;
    let data_len = u16::from_le_bytes(read_array_or(r, name, "data block")?);
    let uncompressed_len = u16::from_le_bytes(read_array_or(r, name, "data block")?);
    if usize::from(data_len) > MAX_DATA_BLOCK {
        return Err(cab_error(name, "a data block is too large"));
    }
    if usize::from(uncompressed_len) > BLOCK_MAX {
        return Err(cab_error(
            name,
            "a data block inflates to more than 32768 bytes",
        ));
    }
    let mut reserve = vec![0u8; usize::from(reserve_data)];
    read_exact_or(r, &mut reserve, name, "data block")?;
    let mut bytes = vec![0u8; usize::from(data_len)];
    read_exact_or(r, &mut bytes, name, "data block")?;
    Ok(DataBlock {
        bytes,
        uncompressed_len,
    })
}

fn cab_error(name: &str, message: &str) -> UnpackError {
    UnpackError::Cab(format!("{name}: {message}"))
}

/// Fill `buf` from `r`. A short read is a truncated `what`, not an I/O error.
fn read_exact_or<R: Read>(
    r: &mut R,
    buf: &mut [u8],
    name: &str,
    what: &str,
) -> Result<(), UnpackError> {
    r.read_exact(buf).map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            cab_error(name, &format!("truncated {what}"))
        } else {
            UnpackError::Io(e)
        }
    })
}

fn read_array_or<const N: usize, R: Read>(
    r: &mut R,
    name: &str,
    what: &str,
) -> Result<[u8; N], UnpackError> {
    let mut buf = [0u8; N];
    read_exact_or(r, &mut buf, name, what)?;
    Ok(buf)
}

fn read_array<const N: usize, R: Read>(r: &mut R, name: &str) -> Result<[u8; N], UnpackError> {
    read_array_or(r, name, "header")
}

fn read_u8<R: Read>(r: &mut R, name: &str) -> Result<u8, UnpackError> {
    Ok(read_array::<1, R>(r, name)?[0])
}

fn read_u16<R: Read>(r: &mut R, name: &str) -> Result<u16, UnpackError> {
    Ok(u16::from_le_bytes(read_array(r, name)?))
}

fn read_u32<R: Read>(r: &mut R, name: &str) -> Result<u32, UnpackError> {
    Ok(u32::from_le_bytes(read_array(r, name)?))
}

/// Skip `count` bytes of a reserved area. A skip past the end shows up as a
/// truncated read in the next field.
fn skip<R: Seek>(r: &mut R, count: u16) -> Result<(), UnpackError> {
    r.seek(SeekFrom::Current(i64::from(count)))?;
    Ok(())
}

/// Read a NUL-terminated string of at most [`MAX_STRING`] bytes, without the NUL.
fn read_cstring<R: Read>(r: &mut R, name: &str) -> Result<Vec<u8>, UnpackError> {
    let mut out = Vec::new();
    loop {
        let byte = read_u8(r, name)?;
        if byte == 0 {
            return Ok(out);
        }
        if out.len() == MAX_STRING {
            return Err(cab_error(name, "a string in the header is too long"));
        }
        out.push(byte);
    }
}

/// A file name is UTF-8 when its attribute says so, and Latin-1 otherwise.
fn decode_name(raw: Vec<u8>, attribs: u16, name: &str) -> Result<String, UnpackError> {
    if attribs & ATTR_UTF8 != 0 {
        String::from_utf8(raw).map_err(|_| cab_error(name, "a file name is not UTF-8"))
    } else {
        Ok(raw.into_iter().map(char::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    /// A file to put in a hand-built cabinet: size, folder offset, `iFolder`,
    /// attribs and the name bytes.
    struct FileSpec<'a> {
        size: u32,
        folder_offset: u32,
        folder: u16,
        attribs: u16,
        name: &'a [u8],
    }

    /// A hand-built cabinet, laid out the way MakeCAB writes one.
    struct Spec<'a> {
        flags: u16,
        /// `(cbCFHeader, cbCFFolder, cbCFData)` when RESERVE is set.
        reserve: Option<(u16, u8, u8)>,
        /// The PREV and NEXT strings, already NUL-terminated.
        strings: &'a [u8],
        /// `(coffCabStart, cCFData, typeCompress)` per folder.
        folders: &'a [(u32, u16, u16)],
        files: &'a [FileSpec<'a>],
    }

    fn build(spec: &Spec) -> Vec<u8> {
        let mut out = b"MSCF".to_vec();
        out.extend_from_slice(&[0; 12]); // reserved1, cbCabinet, reserved2
        out.extend_from_slice(&[0; 4]); // coffFiles, patched below
        out.extend_from_slice(&[0; 4]); // reserved3
        out.extend_from_slice(&[3, 1]); // versionMinor, versionMajor
        out.extend_from_slice(&(spec.folders.len() as u16).to_le_bytes());
        out.extend_from_slice(&(spec.files.len() as u16).to_le_bytes());
        out.extend_from_slice(&spec.flags.to_le_bytes());
        out.extend_from_slice(&7u16.to_le_bytes()); // setID
        out.extend_from_slice(&2u16.to_le_bytes()); // iCabinet
        let folder_reserve = match spec.reserve {
            Some((header, folder, data)) => {
                out.extend_from_slice(&header.to_le_bytes());
                out.push(folder);
                out.push(data);
                out.extend(std::iter::repeat_n(0xAA, usize::from(header)));
                folder
            }
            None => 0,
        };
        out.extend_from_slice(spec.strings);
        for &(offset, count, compress) in spec.folders {
            out.extend_from_slice(&offset.to_le_bytes());
            out.extend_from_slice(&count.to_le_bytes());
            out.extend_from_slice(&compress.to_le_bytes());
            out.extend(std::iter::repeat_n(0xEE, usize::from(folder_reserve)));
        }
        let files_at = out.len() as u32;
        out[16..20].copy_from_slice(&files_at.to_le_bytes());
        for file in spec.files {
            out.extend_from_slice(&file.size.to_le_bytes());
            out.extend_from_slice(&file.folder_offset.to_le_bytes());
            out.extend_from_slice(&file.folder.to_le_bytes());
            out.extend_from_slice(&0x5A21u16.to_le_bytes()); // date
            out.extend_from_slice(&0x6000u16.to_le_bytes()); // time
            out.extend_from_slice(&file.attribs.to_le_bytes());
            out.extend_from_slice(file.name);
            out.push(0);
        }
        out
    }

    fn parse(bytes: &[u8]) -> Result<Cabinet, UnpackError> {
        read_cabinet(&mut Cursor::new(bytes), "DATA1.CAB")
    }

    fn plain_spec<'a>(files: &'a [FileSpec<'a>], folders: &'a [(u32, u16, u16)]) -> Spec<'a> {
        Spec {
            flags: 0,
            reserve: None,
            strings: &[],
            folders,
            files,
        }
    }

    #[test]
    fn one_folder_two_files_parses_to_the_exact_struct() {
        let files = [
            FileSpec {
                size: 10,
                folder_offset: 0,
                folder: 0,
                attribs: 0,
                name: b"a.txt",
            },
            FileSpec {
                size: 20,
                folder_offset: 10,
                folder: 0,
                attribs: 0,
                name: b"dir\\b.txt",
            },
        ];
        let folders = [(0x40, 1, TYPE_MSZIP)];
        let cab = parse(&build(&plain_spec(&files, &folders))).unwrap();
        assert_eq!(
            cab,
            Cabinet {
                set_id: 7,
                index: 2,
                has_prev: false,
                has_next: false,
                reserve_data: 0,
                folders: vec![Folder {
                    data_offset: 0x40,
                    block_count: 1,
                    compression: TYPE_MSZIP
                }],
                files: vec![
                    FileEntry {
                        size: 10,
                        folder_offset: 0,
                        folder: 0,
                        date: 0x5A21,
                        time: 0x6000,
                        attribs: 0,
                        name: "a.txt".into()
                    },
                    FileEntry {
                        size: 20,
                        folder_offset: 10,
                        folder: 0,
                        date: 0x5A21,
                        time: 0x6000,
                        attribs: 0,
                        name: "dir\\b.txt".into()
                    },
                ],
            }
        );
    }

    #[test]
    fn reserve_area_is_skipped_and_its_data_size_is_kept() {
        let files = [FileSpec {
            size: 1,
            folder_offset: 0,
            folder: 0,
            attribs: 0,
            name: b"x",
        }];
        let folders = [(0x40, 1, TYPE_NONE)];
        let spec = Spec {
            flags: FLAG_RESERVE,
            reserve: Some((4, 2, 1)),
            ..plain_spec(&files, &folders)
        };
        let cab = parse(&build(&spec)).unwrap();
        assert_eq!(cab.reserve_data, 1);
        assert_eq!(cab.folders[0].data_offset, 0x40);
        assert_eq!(cab.files[0].name, "x");
    }

    #[test]
    fn prev_and_next_strings_are_skipped_and_flags_are_set() {
        let files = [FileSpec {
            size: 1,
            folder_offset: 0,
            folder: 0,
            attribs: 0,
            name: b"x",
        }];
        let folders = [(0x40, 1, TYPE_MSZIP)];
        let spec = Spec {
            flags: FLAG_PREV | FLAG_NEXT,
            strings: b"DATA0.CAB\0DISK1\0DATA2.CAB\0DISK2\0",
            ..plain_spec(&files, &folders)
        };
        let cab = parse(&build(&spec)).unwrap();
        assert!(cab.has_prev);
        assert!(cab.has_next);
        assert_eq!(cab.files[0].name, "x");
    }

    #[test]
    fn utf8_and_latin1_names_both_decode_to_u_umlaut() {
        let files = [
            FileSpec {
                size: 1,
                folder_offset: 0,
                folder: 0,
                attribs: 0x80,
                name: "Ü.txt".as_bytes(),
            },
            FileSpec {
                size: 1,
                folder_offset: 1,
                folder: 0,
                attribs: 0,
                name: &[0xDC],
            },
        ];
        let folders = [(0x40, 1, TYPE_MSZIP)];
        let cab = parse(&build(&plain_spec(&files, &folders))).unwrap();
        assert_eq!(cab.files[0].name, "Ü.txt");
        assert_eq!(cab.files[1].name, "Ü");
    }

    #[test]
    fn bad_signature_is_a_cab_error() {
        let files = [FileSpec {
            size: 1,
            folder_offset: 0,
            folder: 0,
            attribs: 0,
            name: b"x",
        }];
        let folders = [(0x40, 1, TYPE_MSZIP)];
        let mut bytes = build(&plain_spec(&files, &folders));
        bytes[..4].copy_from_slice(b"XSCF");
        assert!(
            matches!(parse(&bytes), Err(UnpackError::Cab(msg)) if msg.contains("not a cabinet"))
        );
    }

    #[test]
    fn every_truncation_is_an_error_and_never_a_panic() {
        let files = [
            FileSpec {
                size: 1,
                folder_offset: 0,
                folder: 0,
                attribs: 0,
                name: b"a",
            },
            FileSpec {
                size: 2,
                folder_offset: 1,
                folder: 0,
                attribs: 0x80,
                name: "Ü".as_bytes(),
            },
        ];
        let folders = [(0x40, 1, TYPE_MSZIP)];
        let bytes = build(&Spec {
            flags: FLAG_RESERVE | FLAG_NEXT,
            reserve: Some((2, 1, 0)),
            strings: b"DATA2.CAB\0DISK2\0",
            ..plain_spec(&files, &folders)
        });
        for n in 0..bytes.len() {
            assert!(parse(&bytes[..n]).is_err(), "a cut at {n} parsed");
        }
    }

    #[test]
    fn data_block_reads_its_reserve_and_bytes() {
        let mut bytes = vec![0, 0, 0, 0, 3, 0, 5, 0, 0xEE];
        bytes.extend_from_slice(b"CKab");
        let block = read_data_block(&mut Cursor::new(&bytes), 1, "DATA1.CAB").unwrap();
        assert_eq!(
            block,
            DataBlock {
                bytes: b"CKa".to_vec(),
                uncompressed_len: 5
            }
        );
    }

    #[test]
    fn data_block_refuses_an_uncompressed_size_over_32k() {
        let mut bytes = vec![0, 0, 0, 0, 4, 0];
        bytes.extend_from_slice(&40000u16.to_le_bytes());
        bytes.extend_from_slice(b"CKab");
        assert!(read_data_block(&mut Cursor::new(&bytes), 0, "DATA1.CAB").is_err());
    }
}
