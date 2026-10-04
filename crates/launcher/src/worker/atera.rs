//! The Atera debug launches and Fix ASLR: developer tools whose bats
//! start or rewrite `SGW.exe` themselves.
//!
//! The worker cannot follow a game the Atera bat starts, so a launch
//! keeps the launch slot until the process probe sees `SGW.exe` (or
//! [`GAME_APPEARS_WITHIN`] passes); from then on the probe guards the
//! game. A second click in between is refused instead of starting a
//! second game. Fix ASLR holds the maintenance slot until its bat exits.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tracing::{error, warn};

use super::client_prep::{self, ClientPrep};
use super::{Activity, Busy, Event, EventSender, LaunchTelemetryConfig, Worker};
use crate::launch::{
    launch_atera_debug, launch_atera_debug_with_child, launch_atera_fix_aslr_with_child,
};
use crate::telemetry::auth::DevSessionRequest;
use crate::telemetry::endpoint::EndpointPolicy;
use crate::telemetry::process_watch::wait_for_exit;
use crate::telemetry::runner::run_session;
use crate::telemetry::Telemetry;

/// How long a bat launch holds the launch slot waiting for `SGW.exe`.
pub(super) const GAME_APPEARS_WITHIN: Duration = Duration::from_secs(60);

const BAT: &str = "AtreaGameDebug.bat";

impl Worker {
    /// Called once the launch slot is claimed. Setup, the bat, then the
    /// slot is held until the game shows up.
    pub(super) fn spawn_atera_debug(&self, dir: PathBuf, prep: ClientPrep) {
        let events_tx = self.events_tx.clone();
        let activity = self.activity.clone();
        self.runtime.spawn(async move {
            if let Err(why) = client_prep::run(&prep, &events_tx).await {
                let _ = events_tx.send(Event::LaunchError(why));
                activity.game_ended();
                return;
            }
            match launch_atera_debug(&dir) {
                Ok(pid) => {
                    let _ = events_tx.send(Event::Launched(BAT.into(), pid));
                    hold_until_visible(&activity, &dir, &events_tx).await;
                }
                Err(e) => {
                    let _ = events_tx.send(Event::LaunchError(e.to_string()));
                }
            }
            activity.game_ended();
        });
    }

    /// The Atera bat plus a telemetry session for the bat's lifetime.
    /// Called once the launch slot is claimed.
    pub(super) fn spawn_atera_with_telemetry(
        &self,
        install_dir: PathBuf,
        cfg: LaunchTelemetryConfig,
        prep: ClientPrep,
    ) {
        let events_tx = self.events_tx.clone();
        let http = self.telemetry_http.clone();
        let activity = self.activity.clone();
        self.runtime.spawn(async move {
            if let Err(why) = client_prep::run(&prep, &events_tx).await {
                let _ = events_tx.send(Event::LaunchError(why));
                activity.game_ended();
                return;
            }
            let req = DevSessionRequest {
                install_id: cfg.install_id,
                machine_id: cfg.machine_id,
                branch: cfg.branch,
                git_sha: cfg.git_sha,
                launcher_version: cfg.launcher_version.clone(),
                tags: cfg.tags,
            };
            let telemetry = match Telemetry::start_session(
                &http,
                &cfg.auth_base_url,
                req,
                &install_dir,
                &cfg.launcher_version,
                EndpointPolicy::from_login_servers(
                    cfg.login_server_urls.iter().map(String::as_str),
                ),
            )
            .await
            {
                Ok(t) => Arc::new(t),
                Err(e) => {
                    error!("telemetry session start failed: {e}");
                    // The game still launches: telemetry never blocks play.
                    let _ = events_tx.send(Event::TelemetrySessionError(format!(
                        "auth handshake failed: {e}"
                    )));
                    match launch_atera_debug(&install_dir) {
                        Ok(pid) => {
                            let _ = events_tx.send(Event::Launched(BAT.into(), pid));
                            hold_until_visible(&activity, &install_dir, &events_tx).await;
                        }
                        Err(e) => {
                            let _ = events_tx.send(Event::LaunchError(e.to_string()));
                        }
                    }
                    activity.game_ended();
                    return;
                }
            };
            let child = match launch_atera_debug_with_child(&install_dir) {
                Ok(c) => {
                    let _ = events_tx.send(Event::Launched(BAT.into(), c.id()));
                    c
                }
                Err(e) => {
                    let _ = events_tx.send(Event::LaunchError(e.to_string()));
                    activity.game_ended();
                    return;
                }
            };
            hold_until_visible(&activity, &install_dir, &events_tx).await;
            activity.game_ended();
            // The bat starts SGW.exe itself, so the client-patches DLL
            // cannot go in and there is no patch log to summarise.
            let exit = Box::pin(wait_for_exit(child));
            let http_arc = Arc::new(http.clone());
            match run_session(telemetry, http_arc, exit, install_dir, cfg.state_dir, None).await {
                Ok(outcome) => {
                    let _ = events_tx.send(Event::TelemetrySessionComplete(outcome));
                }
                Err(e) => {
                    error!("telemetry session error: {e}");
                    let _ = events_tx.send(Event::TelemetrySessionError(e.to_string()));
                }
            }
        });
    }

    /// Fix ASLR rewrites `SGW.exe`: it holds the maintenance slot from
    /// the claim until the bat exits, so no launch starts meanwhile.
    pub(super) fn spawn_fix_aslr(&self, dir: PathBuf) {
        if let Err(c) = self.activity.begin_maintenance(&dir) {
            self.refuse(Busy::Files, c);
            return;
        }
        let events_tx = self.events_tx.clone();
        let activity = self.activity.clone();
        self.runtime.spawn(async move {
            match launch_atera_fix_aslr_with_child(&dir) {
                Ok(mut child) => {
                    let _ = events_tx.send(Event::Launched("AtreaFixASLR.bat".into(), child.id()));
                    let _ = tokio::task::spawn_blocking(move || child.wait()).await;
                }
                Err(e) => {
                    let _ = events_tx.send(Event::LaunchError(e.to_string()));
                }
            }
            activity.end_maintenance();
        });
    }
}

/// Keep the launch slot until the probe sees the game the bat started.
async fn hold_until_visible(activity: &Activity, dir: &Path, events_tx: &EventSender) {
    if !activity.wait_for_game(dir, GAME_APPEARS_WITHIN).await {
        warn!(dir = %dir.display(), "Atera bat started but SGW.exe never appeared");
        let _ = events_tx.send(Event::LaunchError(format!(
            "{BAT} started, but SGW.exe did not appear within {} s",
            GAME_APPEARS_WITHIN.as_secs()
        )));
    }
}
