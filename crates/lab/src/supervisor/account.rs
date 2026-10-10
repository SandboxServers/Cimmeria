//! This instance's lab account: its credentials file and the cached
//! account name routing matches against (#1312, LP-05b).

use super::{instance, session_file, Supervisor};

/// The cached lab account name, and whether a miss was already logged.
/// A found name lives for the daemon's life: a username changed in the
/// file later is seen by launches (`lab_account`) but not by routing
/// until a restart.
#[derive(Default)]
pub(super) struct AccountCache {
    name: Option<String>,
    warned: bool,
}

impl Supervisor {
    /// This instance's lab account name (`username` of its lab-account file), if
    /// readable. A found name is cached; a miss is retried on the next call and
    /// logged once per run of misses.
    pub fn account_name(&self) -> Option<String> {
        let mut cache = self.account.lock().unwrap_or_else(|p| p.into_inner());
        if cache.name.is_some() {
            return cache.name.clone();
        }
        let dir = self.config.install_dir.as_deref()?;
        let path = instance::account_path(dir, self.instance());
        match session_file::read_lab_account_at(&path) {
            Ok(account) => {
                cache.name = Some(account.username.clone());
                cache.warned = false;
                Some(account.username)
            }
            Err(error) => {
                if !cache.warned {
                    tracing::warn!(
                        target: "lab.instance",
                        instance = self.label(),
                        path = %path.display(),
                        error = %error,
                        "lab account file unreadable; this instance cannot be routed by account name"
                    );
                    cache.warned = true;
                }
                None
            }
        }
    }

    /// This instance's credentials (`lab-account.json`, or
    /// `lab-account.<instance>.json` for a named instance), if the file
    /// exists and parses.
    pub(crate) fn lab_account(&self) -> Option<session_file::LabAccount> {
        let dir = self.config.install_dir.as_deref()?;
        let path = instance::account_path(dir, self.config.instance.as_deref());
        session_file::read_lab_account_at(&path).ok()
    }
}
