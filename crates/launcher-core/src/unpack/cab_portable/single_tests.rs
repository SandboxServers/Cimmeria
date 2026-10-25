use std::path::PathBuf;

use sha2::{Digest, Sha256};

use super::super::fixture::{write_cab_set, CabOptions};
use super::*;
use crate::unpack::extract_cab_member;
use crate::unpack::test_fixtures::incompressible;

/// Text-like bytes with long repeats, so MSZIP matches reach into earlier blocks.
fn repetitive(seed: u8, len: usize) -> Vec<u8> {
    let phrase = format!("[Engine.Core{seed}] Value=0x{seed:02X} ; the same line again\r\n");
    phrase.bytes().cycle().take(len).collect()
}

/// One cabinet holding every file, however many blocks that takes.
fn one_cabinet() -> CabOptions {
    CabOptions {
        blocks_per_cab: 1000,
        ..CabOptions::default()
    }
}

/// The single cabinet `write_cab_set` wrote into `dir`.
fn only_cabinet(dir: &Path, cabinets: &[String]) -> PathBuf {
    assert_eq!(cabinets.len(), 1, "{cabinets:?}");
    dir.join(&cabinets[0])
}

#[test]
fn the_second_of_three_mszip_files_comes_out_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![
        ("Working\\a.upk", repetitive(1, 40_000)),
        ("Working\\b.upk", repetitive(2, 70_000)),
        ("Common\\c.def", b"<root/>".to_vec()),
    ];
    let info = write_cab_set(dir.path(), &files, one_cabinet());
    let cab = only_cabinet(dir.path(), &info.cabinets);
    assert!(info.cross_block_matches > 0, "fixture must reach back");

    let got = extract_member(&cab, "Working\\b.upk", u64::MAX).unwrap();
    assert_eq!(got, files[1].1);
}

#[test]
fn a_stored_folder_is_copied_out() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![
        ("one.bin", incompressible(3, 50_000)),
        ("two.bin", incompressible(4, 9_000)),
    ];
    let info = write_cab_set(
        dir.path(),
        &files,
        CabOptions {
            compression: 0,
            ..one_cabinet()
        },
    );
    let cab = only_cabinet(dir.path(), &info.cabinets);
    assert_eq!(
        extract_member(&cab, "two.bin", u64::MAX).unwrap(),
        files[1].1
    );
}

#[test]
fn the_name_match_ignores_ascii_case() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![("Working\\SGWGame\\x.ini", repetitive(5, 3_000))];
    let info = write_cab_set(dir.path(), &files, one_cabinet());
    let cab = only_cabinet(dir.path(), &info.cabinets);
    let got = extract_member(&cab, "WORKING\\sgwgame\\X.INI", u64::MAX).unwrap();
    assert_eq!(got, files[0].1);
}

#[test]
fn a_missing_member_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let info = write_cab_set(dir.path(), &[("a.txt", b"abc".to_vec())], one_cabinet());
    let cab = only_cabinet(dir.path(), &info.cabinets);
    match extract_member(&cab, "b.txt", u64::MAX).unwrap_err() {
        UnpackError::Cab(message) => assert!(message.contains("0 matches"), "{message}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_name_listed_twice_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![("x.txt", b"one".to_vec()), ("X.TXT", b"two".to_vec())];
    let info = write_cab_set(dir.path(), &files, one_cabinet());
    let cab = only_cabinet(dir.path(), &info.cabinets);
    match extract_member(&cab, "x.txt", u64::MAX).unwrap_err() {
        UnpackError::Cab(message) => assert!(message.contains("2 matches"), "{message}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_file_over_max_bytes_is_refused_before_decoding() {
    let dir = tempfile::tempdir().unwrap();
    let data = repetitive(6, 5_000);
    let info = write_cab_set(dir.path(), &[("big.txt", data.clone())], one_cabinet());
    let cab = only_cabinet(dir.path(), &info.cabinets);
    assert!(matches!(
        extract_member(&cab, "big.txt", 4_999),
        Err(UnpackError::Cab(_))
    ));
    assert_eq!(extract_member(&cab, "big.txt", 5_000).unwrap(), data);
}

#[test]
fn a_folder_spanning_into_another_cabinet_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let files = vec![("Working\\big.bin", incompressible(7, 150_000))];
    let info = write_cab_set(
        dir.path(),
        &files,
        CabOptions {
            blocks_per_cab: 2,
            ..CabOptions::default()
        },
    );
    assert!(info.cabinets.len() >= 2, "{:?}", info.cabinets);
    let first = dir.path().join(&info.cabinets[0]);
    match extract_member(&first, "Working\\big.bin", u64::MAX).unwrap_err() {
        UnpackError::Cab(message) => assert!(message.contains("spans"), "{message}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_member_after_a_continued_file_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    // The writer keeps a zero-length file in the carried-on folder (it needs no
    // data), so the last cabinet lists it as a normal entry beside big.bin's
    // continuation. A file with data there would be moved to a folder of its own.
    let files = vec![
        ("Working\\big.bin", incompressible(7, 150_000)),
        ("Working\\empty.txt", Vec::new()),
    ];
    let info = write_cab_set(
        dir.path(),
        &files,
        CabOptions {
            blocks_per_cab: 2,
            ..CabOptions::default()
        },
    );
    assert!(info.cabinets.len() >= 2, "{:?}", info.cabinets);
    let last = dir.path().join(info.cabinets.last().unwrap());
    match extract_member(&last, "Working\\empty.txt", u64::MAX).unwrap_err() {
        UnpackError::Cab(message) => assert!(message.contains("spans"), "{message}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_quantum_folder_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let info = write_cab_set(
        dir.path(),
        &[("a.txt", b"abc".to_vec())],
        CabOptions {
            compression: 2,
            ..one_cabinet()
        },
    );
    let cab = only_cabinet(dir.path(), &info.cabinets);
    match extract_member(&cab, "a.txt", u64::MAX).unwrap_err() {
        UnpackError::Cab(message) => assert!(message.contains("Quantum"), "{message}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_lzx_folder_with_garbage_fails_as_lzx_not_as_refused() {
    let dir = tempfile::tempdir().unwrap();
    // typeCompress 0x1503: LZX (low nibble 3) with a 2 MiB window (bits 8..=12 = 21).
    let info = write_cab_set(
        dir.path(),
        &[("a.txt", b"abc".to_vec())],
        CabOptions {
            compression: 0x1503,
            ..one_cabinet()
        },
    );
    let cab = only_cabinet(dir.path(), &info.cabinets);
    match extract_member(&cab, "a.txt", u64::MAX).unwrap_err() {
        UnpackError::Cab(message) => {
            assert!(message.contains("LZX"), "{message}");
            assert!(!message.contains("only MSZIP"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

/// A pinned file of C-8, as `PP-16` names them.
struct Pin {
    cab: &'static str,
    dll: &'static str,
    bytes: u64,
    sha256: &'static str,
}

const PINS: [Pin; 2] = [
    Pin {
        cab: "Nov2008_d3dx9_40_x86.cab",
        dll: "d3dx9_40.dll",
        bytes: 4_379_984,
        sha256: "16fd909aeb68d0d1aca8529dc7f78880b97d6649d70ce8d03a2c858bc28e216b",
    },
    Pin {
        cab: "APR2007_xinput_x86.cab",
        dll: "xinput1_3.dll",
        bytes: 81_768,
        sha256: "8d540d484ea41e374fd0107d55d253f87ded4ce780d515d8fd59bbe8c98970a7",
    },
];

/// The revert guard for the LZX path: the real cabinets' DLLs must decode to
/// their pinned sizes and hashes. Run with `CIMMERIA_DX9_DIR` set to the
/// directory holding the two cabinets.
#[test]
#[ignore = "reads the real DirectX cabinets; set CIMMERIA_DX9_DIR"]
fn real_directx_cabs() {
    let Some(dir) = std::env::var_os("CIMMERIA_DX9_DIR") else {
        eprintln!("CIMMERIA_DX9_DIR is unset; skipping");
        return;
    };
    let dir = PathBuf::from(dir);
    for pin in &PINS {
        let bytes = extract_cab_member(&dir.join(pin.cab), pin.dll, pin.bytes).unwrap();
        let digest: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        println!("{} {} bytes sha256 {digest}", pin.dll, bytes.len());
        assert_eq!(bytes.len() as u64, pin.bytes, "{}", pin.dll);
        assert_eq!(digest, pin.sha256, "{}", pin.dll);
    }
}
