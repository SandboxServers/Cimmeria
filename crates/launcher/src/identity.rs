//! Per-install launcher identity for dev-session telemetry (#366).
//!
//! Two ids:
//!
//! 1. **`install_id`** — UUID v4 minted on first launch. Cosmos
//!    partition key on the server side; identifies "this copy of the
//!    launcher on this machine" across upgrades, server restarts, and
//!    log uploads. Mint-once: subsequent launches load the same value
//!    so a developer's session history threads together.
//!
//! 2. **`machine_id`** — derived from
//!    `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid` (a Windows-
//!    installer-set GUID stable for the OS install's lifetime) hashed
//!    with sha256 and truncated to 16 hex chars. Hashed before storage
//!    so we never persist the raw GUID — telemetry data downstream
//!    sees a privacy-preserving derived id, not the registry value.
//!    Falls back to `sha256(hostname()):16` on Linux/macOS and on
//!    Windows when the registry read fails.
//!
//! The original spec snippet (`sha256(hostname || gethostname)[:16]`)
//! was self-referential — `hostname` is `gethostname`. Clara G4 review
//! flagged this; MachineGuid is the canonical Windows-side stable
//! identifier (it survives hostname changes and is what most analytics
//! pipelines key on).
//!
//! Persistence: `install.json` next to the launcher exe — same
//! convention as `launcher-config.json` and `uploaded.json`. Atomic
//! write via [`crate::state::atomic_write`] so a power loss mid-mint
//! either leaves the old file or the new file, never half-written.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::state::atomic_write;

/// Current persisted `install.json` schema. Bump when a breaking
/// change to the on-disk shape lands; the load path then refuses to
/// deserialise against the wrong schema. Adding `#[serde(default)]`
/// fields is always backwards-compatible by construction and does NOT
/// require a bump.
pub const IDENTITY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error(
        "Identity schema version {got} unsupported (expected {expected}). \
         Delete install.json to regenerate — but note that this will mint a \
         new install_id and your past session history will no longer thread \
         to the new one."
    )]
    UnsupportedSchema { got: u32, expected: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LauncherIdentity {
    /// On-disk schema version. See [`IDENTITY_SCHEMA_VERSION`].
    pub schema_version: u32,
    /// UUID v4. Cosmos partition key on the server side. Stable for the
    /// lifetime of this `install.json` — if the file is deleted, the
    /// next mint produces a different value and the dev's old sessions
    /// become orphaned (still queryable by sid, just not by sub).
    pub install_id: Uuid,
    /// 16 hex chars = 64 bits of derived machine identity. Source: see
    /// the module doc. Stable across launcher upgrades on the same
    /// Windows install; changes across OS reinstalls (MachineGuid is
    /// regenerated).
    pub machine_id: String,
    /// Wall-clock ms-since-epoch at which this identity was minted.
    /// Useful for "how long has this dev been generating telemetry?"
    /// queries server-side; never load-bearing for auth.
    pub first_seen_ms: i64,
    /// `CARGO_PKG_VERSION` at mint time. Records which launcher build
    /// minted the identity so a downstream session can correlate weird
    /// behaviour back to a known-buggy launcher rev without having to
    /// guess at deploy timestamps.
    pub created_by_launcher_version: String,
}

impl LauncherIdentity {
    /// Mint a new identity NOW. Caller is responsible for persisting it
    /// via [`Self::save`] — splitting mint from save lets callers
    /// inspect the freshly-minted value (e.g. in tests) without coupling
    /// to a particular on-disk location.
    pub fn mint() -> Self {
        Self {
            schema_version: IDENTITY_SCHEMA_VERSION,
            install_id: Uuid::new_v4(),
            machine_id: derive_machine_id(),
            first_seen_ms: chrono::Utc::now().timestamp_millis(),
            created_by_launcher_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Load `install.json` from `path`, or mint + save + return a fresh
    /// identity if the file doesn't exist.
    ///
    /// **Mint-once invariant:** if the file exists but fails to parse,
    /// this surfaces the error instead of silently re-minting — that
    /// would orphan all of the dev's prior session telemetry. The
    /// load_or_mint convenience is for first-launch only.
    pub fn load_or_mint(path: &Path) -> Result<Self, IdentityError> {
        if path.exists() {
            return Self::load(path);
        }
        let id = Self::mint();
        id.save(path)?;
        Ok(id)
    }

    pub fn load(path: &Path) -> Result<Self, IdentityError> {
        let text = std::fs::read_to_string(path)?;
        let id: LauncherIdentity = serde_json::from_str(&text)?;
        if id.schema_version != IDENTITY_SCHEMA_VERSION {
            return Err(IdentityError::UnsupportedSchema {
                got: id.schema_version,
                expected: IDENTITY_SCHEMA_VERSION,
            });
        }
        Ok(id)
    }

    pub fn save(&self, path: &Path) -> Result<(), IdentityError> {
        let bytes = serde_json::to_vec_pretty(self)?;
        atomic_write(path, &bytes)?;
        Ok(())
    }
}

/// Resolve the canonical `install.json` path: next to the launcher exe,
/// alongside `launcher-config.json` and `uploaded.json`. Keeping the
/// per-install state files co-located makes "wipe + start over" a
/// single-directory operation.
pub fn identity_path() -> PathBuf {
    crate::config::exe_dir().join("install.json")
}

/// Compute the `machine_id` for the current host.
///
/// Windows: read `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid`,
/// hash with sha256, take first 16 hex chars.
///
/// Non-Windows: hash `gethostname()` instead. The launcher only ships
/// on Windows, so the Unix branch only fires in unit tests / dev
/// builds on the CI Ubuntu runner.
///
/// If the Windows registry read fails (locked-down corporate
/// environments, missing key on weird OS variants), falls back to
/// hostname-hash with a tracing::warn — telemetry continues to work,
/// the id just isn't quite as stable as ideal.
fn derive_machine_id() -> String {
    #[cfg(windows)]
    {
        match read_machine_guid_from_registry() {
            Ok(guid) => return hash_to_16_hex(guid.as_bytes()),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to read HKLM\\SOFTWARE\\Microsoft\\Cryptography\\MachineGuid; \
                     falling back to hostname-derived machine_id"
                );
            }
        }
    }
    let host = gethostname::gethostname();
    hash_to_16_hex(host.to_string_lossy().as_bytes())
}

/// `sha256(input)` truncated to the first 16 lowercase hex characters
/// = 64 bits of derived identity. Enough entropy that two random
/// machines almost never collide (~ 1 in 2^32 at the dev-pool sizes
/// we care about), short enough to fit comfortably in log lines and
/// status-bar text.
fn hash_to_16_hex(input: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(input);
    let digest = h.finalize();
    let mut s = String::with_capacity(16);
    for byte in &digest[..8] {
        s.push_str(&format!("{byte:02x}"));
    }
    s
}

#[cfg(windows)]
fn read_machine_guid_from_registry() -> Result<String, std::io::Error> {
    use std::io;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE,
        KEY_WOW64_64KEY, REG_SZ,
    };

    /// Encode `s` as a NUL-terminated UTF-16 string suitable for the
    /// `*W` registry APIs.
    fn to_wide_z(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    // KEY_WOW64_64KEY: ensure we hit the 64-bit registry view even from
    // a 32-bit launcher binary. The launcher is 64-bit today but
    // pinning the view explicitly defends against a future 32-bit
    // build silently reading a Wow6432Node-redirected key.
    let subkey = to_wide_z(r"SOFTWARE\Microsoft\Cryptography");
    let value_name = to_wide_z("MachineGuid");

    let mut hkey = std::ptr::null_mut();
    // SAFETY: subkey is a valid NUL-terminated UTF-16 string.
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            0,
            KEY_QUERY_VALUE | KEY_WOW64_64KEY,
            &mut hkey,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }

    // MachineGuid is a 38-char string with braces stripped (36 + 2 for
    // the braces in some Windows builds). 256 bytes of UTF-16 is
    // comfortably oversized.
    let mut buf = [0u16; 128];
    let mut buf_bytes: u32 = (buf.len() * 2) as u32;
    let mut value_type: u32 = 0;
    // SAFETY: hkey is open, buf has buf_bytes capacity, value_name is
    // NUL-terminated.
    let status = unsafe {
        RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut value_type,
            buf.as_mut_ptr() as *mut u8,
            &mut buf_bytes,
        )
    };
    // SAFETY: hkey is open; closing is best-effort.
    unsafe {
        let _ = RegCloseKey(hkey);
    }
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    if value_type != REG_SZ {
        return Err(io::Error::other(format!(
            "MachineGuid registry value is type {value_type}, expected REG_SZ ({REG_SZ})"
        )));
    }
    // buf_bytes includes the trailing NUL; trim it before decoding.
    let len_u16 = (buf_bytes as usize / 2).saturating_sub(1);
    let s = String::from_utf16_lossy(&buf[..len_u16]);
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_identity_path(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().join("install.json")
    }

    // hash_to_16_hex must be deterministic + exactly 16 lowercase hex
    // chars regardless of input length. A regression that ever returned
    // fewer chars would break the Cosmos key shape downstream.
    #[test]
    fn hash_to_16_hex_is_deterministic_and_16_lowercase_hex() {
        let a = hash_to_16_hex(b"hello");
        let b = hash_to_16_hex(b"hello");
        assert_eq!(a, b, "deterministic");
        assert_eq!(a.len(), 16);
        assert!(
            a.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()),
            "must be lowercase hex, got: {a}"
        );
        let c = hash_to_16_hex(b"world");
        assert_ne!(a, c, "different inputs → different hashes");
    }

    // Empty input is still a valid sha256 → still 16 hex chars. The
    // value is the well-known sha256 of "" so this also pins the
    // truncation (first 8 bytes of e3b0c4...).
    #[test]
    fn hash_to_16_hex_empty_input() {
        assert_eq!(hash_to_16_hex(b""), "e3b0c44298fc1c14");
    }

    // First call → mint + persist + return. Second call → load from
    // disk + return identical struct. install_id must be stable across
    // calls or the dev's session history orphans on every relaunch.
    #[test]
    fn load_or_mint_mints_once_and_reuses() {
        let dir = tempfile::tempdir().unwrap();
        let path = temp_identity_path(&dir);
        let a = LauncherIdentity::load_or_mint(&path).unwrap();
        assert!(path.exists(), "first call must persist");
        let b = LauncherIdentity::load_or_mint(&path).unwrap();
        assert_eq!(a, b, "second call must reuse the persisted identity");
    }

    // Save/load roundtrip preserves every field including the version
    // tag. JSON shape regressions surface as a single failed assertion.
    #[test]
    fn save_load_roundtrip_preserves_all_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = temp_identity_path(&dir);
        let original = LauncherIdentity::mint();
        original.save(&path).unwrap();
        let loaded = LauncherIdentity::load(&path).unwrap();
        assert_eq!(original, loaded);
        assert_eq!(loaded.schema_version, IDENTITY_SCHEMA_VERSION);
        assert_eq!(loaded.machine_id.len(), 16);
    }

    // Future-schema identities must be rejected explicitly, NOT
    // silently coerced — otherwise a downgrade would erase fields the
    // current binary doesn't know about and re-save them gone.
    #[test]
    fn load_rejects_unsupported_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = temp_identity_path(&dir);
        std::fs::write(
            &path,
            r#"{"schema_version":99,"install_id":"00000000-0000-0000-0000-000000000000","machine_id":"abc","first_seen_ms":1,"created_by_launcher_version":"x"}"#,
        )
        .unwrap();
        let err = LauncherIdentity::load(&path).unwrap_err();
        assert!(matches!(
            err,
            IdentityError::UnsupportedSchema {
                got: 99,
                expected: 1
            }
        ));
    }

    // Mint-once invariant: a corrupt install.json must surface as an
    // error rather than silently re-minting (which would orphan the
    // dev's session history). load() returns Err on parse failure;
    // load_or_mint inherits that via the early `if path.exists()` path.
    #[test]
    fn load_or_mint_surfaces_corrupt_file_instead_of_silently_reminting() {
        let dir = tempfile::tempdir().unwrap();
        let path = temp_identity_path(&dir);
        std::fs::write(&path, b"this is not json").unwrap();
        let err = LauncherIdentity::load_or_mint(&path).unwrap_err();
        assert!(matches!(err, IdentityError::Json(_)));
    }

    // Distinct install_ids on independent mints — sanity-check that we
    // aren't accidentally reusing UUIDs across processes (e.g. if
    // someone replaced Uuid::new_v4() with a constant).
    #[test]
    fn distinct_mints_produce_distinct_install_ids() {
        let a = LauncherIdentity::mint();
        let b = LauncherIdentity::mint();
        assert_ne!(a.install_id, b.install_id);
        // Both should still share the same machine_id since they ran
        // on the same host.
        assert_eq!(a.machine_id, b.machine_id);
    }

    // Mint records the current launcher's CARGO_PKG_VERSION.
    #[test]
    fn mint_records_current_launcher_version() {
        let id = LauncherIdentity::mint();
        assert_eq!(id.created_by_launcher_version, env!("CARGO_PKG_VERSION"));
    }

    // Windows-only: derive_machine_id reads MachineGuid and produces a
    // stable 16-hex-char output. On CI Windows runners the registry
    // value exists; on locked-down environments the fallback kicks in.
    // Either path yields a 16-char id — the assertion below works for
    // both.
    #[test]
    fn derive_machine_id_returns_16_hex_chars() {
        let id = derive_machine_id();
        assert_eq!(id.len(), 16, "must be 16 chars regardless of source");
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
