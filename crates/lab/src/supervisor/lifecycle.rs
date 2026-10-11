//! The client's lifecycle: `lab_client_start`, `_stop` and `_restart`.
//! Split out of `mod.rs` to keep it under the file-size cap.

use std::time::Duration;

use serde_json::{json, Value};

use super::{instance, process, LoginState, Supervisor};

/// How long `lab_client_stop` waits for the client to be gone.
const STOP_WAIT: Duration = Duration::from_secs(10);

/// Warn when a launch replaces a pid that is still recorded and alive.
/// `start` refuses that case, so reaching it means the record went stale
/// under the state lock (#1342). A dead or absent previous pid is silent.
pub(super) fn warn_if_pid_overwritten(previous: Option<u32>, pid: u32) {
    let Some(previous_pid) = previous else {
        return;
    };
    let previous_alive = process::is_alive(previous_pid);
    if previous_alive {
        tracing::warn!(
            target: "lab.client",
            event = "pid_overwritten",
            previous_pid,
            previous_alive,
            pid,
            "a launch replaced a live client's pid"
        );
    }
}

impl Supervisor {
    /// `lab_client_start` — launch suspended, inject the lab DLL, resume.
    pub async fn start(&self, server_override: Option<String>) -> Result<Value, String> {
        self.start_as(server_override, "start").await
    }

    /// [`Self::start`] with the launch's `cause` for the `launched` row.
    pub(super) async fn start_as(
        &self,
        server_override: Option<String>,
        cause: &'static str,
    ) -> Result<Value, String> {
        {
            let st = self.state.lock().await;
            if let Some(pid) = st.pid {
                if process::is_alive(pid) {
                    return Err(format!(
                        "a client is already running (pid {pid}); stop it first"
                    ));
                }
            }
        }
        // A client the lab did not start (the launcher, a player) is not
        // the lab's to stop or to share a machine with. Another lab
        // instance's client is fine, up to the cap ([`instance::check_launch`]).
        let running = process::running_sgw_pids();
        let peers: Vec<_> = self
            .config
            .install_dir
            .as_deref()
            .map(|d| instance::read_peers(d, self.config.instance.as_deref()))
            .unwrap_or_default()
            .into_iter()
            .filter(|p| process::is_alive(p.pid))
            .collect();
        instance::check_launch(
            &running,
            &peers,
            self.config.port,
            instance::max_clients_from_env(instance::hosted_count_from_env()),
        )?;
        let pid = self.launch_client(server_override, cause).await?;
        self.spawn_watchdog(pid);
        let telemetry = self.state.lock().await.telemetry.clone();
        Ok(
            json!({ "pid": pid, "bridge_port": self.config.port, "started": true,
                   "telemetry": telemetry }),
        )
    }

    /// `lab_client_stop` — terminate the client and wait (up to
    /// [`STOP_WAIT`]) until it is gone. `exited: false` means it was still
    /// there when the wait ran out, and the next start will refuse it.
    pub async fn stop(&self) -> Result<Value, String> {
        self.stop_as("stop").await
    }

    /// [`Self::stop`] with the stop's `cause` for the `stopped` row.
    pub(super) async fn stop_as(&self, cause: &'static str) -> Result<Value, String> {
        let pid = {
            let mut st = self.state.lock().await;
            let pid = st.pid.take();
            st.login = LoginState::NotStarted;
            pid
        };
        match pid {
            Some(pid) => {
                // Wait for the exit, not just the request: a start right
                // after would otherwise refuse the dying client. The exit
                // code is set at once, but the start guard enumerates
                // SGW.exe windows, and a terminated client's window outlives
                // it (2026-10-10: `exited` after 1 ms, then the next start
                // refused the same pid), so wait for both to go.
                let exited = match tokio::task::spawn_blocking(move || {
                    process::terminate(pid);
                    process::wait_for_exit(
                        || {
                            process::still_present(
                                pid,
                                process::is_alive(pid),
                                &process::running_sgw_pids(),
                            )
                        },
                        STOP_WAIT,
                        Duration::from_millis(100),
                    )
                })
                .await
                {
                    Ok(exited) => exited,
                    Err(e) => {
                        tracing::warn!(pid, error = %e, "lab_client_stop: the exit wait failed");
                        false
                    }
                };
                if !exited {
                    tracing::warn!(
                        pid,
                        wait_s = STOP_WAIT.as_secs(),
                        "lab_client_stop: the client is still present after the wait; \
                         the next start will refuse it"
                    );
                }
                if let Some(d) = self.config.install_dir.as_deref() {
                    instance::remove_entry(d, self.config.instance.as_deref());
                }
                tracing::info!(
                    target: "lab.client",
                    event = "stopped",
                    pid,
                    exited,
                    cause,
                    "client stopped"
                );
                Ok(json!({ "stopped": true, "pid": pid, "exited": exited }))
            }
            None => Ok(json!({ "stopped": false, "reason": "no client running" })),
        }
    }

    /// `lab_client_restart` — stop then start.
    pub async fn restart(&self, server_override: Option<String>) -> Result<Value, String> {
        let _ = self.stop_as("restart").await;
        // Brief settle so the OS releases the port + the old process.
        tokio::time::sleep(Duration::from_millis(500)).await;
        self.start_as(server_override, "restart").await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing::Event;
    use tracing_subscriber::layer::{Context, SubscriberExt};
    use tracing_subscriber::Layer;

    use super::*;

    /// Collects `(target, event)` for every event a test emits.
    #[derive(Clone, Default)]
    struct LogCapture(Arc<Mutex<Vec<(String, String)>>>);

    /// Reads the `event` field of one row.
    #[derive(Default)]
    struct EventName(String);

    impl Visit for EventName {
        fn record_str(&mut self, field: &Field, value: &str) {
            if field.name() == "event" {
                self.0 = value.to_string();
            }
        }

        fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {}
    }

    impl<S: tracing::Subscriber> Layer<S> for LogCapture {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut name = EventName::default();
            event.record(&mut name);
            let target = event.metadata().target().to_string();
            self.0.lock().unwrap().push((target, name.0));
        }
    }

    #[test]
    fn second_launch_without_stop_warns_pid_overwritten() {
        let cap = LogCapture::default();
        let subscriber = tracing_subscriber::registry().with(cap.clone());
        tracing::subscriber::with_default(subscriber, || {
            // First launch: nothing is recorded, so nothing is overwritten.
            warn_if_pid_overwritten(None, 100);
            // Second launch, no stop in between: the first client (this
            // test process, so alive) is still recorded.
            warn_if_pid_overwritten(Some(std::process::id()), 200);
        });
        let rows = cap.0.lock().unwrap();
        let warned = rows
            .iter()
            .filter(|(target, event)| target == "lab.client" && event == "pid_overwritten")
            .count();
        assert_eq!(warned, 1);
    }
}
