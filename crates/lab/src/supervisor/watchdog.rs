//! The background heartbeat watchdog and crash recovery for one client:
//! poll the bridge heartbeat, and on a hang or death terminate the process,
//! quarantine the in-flight command, and relaunch within the recovery cap
//! (ADR section 6). Split out of `mod.rs` to keep it under the file-size cap.

use super::heartbeat::{self, HeartbeatState, BUSY_GRACE_MS};
use super::{now_ms, process, LoginState, Supervisor, MAX_HEARTBEAT_FAILS, WATCHDOG_POLL};

impl Supervisor {
    /// Spawn the background heartbeat watchdog for `pid`. It exits once
    /// the current launch's pid changes (a restart), or after it handles
    /// this launch's death.
    pub(super) fn spawn_watchdog(&self, pid: u32) {
        let this = self.clone();
        tokio::spawn(async move { this.watchdog_loop(pid).await });
    }

    async fn watchdog_loop(&self, my_pid: u32) {
        let mut fails = 0u32;
        loop {
            tokio::time::sleep(WATCHDOG_POLL).await;

            // Stop if this launch has been superseded.
            {
                let st = self.state.lock().await;
                if st.pid != Some(my_pid) {
                    return;
                }
            }

            match self.bridge.heartbeat().await {
                Ok(count) => {
                    fails = 0;
                    let stale = {
                        let mut st = self.state.lock().await;
                        let ts = now_ms();
                        st.record_heartbeat(count, ts);
                        st.watchdog.observe(count, ts)
                    };
                    if stale == HeartbeatState::Stale {
                        tracing::warn!(pid = my_pid, "heartbeat stale; terminating hung client");
                        self.handle_death(my_pid).await;
                        return;
                    }
                }
                Err(_) => {
                    fails = heartbeat::next_fail_count(
                        fails,
                        self.bridge.ms_since_last_ok(),
                        BUSY_GRACE_MS,
                    );
                    if !process::is_alive(my_pid) || fails >= MAX_HEARTBEAT_FAILS {
                        tracing::warn!(pid = my_pid, fails, "client dead/unreachable");
                        self.handle_death(my_pid).await;
                        return;
                    }
                }
            }
        }
    }

    /// A death/hang was detected: terminate (in case it's hung),
    /// quarantine the in-flight command, record the crash, and — if
    /// under the recovery cap — relaunch and log back in.
    async fn handle_death(&self, dead_pid: u32) {
        let _ = tokio::task::spawn_blocking(move || process::terminate(dead_pid)).await;

        let may_relaunch = {
            let mut st = self.state.lock().await;
            st.journal.quarantine_in_flight();
            st.recovery.record_crash(now_ms());
            st.login = LoginState::Crashed;
            st.pid = None;
            st.recovery.should_relaunch(now_ms())
        };

        if !may_relaunch {
            tracing::error!("recovery cap reached (3 crashes / 10 min); not relaunching");
            return;
        }

        match self.launch_client(None).await {
            Ok(new_pid) => {
                tracing::info!(new_pid, "relaunched after crash; logging back in");
                self.spawn_watchdog(new_pid);
                self.relogin_after_crash().await;
                // Restore probes: re-apply persistent hooks (never writes
                // or native calls — the quarantined in-flight command stays
                // quarantined). ADR §6 "Restore probes".
                self.reapply_persistent_hooks().await;
            }
            Err(e) => tracing::error!(error = %e, "relaunch after crash failed"),
        }
    }
}
