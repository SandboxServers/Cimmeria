//! Switch ASLR off in `SGW.exe`'s PE header.
//!
//! `IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE` (0x0040) in the optional
//! header's `DllCharacteristics` asks Windows to relocate the image. With
//! it cleared, the 32-bit client always loads at `0x00400000`, which the
//! client-patches DLL and every RE address in `docs/` assume. It is the
//! only byte a known-good QA client differs from stock in `SGW.exe`
//! (offset 0x186 in the 0.8348 build), and the same change the modding
//! kit's "Fix ASLR" makes. The PE checksum is not enforced for executables,
//! so it is left alone.

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const DYNAMIC_BASE: u16 = 0x0040;
/// `DllCharacteristics` sits at this offset in both the PE32 and PE32+
/// optional headers.
const DLL_CHARACTERISTICS_OFFSET: u64 = 70;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AslrOutcome {
    /// The flag was set and has been cleared.
    Disabled,
    /// Already off; nothing written.
    AlreadyOff,
    /// No `SGW.exe` to patch (not installed yet).
    NoExe,
}

fn invalid(msg: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, format!("SGW.exe: {msg}"))
}

/// Clear the dynamic-base flag in the exe at `exe`, writing two bytes.
pub fn disable(exe: &Path) -> std::io::Result<AslrOutcome> {
    if !exe.is_file() {
        return Ok(AslrOutcome::NoExe);
    }
    let mut f = OpenOptions::new().read(true).write(true).open(exe)?;
    let mut dos = [0u8; 64];
    f.read_exact(&mut dos)?;
    if &dos[..2] != b"MZ" {
        return Err(invalid("no MZ header"));
    }
    let pe = u64::from(u32::from_le_bytes(dos[60..64].try_into().unwrap()));
    let mut sig = [0u8; 4];
    f.seek(SeekFrom::Start(pe))?;
    f.read_exact(&mut sig)?;
    if &sig != b"PE\0\0" {
        return Err(invalid("no PE signature"));
    }
    // Signature (4) + COFF file header (20) = optional header.
    let at = pe + 24 + DLL_CHARACTERISTICS_OFFSET;
    let mut raw = [0u8; 2];
    f.seek(SeekFrom::Start(at))?;
    f.read_exact(&mut raw)?;
    let flags = u16::from_le_bytes(raw);
    if flags & DYNAMIC_BASE == 0 {
        return Ok(AslrOutcome::AlreadyOff);
    }
    f.seek(SeekFrom::Start(at))?;
    f.write_all(&(flags & !DYNAMIC_BASE).to_le_bytes())?;
    f.flush()?;
    Ok(AslrOutcome::Disabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal PE32 header with `e_lfanew` = 0x128 (the 0.8348 build's
    /// value, which puts `DllCharacteristics` at 0x186).
    fn fake_exe(flags: u16) -> Vec<u8> {
        let mut b = vec![0u8; 0x200];
        b[..2].copy_from_slice(b"MZ");
        b[60..64].copy_from_slice(&0x128u32.to_le_bytes());
        b[0x128..0x12C].copy_from_slice(b"PE\0\0");
        b[0x186..0x188].copy_from_slice(&flags.to_le_bytes());
        b
    }

    #[test]
    fn clears_only_the_dynamic_base_bit_at_0x186() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("SGW.exe");
        std::fs::write(&exe, fake_exe(0x8140)).unwrap();
        assert_eq!(disable(&exe).unwrap(), AslrOutcome::Disabled);
        let after = std::fs::read(&exe).unwrap();
        let mut expected = fake_exe(0x8140);
        expected[0x186] = 0x00;
        assert_eq!(after, expected, "one byte changes: 0x40 -> 0x00 at 0x186");
        assert_eq!(disable(&exe).unwrap(), AslrOutcome::AlreadyOff);
    }

    #[test]
    fn refuses_a_file_that_is_not_a_pe() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("SGW.exe");
        std::fs::write(&exe, vec![0u8; 256]).unwrap();
        assert!(disable(&exe).is_err());
        assert_eq!(
            disable(&dir.path().join("missing.exe")).unwrap(),
            AslrOutcome::NoExe
        );
    }

    // Manual check: the stock 0.8348 SGW.exe with ASLR cleared must equal a
    // known-good QA client's SGW.exe byte for byte.
    //   SGW_STOCK_EXE=<stock SGW.exe> SGW_GOOD_EXE=<QA client SGW.exe> \
    //     cargo test -p sgw-launcher real_sgw_exe -- --ignored
    #[test]
    #[ignore = "needs a stock and a known-good SGW.exe; see the comment"]
    fn real_sgw_exe() {
        let stock = std::fs::read(std::env::var("SGW_STOCK_EXE").unwrap()).unwrap();
        let good = std::fs::read(std::env::var("SGW_GOOD_EXE").unwrap()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("SGW.exe");
        std::fs::write(&exe, &stock).unwrap();
        assert_eq!(disable(&exe).unwrap(), AslrOutcome::Disabled);
        assert!(std::fs::read(&exe).unwrap() == good);
    }
}
