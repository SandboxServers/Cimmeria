//! The daemon's copy of the DLL watch (C6, C15): `client_watch` pulls the
//! per-frame samples with `watch_read` and keeps them here for the C12
//! detectors. Nothing else reads the store.

use std::collections::VecDeque;

use serde_json::{json, Value};

use super::{now_ms, Supervisor};

/// One per-frame sample from `watch_read` (C6). `wall_ms` is set by the
/// store, never by the DLL.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct WatchSample {
    pub i: u64,
    pub f: u64,
    pub ms: u64,
    /// Daemon wall clock of the sample, mapped from `ms` by [`WatchStore::ingest`].
    #[serde(default)]
    pub wall_ms: i64,
    #[serde(default)]
    pub t: Option<f64>,
    #[serde(default)]
    pub pos: Option<[f32; 3]>,
    #[serde(default)]
    pub rot: Option<[i32; 3]>,
    #[serde(default)]
    pub hover: Option<u32>,
    #[serde(default)]
    pub mode: Option<u32>,
    #[serde(default)]
    pub buttons: Option<u32>,
    #[serde(default)]
    pub actors: Option<u32>,
}

/// Most samples the store keeps; the oldest go first.
pub const STORE_CAP: usize = 4096;

/// The samples of one instance's watch, oldest first.
#[derive(Debug, Default)]
pub struct WatchStore {
    samples: VecDeque<WatchSample>,
    /// Whether the DLL reported its watch running at the last read.
    pub running: bool,
    /// The highest sample index kept so far.
    last_i: Option<u64>,
}

impl WatchStore {
    /// Keep the samples of one `watch_read`. `read_wall_ms` is the daemon wall
    /// clock when the response arrived; `meta_mono_ms` its `meta.mono_ms`.
    /// Returns how many samples were new.
    pub fn ingest(
        &mut self,
        samples: Vec<WatchSample>,
        read_wall_ms: i64,
        meta_mono_ms: Option<u64>,
    ) -> usize {
        let mut kept = 0;
        for mut s in samples {
            if self.last_i.is_some_and(|last| s.i <= last) {
                continue;
            }
            s.wall_ms = match meta_mono_ms {
                Some(mono) => read_wall_ms - (mono as i64 - s.ms as i64),
                None => read_wall_ms,
            };
            self.last_i = Some(s.i);
            self.samples.push_back(s);
            if self.samples.len() > STORE_CAP {
                self.samples.pop_front();
            }
            kept += 1;
        }
        kept
    }

    /// The samples with `wall_ms >= wall_ms`, oldest first.
    // AX-51 (the detectors) is the first caller; until it lands nothing reads this.
    #[allow(dead_code)]
    pub fn since(&self, wall_ms: i64) -> Vec<WatchSample> {
        self.samples
            .iter()
            .filter(|s| s.wall_ms >= wall_ms)
            .cloned()
            .collect()
    }

    /// Forget every sample: a new watch starts its indexes again.
    pub fn clear(&mut self) {
        self.samples.clear();
        self.last_i = None;
    }
}

impl Supervisor {
    /// `client_watch`: `start`, `read` or `stop` the DLL watch. `params` are
    /// the set fields of the tool's arguments, forwarded to the DLL method.
    pub async fn watch(&self, action: &str, params: Value) -> Result<Value, String> {
        let out = match action {
            "start" => self.watch_start(params).await,
            "read" => self.watch_read(params).await,
            "stop" => self.watch_stop().await,
            _ => return Err("action must be start, read or stop".into()),
        };
        out.map_err(|e| {
            if e.contains("-32601") {
                "watch needs a lab DLL with watch_start (AX-24)".into()
            } else {
                e
            }
        })
    }

    async fn watch_start(&self, params: Value) -> Result<Value, String> {
        let v = self.bridge_call("watch_start", params).await?;
        let mut store = self.watch.lock().await;
        store.clear();
        store.running = true;
        Ok(v)
    }

    async fn watch_stop(&self) -> Result<Value, String> {
        let v = self.bridge_call("watch_stop", json!({})).await?;
        self.watch.lock().await.running = false;
        Ok(v)
    }

    /// `watch_read`: a guarded, journaled call that also stamps the samples
    /// with the response's C4 `meta`. The result comes back with `pulled`.
    async fn watch_read(&self, params: Value) -> Result<Value, String> {
        crate::lease::permit::ensure("bridge watch_read")?;
        let seq = {
            let mut st = self.state.lock().await;
            st.journal.record("watch_read", now_ms())
        };
        let out = self
            .bridge
            .call_meta("watch_read", params)
            .await
            .map_err(|e| format!("bridge: {e}"));
        self.state.lock().await.journal.complete(seq, out.is_ok());
        let (mut result, meta) = out?;
        let samples: Vec<WatchSample> = serde_json::from_value(result["samples"].clone())
            .map_err(|e| format!("watch_read samples: {e}"))?;
        let mut store = self.watch.lock().await;
        let kept = store.ingest(samples, now_ms(), meta.map(|m| m.mono_ms));
        if let Some(running) = result.get("running").and_then(Value::as_bool) {
            store.running = running;
        }
        drop(store);
        result["pulled"] = json!(kept);
        Ok(result)
    }

    /// Whether the watch runs, and the stored samples at or after `wall_ms`
    /// (the C12 detectors read these).
    // AX-51 (the detectors) is the first caller; until it lands nothing reads this.
    #[allow(dead_code)]
    pub async fn watch_samples_since(&self, wall_ms: i64) -> (bool, Vec<WatchSample>) {
        let store = self.watch.lock().await;
        (store.running, store.since(wall_ms))
    }
}

#[cfg(test)]
#[path = "watch_store_tests.rs"]
mod tests;
