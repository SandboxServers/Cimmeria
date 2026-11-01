//! World-tool key holds through the lab DLL (contract C5, daemon half). A
//! held key is an `input_hold` with a lease the DLL enforces, so a daemon
//! that dies mid-walk cannot leave a movement key stuck down. The walker
//! renews the lease between sleep slices ([`Supervisor::renew_holds`]).

use std::sync::OnceLock;
use std::time::Instant;

use serde_json::{json, Value};

use super::hold_book::LEASE_MS;
use super::{keys, Supervisor};

/// Monotonic milliseconds for the hold book (never the wall clock: a clock
/// step must not postpone a renewal).
fn mono_ms() -> u64 {
    static T0: OnceLock<Instant> = OnceLock::new();
    T0.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn hold_params(hwnd: isize, k: keys::Key, down: bool) -> Value {
    json!({
        "hwnd": hwnd,
        "vk": k.vk,
        "lparam_down": k.lparam_down() as u32,
        "lparam_up": k.lparam_up() as u32,
        "down": down,
        "lease_ms": LEASE_MS,
    })
}

impl Supervisor {
    /// Press or release a key the way a walking tool needs: held through
    /// the DLL with a lease, or, against a DLL without `input_hold`, as the
    /// window messages the lab always posted.
    pub(crate) async fn hold_key(&self, key: &str, down: bool) -> Result<(), String> {
        let k = keys::named(key).ok_or_else(|| format!("unknown key {key:?}"))?;
        let hwnd = match self.game_hwnd().await {
            Ok(h) => h,
            Err(e) => {
                if !down {
                    self.holds.lock().await.released(k.vk);
                }
                return Err(e);
            }
        };
        self.hold_key_at(hwnd, key, k, down).await
    }

    /// [`Self::hold_key`] with the window already resolved (the test seam).
    pub(super) async fn hold_key_at(
        &self,
        hwnd: isize,
        name: &str,
        k: keys::Key,
        down: bool,
    ) -> Result<(), String> {
        if self.holds.lock().await.is_fallback() {
            return self.post_key(hwnd, k, down).await;
        }
        match self.send_hold(hwnd, k, down).await {
            Ok(_) => {
                let mut book = self.holds.lock().await;
                if down {
                    book.pressed(k.vk, name, mono_ms());
                } else {
                    book.released(k.vk);
                }
                Ok(())
            }
            // An older DLL has no `input_hold`: remember that, post instead.
            Err(e) if e.contains("-32601") => {
                tracing::debug!(target: "lab.hold", "input_hold unsupported; posting key messages");
                self.holds.lock().await.set_fallback();
                self.post_key(hwnd, k, down).await
            }
            Err(e) => {
                if !down {
                    self.holds.lock().await.released(k.vk);
                }
                Err(e)
            }
        }
    }

    /// A release must run even when the tool's lease was revoked mid-walk,
    /// so only a press is lease-guarded.
    async fn send_hold(&self, hwnd: isize, k: keys::Key, down: bool) -> Result<Value, String> {
        let params = hold_params(hwnd, k, down);
        if down {
            self.bridge_call("input_hold", params).await
        } else {
            self.bridge_release_call("input_hold", params).await
        }
    }

    /// Renew every hold whose lease is half spent. Returns how many the DLL
    /// had to press again. Failures are not fatal: the DLL lease releases
    /// the key on its own.
    pub(crate) async fn renew_holds(&self) -> usize {
        if self.holds.lock().await.due(mono_ms()).is_empty() {
            return 0;
        }
        match self.game_hwnd().await {
            Ok(hwnd) => self.renew_holds_at(hwnd).await,
            Err(e) => {
                tracing::debug!(target: "lab.hold", error = %e, "hold renewal skipped");
                0
            }
        }
    }

    pub(super) async fn renew_holds_at(&self, hwnd: isize) -> usize {
        let due = {
            let book = self.holds.lock().await;
            book.due(mono_ms())
                .into_iter()
                .filter_map(|vk| Some((vk, keys::named(&book.name_of(vk)?)?)))
                .collect::<Vec<_>>()
        };
        let mut reasserted = 0;
        for (vk, k) in due {
            match self.send_hold(hwnd, k, true).await {
                Ok(v) => {
                    let posted = v.get("posted").and_then(Value::as_bool).unwrap_or(false);
                    if self.holds.lock().await.renewed(vk, mono_ms(), posted) {
                        reasserted += 1;
                        tracing::warn!(target: "lab.hold", vk, "lab.hold.reasserted");
                    }
                }
                Err(e) => {
                    tracing::debug!(target: "lab.hold", vk, error = %e, "hold renewal failed")
                }
            }
        }
        reasserted
    }

    /// Let go of every key this book holds (a finished or cancelled tool).
    pub(crate) async fn release_all_holds(&self) {
        if self.holds.lock().await.is_empty() {
            return;
        }
        match self.game_hwnd().await {
            Ok(hwnd) => self.release_all_holds_at(hwnd).await,
            Err(e) => {
                // No window to name: the DLL lease releases them.
                tracing::debug!(target: "lab.hold", error = %e, "hold release left to the lease");
                self.holds.lock().await.clear();
            }
        }
    }

    pub(super) async fn release_all_holds_at(&self, hwnd: isize) {
        let all = self.holds.lock().await.all();
        for (vk, name) in all {
            let Some(k) = keys::named(&name) else {
                continue;
            };
            if let Err(e) = self.send_hold(hwnd, k, false).await {
                tracing::debug!(target: "lab.hold", vk, error = %e, "hold release failed");
            }
        }
        self.holds.lock().await.clear();
    }
}

#[cfg(test)]
#[path = "held_keys_tests.rs"]
mod tests;
