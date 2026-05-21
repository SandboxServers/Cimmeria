//! One-time `.rdata` byte patch that redirects SGW.exe's hardcoded SOAP login
//! hostname (`www.stargateworlds.com`) to the configured emulator server.
//!
//! See `docs/plans/2026-03-06-sgw-launcher-design.md` (superseded by
//! `docs/client/sgw-launcher.md`) for the rationale: data-section string
//! edits aren't subject to ASLR or PE-checksum recalculation, so this is
//! simpler than runtime DLL injection via AtreaRL.

use std::path::Path;

use thiserror::Error;

const ORIGINAL_HOST: &[u8] = b"www.stargateworlds.com";
/// The original CME hostname is exactly 22 bytes. Any replacement must fit
/// within that fixed slot (zero-padded out to 22) so we don't shift any
/// surrounding `.rdata` strings or pointers.
const MAX_HOST_LEN: usize = 22;

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Server address too long: {len} bytes (max {MAX_HOST_LEN})")]
    AddressTooLong { len: usize },
    #[error("Original hostname not found in binary — may already be patched")]
    PatternNotFound,
}

fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    data.windows(pattern.len())
        .position(|window| window == pattern)
}

pub fn patch_hostname(data: &mut [u8], new_host: &str) -> Result<usize, PatchError> {
    let host_bytes = new_host.as_bytes();
    if host_bytes.len() > MAX_HOST_LEN {
        return Err(PatchError::AddressTooLong {
            len: host_bytes.len(),
        });
    }

    let offset = find_pattern(data, ORIGINAL_HOST).ok_or(PatchError::PatternNotFound)?;

    data[offset..offset + host_bytes.len()].copy_from_slice(host_bytes);
    for i in host_bytes.len()..MAX_HOST_LEN {
        data[offset + i] = 0;
    }

    Ok(offset)
}

pub fn needs_patching(data: &[u8]) -> bool {
    find_pattern(data, ORIGINAL_HOST).is_some()
}

pub fn patch_exe(exe_path: &Path, new_host: &str) -> Result<usize, PatchError> {
    let mut data = std::fs::read(exe_path)?;
    let offset = patch_hostname(&mut data, new_host)?;
    std::fs::write(exe_path, &data)?;
    Ok(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fake_exe(host: &[u8]) -> Vec<u8> {
        let mut data = vec![0u8; 100];
        data[40..40 + host.len()].copy_from_slice(host);
        data
    }

    #[test]
    fn patch_hostname_success() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        let offset = patch_hostname(&mut data, "localhost").unwrap();
        assert_eq!(offset, 40);
        assert_eq!(&data[40..49], b"localhost");
        assert!(data[49..62].iter().all(|&b| b == 0));
    }

    #[test]
    fn patch_hostname_at_max_length() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        let host = "abcdefghijklmnopqrstuv";
        assert_eq!(host.len(), 22);
        let offset = patch_hostname(&mut data, host).unwrap();
        assert_eq!(offset, 40);
        assert_eq!(&data[40..62], host.as_bytes());
    }

    #[test]
    fn patch_hostname_rejects_too_long() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        let host = "this-hostname-is-way-too-long.example.com";
        let result = patch_hostname(&mut data, host);
        assert!(matches!(result, Err(PatchError::AddressTooLong { .. })));
    }

    #[test]
    fn patch_hostname_pattern_not_found() {
        let mut data = vec![0u8; 100];
        let result = patch_hostname(&mut data, "localhost");
        assert!(matches!(result, Err(PatchError::PatternNotFound)));
    }

    #[test]
    fn needs_patching_flips_after_patch() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        assert!(needs_patching(&data));
        patch_hostname(&mut data, "play.cimmeria.gg").unwrap();
        assert!(!needs_patching(&data));
    }

    #[test]
    fn patch_exe_on_disk_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let exe_path = dir.path().join("SGW.exe");
        let data = make_fake_exe(b"www.stargateworlds.com");
        std::fs::write(&exe_path, &data).unwrap();

        let offset = patch_exe(&exe_path, "localhost").unwrap();
        assert_eq!(offset, 40);

        let patched = std::fs::read(&exe_path).unwrap();
        assert_eq!(&patched[40..49], b"localhost");
    }
}
