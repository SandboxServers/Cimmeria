//! Client setup before a launch, run by the worker only after it has
//! claimed the launch slot (or, for a saved login-server list, the
//! maintenance slot).
//!
//! Setup renames stock-case files, writes `LoginInternal.lua` and turns
//! ASLR off in `SGW.exe`. Done on the UI thread before the claim, a stale
//! frame could rewrite those files under a running game; here the claim
//! comes first, so a conflicting game refuses the job before any write.

use std::path::PathBuf;

use crate::client_setup::{self, LoginServer, SetupReport};

use super::{Busy, Event, EventSender, Worker};

/// What client setup needs: the install root and the login servers.
#[derive(Debug, Clone)]
pub struct ClientPrep {
    pub install_root: PathBuf,
    pub login_servers: Vec<LoginServer>,
}

/// Status lines for a successful setup.
pub(super) fn report_lines(report: &SetupReport) -> Vec<String> {
    let mut lines = Vec::new();
    for r in &report.restored_names {
        lines.push(format!(
            "Renamed {} back to its stock name {} (the game can't find it otherwise).",
            r.from.display(),
            r.to.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
    if report.login_servers_written {
        lines.push("Wrote the login server list (LoginInternal.lua).".into());
    }
    if report.aslr == client_setup::AslrOutcome::Disabled {
        lines.push("Switched ASLR off in SGW.exe.".into());
    }
    lines
}

/// Run setup off the async pool. Each change is reported as a
/// [`Event::SetupNote`]; a failure comes back as the reason, and the
/// caller does not launch: a client with ASLR still on breaks the patches
/// DLL, and one without the server list cannot log in.
pub(super) async fn run(prep: &ClientPrep, events_tx: &EventSender) -> Result<(), String> {
    let p = prep.clone();
    let result = tokio::task::spawn_blocking(move || {
        client_setup::prepare(&p.install_root, &p.login_servers)
    })
    .await
    .map_err(|e| format!("client setup failed: {e}"))?;
    match result {
        Ok(report) => {
            for line in report_lines(&report) {
                let _ = events_tx.send(Event::SetupNote(line));
            }
            Ok(())
        }
        Err(e) => Err(format!("client setup failed: {e}")),
    }
}

impl Worker {
    /// Apply a saved login-server list to the installed client now.
    pub(super) fn spawn_prepare_client(&self, prep: ClientPrep) {
        if let Err(c) = self.activity.begin_maintenance(&prep.install_root) {
            self.refuse(Busy::Files, c);
            return;
        }
        let events_tx = self.events_tx.clone();
        let activity = self.activity.clone();
        self.runtime.spawn(async move {
            if let Err(why) = run(&prep, &events_tx).await {
                let _ = events_tx.send(Event::SetupNote(format!("Not applied: {why}")));
            }
            activity.end_maintenance();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_setup::{AslrOutcome, Restored};

    #[test]
    fn a_setup_report_names_each_change() {
        let eula = std::path::Path::new("Working/SGWGame/Content/UI/Startup/EULA");
        let lines = report_lines(&SetupReport {
            restored_names: vec![Restored {
                from: eula.join("eula.lua"),
                to: eula.join("EULA.lua"),
            }],
            login_servers_written: true,
            aslr: AslrOutcome::Disabled,
        });
        assert_eq!(lines.len(), 3, "{lines:?}");
        assert!(
            lines[0].contains("eula.lua") && lines[0].contains("stock name EULA.lua"),
            "{}",
            lines[0]
        );
        let quiet = report_lines(&SetupReport {
            restored_names: Vec::new(),
            login_servers_written: false,
            aslr: AslrOutcome::AlreadyOff,
        });
        assert!(quiet.is_empty());
    }
}
