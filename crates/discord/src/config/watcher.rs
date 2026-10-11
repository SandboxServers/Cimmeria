//! [`ConfigWatcher`]: hold the live config and reload it on file change.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use notify::Watcher;

use super::model::Config;
use super::ConfigError;

/// Holds the live config and reloads it when the file changes.
///
/// Drop the watcher to stop the reload task. The atomic snapshot returned
/// by [`ConfigWatcher::load`] is safe to clone freely.
pub struct ConfigWatcher {
    config: Arc<ArcSwap<Config>>,
    _watcher: notify::RecommendedWatcher,
    _reload_task: tokio::task::JoinHandle<()>,
    /// Send-side of the manual-reload channel. Calling [`reload`] pushes
    /// a unit message that the task drains to re-read the file. Intended
    /// for an admin-api endpoint that forces a reload even when the
    /// file mtime didn't change — endpoint wiring lives outside this
    /// crate.
    ///
    /// [`reload`]: ConfigWatcher::reload
    reload_tx: tokio::sync::mpsc::Sender<()>,
    path: PathBuf,
}

impl ConfigWatcher {
    /// Construct a watcher around a TOML file. Reads the file once at
    /// startup; if that read or parse fails, returns the error (caller
    /// can decide to fall back to `Config::disabled()`). Once running,
    /// subsequent reload failures keep the previous config in place and
    /// log at `warn!`.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref().to_path_buf();

        // Initial load — fail loudly if the operator's TOML is broken
        // on startup. This is intentional: silently disabling Discord
        // because the TOML had a typo would defeat the purpose.
        let initial = read_and_parse(&path)?;
        let config = Arc::new(ArcSwap::new(Arc::new(initial)));

        // notify::Watcher needs a callback that runs on its own thread.
        // We funnel events through a tokio mpsc so the reload task can
        // debounce + retry on the tokio runtime.
        let (fs_tx, mut fs_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut watcher = notify::RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                // Only content changes queue a reload. Reading the file
                // (our own reload included) raises `Access` events, and
                // those re-triggered a reload that logged "no semantic
                // change" on every pass. Errors still pass through.
                if let Ok(event) = &res {
                    match event.kind {
                        notify::EventKind::Create(_)
                        | notify::EventKind::Modify(_)
                        | notify::EventKind::Remove(_) => {}
                        _ => return,
                    }
                }
                // Best-effort send — if the channel is closed (watcher
                // dropped) we silently discard.
                let _ = fs_tx.send(res);
            },
            notify::Config::default(),
        )
        .map_err(ConfigError::Watcher)?;
        notify::Watcher::watch(&mut watcher, &path, notify::RecursiveMode::NonRecursive)
            .map_err(ConfigError::Watcher)?;

        // Manual reload channel — `ConfigWatcher::reload()` pushes here
        // to force a re-read even when the file mtime hasn't changed
        // (e.g. an admin-api endpoint calls it on operator demand).
        let (reload_tx, mut reload_rx) = tokio::sync::mpsc::channel::<()>(4);

        let config_for_task = config.clone();
        let path_for_task = path.clone();
        let reload_task = tokio::spawn(async move {
            // Debounce file events. Editors often write in two stages
            // (truncate, then write content) — without debouncing we'd
            // parse an empty file in between.
            const DEBOUNCE: Duration = Duration::from_millis(150);

            loop {
                tokio::select! {
                    Some(_) = fs_rx.recv() => {
                        // Drain any further events that arrived during
                        // the debounce window so we don't reload twice
                        // back to back.
                        tokio::time::sleep(DEBOUNCE).await;
                        while fs_rx.try_recv().is_ok() {}
                        reload_from_disk(&path_for_task, &config_for_task);
                    }
                    Some(_) = reload_rx.recv() => {
                        reload_from_disk(&path_for_task, &config_for_task);
                    }
                    else => break,
                }
            }
        });

        Ok(Self {
            config,
            _watcher: watcher,
            _reload_task: reload_task,
            reload_tx,
            path,
        })
    }

    /// Construct from an in-memory config — used by tests and by the
    /// server bin when no config file is provided (in which case Discord
    /// is just disabled).
    pub fn from_static(config: Config) -> Self {
        let (reload_tx, _) = tokio::sync::mpsc::channel(1);
        Self {
            config: Arc::new(ArcSwap::new(Arc::new(config))),
            // We can't construct a no-op `RecommendedWatcher`; use a
            // throw-away tempfile that we don't actually watch. Easiest
            // path: leak a watcher pointed at the current dir but
            // ignore everything. The cost is one `notify::Watcher` per
            // server instance; the gain is a uniform `ConfigWatcher`
            // type for both paths.
            //
            // Concretely: re-using the file-watching constructor would
            // require a real file. For static configs we accept the
            // dummy here.
            _watcher: notify::RecommendedWatcher::new(|_| {}, notify::Config::default())
                .expect("constructing a no-op notify watcher cannot fail"),
            _reload_task: tokio::spawn(async {}),
            reload_tx,
            path: PathBuf::new(),
        }
    }

    /// Snapshot of the current config. Calling this in a hot path is
    /// fine — `ArcSwap::load` is lock-free.
    pub fn load(&self) -> Arc<Config> {
        self.config.load_full()
    }

    /// Get the underlying `ArcSwap` so callers (sender, layer) can hold
    /// a long-lived `Arc<ArcSwap<Config>>` and `load_full()` per event.
    pub fn handle(&self) -> Arc<ArcSwap<Config>> {
        self.config.clone()
    }

    /// Force an immediate reload from disk. Returns immediately; the
    /// actual file read happens on the reload task. If the parse fails,
    /// the old config stays in place and the error is logged at `warn!`.
    pub fn reload(&self) {
        // Best-effort: if the channel is full (4 pending reloads), drop
        // the request. Excess reload pressure has no value.
        let _ = self.reload_tx.try_send(());
    }

    /// Path being watched (for diagnostics / stats endpoint).
    pub fn watched_path(&self) -> &Path {
        &self.path
    }
}

fn read_and_parse(path: &Path) -> Result<Config, ConfigError> {
    let bytes = std::fs::read_to_string(path).map_err(|error| ConfigError::Io {
        path: path.to_path_buf(),
        error,
    })?;
    Config::from_toml_str(&bytes)
}

/// Re-read `path`, parse, and atomically swap. Failures log at warn
/// level and leave the previous config in place.
fn reload_from_disk(path: &Path, target: &Arc<ArcSwap<Config>>) {
    match read_and_parse(path) {
        Ok(new) => {
            // Equality check skips the swap when the file changed but
            // the meaningful content didn't — common when editors touch
            // timestamps without modifying bytes.
            let current = target.load();
            if **current == new {
                tracing::debug!(target: "cimmeria_discord", path = %path.display(), "Discord config reload: no semantic change");
                return;
            }
            tracing::info!(target: "cimmeria_discord", path = %path.display(), "Discord config reloaded");
            target.store(Arc::new(new));
        }
        Err(e) => {
            tracing::warn!(
                target: "cimmeria_discord",
                path = %path.display(),
                error = %e,
                "Discord config reload failed — keeping previous config"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;
    use tracing_subscriber::fmt::MakeWriter;

    /// Wait past the 150 ms debounce so a queued reload has run.
    const SETTLE: Duration = Duration::from_millis(600);

    /// In-memory log sink shared between the subscriber and the test.
    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);

    impl Sink {
        fn count(&self, needle: &str) -> usize {
            let bytes = self.0.lock().unwrap();
            String::from_utf8_lossy(&bytes).matches(needle).count()
        }
    }

    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Sink {
        type Writer = Sink;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn config_toml(username: &str) -> String {
        format!(
            "[discord]\nenabled = true\nusername = \"{username}\"\n\n\
             [discord.channels.lifecycle]\n\
             url = \"https://discord.com/api/webhooks/1/abc\"\n\
             rate_limit_per_min = 30\n"
        )
    }

    /// Reads must not re-trigger a reload. Access events only exist on
    /// Linux (inotify); Windows reports none, so there the read loop is a
    /// no-op and the test only checks the single real change. Run it on
    /// Linux CI for the guard to bite.
    #[tokio::test(flavor = "current_thread")]
    async fn config_reads_do_not_trigger_reload() {
        let dir =
            std::env::temp_dir().join(format!("cimmeria-discord-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("discord.toml");
        std::fs::write(&path, config_toml("first")).unwrap();

        let sink = Sink::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(sink.clone())
            .with_ansi(false)
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let watcher = ConfigWatcher::new(&path).unwrap();

        // One real change: exactly one "reloaded" row.
        std::fs::write(&path, config_toml("second")).unwrap();
        tokio::time::sleep(SETTLE).await;
        assert_eq!(sink.count("Discord config reloaded"), 1);
        assert_eq!(watcher.load().username.as_deref(), Some("second"));

        // Ten reads, then wait past the debounce: no new reload rows of
        // either kind ("reloaded" or "no semantic change").
        let before = sink.count("Discord config reload");
        for _ in 0..10 {
            let _ = std::fs::read_to_string(&path).unwrap();
        }
        tokio::time::sleep(SETTLE).await;
        assert_eq!(sink.count("Discord config reload"), before);

        drop(watcher);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
