//! The background heartbeat watchdog and crash recovery for one client:
//! poll the bridge heartbeat, and on a hang or death terminate the process,
//! quarantine the in-flight command, and relaunch within the recovery cap
//! (ADR section 6). Split out of `mod.rs` to keep it under the file-size cap.

use super::heartbeat::{self, HeartbeatState, BUSY_GRACE_MS};
use serde_json::Value;

use super::flows::{self, login::LoginRequest};
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
    /// under the recovery cap and someone holds the lab lease — relaunch
    /// and log back in.
    async fn handle_death(&self, dead_pid: u32) {
        if self.after_death(dead_pid).await != AfterDeath::Relaunch {
            return;
        }
        // Recovery acts only while a lease is held: re-checked before the
        // spawn, every relogin step and every hook re-apply, so a release
        // during the launch preparation stops it (lease::permit).
        let permit = crate::lease::permit::Permit::AnyHolder {
            book: self.leases().clone(),
        };
        crate::lease::permit::scope(permit, self.relaunch()).await;
    }

    async fn relaunch(&self) {
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

    /// Everything a death does short of the relaunch, and whether to
    /// relaunch. Split out so the lease gate is testable without launching.
    async fn after_death(&self, dead_pid: u32) -> AfterDeath {
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
            return AfterDeath::CapReached;
        }
        // Nobody holds the lab, so nobody is driving this client (or the
        // driver's lease ran out): a relaunch would only fight whoever
        // stopped it. Per-session stdio supervisors each relaunched a client
        // another session had closed.
        if !self.leases().is_held() {
            tracing::info!(target: "lab.lease", event = "watchdog_idle_no_lease",
                pid = dead_pid, "client died with no lab lease held; not relaunching");
            return AfterDeath::IdleNoLease;
        }
        AfterDeath::Relaunch
    }
}

/// Crash recovery steps, moved here from `mod.rs` beside the watchdog
/// that runs them.
impl Supervisor {
    /// Re-apply persistent hooks after a crash relaunch (ADR §6): replay
    /// each recorded `hook_install`, then re-key the set with the fresh ids
    /// the relaunched client assigns (the old ids died with the crash).
    /// Best-effort — a hook that fails to re-apply is dropped from tracking
    /// rather than retried forever. Writes and native calls are **never**
    /// replayed; only persistent hooks reach here.
    async fn reapply_persistent_hooks(&self) {
        let to_reapply = {
            let st = self.state.lock().await;
            st.persistent_hooks.to_reapply()
        };
        if to_reapply.is_empty() {
            return;
        }
        tracing::info!(
            count = to_reapply.len(),
            "re-applying persistent hooks after crash"
        );

        let mut refreshed: Vec<(u32, Value)> = Vec::new();
        for params in to_reapply {
            if let Err(e) = crate::lease::permit::ensure("hook re-apply") {
                tracing::info!(target: "lab.lease", reason = %e, "stopping hook re-apply");
                break;
            }
            match self.bridge.call("hook_install", params.clone()).await {
                Ok(res) => {
                    if let Some(hid) = res.get("id").and_then(Value::as_u64) {
                        refreshed.push((hid as u32, params));
                    }
                }
                Err(e) => tracing::warn!(error = %e, "persistent hook re-apply failed"),
            }
        }

        let mut st = self.state.lock().await;
        st.persistent_hooks.clear();
        for (hid, params) in refreshed {
            st.persistent_hooks.note_install(hid, &params);
        }
    }

    /// After a crash relaunch: log in with the native flow (Escape skips
    /// the intro movies) and play the lab account's character if it names
    /// one. Best effort; a failure is logged and leaves the client at
    /// whatever screen it reached.
    async fn relogin_after_crash(&self) {
        if let Err(e) = self.login_flow(LoginRequest::default()).await {
            tracing::warn!(error = %e.summary(), "post-crash login failed");
            return;
        }
        let character = self
            .lab_account()
            .map(|a| a.character)
            .filter(|c| !c.is_empty());
        if let Some(name) = character {
            if let Err(e) = self
                .play_flow(&name, true, flows::world::DEFAULT_PLAY_TIMEOUT)
                .await
            {
                tracing::warn!(error = %e.summary(), "post-crash play failed");
            }
        }
    }
}

/// What the watchdog does after a client death.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AfterDeath {
    Relaunch,
    /// Three crashes in ten minutes.
    CapReached,
    /// No lease held: leave the client down.
    IdleNoLease,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::lease::{AcquireRequest, LeaseBook};
    use crate::supervisor::events::fake_bridge;

    /// A pid no process has (Windows pids are multiples of four).
    const NO_SUCH_PID: u32 = 0xFFFF_FFF1;

    /// Regression guard: with no lease the watchdog leaves a dead client
    /// down; with one it relaunches; a released lease is no lease.
    #[tokio::test]
    async fn the_watchdog_relaunches_only_while_leased() {
        let book = Arc::new(LeaseBook::default());
        // A fresh supervisor per death, so the 3-in-10-minutes recovery cap
        // never decides the outcome.
        let fresh = || async {
            fake_bridge::supervisor(Arc::new(|_, _| Ok(serde_json::json!({}))))
                .await
                .with_leases(book.clone())
        };
        let died =
            |sup: crate::supervisor::Supervisor| async move { sup.after_death(NO_SUCH_PID).await };
        assert_eq!(died(fresh().await).await, AfterDeath::IdleNoLease);

        let l = book
            .acquire(AcquireRequest {
                owner: "test".into(),
                purpose: "watchdog".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(died(fresh().await).await, AfterDeath::Relaunch);

        book.release(&l.lease_id).unwrap();
        assert_eq!(died(fresh().await).await, AfterDeath::IdleNoLease);
    }
}
