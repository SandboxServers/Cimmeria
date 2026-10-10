//! The client's lifecycle: `lab_client_start`, `_stop` and `_restart`.
//! Split out of `mod.rs` to keep it under the file-size cap.

use std::time::Duration;

use serde_json::{json, Value};

use super::{instance, process, LoginState, Supervisor};

/// How long `lab_client_stop` waits for the client to be gone.
const STOP_WAIT: Duration = Duration::from_secs(10);

impl Supervisor {
    /// `lab_client_start` — launch suspended, inject the lab DLL, resume.
    pub async fn start(&self, server_override: Option<String>) -> Result<Value, String> {
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
        let pid = self.launch_client(server_override).await?;
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
                Ok(json!({ "stopped": true, "pid": pid, "exited": exited }))
            }
            None => Ok(json!({ "stopped": false, "reason": "no client running" })),
        }
    }

    /// `lab_client_restart` — stop then start.
    pub async fn restart(&self, server_override: Option<String>) -> Result<Value, String> {
        let _ = self.stop().await;
        // Brief settle so the OS releases the port + the old process.
        tokio::time::sleep(Duration::from_millis(500)).await;
        self.start(server_override).await
    }
}
