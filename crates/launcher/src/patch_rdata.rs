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
    #[error("Server address {addr:?} is not a valid hostname (allowed: ASCII alphanumeric, '.', '-'; no leading/trailing '.' or '-')")]
    InvalidHostname { addr: String },
    #[error("Original hostname not found in binary — may already be patched")]
    PatternNotFound,
}

/// Validates that `host` looks like a DNS hostname before we write it into
/// SGW.exe's `.rdata`. The .exe won't execute the string (it's just a
/// string in a data section) but garbage input here produces a binary
/// that silently can't reach any server, which is hard to diagnose. We
/// also defensively reject characters that could be misinterpreted by
/// any future parsing logic on the client side.
fn is_valid_hostname(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= MAX_HOST_LEN
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && !host.starts_with('-')
        && !host.ends_with('-')
        && !host.starts_with('.')
        && !host.ends_with('.')
}

fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    data.windows(pattern.len())
        .position(|window| window == pattern)
}

/// Replace either the original CME hostname **or** a previously-patched
/// host (`previous_host`) with `new_host`. Used when re-patching after a
/// `server_host` config change — the first `patch_hostname` call removed
/// the original `www.stargateworlds.com` literal, so subsequent re-patches
/// must search for whatever host was written in last time.
///
/// Falls back to the original CME pattern if `previous_host` is `None` or
/// not found in `data`. Returns the offset where the new host was written.
pub fn patch_hostname_any(
    data: &mut [u8],
    new_host: &str,
    previous_host: Option<&str>,
) -> Result<usize, PatchError> {
    let host_bytes = new_host.as_bytes();
    if host_bytes.len() > MAX_HOST_LEN {
        return Err(PatchError::AddressTooLong {
            len: host_bytes.len(),
        });
    }
    if !is_valid_hostname(new_host) {
        return Err(PatchError::InvalidHostname {
            addr: new_host.to_string(),
        });
    }

    let offset = find_existing_host(data, previous_host).ok_or(PatchError::PatternNotFound)?;

    data[offset..offset + host_bytes.len()].copy_from_slice(host_bytes);
    for i in host_bytes.len()..MAX_HOST_LEN {
        data[offset + i] = 0;
    }

    Ok(offset)
}

/// Search for whichever host string is currently sitting in the `.rdata`
/// slot. Tries the previously-patched host first (when known), then falls
/// back to the original CME literal. The previous host is searched as a
/// 22-byte slot — the previous-host bytes followed by zero padding — so
/// we match the exact run we wrote in last time and don't accidentally
/// hit an unrelated substring.
fn find_existing_host(data: &[u8], previous_host: Option<&str>) -> Option<usize> {
    if let Some(prev) = previous_host {
        if let Some(pattern) = padded_host_slot(prev) {
            if let Some(idx) = find_pattern(data, &pattern) {
                return Some(idx);
            }
        }
    }
    find_pattern(data, ORIGINAL_HOST)
}

/// Builds the exact 22-byte slot that `patch_hostname_any` writes for
/// `host` (host bytes + null padding to MAX_HOST_LEN), so we can search
/// for the slot we previously wrote. Returns `None` for hosts that
/// wouldn't have been valid to write in the first place.
fn padded_host_slot(host: &str) -> Option<[u8; MAX_HOST_LEN]> {
    let bytes = host.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_HOST_LEN {
        return None;
    }
    let mut slot = [0u8; MAX_HOST_LEN];
    slot[..bytes.len()].copy_from_slice(bytes);
    Some(slot)
}

/// True iff the .exe's `.rdata` currently advertises a host different
/// from `expected_host` (in the slot previously patched, or in the
/// original CME slot if never patched).
pub fn host_differs(data: &[u8], expected_host: &str, previous_host: Option<&str>) -> bool {
    // Original CME literal still present → definitely needs patching.
    if find_pattern(data, ORIGINAL_HOST).is_some() {
        return true;
    }
    // Previously-patched host slot present and matches expected → no work.
    if let Some(slot) = padded_host_slot(expected_host) {
        if find_pattern(data, &slot).is_some() {
            return false;
        }
    }
    // Otherwise: either the previous host is what's in the slot (and
    // differs from expected), or some other host entirely. Either way:
    // re-patch needed.
    let _ = previous_host;
    true
}

/// Write `new_host` into SGW.exe's `.rdata`. Pass `previous_host` when
/// re-patching after a `server_host` change so we can locate the slot
/// even though the original CME literal is long gone.
///
/// Writes are **atomic**: the modified bytes go to a sibling
/// `.patching` file, then rename onto SGW.exe. NTFS rename is atomic on
/// the same volume, so a power loss between the truncate and the finish
/// leaves either the old SGW.exe or the new one — never a half-written
/// .exe that the game can't load (would otherwise force a full multi-GB
/// re-seed for recovery).
pub fn patch_exe_any(
    exe_path: &Path,
    new_host: &str,
    previous_host: Option<&str>,
) -> Result<usize, PatchError> {
    let mut data = std::fs::read(exe_path)?;
    let offset = patch_hostname_any(&mut data, new_host, previous_host)?;
    let tmp = exe_path.with_extension("exe.patching");
    std::fs::write(&tmp, &data)?;
    std::fs::rename(&tmp, exe_path)?;
    Ok(offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test-only shims kept for readability of the original simple cases.
    // Production code goes through the `_any` variants exclusively.
    fn patch_hostname(data: &mut [u8], new_host: &str) -> Result<usize, PatchError> {
        patch_hostname_any(data, new_host, None)
    }
    fn patch_exe(exe_path: &Path, new_host: &str) -> Result<usize, PatchError> {
        patch_exe_any(exe_path, new_host, None)
    }
    fn needs_patching(data: &[u8]) -> bool {
        find_pattern(data, ORIGINAL_HOST).is_some()
    }

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
    fn patch_hostname_rejects_empty() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        assert!(matches!(
            patch_hostname(&mut data, ""),
            Err(PatchError::InvalidHostname { .. })
        ));
    }

    #[test]
    fn patch_hostname_rejects_disallowed_chars() {
        let data = make_fake_exe(b"www.stargateworlds.com");
        for bad in &["bad host", "spaces here", "x@y.com", "x/y", "1.2.3/4"] {
            let mut d = data.clone();
            assert!(
                matches!(
                    patch_hostname(&mut d, bad),
                    Err(PatchError::InvalidHostname { .. })
                ),
                "expected InvalidHostname for {bad:?}"
            );
        }
    }

    #[test]
    fn patch_hostname_rejects_leading_or_trailing_dash_or_dot() {
        let data = make_fake_exe(b"www.stargateworlds.com");
        for bad in &["-host.com", "host.com-", ".host.com", "host.com."] {
            let mut d = data.clone();
            assert!(
                matches!(
                    patch_hostname(&mut d, bad),
                    Err(PatchError::InvalidHostname { .. })
                ),
                "expected InvalidHostname for {bad:?}"
            );
        }
    }

    #[test]
    fn patch_hostname_accepts_typical_dns_names() {
        for good in &[
            "localhost",
            "play.cimmeria.gg",
            "auth.example.com",
            "host-1.x",
        ] {
            let mut data = make_fake_exe(b"www.stargateworlds.com");
            assert!(
                patch_hostname(&mut data, good).is_ok(),
                "expected success for {good:?}"
            );
        }
    }

    #[test]
    fn needs_patching_flips_after_patch() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        assert!(needs_patching(&data));
        patch_hostname(&mut data, "play.cimmeria.gg").unwrap();
        assert!(!needs_patching(&data));
    }

    // Regression guard for the silent re-patch bug: after the first patch,
    // the original CME string is gone, so `patch_hostname` (which only
    // searches for the original) would refuse to repatch when the user
    // changes `server_host`. `patch_hostname_any` with the previously-
    // patched host MUST find the slot and overwrite it.
    #[test]
    fn patch_hostname_any_repatches_when_host_changes() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        // First patch — original CME literal in the slot.
        let off1 = patch_hostname_any(&mut data, "play.cimmeria.gg", None).unwrap();
        assert_eq!(off1, 40);
        assert!(
            data.windows(b"play.cimmeria.gg".len())
                .any(|w| w == b"play.cimmeria.gg"),
            "first patch should have written play.cimmeria.gg"
        );
        assert!(
            !data
                .windows(ORIGINAL_HOST.len())
                .any(|w| w == ORIGINAL_HOST),
            "original CME literal should be gone after the first patch"
        );

        // Second patch with a different host — must find the previously-
        // patched slot via the `previous_host` fallback.
        let off2 =
            patch_hostname_any(&mut data, "staging.cimmeria.gg", Some("play.cimmeria.gg")).unwrap();
        assert_eq!(off2, 40, "should target the same .rdata offset");
        assert!(
            data.windows(b"staging.cimmeria.gg".len())
                .any(|w| w == b"staging.cimmeria.gg"),
            "second patch should have written staging.cimmeria.gg"
        );
        assert!(
            !data
                .windows(b"play.cimmeria.gg".len())
                .any(|w| w == b"play.cimmeria.gg"),
            "old patched host should be gone after the re-patch"
        );
    }

    // Without `previous_host`, patch_hostname_any falls back to the
    // original CME literal — same behaviour as the plain patch_hostname.
    #[test]
    fn patch_hostname_any_falls_back_to_original_when_previous_none() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        patch_hostname_any(&mut data, "x.example.com", None).unwrap();
        assert!(data
            .windows(b"x.example.com".len())
            .any(|w| w == b"x.example.com"));
    }

    // When the binary is already on the expected host, host_differs is false.
    #[test]
    fn host_differs_false_when_slot_matches_expected() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        patch_hostname_any(&mut data, "play.cimmeria.gg", None).unwrap();
        assert!(!host_differs(
            &data,
            "play.cimmeria.gg",
            Some("play.cimmeria.gg")
        ));
    }

    #[test]
    fn host_differs_true_when_binary_still_has_original() {
        let data = make_fake_exe(b"www.stargateworlds.com");
        assert!(host_differs(&data, "play.cimmeria.gg", None));
    }

    #[test]
    fn host_differs_true_when_expected_changed() {
        let mut data = make_fake_exe(b"www.stargateworlds.com");
        patch_hostname_any(&mut data, "play.cimmeria.gg", None).unwrap();
        // User edits server_host to a new value — host_differs must report
        // the binary as out of sync.
        assert!(host_differs(
            &data,
            "staging.cimmeria.gg",
            Some("play.cimmeria.gg")
        ));
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
