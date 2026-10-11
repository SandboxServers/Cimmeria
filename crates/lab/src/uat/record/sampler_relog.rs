//! The sampler's tap hygiene: starting a tap (and saying when it replaced
//! one), following the player to a new entity on a relog, restarting a tap
//! that vanished, and the guard that stops the tap when the sampler task
//! ends without [`Sampler::finish`].
//!
//! Tap semantics (`wire-log` `tap.rs`): `start` replaces any ring on that
//! entity and reports `replaced_existing`; a disconnect never stops a tap,
//! so only a stop or a replacement makes one vanish.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};

use super::super::log::Tick;
use super::reads::find_session;
use super::server::call_ok;
use super::Sampler;
use crate::uat::invoke::ServerInvoker;

/// The error entry for a `start` that replaced someone's tap: their ring is
/// gone, and they will read "no active packet tap" or our messages.
pub(super) const TAP_REPLACED: &str = "tap replaced an existing tap";

/// `server_packet_tap_start` on `entity`; `Ok(replaced_existing)`.
pub(super) async fn start_tap(
    server: &dyn ServerInvoker,
    entity: u32,
    capacity: u32,
) -> Result<bool, String> {
    let r = call_ok(
        server,
        "server_packet_tap_start",
        json!({ "entity_id": entity, "capacity": capacity }),
    )
    .await?;
    Ok(r.get("replaced_existing")
        .and_then(Value::as_bool)
        .unwrap_or(false))
}

impl Sampler<'_> {
    /// The player's tap or entity went away: find the session carrying
    /// this `player_id` again. On another entity (a relog), move the tap
    /// there and stop the old one. On the same entity with `tap_missing`
    /// (the tap read said "no active packet tap"), restart the tap.
    pub(super) async fn check_relog(&mut self, t: &mut Tick, tap_missing: bool) {
        let found = match call_ok(self.server, "server_sessions", json!({})).await {
            Ok(s) => find_session(&s, self.player_id),
            Err(e) => Err(e),
        };
        let id = match found {
            Ok(id) => id,
            Err(e) => {
                // Not in the world (yet), or the roster read failed: the
                // next failed read checks again.
                tracing::debug!(
                    target: "lab.record",
                    character = %self.character, player_id = self.player_id,
                    entity = self.entity, error = %e,
                    "recording: relog check, no session for the player"
                );
                return;
            }
        };

        if id == self.entity {
            if !tap_missing {
                tracing::debug!(
                    target: "lab.record",
                    character = %self.character, player_id = self.player_id, entity = id,
                    "recording: relog check, same entity"
                );
                return;
            }
            match start_tap(self.server, id, self.cfg.tap_capacity).await {
                Ok(replaced) => {
                    tracing::debug!(
                        target: "lab.record",
                        character = %self.character, player_id = self.player_id, entity = id, replaced,
                        "recording: relog check, vanished tap restarted"
                    );
                    self.err(t, "tap_restart", "tap restarted");
                    if replaced {
                        self.err(t, "tap_replaced", TAP_REPLACED);
                    }
                }
                Err(e) => {
                    tracing::debug!(
                        target: "lab.record",
                        character = %self.character, player_id = self.player_id, entity = id, error = %e,
                        "recording: relog check, tap restart failed"
                    );
                    self.err(t, "tap_restart", format!("tap restart failed: {e}"));
                }
            }
            return;
        }

        let old = self.entity;
        match start_tap(self.server, id, self.cfg.tap_capacity).await {
            Ok(replaced) => {
                self.entity = id;
                self.tap.store(id, Ordering::SeqCst);
                self.seen.insert(id);
                // Best effort: a tap left on the old id would keep a ring
                // the server holds until someone stops it.
                if let Err(e) = call_ok(
                    self.server,
                    "server_packet_tap_stop",
                    json!({ "entity_id": old }),
                )
                .await
                {
                    tracing::debug!(target: "lab.record", entity = old, error = %e, "recording: old tap stop failed");
                }
                tracing::info!(
                    target: "lab.record",
                    character = %self.character, player_id = self.player_id, old, entity = id, replaced,
                    "recording: relog, tap restarted"
                );
                self.err(t, "relog", format!("relog: tap restarted on {id}"));
                if replaced {
                    self.err(t, "tap_replaced", TAP_REPLACED);
                }
            }
            Err(e) => {
                tracing::debug!(
                    target: "lab.record",
                    character = %self.character, player_id = self.player_id, old, entity = id, error = %e,
                    "recording: relog, tap restart failed"
                );
                self.err(
                    t,
                    "relog",
                    format!("relog: tap restart on {id} failed: {e}"),
                );
            }
        }
    }
}

/// Stops the recording's tap when the sampler task ends any way other
/// than [`Sampler::finish`] (aborted, or a panic unwinding it). LR-07
/// builds one inside the task, before [`Sampler::begin`], and passes
/// [`cell`](Self::cell) to it; `finish` zeroes the cell.
pub struct TapStopGuard {
    server: Arc<dyn ServerInvoker>,
    tap: Arc<AtomicU32>,
}

impl TapStopGuard {
    pub fn new(server: Arc<dyn ServerInvoker>) -> Self {
        Self {
            server,
            tap: Arc::default(),
        }
    }

    /// The tapped entity as the sampler last set it (0: none).
    pub fn cell(&self) -> Arc<AtomicU32> {
        Arc::clone(&self.tap)
    }
}

impl Drop for TapStopGuard {
    fn drop(&mut self) {
        let id = self.tap.swap(0, Ordering::SeqCst);
        if id == 0 {
            return;
        }
        // Drop cannot await: the stop runs as its own task, best effort.
        let Ok(rt) = tokio::runtime::Handle::try_current() else {
            tracing::debug!(target: "lab.record", entity = id, "recording: no runtime to stop the tap");
            return;
        };
        let server = Arc::clone(&self.server);
        rt.spawn(async move {
            let out = server
                .call("server_packet_tap_stop", json!({ "entity_id": id }))
                .await;
            tracing::debug!(
                target: "lab.record",
                entity = id, ok = out.ok, error = ?out.error,
                "recording: tap stopped by the guard"
            );
        });
    }
}
