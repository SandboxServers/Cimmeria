//! The client-patches DLL's two events for one Play, read from
//! `cimmeria-client-patches.log` beside `SGW.exe` by core's
//! `PatchLogWatcher`, so the fields match the Windows launcher's:
//!
//! - `client.patches.boot`: injection outcome, DLL version, fingerprint per
//!   hooked site and the boot verdict. Once per Play.
//! - `client.patches.counts`: Black Market claimed / delivered / dropped
//!   counts (lower bounds). At most once a minute while they move, and once
//!   at game exit.
//!
//! Play creates one [`PatchEvents`] just before the game starts. A session
//! loop calls [`PatchEvents::poll`] on each tick and sends what it returns;
//! at game exit [`PatchEvents::finish`] returns what is still owed. Each
//! event is returned once, however the two are interleaved. Play hands it to
//! the session loop when a telemetry session runs. Without one the exit
//! events go to the disk queue ([`queue_at_exit`]) and ship with the next
//! session's first flush.
use super::transport::{queue_now, Consent, TelemetryEvent, TransportError};
use super::DesktopState;
pub use cimmeria_launcher_core::telemetry::patch_log::PatchInjection;
use cimmeria_launcher_core::telemetry::patch_log::PatchLogWatcher;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Instant, SystemTime};

/// What Play did with the client-patches DLL. `listed`: the DLL was on the
/// injection list; `wanted`: Play's injection policy asked for it.
pub fn injection(listed: bool, wanted: bool) -> PatchInjection {
    match (listed, wanted) {
        (true, _) => PatchInjection::Injected,
        (false, false) => PatchInjection::OptedOut,
        (false, true) => PatchInjection::Unavailable,
    }
}

impl DesktopState {
    /// [`injection`] for a Play whose resolved resources did (`listed`) or
    /// did not carry the DLL. "Wanted" is Play's own policy, never the raw
    /// preference: an adopted copy launches with the choice reviewed at
    /// adoption, so its patches-off is an opt-out too.
    pub(crate) fn patch_injection(&self, listed: bool) -> PatchInjection {
        // An unreadable policy would have refused admission. Count it as
        // wanted: `unavailable` warns, where `opted_out` would hide it.
        let wanted = self
            .injection_policy()
            .map_or(true, |policy| policy.client_patches);
        injection(listed, wanted)
    }
}

#[derive(Debug)]
pub struct PatchEvents(PatchLogWatcher);

impl PatchEvents {
    /// `binaries` is the host directory holding `SGW.exe`. Call just before
    /// the game starts: a log older than that belongs to an earlier launch.
    pub fn new(binaries: &Path, injection: PatchInjection) -> Self {
        Self(PatchLogWatcher::new(binaries, SystemTime::now(), injection))
    }

    /// During the session: the boot event once the DLL logged its verdict
    /// (at once when it was not injected), and the counts when due.
    pub fn poll(&mut self, now: Instant) -> Vec<TelemetryEvent> {
        self.0
            .poll()
            .into_iter()
            .chain(self.0.poll_counts(now))
            .collect()
    }

    /// At game exit: the boot event if it was never sent (the DLL logged no
    /// verdict, or no log at all), then the final counts.
    pub fn finish(&mut self) -> Vec<TelemetryEvent> {
        self.0
            .finish()
            .into_iter()
            .chain(self.0.finish_counts())
            .collect()
    }
}

/// At game exit with no session to send through: queue what
/// [`PatchEvents::finish`] returns for the next session, stamped now. The
/// log is not read unless the player is opted in. Returns how many were
/// queued.
pub fn queue_at_exit(state: &Mutex<DesktopState>, events: &mut PatchEvents) -> usize {
    let consent = match state.lock() {
        Ok(owner) => Consent::read(&owner),
        Err(_) => return 0,
    };
    // The lock is released: the log is read and queued without it.
    let Ok(Some(consent)) = consent else {
        return 0;
    };
    let now = chrono::Utc::now().timestamp_millis();
    let mut queued = 0;
    for mut event in events.finish() {
        if let TelemetryEvent::ClientNative(e) = &mut event {
            if e.ts_ms == 0 {
                e.ts_ms = now;
            }
        }
        match queue_now(&consent, &event) {
            Ok(()) => queued += 1,
            // The player opted out since the consent was read: stop.
            Err(TransportError::NotOptedIn) => break,
            Err(_) => tracing::warn!(
                reason = "queue_write_failed",
                "client-patches event not queued"
            ),
        }
    }
    queued
}
