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
        // Posted buttons are window messages, not DLL holds: undo them here.
        let buttons = self.holds.lock().await.take_buttons();
        if !buttons.is_empty() {
            let at = self.cursor().await.unwrap_or((0, 0));
            for b in buttons {
                if let Err(e) = self.post_button(b, "up", at, None).await {
                    tracing::warn!(target: "lab.input", reason, button = b, error = %e,
                        "lab.input.button_release_failed");
                }
            }
        }
        match &result {
            Ok(_) => tracing::info!(target: "lab.input", reason, "lab.input.released"),
            Err(e) => {
                tracing::warn!(target: "lab.input", reason, error = %e, "lab.input.release_failed")
            }
        }
        result
    }
}

/// Whether the lease the guard last saw is over: gone, or replaced by another.
/// `None` to `Some` is a first acquire, not an end.
pub fn lease_ended(seen: Option<&str>, now: Option<&str>) -> bool {
    seen.is_some() && seen != now
}

/// Keep held keys alive while the daemon lives, and let go of every key and
/// button when the lease they were pressed under ends (release, expiry,
/// takeover). Expiry does not notify, so the 1 s tick is what sees it.
pub fn spawn_input_guards(sup: Arc<Supervisor>) {
    tokio::spawn(
        async move {
            let mut seen = sup.leases().current_id();
            let mut changes = sup.leases().subscribe();
            loop {
                tokio::select! {
                    _ = changes.changed() => {}
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {}
                }
                let now = sup.leases().current_id();
                if lease_ended(seen.as_deref(), now.as_deref()) {
                    let how = seen
                        .as_deref()
                        .and_then(|old| sup.leases().ended_how(old))
                        .unwrap_or("?");
                    let _ = sup.release_all_input("lease_end").await;
                    tracing::info!(target: "lab.input", how, "lab.input.released_on_lease_end");
                }
                // Release before the renew round trip, so a new holder's key
                // taken in that window is not cleared by the old lease's release.
                sup.renew_holds().await;
                seen = now;
            }
        }
        .in_current_span(),
    );
}

#[cfg(test)]
#[path = "input_release_tests.rs"]
mod tests;
