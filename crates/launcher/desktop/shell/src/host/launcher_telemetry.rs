//! The launcher's adoption events for the forwarder (C9). Reads only: the state
//! lock is held to read the target and dropped before anything is built or sent.
use super::NativeHost;
use cimmeria_launcher_engine::game_telemetry::launcher_event::{
    launcher_events, LauncherRecord, Redaction,
};
use cimmeria_launcher_engine::game_telemetry::launcher_forwarder::{consent_epoch, ForwardSource};
use cimmeria_launcher_engine::game_telemetry::launcher_session::LauncherTarget;
use cimmeria_launcher_engine::game_telemetry::transport::TelemetryEvent;
use std::sync::Arc;

impl NativeHost {
    /// The target and events for `records`, or None. Returns None unless this build
    /// bundles the telemetry DLL (`launch_resources…client_telemetry.is_some()`, read
    /// as `host/game_telemetry.rs` line 35 does) and `launcher_target()` is Some.
    /// Holds the state lock only to read. Never sends.
    pub(crate) fn launcher_batch(
        &self,
        records: &[LauncherRecord],
        redaction: &Redaction,
    ) -> Option<(LauncherTarget, Vec<TelemetryEvent>)> {
        let target = self.launcher_flush_target()?;
        Some((target, launcher_events(records, redaction)))
    }

    /// The target alone, under the same module check as `launcher_batch`. The forwarder
    /// reads it again before every send, so an opt-out is seen at once.
    pub(crate) fn launcher_flush_target(&self) -> Option<LauncherTarget> {
        if !self.module_available() {
            return None;
        }
        let store = self.store().ok()?;
        let state = store.lock().ok()?;
        state.launcher_target()
    }

    fn module_available(&self) -> bool {
        self.launch_resources
            .as_ref()
            .is_some_and(|resources| resources.client_telemetry.is_some())
    }
}

/// The forwarder's view of the host. It caches nothing: the target and the consent
/// epoch are read fresh on every call (C4).
pub(crate) struct HostSource {
    pub host: Arc<NativeHost>,
    pub redaction: Redaction,
}

impl ForwardSource for HostSource {
    fn events(&self, records: &[LauncherRecord]) -> Option<Vec<TelemetryEvent>> {
        self.host
            .launcher_batch(records, &self.redaction)
            .map(|(_, events)| events)
    }

    fn target(&self) -> Option<LauncherTarget> {
        self.host.launcher_flush_target()
    }

    fn epoch(&self) -> u64 {
        consent_epoch()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::GameTelemetryCommand;
    use cimmeria_launcher_engine::launch::{Artifact, Resources};
    use sha2::{Digest, Sha256};

    fn artifact(root: &std::path::Path, name: &str) -> Artifact {
        let path = root.join(name);
        std::fs::write(&path, b"fixture").unwrap();
        let digest: String = Sha256::digest(b"fixture")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Artifact::open(path, &digest).unwrap()
    }
    fn host(root: &std::path::Path, with_dll: bool, tag: &str) -> NativeHost {
        let mut host = NativeHost::new(root.join(format!("state-{tag}")));
        host.launch_resources = Some(Resources {
            proton: None,
            helper: artifact(root, "helper.exe"),
            client_patches: None,
            client_telemetry: with_dll.then(|| artifact(root, "telemetry.dll")),
            graphics: None,
        });
        host
    }

    #[test]
    fn a_batch_needs_the_module_and_the_opt_in() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let records = [LauncherRecord {
            ts_ms: 1,
            level: "info".into(),
            target: "client.launcher.adoption",
            fields: serde_json::Map::new(),
        }];
        let redaction = Redaction::default();
        // The command refuses an opt-in without the module, so the choice is saved directly.
        let no_module = host(&root, false, "a");
        no_module
            .store()
            .unwrap()
            .lock()
            .unwrap()
            .set_game_telemetry(true)
            .unwrap();
        assert!(no_module.launcher_batch(&records, &redaction).is_none());
        let host = host(&root, true, "b");
        assert!(host.launcher_batch(&records, &redaction).is_none());
        host.game_telemetry_command(GameTelemetryCommand::Set {
            schema_version: 1,
            opted_in: true,
        })
        .unwrap();
        let (_, events) = host.launcher_batch(&records, &redaction).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            TelemetryEvent::ClientNative(event) if event.target == "client.launcher.adoption"
        ));
    }

    #[test]
    fn the_flush_target_needs_the_module_and_the_opt_in() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let no_module = host(&root, false, "c");
        no_module
            .store()
            .unwrap()
            .lock()
            .unwrap()
            .set_game_telemetry(true)
            .unwrap();
        assert!(no_module.launcher_flush_target().is_none());
        let host = host(&root, true, "d");
        assert!(host.launcher_flush_target().is_none());
        host.game_telemetry_command(GameTelemetryCommand::Set {
            schema_version: 1,
            opted_in: true,
        })
        .unwrap();
        assert!(host.launcher_flush_target().is_some());
    }
}
