//! Where the sign-in refresh token waits between launches (C9 `store`).
//!
//! One entry per auth host, under the service `cimmeria-launcher`; its value is
//! the JSON of [`Stored`]. It is never written to `launcher-config.json`,
//! `preferences.json` or a log. A store error is returned to the caller, never
//! dropped; the log names only the operation and the [`StoreError`] kind.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

/// The keyring service name every launcher entry uses.
pub const SERVICE: &str = "cimmeria-launcher";

/// The stored sign-in: the rotating refresh token and the account's names.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stored {
    pub refresh_token: String,
    pub account_name: String,
    pub display_name: String,
}

impl fmt::Debug for Stored {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stored")
            .field("refresh_token", &"<hidden>")
            .field("account_name", &self.account_name)
            .field("display_name", &self.display_name)
            .finish()
    }
}

/// Why the credential store failed. Its `Display` names no secret and no host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreError {
    /// The OS store is not there or refuses access.
    Unavailable,
    /// The store answered, but the read or write did not complete.
    Io,
}

impl StoreError {
    /// The short kind a log line carries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::Io => "io",
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => f.write_str("credential store unavailable"),
            Self::Io => f.write_str("credential store i/o error"),
        }
    }
}

/// The store the sign-in session reads and writes, keyed by the auth host.
pub trait CredentialStore: Send + Sync {
    /// The entry for `host`: `Ok(None)` when there is none, `Err` when the store
    /// could not be read.
    fn get(&self, host: &str) -> Result<Option<Stored>, StoreError>;
    /// Replaces the entry for `host`. `Err` when the write did not land.
    fn set(&self, host: &str, stored: &Stored) -> Result<(), StoreError>;
    /// Removes the entry for `host`. Deleting a missing entry is `Ok`.
    fn delete(&self, host: &str) -> Result<(), StoreError>;
}

/// A store that lives in this process only. Tests use it, and targets without an
/// OS credential store use it for the whole session.
#[derive(Default)]
pub struct MemoryStore {
    entries: Mutex<HashMap<String, Stored>>,
}

impl CredentialStore for MemoryStore {
    fn get(&self, host: &str) -> Result<Option<Stored>, StoreError> {
        let entries = self.entries.lock().map_err(|_| StoreError::Io)?;
        Ok(entries.get(host).cloned())
    }

    fn set(&self, host: &str, stored: &Stored) -> Result<(), StoreError> {
        let mut entries = self.entries.lock().map_err(|_| StoreError::Io)?;
        entries.insert(host.to_owned(), stored.clone());
        Ok(())
    }

    fn delete(&self, host: &str) -> Result<(), StoreError> {
        let mut entries = self.entries.lock().map_err(|_| StoreError::Io)?;
        entries.remove(host);
        Ok(())
    }
}

/// The OS credential store: Windows Credential Manager or the macOS keychain.
#[cfg(any(windows, target_os = "macos"))]
pub struct KeyringStore;

/// The contract's two kinds: no store to reach is `Unavailable`, anything else `Io`.
#[cfg(any(windows, target_os = "macos"))]
fn classify(e: &keyring::Error) -> StoreError {
    match e {
        keyring::Error::PlatformFailure(_) | keyring::Error::NoStorageAccess(_) => {
            StoreError::Unavailable
        }
        _ => StoreError::Io,
    }
}

/// Logs a failed operation with its name and kind only, never the host or the value.
#[cfg(any(windows, target_os = "macos"))]
fn logged(op: &'static str, kind: StoreError) -> StoreError {
    tracing::warn!(
        op,
        error = kind.as_str(),
        "sign-in store: an operation failed"
    );
    kind
}

#[cfg(any(windows, target_os = "macos"))]
impl KeyringStore {
    fn entry(op: &'static str, host: &str) -> Result<keyring::Entry, StoreError> {
        keyring::Entry::new(SERVICE, host).map_err(|e| logged(op, classify(&e)))
    }
}

// CT-U6 covers this impl on a real host: CI has no OS credential store.
#[cfg(any(windows, target_os = "macos"))]
impl CredentialStore for KeyringStore {
    fn get(&self, host: &str) -> Result<Option<Stored>, StoreError> {
        match Self::entry("get", host)?.get_password() {
            Ok(secret) => serde_json::from_str(&secret)
                .map(Some)
                .map_err(|_| logged("get", StoreError::Io)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(logged("get", classify(&e))),
        }
    }

    fn set(&self, host: &str, stored: &Stored) -> Result<(), StoreError> {
        let secret = serde_json::to_string(stored).map_err(|_| logged("set", StoreError::Io))?;
        Self::entry("set", host)?
            .set_password(&secret)
            .map_err(|e| logged("set", classify(&e)))
    }

    fn delete(&self, host: &str) -> Result<(), StoreError> {
        match Self::entry("delete", host)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(logged("delete", classify(&e))),
        }
    }
}

/// The store a launcher session uses: the OS credential store where there is one.
#[cfg(any(windows, target_os = "macos"))]
pub fn platform_store() -> Arc<dyn CredentialStore> {
    Arc::new(KeyringStore)
}

/// The store a launcher session uses: memory only, on targets without an OS store.
#[cfg(not(any(windows, target_os = "macos")))]
pub fn platform_store() -> Arc<dyn CredentialStore> {
    Arc::new(MemoryStore::default())
}
