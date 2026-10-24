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
    assert!(matches!(parse(&bytes), Err(UnpackError::Cab(msg)) if msg.contains("not a cabinet")));
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
