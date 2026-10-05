//! One daemon per user session.
//!
//! The guard is a named Win32 mutex (`Local\cimmeria-labd`): the kernel
//! releases it when the holder dies, so a crashed daemon never leaves a stale
//! lock behind the way a pidfile alone would (a recycled pid would read as
//! "still running"). The pidfile beside the log is informational: it tells
//! `tools/lab/daemon.ps1` which process to stop and where it listens.
//!
//! The port is the second guard: a bind failure on the configured address is
//! a refusal too, with the same exit code.

use std::path::{Path, PathBuf};

use serde_json::json;

/// The mutex every `cimmeria-lab --http` takes. `Local\` = this logon
/// session, which is where the at-logon task and the game client both run.
pub const MUTEX_NAME: &str = r"Local\cimmeria-labd";

/// Held for the daemon's life; dropping it releases the mutex.
#[derive(Debug)]
pub struct InstanceGuard {
    #[cfg(windows)]
    handle: isize,
    pidfile: Option<PathBuf>,
}

impl InstanceGuard {
    /// Take the named mutex, or say that another daemon holds it.
    pub fn acquire(name: &str) -> Result<Self, String> {
        #[cfg(windows)]
        {
            let handle = win::create_mutex(name)?;
            Ok(Self {
                handle,
                pidfile: None,
            })
        }
        #[cfg(not(windows))]
        {
            let _ = name;
            Ok(Self { pidfile: None })
        }
    }

    /// Write `{pid, bind, started_at}` to `path`; removed again on drop.
    pub fn write_pidfile(&mut self, path: &Path, bind: &str) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let body = json!({
            "pid": std::process::id(),
            "bind": bind,
            "started_at": chrono::Utc::now().to_rfc3339(),
        });
        std::fs::write(path, serde_json::to_vec_pretty(&body).unwrap_or_default())?;
        self.pidfile = Some(path.to_path_buf());
        Ok(())
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        if let Some(p) = self.pidfile.take() {
            let _ = std::fs::remove_file(p);
        }
        #[cfg(windows)]
        win::close(self.handle);
    }
}

/// The holder's pidfile contents, for the refusal message.
pub fn describe_holder(pidfile: &Path) -> String {
    std::fs::read_to_string(pidfile)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|v| format!("pid {} on {}", v["pid"], v["bind"].as_str().unwrap_or("?")))
        .unwrap_or_else(|| "pid unknown".into())
}

#[cfg(windows)]
mod win {
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;

    pub fn create_mutex(name: &str) -> Result<isize, String> {
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: null security attributes, a NUL-terminated wide name.
        let h = unsafe { CreateMutexW(std::ptr::null(), 0, wide.as_ptr()) };
        // Read before any other call can overwrite the thread's last error.
        let err = unsafe { GetLastError() };
        if h.is_null() {
            return Err(format!("CreateMutexW({name}) failed: error {err}"));
        }
        if err == ERROR_ALREADY_EXISTS {
            // SAFETY: h is a valid handle we own.
            unsafe { CloseHandle(h) };
            return Err(format!("another lab daemon holds {name}"));
        }
        Ok(h as isize)
    }

    pub fn close(h: isize) {
        // SAFETY: the handle came from CreateMutexW and is closed once.
        unsafe { CloseHandle(h as _) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard: a second daemon must be refused while the first
    /// lives, and allowed once it is gone.
    #[cfg(windows)]
    #[test]
    fn a_second_guard_is_refused_until_the_first_drops() {
        let name = format!(r"Local\cimmeria-labd-test-{}", uuid::Uuid::new_v4());
        let first = InstanceGuard::acquire(&name).expect("first");
        let e = InstanceGuard::acquire(&name).unwrap_err();
        assert!(e.contains("another lab daemon"), "{e}");
        drop(first);
        InstanceGuard::acquire(&name).expect("free again after drop");
    }

    #[test]
    fn the_pidfile_names_the_holder_and_goes_with_the_guard() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("labd.pid");
        let name = format!(r"Local\cimmeria-labd-test-{}", uuid::Uuid::new_v4());
        let mut g = InstanceGuard::acquire(&name).unwrap();
        g.write_pidfile(&path, "127.0.0.1:8779").unwrap();
        let d = describe_holder(&path);
        assert!(d.contains(&std::process::id().to_string()), "{d}");
        assert!(d.contains("127.0.0.1:8779"), "{d}");
        drop(g);
        assert!(!path.exists());
    }
}
