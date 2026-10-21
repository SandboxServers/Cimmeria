//! The launcher's adoption events for the forwarder (C9). Reads only: the state
//! lock is held to read the target and dropped before anything is built or sent.
use super::NativeHost;
use cimmeria_launcher_engine::game_telemetry::launcher_event::{
    launcher_events, LauncherRecord, Redaction,
};
use cimmeria_launcher_engine::game_telemetry::launcher_session::LauncherTarget;
use cimmeria_launcher_engine::game_telemetry::transport::TelemetryEvent;

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
        let available = self
            .launch_resources
            .as_ref()
            .is_some_and(|resources| resources.client_telemetry.is_some());
        if !available {
            return None;
        }
        let store = self.store().ok()?;
        let target = store.lock().ok()?.launcher_target()?;
        Some((target, launcher_events(records, redaction)))
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
}
