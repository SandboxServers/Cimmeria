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
    Prepare {
        schema_version: u32,
        revision: u64,
        operation_revision: u64,
        offer_id: Uuid,
    },
}
impl NativeHost {
    pub async fn updater_command(&self, request: UpdaterCommand) -> Result<Snapshot, Error> {
        let schema = match &request {
            UpdaterCommand::Inspect { schema_version }
            | UpdaterCommand::Check { schema_version, .. }
            | UpdaterCommand::Prepare { schema_version, .. } => *schema_version,
        };
        if schema != 1 {
            return Err(StorageError::UnsupportedSchema.into());
        }
        let store = self.store()?;
        let config = self.updater_config.clone();
        let ticket = {
            let mut state = store.lock().map_err(|_| StorageError::Io)?;
            match request {
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
    fn updater_ipc_rejects_renderer_url_key_bytes_and_apply() {
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
}
