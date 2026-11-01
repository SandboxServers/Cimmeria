//! One path for letting go of input (AX-26). Raw `client_input_key` presses
//! are DLL holds now, so a key dies with the bridge connection or the DLL
//! lease; this is the single place the daemon releases everything at once.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tracing::Instrument;

use super::input::DEFAULT_HOLD_MS;
use super::{keys, Supervisor};

impl Supervisor {
    /// The press/release/tap logic of `client_input_key` with the window
    /// already resolved (the test seam: the fake bridge has no game window).
    pub(crate) async fn input_key_at(
        &self,
        hwnd: isize,
        key: &str,
        action: &str,
        hold_ms: Option<u64>,
    ) -> Result<Value, String> {
        let k = keys::named(key).ok_or_else(|| format!("unknown key {key:?}"))?;
        match action {
            "down" => self.hold_key_at(hwnd, key, k, true).await?,
            "up" => self.hold_key_at(hwnd, key, k, false).await?,
            "tap" => {
                self.hold_key_at(hwnd, key, k, true).await?;
                tokio::time::sleep(Duration::from_millis(hold_ms.unwrap_or(DEFAULT_HOLD_MS))).await;
                self.hold_key_at(hwnd, key, k, false).await?;
            }
            other => return Err(format!("action must be down, up or tap, not {other:?}")),
        }
        Ok(json!({ "key": key, "action": action }))
    }

    /// How many keys the book holds (server tests read it without the field).
    #[cfg(test)]
    pub(crate) async fn held_count(&self) -> usize {
        self.holds.lock().await.all().len()
    }

    /// Let go of every key the DLL holds. The book is cleared whatever the
    /// call returned: a failed release is left to the DLL lease, and a stale
    /// book entry would only be renewed forever.
    pub(crate) async fn release_all_input(&self, reason: &str) -> Result<Value, String> {
        self.holds.lock().await.clear();
        let result = self.bridge_call("input_release", json!({})).await;
        // AX-27: button-ups
        match &result {
            Ok(_) => tracing::info!(target: "lab.input", reason, "lab.input.released"),
            Err(e) => {
                tracing::warn!(target: "lab.input", reason, error = %e, "lab.input.release_failed")
            }
        }
        result
    }
}

/// Keep held keys alive while the daemon lives. AX-27 adds the lease watch.
pub fn spawn_input_guards(sup: Arc<Supervisor>) {
    tokio::spawn(
        async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                sup.renew_holds().await;
            }
        }
        .in_current_span(),
    );
}

#[cfg(test)]
#[path = "input_release_tests.rs"]
mod tests;
