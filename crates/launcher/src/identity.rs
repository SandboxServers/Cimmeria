//! Per-install launcher identity for dev-session telemetry.
//!
//! `install_id` is a UUID v4 minted once into `install.json`; mint-once
//! semantics matter because reissuing would orphan a dev's prior
//! session history server-side.
//!
//! `machine_id` is `sha256(MachineGuid registry value)[:16 hex]` on
//! Windows, hashed so the raw GUID never leaves the host. Falls back to
//! `sha256(hostname()):16` on Unix and on Windows registry-read
//! failure. MachineGuid (not hostname) because hostname changes
//! produce spurious "new machine" rows in server-side analytics.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::state::atomic_write;

/// Bump on breaking on-disk shape changes; load() refuses to
/// deserialise against the wrong version rather than silently
/// defaulting fields.
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
    pub schema_version: u32,
    pub install_id: Uuid,
    pub machine_id: String,
    pub first_seen_ms: i64,
    pub created_by_launcher_version: String,
}

impl LauncherIdentity {
    pub fn mint() -> Self {
        Self {
            schema_version: IDENTITY_SCHEMA_VERSION,
            install_id: Uuid::new_v4(),
            machine_id: derive_machine_id(),
            first_seen_ms: chrono::Utc::now().timestamp_millis(),
            created_by_launcher_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Mint-once: a parse failure on an existing file surfaces as an
    /// error rather than silently re-minting (which would orphan
    /// server-side session history).
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

pub fn identity_path() -> PathBuf {
    crate::config::exe_dir().join("install.json")
}

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

    fn to_wide_z(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    // KEY_WOW64_64KEY: defend against a future 32-bit launcher build
    // getting a Wow6432Node-redirected key.
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

    // MachineGuid is ~38 chars; 256 bytes of UTF-16 is oversized.
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
