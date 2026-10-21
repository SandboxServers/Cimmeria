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
