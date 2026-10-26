//! The game's own DirectX DLLs in the Proton prefix (C-8, PP-16).
//!
//! `d3dx9_40.dll` and `xinput1_3.dll` come out of the game's own prerequisite
//! cabinets, are checked against pinned sizes and SHA-256 hashes, and are
//! written into the prefix's `syswow64` folder. `OVERRIDES` tells Wine to load
//! these native copies instead of its own. The work runs on every Play, because
//! a Proton prefix upgrade may rewrite `syswow64`.

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;

#[cfg(target_os = "linux")]
use cimmeria_launcher_core::unpack::extract_cab_member;
use cimmeria_launcher_core::unpack::UnpackError;
use sha2::{Digest, Sha256};

use super::prefix::{Prefix, ProtonError};

/// One DLL the game needs natively: the cabinet it comes from, its member name,
/// and the size and SHA-256 of the bytes that member holds.
pub struct Pinned {
    /// The cabinet file name, under the game's prerequisite folder.
    pub cab: &'static str,
    /// The DLL name, both the cabinet member and the file in `syswow64`.
    pub dll: &'static str,
    /// The exact size of the DLL in bytes.
    pub bytes: u64,
    /// The lower-case hex SHA-256 of the DLL.
    pub sha256: &'static str,
}

/// The two DLLs the game loads natively, in the order they are installed.
pub const DLLS: [Pinned; 2] = [
    Pinned {
        cab: "Nov2008_d3dx9_40_x86.cab",
        dll: "d3dx9_40.dll",
        bytes: 4_379_984,
        sha256: "16fd909aeb68d0d1aca8529dc7f78880b97d6649d70ce8d03a2c858bc28e216b",
    },
    Pinned {
        cab: "APR2007_xinput_x86.cab",
        dll: "xinput1_3.dll",
        bytes: 81_768,
        sha256: "8d540d484ea41e374fd0107d55d253f87ded4ce780d515d8fd59bbe8c98970a7",
    },
];

/// The `WINEDLLOVERRIDES` entries that make Wine use the native DLLs.
pub const OVERRIDES: &str = "d3dx9_40=n;xinput1_3=n";

/// The game's prerequisite folder, relative to the game root.
const PREREQUISITES: &str = ".cimmeria-prerequisites/DX9.0c";

/// The 32-bit system folder inside the Wine prefix, relative to `pfx`.
const SYSWOW64: &str = "drive_c/windows/syswow64";

/// Puts the pinned DLLs in the prefix's `syswow64` folder, extracting each one
/// that is missing or wrong. Any failure is `Setup("directx")`, and a DLL whose
/// bytes do not match its pin never replaces the file in the prefix.
#[cfg(target_os = "linux")]
pub fn ensure(prefix: &Prefix, game_root: &Path) -> Result<(), ProtonError> {
    let result = ensure_with(prefix, game_root, &DLLS, extract_cab_member);
    if result.is_err() {
        tracing::warn!(target: "launcher.proton", event = "proton_directx_failed", "proton: directx failed");
    }
    result
}

/// `ensure` with the pins and the extractor as parameters, so the tests can use
/// fixture cabinets and count the extractor calls.
fn ensure_with(
    prefix: &Prefix,
    game_root: &Path,
    pins: &[Pinned],
    extract: impl Fn(&Path, &str, u64) -> Result<Vec<u8>, UnpackError>,
) -> Result<(), ProtonError> {
    let folder = prefix.pfx.join(SYSWOW64);
    if !folder.is_dir() {
        return Err(ProtonError::Setup("directx"));
    }
    for pin in pins {
        let target = folder.join(pin.dll);
        if is_current(&target, pin)? {
            log(pin.dll, "present");
            continue;
        }
        // The game's folder may be spelled in another case on disk.
        let cab = cimmeria_patchset::resolve_existing_case(
            game_root,
            &Path::new(PREREQUISITES).join(pin.cab),
        );
        let bytes = extract(&cab, pin.dll, pin.bytes).map_err(|_| ProtonError::Setup("directx"))?;
        if bytes.len() as u64 != pin.bytes || sha256_hex(&bytes) != pin.sha256 {
            return Err(ProtonError::Setup("directx"));
        }
        write_over(&folder, &target, pin.dll, &bytes)?;
        log(pin.dll, "installed");
    }
    Ok(())
}

/// True when `target` is a regular file (not a symlink) with the pinned length
/// and hash. A missing file is simply not current.
fn is_current(target: &Path, pin: &Pinned) -> Result<bool, ProtonError> {
    let meta = match fs::symlink_metadata(target) {
        Ok(meta) => meta,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(ProtonError::Setup("directx")),
    };
    if !meta.is_file() || meta.len() != pin.bytes {
        return Ok(false);
    }
    let data = fs::read(target).map_err(|_| ProtonError::Setup("directx"))?;
    Ok(sha256_hex(&data) == pin.sha256)
}

/// Writes `bytes` to `<dll>.cimmeria-tmp` in `folder`, syncs it, then renames it
/// over `target`. A stale temporary file or symlink is removed first, and the
/// new file is created exclusively, so the write never follows a link. A failed
/// write removes the temporary file.
fn write_over(folder: &Path, target: &Path, dll: &str, bytes: &[u8]) -> Result<(), ProtonError> {
    let temp = folder.join(format!("{dll}.cimmeria-tmp"));
    let _ = fs::remove_file(&temp);
    let written = (|| -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, target)
    })();
    written.map_err(|_| {
        let _ = fs::remove_file(&temp);
        ProtonError::Setup("directx")
    })
}

/// The lower-case hex SHA-256 of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Logs one DLL's outcome (`installed` or `present`). Only the DLL name is
/// logged, never a path (R8).
fn log(dll: &'static str, state: &'static str) {
    tracing::info!(
        target: "launcher.proton",
        event = "proton_directx",
        dll,
        state,
        "proton: directx {state}"
    );
}

#[cfg(test)]
#[path = "directx_tests.rs"]
mod tests;
