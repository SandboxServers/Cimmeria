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
use crate::supervisor::{now_ms, LoginState, Supervisor};

/// Gap between keeper ticks.
pub const KEEPER_TICK: Duration = Duration::from_secs(1);
/// The Lua install/re-arm step of a tick that takes longer than this is
/// abandoned (a loading client). The drain and its ingest are never cut off:
/// once the DLL has handed events over they must reach the store.
const TICK_TIMEOUT: Duration = if cfg!(test) {
    Duration::from_millis(100)
} else {
    Duration::from_secs(3)
};
/// A heartbeat that has not advanced for this long means the client's main
/// thread is busy (a world load): the keeper leaves the bridge alone.
const STALLED_HEARTBEAT_MS: i64 = 3_000;
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
    #[cfg(test)]
    pub async fn keeper_tick(&self) -> Result<(), String> {
        self.keeper_drain().await?;
        self.keeper_rearm().await
    }

    /// Drain the bridge ring into the store. Callers must not cancel it
    /// (no timeout around it): events the DLL handed over would be lost.
    async fn keeper_drain(&self) -> Result<(), String> {
        // One drain at a time, so a reader pump cannot ingest a newer
        // batch ahead of this one. Never held across the Lua call.
        let _drain = self.events.drain.lock().await;
        let drained = self
            .bridge
            .call("events_read", json!({ "max": BRIDGE_DRAIN_MAX }))
            .await
            .map_err(|e| format!("bridge: {e}"))?;
        let mut st = self.events.inner.lock().await;
        ingest_bridge(&mut st, &drained);
        Ok(())
    }

    /// Install the Lua wrappers, reading no record.
    async fn keeper_rearm(&self) -> Result<(), String> {
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

    /// One keeper loop pass for `pid`: `false` when the launch is gone and
    /// the keeper should end. It ticks unless the client is known to be busy
    /// (D-LH15): logging in, at character select, crashed, not yet answering
    /// the watchdog's heartbeat, or with a heartbeat that has stopped
    /// advancing (a world load). A tick can hold the bridge mutex up to
    /// [`TICK_TIMEOUT`], and during a load that contends with the watchdog.
    /// Any other state ticks, so a client logged in by `ui_sequence`, in the
    /// world when the lab started, or after a `play_flow` timeout is kept too.
    pub(in crate::supervisor) async fn keeper_pass(&self, pid: u32) -> bool {
        let busy = {
            let st = self.state.lock().await;
            if st.pid != Some(pid) {
                return false;
            }
            let loading = matches!(
                st.login,
                LoginState::LoggingIn | LoginState::CharSelect | LoginState::Crashed
            );
            let no_heartbeat = st.heartbeats.is_empty();
            loading || no_heartbeat || st.watchdog.age_ms(now_ms()) > STALLED_HEARTBEAT_MS
        };
        if !busy {
            // A failed tick is ignored: the next one retries. The drain runs
            // to the end; only the Lua step has a time limit.
            if self.keeper_drain().await.is_ok() {
                let _ = tokio::time::timeout(TICK_TIMEOUT, self.keeper_rearm()).await;
            }
        }
        true
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
                    if !this.keeper_pass(pid).await {
                        return;
                    }
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

    /// Drops are told to the reader whose range holds them, once, whoever
    /// pumped the drain that saw them (a keeper tick, a recording sampler or
    /// a wait all call `pump_events`), and counted in the total.
    #[tokio::test]
    async fn drops_any_pumper_saw_reach_events_read_once() {
        let (r, _) = fake(vec![5, 3, 0]);
        let sup = fake_bridge::supervisor(r).await;
        sup.keeper_tick().await.unwrap();
        // A different pumper takes the next drain, with 3 more drops.
        let pumped = sup.pump_events(false).await.unwrap();
        assert_eq!(pumped.bridge_dropped_total, 8);
        let first = sup.events_read(None).await.unwrap();
        assert_eq!(first["dropped"], 8);
        assert_eq!(first["dropped_total"], 8);
        assert!(first["bridge_note"].as_str().unwrap().contains('8'));
        let second = sup.events_read(None).await.unwrap();
        assert_eq!(second["dropped"], 0);
        assert_eq!(second["dropped_total"], 8);
        assert!(second["bridge_note"].is_null());
    }

    /// Events the keeper drained stay visible to the reader's cursor, and
    /// the keeper's drops come back in the read's `dropped`, once.
    #[tokio::test]
    async fn keeper_drained_events_are_still_returned_to_a_reader() {
        let (r, _) = fake(vec![4, 0]);
        let sup = fake_bridge::supervisor(r).await;
        sup.keeper_tick().await.unwrap();
        let out = sup.events_read(None).await.unwrap();
        // One event from the tick, one from the reader's own drain.
        assert_eq!(out["returned"], 2);
        assert_eq!(out["dropped"], 4);
        let again = sup.events_read(None).await.unwrap();
        assert_eq!(again["dropped"], 0);
    }

    /// A heartbeat the watchdog saw just now.
    async fn seed_heartbeat(sup: &Supervisor) {
        let mut st = sup.state.lock().await;
        st.pid = Some(77);
        st.record_heartbeat(1, now_ms());
        st.watchdog.observe(1, now_ms());
    }

    fn recording_fake() -> (Responder, Arc<Mutex<Vec<String>>>) {
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let c2 = calls.clone();
        let r: Responder = Arc::new(move |method, params| {
            c2.lock().unwrap().push(method.to_string());
            match method {
                "events_read" => Ok(json!({ "returned": 0, "dropped": 0, "events": [] })),
                "lua_eval" if is_ring_pump(params) => Ok(empty_rings()),
                other => Err(format!("unexpected {other}")),
            }
        });
        (r, calls)
    }

    /// D-LH15: no bridge call while the client is logging in, at character
    /// select or crashed; every other state ticks (a client logged in by
    /// `ui_sequence`, already in world, or after a `play_flow` timeout).
    #[tokio::test]
    async fn the_keeper_skips_only_known_busy_states() {
        let (r, calls) = recording_fake();
        let sup = fake_bridge::supervisor(r).await;
        seed_heartbeat(&sup).await;
        for s in [
            LoginState::LoggingIn,
            LoginState::CharSelect,
            LoginState::Crashed,
        ] {
            sup.set_login_state(s).await;
            assert!(sup.keeper_pass(77).await, "the keeper lives while waiting");
            assert!(calls.lock().unwrap().is_empty(), "ticked in {s:?}");
        }
        for s in [
            LoginState::NotStarted,
            LoginState::Failed,
            LoginState::InWorld,
        ] {
            calls.lock().unwrap().clear();
            sup.set_login_state(s).await;
            assert!(sup.keeper_pass(77).await);
            assert_eq!(
                *calls.lock().unwrap(),
                ["events_read", "lua_eval"],
                "no tick in {s:?}"
            );
        }
        // A different pid ends the keeper without a call.
        calls.lock().unwrap().clear();
        assert!(!sup.keeper_pass(78).await);
        assert!(calls.lock().unwrap().is_empty());
    }

    /// D-LH15: before the watchdog has a heartbeat (boot) and while the
    /// heartbeat has stopped advancing (a world load) the keeper stays off
    /// the bridge, whatever the login state says.
    #[tokio::test]
    async fn the_keeper_stays_off_the_bridge_during_boot_and_load() {
        let (r, calls) = recording_fake();
        let sup = fake_bridge::supervisor(r).await;
        sup.state.lock().await.pid = Some(77);
        sup.set_login_state(LoginState::InWorld).await;
        assert!(sup.keeper_pass(77).await);
        assert!(calls.lock().unwrap().is_empty(), "ticked during boot");
        {
            let mut st = sup.state.lock().await;
            let old = now_ms() - 10_000;
            st.record_heartbeat(1, old);
            st.watchdog.observe(1, old);
        }
        assert!(sup.keeper_pass(77).await);
        assert!(
            calls.lock().unwrap().is_empty(),
            "ticked with a stalled heartbeat"
        );
    }

    /// The drain and its ingest are not cut off by the tick timeout: a slow
    /// `events_read` still lands its events in the store.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_slow_drain_is_not_cancelled_by_the_tick_timeout() {
        let r: Responder = Arc::new(move |method, params| match method {
            "events_read" => {
                std::thread::sleep(TICK_TIMEOUT * 4);
                Ok(json!({ "returned": 1, "dropped": 0, "events": [
                    { "kind": "cme.event", "ts_ms": 1, "fields": { "event": "e" } } ] }))
            }
            "lua_eval" if is_ring_pump(params) => Ok(empty_rings()),
            other => Err(format!("unexpected {other}")),
        });
        let sup = fake_bridge::supervisor(r).await;
        seed_heartbeat(&sup).await;
        sup.set_login_state(LoginState::InWorld).await;
        assert!(sup.keeper_pass(77).await);
        assert_eq!(sup.event_head().await, 1, "the drained event was lost");
    }

    /// `spawn_watchdog` starts the keeper: with the launch in the world and
    /// the heartbeat answering, the bridge ring gets drained without any
    /// tool call.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn spawn_watchdog_starts_the_keeper() {
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let ticks = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let (c2, t2) = (calls.clone(), ticks.clone());
        let r: Responder = Arc::new(move |method, params| {
            c2.lock().unwrap().push(method.to_string());
            match method {
                "heartbeat" => Ok(json!({
                    "tick_count": t2.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
                })),
                "events_read" => Ok(json!({ "returned": 0, "dropped": 0, "events": [] })),
                "lua_eval" if is_ring_pump(params) => Ok(empty_rings()),
                other => Err(format!("unexpected {other}")),
            }
        });
        let sup = fake_bridge::supervisor(r).await;
        // The test process stands in for the client, so the watchdog finds it alive.
        let pid = std::process::id();
        sup.state.lock().await.pid = Some(pid);
        sup.set_login_state(LoginState::InWorld).await;
        sup.spawn_watchdog(pid);
        let mut drained = false;
        for _ in 0..100 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if calls.lock().unwrap().iter().any(|m| m == "events_read") {
                drained = true;
                break;
            }
        }
        // End both loops at their next pass.
        sup.state.lock().await.pid = None;
        assert!(drained, "no keeper tick within 10 s of spawn_watchdog");
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
