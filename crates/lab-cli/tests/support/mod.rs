//! Shared test support: the environment lock and guard (Part A); the stub
//! HTTP server is appended in LX-04b.
#![allow(dead_code)]

use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard};

/// Tests that change the process environment hold this lock (nextest gives
/// each test its own process, but `cargo test` runs them as threads of one).
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Takes [`ENV_LOCK`], ignoring poison from an earlier failed test.
pub fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Sets or removes environment variables and restores them when dropped.
pub struct EnvGuard {
    saved: Vec<(String, Option<OsString>)>,
}

impl EnvGuard {
    /// `Some(value)` sets the variable, `None` removes it.
    pub fn set(vars: &[(&str, Option<&str>)]) -> Self {
        let mut saved = Vec::new();
        for (key, value) in vars {
            saved.push((key.to_string(), std::env::var_os(key)));
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        EnvGuard { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, old) in self.saved.iter().rev() {
            match old {
                Some(old) => std::env::set_var(key, old),
                None => std::env::remove_var(key),
            }
        }
    }
}
