//! Single native updater owner. Renderer inputs never include URLs, keys or bytes.
use super::NativeHost;
use cimmeria_launcher_engine::{
    updater::{self, Error, Snapshot},
    StorageError,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum UpdaterCommand {
    Inspect {
        schema_version: u32,
    },
    Check {
        schema_version: u32,
        revision: u64,
        operation_revision: u64,
    },
    Apply {
        schema_version: u32,
        revision: u64,
        operation_revision: u64,
        offer_id: Uuid,
    },
    Prepare {
        schema_version: u32,
        revision: u64,
        operation_revision: u64,
        offer_id: Uuid,
    },
}
impl NativeHost {
    /// Native composition installs shutdown before any Apply capability is exposed.
    pub fn with_updater_shutdown(mut self, shutdown: impl Fn() + Send + Sync + 'static) -> Self {
        self.updater_shutdown = Some(std::sync::Arc::new(shutdown));
        self
    }

    pub async fn updater_command(&self, request: UpdaterCommand) -> Result<Snapshot, Error> {
        let schema = match &request {
            UpdaterCommand::Inspect { schema_version }
            | UpdaterCommand::Check { schema_version, .. }
            | UpdaterCommand::Prepare { schema_version, .. }
            | UpdaterCommand::Apply { schema_version, .. } => *schema_version,
        };
        if schema != 1 {
            return Err(StorageError::UnsupportedSchema.into());
        }
        let store = self.store()?;
        let config = self.updater_config.clone();
        if let UpdaterCommand::Apply {
            offer_id,
            revision,
            operation_revision,
            ..
        } = request
        {
            // The detached blocking owner outlives a dropped renderer request.
            let shutdown = self.updater_shutdown.clone().ok_or(Error::Disabled)?;
            return retained_apply(
                move |on_handoff| {
                    if config.is_none() {
                        return Err(Error::Disabled);
                    }
                    let target = updater::InstalledTarget::current()?;
                    store
                        .lock()
                        .map_err(|_| StorageError::Io)?
                        .apply_launcher_update_with_handoff(
                            config.as_ref(),
                            &target,
                            offer_id,
                            revision,
                            operation_revision,
                            on_handoff,
                        )
                },
                shutdown,
            )
            .await
            .map_err(|_| Error::Interrupted)?;
        }
        let ticket = {
            let mut state = store.lock().map_err(|_| StorageError::Io)?;
            match request {
                UpdaterCommand::Apply { .. } => unreachable!(),
                UpdaterCommand::Inspect { .. } => {
                    return state.launcher_update_snapshot(config.as_ref())
                }
                UpdaterCommand::Check {
                    revision,
                    operation_revision,
                    ..
                } => state.begin_launcher_update_check(
                    config.as_ref(),
                    revision,
                    operation_revision,
                )?,
                UpdaterCommand::Prepare {
                    revision,
                    operation_revision,
                    offer_id,
                    ..
                } => state.begin_launcher_update_prepare(
                    config.as_ref(),
                    offer_id,
                    revision,
                    operation_revision,
                )?,
            }
        };
        let config = config.ok_or(Error::Disabled)?;
        // Detach-safe native ownership: dropping a renderer Promise never cancels
        // this task or admits another game/setup mutation while it is in flight.
        tokio::spawn(async move {
            let mut ticket = ticket;
            if matches!(request, UpdaterCommand::Check { .. }) {
                let outcome = updater::check(&config).await;
                store
                    .lock()
                    .map_err(|_| StorageError::Io)?
                    .finish_launcher_update_check(ticket, outcome)?;
            } else {
                let outcome = updater::download(&config, ticket.offer()?).await;
                if outcome.is_ok() {
                    store
                        .lock()
                        .map_err(|_| StorageError::Io)?
                        .mark_launcher_update_verifying(&mut ticket)?;
                }
                // Verification and bounded filesystem writes run off the async executor.
                let worker_store = store.clone();
                let worker_config = config.clone();
                tokio::task::spawn_blocking(move || {
                    worker_store
                        .lock()
                        .map_err(|_| StorageError::Io)?
                        .finish_launcher_update_prepare(&worker_config, ticket, outcome)
                })
                .await
                .map_err(|_| Error::Interrupted)??;
            }
            let result = store
                .lock()
                .map_err(|_| StorageError::Io)?
                .launcher_update_snapshot(Some(&config));
            result
        })
        .await
        .map_err(|_| Error::Interrupted)?
    }
}

// Dropping the request future drops only its JoinHandle, never the already
// admitted native owner or shutdown after a successful spawn, even if later persistence fails.
fn retained_apply(
    apply: impl FnOnce(&mut dyn FnMut()) -> Result<Snapshot, Error> + Send + 'static,
    shutdown: std::sync::Arc<dyn Fn() + Send + Sync>,
) -> tokio::task::JoinHandle<Result<Snapshot, Error>> {
    tokio::task::spawn_blocking(move || {
        let mut handed_off = false;
        let result = apply(&mut || handed_off = true);
        if handed_off {
            shutdown();
        }
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn updater_host_without_release_configuration_is_disabled() {
        let root = tempfile::tempdir().unwrap();
        let host = NativeHost::new(root.path().join("state"));
        let status = host
            .updater_command(UpdaterCommand::Inspect { schema_version: 1 })
            .await
            .unwrap();
        assert_eq!(status.phase, updater::Phase::Disabled);
        assert!(status.offer.is_none());
        assert!(matches!(
            host.updater_command(UpdaterCommand::Check {
                schema_version: 1,
                revision: 0,
                operation_revision: 0
            })
            .await,
            Err(Error::Disabled)
        ));
        assert!(!root.path().join("state/launcher-update.json").exists());
    }
    #[test]
    fn updater_ipc_rejects_renderer_url_key_bytes_and_incomplete_apply() {
        for field in ["url", "public_key", "bytes"] {
            let mut request = serde_json::json!({"command":"check","schema_version":1,"revision":0,"operation_revision":0});
            request[field] = serde_json::json!("attacker");
            assert!(serde_json::from_value::<UpdaterCommand>(request).is_err());
        }
        assert!(serde_json::from_value::<UpdaterCommand>(
            serde_json::json!({"command":"apply","schema_version":1})
        )
        .is_err());
    }
    #[tokio::test]
    async fn lost_apply_reply_still_shuts_down_once_after_durable_handoff_and_never_on_failure() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("handoff.json");
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let done = std::sync::Mutex::new(Some(done_tx));
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let saved = path.clone();
        let handle = retained_apply(
            move |on_handoff| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                let snapshot = Snapshot {
                    schema_version: 1,
                    revision: 5,
                    operation_revision: 0,
                    phase: updater::Phase::RestartRequired,
                    offer: None,
                    failure: None,
                    requires_reopen: false,
                };
                std::fs::write(saved, serde_json::to_vec(&snapshot).unwrap()).unwrap();
                on_handoff();
                Ok(snapshot)
            },
            Arc::new(move || {
                observed.fetch_add(1, Ordering::SeqCst);
                done.lock().unwrap().take().unwrap().send(()).unwrap();
            }),
        );
        started_rx.await.unwrap();
        drop(handle); // renderer abandoned its reply
        release_tx.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), done_rx)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let saved: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(saved["phase"], "restart_required");
        let observed = calls.clone();
        assert_eq!(
            retained_apply(
                |_| Err(Error::Spawn),
                Arc::new(move || {
                    observed.fetch_add(1, Ordering::SeqCst);
                })
            )
            .await
            .unwrap()
            .unwrap_err(),
            Error::Spawn
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[cfg(test)]
mod handoff_tests;
