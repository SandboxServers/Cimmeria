//! Background watcher that hot-reloads the auth TLS certificate when the
//! cert/key files change on disk.
//!
//! Why mtime polling, not `notify`/inotify: a two-`stat`-per-tick poll is
//! dependency-free, behaves identically on every platform (Linux inotify vs.
//! Windows `ReadDirectoryChangesW` have meaningfully different edge cases around
//! atomic renames and editor write patterns), and is trivially testable — the
//! "check + maybe reload" step is a pure-ish function we can drive directly
//! without a running task or a real clock. Cert rotation is rare and not
//! latency-sensitive (a renewed Let's Encrypt cert being picked up within one
//! poll interval is fine), so the simplicity wins.
//!
//! Resilience contract: an operator mid-write of a PEM file (truncated cert,
//! key not yet rewritten) must never crash the watcher or swap a broken config
//! in. [`TlsCertStore::reload`] only swaps after a *successful* rebuild, so a
//! failed read/parse leaves the previous config live; the watcher logs the
//! failure (negative-logging convention) and retries on the next tick. Because
//! the watcher only advances its "last seen" mtime snapshot when a reload
//! *succeeds*, a transient failure is automatically retried until the operator
//! finishes writing both files.

use std::path::Path;
use std::time::{Duration, SystemTime};

use tokio::time::{interval, MissedTickBehavior};

use super::tls::TlsCertStore;

/// The last-observed modification times of the cert and key files.
///
/// `None` means the file could not be stat'd at the last check (missing, or a
/// transient I/O error) — treated as "no known mtime yet" so the next
/// successful stat that differs triggers a reload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct FileMtimes {
    cert: Option<SystemTime>,
    key: Option<SystemTime>,
}

impl FileMtimes {
    /// Stat both files and capture their mtimes. A file that cannot be stat'd
    /// contributes `None` (logged at debug); this is not an error here — the
    /// reload attempt downstream surfaces a genuine read failure.
    pub(super) fn read(cert_path: &Path, key_path: &Path) -> Self {
        Self {
            cert: mtime_of(cert_path),
            key: mtime_of(key_path),
        }
    }

    /// True when either file's mtime differs from `other` (advanced, regressed,
    /// or appeared/disappeared). We deliberately treat *any* difference as a
    /// reload trigger rather than only forward movement: a restore-from-backup
    /// or a clock adjustment can move an mtime backward, and the operator still
    /// means "use what's on disk now".
    fn differs_from(&self, other: &FileMtimes) -> bool {
        self.cert != other.cert || self.key != other.key
    }
}

/// Read a single file's modification time, returning `None` (and logging at
/// debug) if it can't be stat'd. Never an error — a missing file mid-rotation
/// is expected and handled by the retry-next-tick loop.
fn mtime_of(path: &Path) -> Option<SystemTime> {
    match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(t) => Some(t),
        Err(e) => {
            tracing::debug!(
                path = %path.display(),
                error = %e,
                "auth TLS cert watcher could not stat file; treating mtime as unknown"
            );
            None
        }
    }
}

/// The single "check + maybe reload" step, factored out of the task loop so it
/// is directly testable without a running interval or real wall-clock waits.
///
/// Compares the current on-disk mtimes against `last`. If nothing changed, logs
/// at debug and returns `last` unchanged. If either file changed, attempts a
/// [`TlsCertStore::reload`]:
///
/// - On success: logs `info` with the new cert mtime and returns the *new*
///   mtime snapshot, so subsequent ticks compare against the just-loaded state.
/// - On failure (mid-write PEM, transient I/O): logs a negative `warn` and
///   returns `last` **unchanged**, so the change is retried on the next tick
///   until the read succeeds. The live config is untouched (`reload` only swaps
///   after a successful rebuild).
pub(super) fn check_and_reload(store: &TlsCertStore, last: FileMtimes) -> FileMtimes {
    let current = FileMtimes::read(store.cert_path(), store.key_path());

    if !current.differs_from(&last) {
        tracing::debug!(
            cert = %store.cert_path().display(),
            "auth TLS cert unchanged; no reload"
        );
        return last;
    }

    match store.reload() {
        Ok(()) => {
            tracing::info!(
                cert = %store.cert_path().display(),
                new_cert_mtime = ?current.cert,
                new_key_mtime = ?current.key,
                "auth TLS cert changed on disk; hot-swapped live config"
            );
            current
        }
        Err(e) => {
            // Negative-logging convention: a change was detected but the reload
            // did not take effect. Keep the old mtime snapshot so we retry.
            tracing::warn!(
                cert = %store.cert_path().display(),
                error = %e,
                "auth TLS cert changed but reload failed (mid-write?); keeping previous config, will retry"
            );
            last
        }
    }
}

/// Spawn the background mtime-watcher task.
///
/// Polls the cert/key mtimes every `interval_secs` seconds and calls
/// [`check_and_reload`]. The task runs until the process exits. Only call this
/// when the TLS listener is actually configured; an `interval_secs` of `0`
/// disables the watcher (the caller should not spawn it at all in that case,
/// but this is defended here too).
pub(super) fn spawn_cert_watcher(store: TlsCertStore, interval_secs: u64) {
    if interval_secs == 0 {
        tracing::debug!("auth TLS cert watcher disabled (interval = 0)");
        return;
    }

    tracing::info!(
        cert = %store.cert_path().display(),
        key = %store.key_path().display(),
        interval_secs,
        "Starting auth TLS cert watcher"
    );

    tokio::spawn(async move {
        // Seed the snapshot with the mtimes at start so we only react to changes
        // that happen *after* the listener came up with its initial cert.
        let mut last = FileMtimes::read(store.cert_path(), store.key_path());

        let mut ticker = interval(Duration::from_secs(interval_secs));
        // If the runtime stalls and we miss ticks, collapse them — we only ever
        // need one check, not a backlog of catch-up reloads.
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // The first tick fires immediately; consume it so the first *real* check
        // happens one interval after start, not instantly.
        ticker.tick().await;

        loop {
            ticker.tick().await;
            last = check_and_reload(&store, last);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// RAII temp dir under the OS temp root. Deletes its tree on drop. Mirrors
    /// the helper in `tls.rs`/`tls_smoke.rs` to avoid a `tempfile` dep.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static SEQ: AtomicU64 = AtomicU64::new(0);
            let n = SEQ.fetch_add(1, Ordering::Relaxed);
            let mut p = std::env::temp_dir();
            p.push(format!(
                "cimmeria-certwatch-{tag}-{}-{n}",
                std::process::id()
            ));
            std::fs::create_dir_all(&p).expect("create temp dir");
            TempDir(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Write a fresh self-signed cert/key pair to the given paths.
    fn write_self_signed(cert_path: &Path, key_path: &Path) {
        let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
            .expect("self-signed cert");
        std::fs::write(cert_path, certified.cert.pem()).expect("write cert");
        std::fs::write(key_path, certified.key_pair.serialize_pem()).expect("write key");
    }

    /// Bump a file's mtime forward unconditionally so the change is detectable
    /// even on filesystems with coarse mtime granularity, where rewriting
    /// within the same tick could otherwise leave mtime unchanged.
    fn bump_mtime(path: &Path) {
        let future = SystemTime::now() + Duration::from_secs(10);
        // `set_modified` is stable since Rust 1.75; the workspace MSRV is newer.
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .expect("open for mtime bump");
        f.set_modified(future).expect("set mtime");
    }

    #[test]
    fn check_and_reload_swaps_in_new_cert_when_files_change() {
        let dir = TempDir::new("reload");
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        write_self_signed(&cert_path, &key_path);

        let store = TlsCertStore::load(&cert_path, &key_path).expect("store loads");
        let before_der = store.current_leaf_cert_der();
        let last = FileMtimes::read(&cert_path, &key_path);

        // Operator rotates the cert: a *different* valid self-signed pair lands
        // at the same paths, with a forward mtime so the poll detects it.
        write_self_signed(&cert_path, &key_path);
        bump_mtime(&cert_path);
        bump_mtime(&key_path);

        let new_snapshot = check_and_reload(&store, last);

        // The watcher must have advanced its mtime snapshot...
        assert_ne!(
            new_snapshot, last,
            "mtime snapshot must advance after a detected change"
        );
        // ...and, crucially, the *served* leaf cert must now be the new one.
        // This is what fails if the reload is removed: with no reload the store
        // keeps serving `before_der`.
        let after_der = store.current_leaf_cert_der();
        assert_ne!(
            before_der, after_der,
            "live config must serve the rotated cert after check_and_reload"
        );
    }

    #[test]
    fn check_and_reload_is_noop_when_files_unchanged() {
        let dir = TempDir::new("noop");
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        write_self_signed(&cert_path, &key_path);

        let store = TlsCertStore::load(&cert_path, &key_path).expect("store loads");
        let before_der = store.current_leaf_cert_der();
        let last = FileMtimes::read(&cert_path, &key_path);

        // No file change between snapshot and check.
        let after_snapshot = check_and_reload(&store, last);

        assert_eq!(after_snapshot, last, "snapshot must not change on a no-op");
        assert_eq!(
            before_der,
            store.current_leaf_cert_der(),
            "served cert must be untouched when nothing changed"
        );
    }

    #[test]
    fn corrupt_pem_on_tick_is_ignored_and_old_config_retained() {
        let dir = TempDir::new("corrupt");
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        write_self_signed(&cert_path, &key_path);

        let store = TlsCertStore::load(&cert_path, &key_path).expect("store loads");
        let good_der = store.current_leaf_cert_der();
        let last = FileMtimes::read(&cert_path, &key_path);

        // Operator is mid-write: the cert file is now a truncated/garbage PEM
        // while the key may already have been rewritten. mtime advances so the
        // poll sees a change and *attempts* a reload — which must fail safely.
        std::fs::write(&cert_path, b"-----BEGIN CERTIFICATE-----\ngarbage\n")
            .expect("write corrupt");
        bump_mtime(&cert_path);

        let new_snapshot = check_and_reload(&store, last);

        // The reload failed, so the snapshot must NOT advance — guaranteeing the
        // change is retried on the next tick once the write completes.
        assert_eq!(
            new_snapshot, last,
            "snapshot must stay put when reload fails so the change is retried"
        );
        // And the live config must still serve the original, valid cert.
        assert_eq!(
            good_der,
            store.current_leaf_cert_der(),
            "a broken PEM must never replace the live config"
        );
    }

    #[test]
    fn spawn_with_zero_interval_is_disabled() {
        // A zero interval must not spawn a polling task. We can't easily observe
        // "no task spawned" directly, but the function must return without panic
        // and the call is a documented no-op.
        let dir = TempDir::new("disabled");
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        write_self_signed(&cert_path, &key_path);
        let store = TlsCertStore::load(&cert_path, &key_path).expect("store loads");

        // Must not panic; nothing to await.
        spawn_cert_watcher(store, 0);
    }
}
