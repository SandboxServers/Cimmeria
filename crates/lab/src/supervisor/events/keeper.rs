//! The event keeper: between tool calls, drain the client's event ring and
//! keep the lab's Lua wrappers installed.
//!
//! Two losses came from nobody being there (#1660). The DLL's event ring
//! (4096, drops the newest) overflowed whenever no tool pumped for a while,
//! and the chat wrapper was only installed by a reader, so the first chat
//! line after a relog arrived before the wrapper did. The keeper ticks once a
//! second for as long as its launch lives. It reads the bridge directly (not
//! through `bridge_call`): it needs no lease and must not fill the crash
//! journal. The Lua install chunk runs with `max = 0`: it installs, and reads
//! no record, so the in-client rings keep their records for a reader pump and
//! a chat line cannot be evicted from the store while it waits.
//! `CIMMERIA_LAB_EVENT_KEEPER=0` turns it off.

use std::time::Duration;

use serde_json::json;
use tracing::Instrument;

use super::{ingest_bridge, lua_rings, BRIDGE_DRAIN_MAX};
use crate::supervisor::Supervisor;

/// Gap between keeper ticks.
pub const KEEPER_TICK: Duration = Duration::from_secs(1);
/// A tick that takes longer than this is abandoned (a loading client).
const TICK_TIMEOUT: Duration = Duration::from_secs(3);
/// Environment switch; `0`, `off` or `false` disables the keeper.
pub const KEEPER_ENV: &str = "CIMMERIA_LAB_EVENT_KEEPER";

/// Whether the keeper runs, given the environment value.
pub fn enabled_from(v: Option<&str>) -> bool {
    !matches!(
        v.map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("0" | "off" | "false")
    )
}

impl Supervisor {
    /// One keeper tick: drain the bridge ring into the store and make sure
    /// the Lua wrappers are installed, reading no ring record.
    pub async fn keeper_tick(&self) -> Result<(), String> {
        let drained = self
            .bridge
            .call("events_read", json!({ "max": BRIDGE_DRAIN_MAX }))
            .await
            .map_err(|e| format!("bridge: {e}"))?;
        {
            let mut st = self.events.inner.lock().await;
            ingest_bridge(&mut st, &drained);
        }
        // max = 0: install the wrappers, return no records.
        let chunk = lua_rings::pump_chunk(0, 0, 0, 0);
        let v = self
            .bridge
            .call("lua_eval", json!({ "chunk": chunk }))
            .await
            .map_err(|e| format!("bridge: {e}"))?;
        let _ = v;
        Ok(())
    }

    /// Spawn the keeper for `pid`; it ends when the launch's pid changes.
    pub(in crate::supervisor) fn spawn_keeper(&self, pid: u32) {
        if !enabled_from(std::env::var(KEEPER_ENV).ok().as_deref()) {
            return;
        }
        let this = self.clone();
        let span = tracing::info_span!("lab_event_keeper", instance = self.label(), pid);
        tokio::spawn(
            async move {
                loop {
                    tokio::time::sleep(KEEPER_TICK).await;
                    if this.state.lock().await.pid != Some(pid) {
                        return;
                    }
                    // A failed or slow tick is ignored: the next one retries.
                    let _ = tokio::time::timeout(TICK_TIMEOUT, this.keeper_tick()).await;
                }
            }
            .instrument(span),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::supervisor::events::fake_bridge::{self, empty_rings, is_ring_pump, Responder};

    /// A fake whose events_read reports these drop counts in order (then 0),
    /// with one event per read, and records every ring chunk it is sent.
    fn fake(drops: Vec<u64>) -> (Responder, Arc<Mutex<Vec<String>>>) {
        let drops = Arc::new(Mutex::new(VecDeque::from(drops)));
        let chunks = Arc::new(Mutex::new(Vec::new()));
        let c2 = chunks.clone();
        let r: Responder = Arc::new(move |method, params| match method {
            "events_read" => {
                let d = drops.lock().unwrap().pop_front().unwrap_or(0);
                Ok(json!({ "returned": 1, "dropped": d, "events": [
                    { "kind": "cme.event", "ts_ms": 1, "fields": { "event": "e" } } ] }))
            }
            "lua_eval" if is_ring_pump(params) => {
                c2.lock()
                    .unwrap()
                    .push(params["chunk"].as_str().unwrap_or_default().to_string());
                Ok(empty_rings())
            }
            other => Err(format!("unexpected {other}")),
        });
        (r, chunks)
    }

    #[tokio::test]
    async fn a_tick_drains_the_bridge_and_installs_without_reading_records() {
        let (r, chunks) = fake(vec![0]);
        let sup = fake_bridge::supervisor(r).await;
        sup.keeper_tick().await.unwrap();
        assert_eq!(
            sup.event_head().await,
            1,
            "the drained event is in the store"
        );
        let c = chunks.lock().unwrap();
        assert_eq!(c.len(), 1);
        // max = 0: the chunk installs the wrappers and returns no record.
        assert!(c[0].contains("e.seq > 0 and n < 0"), "{}", c[0]);
        assert!(c[0].contains("e.seq > 0 and m < 0"), "{}", c[0]);
    }

    /// Drops the keeper saw are told to the next reader, once, and counted.
    #[tokio::test]
    async fn drops_a_tick_saw_reach_the_next_reader_once() {
        let (r, _) = fake(vec![5, 0, 0]);
        let sup = fake_bridge::supervisor(r).await;
        sup.keeper_tick().await.unwrap();
        let first = sup.pump_events(false).await.unwrap();
        assert_eq!(first.bridge_dropped, 5);
        assert_eq!(first.bridge_dropped_total, 5);
        assert!(first.to_json()["bridge_note"]
            .as_str()
            .unwrap()
            .contains('5'));
        let second = sup.pump_events(false).await.unwrap();
        assert_eq!(second.bridge_dropped, 0);
        assert_eq!(second.bridge_dropped_total, 5);
        assert!(second.to_json()["bridge_note"].is_null());
    }

    #[test]
    fn the_env_switch_reads_off_values() {
        assert!(enabled_from(None));
        assert!(enabled_from(Some("1")));
        assert!(!enabled_from(Some("0")));
        assert!(!enabled_from(Some(" Off ")));
        assert!(!enabled_from(Some("false")));
    }
}
