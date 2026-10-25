//! Where the sign-in refresh token waits between launches (C9 `store`).
//!
//! One entry per auth host, under the service `cimmeria-launcher`; its value is
//! the JSON of [`Stored`]. It is never written to `launcher-config.json`,
//! `preferences.json` or a log. A store error gives `None` or is dropped, and the
//! log names only the operation, never the value.

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

/// The store the sign-in session reads and writes, keyed by the auth host.
pub trait CredentialStore: Send + Sync {
    /// The entry for `host`, or `None` when there is none or it cannot be read.
    fn get(&self, host: &str) -> Option<Stored>;
    /// Replaces the entry for `host`.
    fn set(&self, host: &str, stored: &Stored);
    /// Removes the entry for `host`. A missing entry is not an error.
    fn delete(&self, host: &str);
}

/// A store that lives in this process only. Tests use it, and targets without an
/// OS credential store use it for the whole session.
#[derive(Default)]
pub struct MemoryStore {
    entries: Mutex<HashMap<String, Stored>>,
}

impl CredentialStore for MemoryStore {
    fn get(&self, host: &str) -> Option<Stored> {
        self.entries.lock().ok()?.get(host).cloned()
    }

    fn set(&self, host: &str, stored: &Stored) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.insert(host.to_owned(), stored.clone());
        }
    }

    fn delete(&self, host: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(host);
        }
    }
}

/// The OS credential store: Windows Credential Manager or the macOS keychain.
#[cfg(any(windows, target_os = "macos"))]
pub struct KeyringStore;

#[cfg(any(windows, target_os = "macos"))]
impl KeyringStore {
    fn entry(host: &str) -> Option<keyring::Entry> {
        match keyring::Entry::new(SERVICE, host) {
            Ok(entry) => Some(entry),
            Err(_) => {
                tracing::tracing::debug!("sign-in store: the credential entry could not be opened");
                None
            }
        }
    }
}

#[cfg(any(windows, target_os = "macos"))]
impl CredentialStore for KeyringStore {
    fn get(&self, host: &str) -> Option<Stored> {
        let secret = match Self::entry(host)?.get_password() {
            Ok(secret) => secret,
            Err(_) => {
                tracing::tracing::debug!("sign-in store: the entry could not be read");
                return None;
            }
        };
        serde_json::from_str(&secret).ok()
    }

    fn set(&self, host: &str, stored: &Stored) {
        let Ok(secret) = serde_json::to_string(stored) else {
            return;
        };
        let Some(entry) = Self::entry(host) else {
            return;
        };
        if entry.set_password(&secret).is_err() {
            tracing::tracing::debug!("sign-in store: the entry could not be written");
        }
    }

    fn delete(&self, host: &str) {
        let Some(entry) = Self::entry(host) else {
            return;
        };
        if entry.delete_credential().is_err() {
            tracing::tracing::debug!("sign-in store: the entry could not be removed");
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
