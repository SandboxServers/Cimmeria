//! Client process identity and `lab_client_status` assembly.

use serde_json::{json, Value};

use super::heartbeat::HeartbeatState;
use super::process::{self, ProcessObservation};
use super::{now_ms, Supervisor};

fn process_uptime(
    expected_created_ms: Option<i64>,
    observed: ProcessObservation,
    now: i64,
) -> (bool, i64, &'static str) {
    match observed {
        ProcessObservation::Exited => (false, 0, "exited"),
        ProcessObservation::Unavailable => (false, 0, "process_unavailable"),
        ProcessObservation::Alive(None) => (true, 0, "creation_time_unavailable"),
        ProcessObservation::Alive(Some(_)) if expected_created_ms.is_none() => {
            (true, 0, "identity_unavailable")
        }
        ProcessObservation::Alive(Some(created_ms)) if expected_created_ms != Some(created_ms) => {
            (false, 0, "pid_reused")
        }
        ProcessObservation::Alive(Some(created_ms)) => {
            (true, now.saturating_sub(created_ms).max(0), "available")
        }
    }
}

impl Supervisor {
    /// `lab_client_status` — PID, process uptime, heartbeat age, login state,
    /// and crash count. Uptime is reported only for the launched process.
    pub async fn status(&self) -> Result<Value, String> {
        self.status_with_process(now_ms(), process::observe).await
    }

    // Injected clock/process source lets tests exercise the actual status
    // response, including state snapshots and supervisor recreation.
    async fn status_with_process(
        &self,
        now: i64,
        observe: impl FnOnce(u32) -> ProcessObservation,
    ) -> Result<Value, String> {
        let (pid, expected_created_ms, login, crash_count, persistent_hooks) = {
            let mut st = self.state.lock().await;
            (
                st.pid,
                st.process_created_ms,
                st.login,
                st.recovery.recent_crash_count(now),
                st.persistent_hooks.count(),
            )
        };
        let (running, uptime_ms, uptime_state) = match pid {
            Some(pid) => process_uptime(expected_created_ms, observe(pid), now),
            None => (false, 0, "not_running"),
        };

        // Heartbeat poll happens without the state lock held.
        let hb = self.bridge.heartbeat().await;
        let (hb_count, hb_state, hb_age): (Option<u64>, &str, i64) = match hb {
            Ok(count) => {
                let mut st = self.state.lock().await;
                let ts = now_ms();
                st.record_heartbeat(count, ts);
                let s = st.watchdog.observe(count, ts);
                let age = st.watchdog.age_ms(ts);
                (
                    Some(count),
                    if s == HeartbeatState::Stale {
                        "stale"
                    } else {
                        "alive"
                    },
                    age,
                )
            }
            Err(_) => (None, "unreachable", 0),
        };

        Ok(json!({
            "pid": pid,
            "running": running,
            "uptime_ms": uptime_ms,
            "uptime_state": uptime_state,
            "login_state": login.as_str(),
            "crash_count_10min": crash_count,
            "persistent_hooks": persistent_hooks,
            "heartbeat": { "tick_count": hb_count, "state": hb_state, "age_ms": hb_age },
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::supervisor::events::fake_bridge::{self, Responder};

    async fn supervisor() -> Supervisor {
        let responder: Responder = Arc::new(|method, _| match method {
            "heartbeat" => Ok(json!({ "tick_count": 1 })),
            other => Err(format!("unexpected {other}")),
        });
        fake_bridge::supervisor(responder).await
    }

    async fn record(sup: &Supervisor, pid: u32, created_ms: Option<i64>) {
        let mut st = sup.state.lock().await;
        st.pid = Some(pid);
        st.process_created_ms = created_ms;
    }

    #[tokio::test]
    async fn status_survives_supervisor_recreation_and_tracks_new_pid() {
        let first = supervisor().await;
        record(&first, 41, Some(10_000)).await;
        let first_status = first
            .status_with_process(50_000, |pid| {
                assert_eq!(pid, 41);
                ProcessObservation::Alive(Some(10_000))
            })
            .await
            .unwrap();
        assert_eq!(first_status["uptime_ms"], 40_000);

        let recreated = supervisor().await;
        record(&recreated, 41, Some(10_000)).await;
        let continued = recreated
            .status_with_process(55_000, |_| ProcessObservation::Alive(Some(10_000)))
            .await
            .unwrap();
        assert_eq!(continued["uptime_ms"], 45_000);
        assert_eq!(continued["uptime_state"], "available");

        record(&recreated, 42, Some(48_000)).await;
        let new_client = recreated
            .status_with_process(55_000, |pid| {
                assert_eq!(pid, 42);
                ProcessObservation::Alive(Some(48_000))
            })
            .await
            .unwrap();
        assert_eq!(new_client["uptime_ms"], 7_000);
    }

    #[tokio::test]
    async fn status_rejects_exited_reused_and_unreadable_pid() {
        let sup = supervisor().await;
        record(&sup, 41, Some(10_000)).await;
        for (observed, state, running) in [
            (ProcessObservation::Exited, "exited", false),
            (
                ProcessObservation::Unavailable,
                "process_unavailable",
                false,
            ),
            (
                ProcessObservation::Alive(None),
                "creation_time_unavailable",
                true,
            ),
            (ProcessObservation::Alive(Some(12_000)), "pid_reused", false),
        ] {
            let status = sup.status_with_process(50_000, |_| observed).await.unwrap();
            assert_eq!(status["running"], running, "{state}");
            assert_eq!(status["uptime_ms"], 0, "{state}");
            assert_eq!(status["uptime_state"], state);
        }
        record(&sup, 41, None).await;
        let unknown = sup
            .status_with_process(50_000, |_| ProcessObservation::Alive(Some(10_000)))
            .await
            .unwrap();
        assert_eq!(unknown["uptime_state"], "identity_unavailable");
        assert_eq!(unknown["running"], true);
    }
}
