//! Client process identity and `lab_client_status` assembly.

use serde_json::{json, Value};

use super::heartbeat::HeartbeatState;
use super::instance;
use super::process::{self, ProcessObservation};
use super::{now_ms, Supervisor, SupervisorConfig};

/// Restore only the exact process this instance registered at launch.
/// Legacy entries without creation identity remain readable to peers, but
/// cannot safely be adopted by a new supervisor.
pub(super) fn recover_registered_process(
    config: &SupervisorConfig,
    observe: impl FnOnce(u32) -> ProcessObservation,
) -> Option<(u32, u64)> {
    let install = config.install_dir.as_deref()?;
    let entry = instance::read_own_entry(install, config.instance.as_deref())?;
    if entry.bridge_port != config.port {
        return None;
    }
    let created_ticks = entry.process_created_ticks?;
    (observe(entry.pid) == ProcessObservation::Alive(Some(created_ticks)))
        .then_some((entry.pid, created_ticks))
}

fn process_uptime(
    expected_created_ticks: Option<u64>,
    observed: ProcessObservation,
    now: i64,
) -> (bool, i64, &'static str) {
    match observed {
        ProcessObservation::Exited => (false, 0, "exited"),
        ProcessObservation::Unavailable => (false, 0, "process_unavailable"),
        ProcessObservation::Alive(None) => (true, 0, "creation_time_unavailable"),
        ProcessObservation::Alive(Some(_)) if expected_created_ticks.is_none() => {
            (true, 0, "identity_unavailable")
        }
        ProcessObservation::Alive(Some(created_ticks))
            if expected_created_ticks != Some(created_ticks) =>
        {
            (false, 0, "pid_reused")
        }
        ProcessObservation::Alive(Some(created_ticks)) => {
            match process::unix_ms_from_creation_ticks(created_ticks) {
                Some(created_ms) => (true, now.saturating_sub(created_ms).max(0), "available"),
                None => (true, 0, "creation_time_unavailable"),
            }
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
        let (pid, expected_created_ticks, login, crash_count, persistent_hooks) = {
            let mut st = self.state.lock().await;
            (
                st.pid,
                st.process_created_ticks,
                st.login,
                st.recovery.recent_crash_count(now),
                st.persistent_hooks.count(),
            )
        };
        let (running, uptime_ms, uptime_state) = match pid {
            Some(pid) => process_uptime(expected_created_ticks, observe(pid), now),
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

    fn ticks(unix_ms: u64) -> u64 {
        116_444_736_000_000_000 + unix_ms * 10_000
    }

    async fn supervisor() -> Supervisor {
        let responder: Responder = Arc::new(|method, _| match method {
            "heartbeat" => Ok(json!({ "tick_count": 1 })),
            other => Err(format!("unexpected {other}")),
        });
        fake_bridge::supervisor(responder).await
    }

    async fn record(sup: &Supervisor, pid: u32, created_ticks: Option<u64>) {
        let mut st = sup.state.lock().await;
        st.pid = Some(pid);
        st.process_created_ticks = created_ticks;
    }

    #[tokio::test]
    async fn status_survives_supervisor_recreation_and_tracks_new_pid() {
        let fake = supervisor().await;
        let dir = tempfile::tempdir().unwrap();
        let mut config = fake.config.clone();
        config.install_dir = Some(dir.path().to_owned());
        instance::write_entry(dir.path(), None, 41, config.port, Some(ticks(10_000))).unwrap();

        // A newly constructed supervisor reads the first launch's on-disk
        // identity instead of inheriting its in-memory start timestamp.
        let recreated = Supervisor::new_with_process(fake.bridge.clone(), config.clone(), |pid| {
            assert_eq!(pid, 41);
            ProcessObservation::Alive(Some(ticks(10_000)))
        });
        assert_eq!(recreated.client_pid().await, Some(41));
        let continued = recreated
            .status_with_process(55_000, |_| ProcessObservation::Alive(Some(ticks(10_000))))
            .await
            .unwrap();
        assert_eq!(continued["uptime_ms"], 45_000);
        assert_eq!(continued["uptime_state"], "available");

        instance::write_entry(dir.path(), None, 42, config.port, Some(ticks(48_000))).unwrap();
        let new_supervisor = Supervisor::new_with_process(fake.bridge.clone(), config, |pid| {
            assert_eq!(pid, 42);
            ProcessObservation::Alive(Some(ticks(48_000)))
        });
        let new_client = new_supervisor
            .status_with_process(55_000, |pid| {
                assert_eq!(pid, 42);
                ProcessObservation::Alive(Some(ticks(48_000)))
            })
            .await
            .unwrap();
        assert_eq!(new_client["uptime_ms"], 7_000);
    }

    #[tokio::test]
    async fn startup_refuses_reused_pid_and_legacy_unverified_registry() {
        let fake = supervisor().await;
        let dir = tempfile::tempdir().unwrap();
        let mut config = fake.config.clone();
        config.install_dir = Some(dir.path().to_owned());
        instance::write_entry(dir.path(), None, 41, config.port, Some(ticks(10_000))).unwrap();
        let reused = Supervisor::new_with_process(fake.bridge.clone(), config.clone(), |_| {
            ProcessObservation::Alive(Some(ticks(12_000)))
        });
        assert_eq!(reused.client_pid().await, None);
        let status = reused
            .status_with_process(50_000, |_| panic!("no PID"))
            .await
            .unwrap();
        assert_eq!(status["uptime_state"], "not_running");

        // A second process created within the same millisecond is still a
        // distinct Windows process; the full FILETIME rejects it.
        let same_ms_reuse =
            Supervisor::new_with_process(fake.bridge.clone(), config.clone(), |_| {
                ProcessObservation::Alive(Some(ticks(10_000) + 1))
            });
        assert_eq!(same_ms_reuse.client_pid().await, None);

        instance::write_entry(dir.path(), None, 41, config.port + 1, Some(ticks(10_000))).unwrap();
        let wrong_port = Supervisor::new_with_process(fake.bridge.clone(), config.clone(), |_| {
            panic!("wrong bridge port must not observe the PID")
        });
        assert_eq!(wrong_port.client_pid().await, None);

        instance::write_entry(dir.path(), None, 41, config.port, None).unwrap();
        let legacy = Supervisor::new_with_process(fake.bridge.clone(), config, |_| {
            panic!("legacy entry has no identity")
        });
        assert_eq!(legacy.client_pid().await, None);
    }

    #[tokio::test]
    async fn status_rejects_exited_reused_and_unreadable_pid() {
        let sup = supervisor().await;
        record(&sup, 41, Some(ticks(10_000))).await;
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
            (
                ProcessObservation::Alive(Some(ticks(12_000))),
                "pid_reused",
                false,
            ),
        ] {
            let status = sup.status_with_process(50_000, |_| observed).await.unwrap();
            assert_eq!(status["running"], running, "{state}");
            assert_eq!(status["uptime_ms"], 0, "{state}");
            assert_eq!(status["uptime_state"], state);
        }
        record(&sup, 41, None).await;
        let unknown = sup
            .status_with_process(50_000, |_| ProcessObservation::Alive(Some(ticks(10_000))))
            .await
            .unwrap();
        assert_eq!(unknown["uptime_state"], "identity_unavailable");
        assert_eq!(unknown["running"], true);
    }
}
