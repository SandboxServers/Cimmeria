//! The player's game-telemetry choice. It is its own command and its own
//! record: launcher-summary consent never turns it on. Path-free: the webview
//! learns whether this build can offer it, never where the DLL is.
use super::{JobError, NativeHost};
use cimmeria_launcher_engine::game_telemetry::Outcome;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum GameTelemetryCommand {
    Inspect { schema_version: u32 },
    Set { schema_version: u32, opted_in: bool },
}
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct GameTelemetryStatus {
    schema_version: u32,
    /// This build bundles a verified player build of the telemetry DLL.
    available: bool,
    opted_in: bool,
    /// What the most recent Play did about it; `None` before the first.
    last_outcome: Option<Outcome>,
}
impl NativeHost {
    pub fn game_telemetry_command(
        &self,
        request: GameTelemetryCommand,
    ) -> Result<GameTelemetryStatus, JobError> {
        let version = match &request {
            GameTelemetryCommand::Inspect { schema_version }
            | GameTelemetryCommand::Set { schema_version, .. } => *schema_version,
        };
        if version != 1 {
            return Err(JobError::UnsupportedSchema);
        }
        let available = self
            .launch_resources
            .as_ref()
            .is_some_and(|resources| resources.client_telemetry.is_some());
        let store = self.store()?;
        let mut state = store.lock().map_err(|_| JobError::Io)?;
        if let GameTelemetryCommand::Set { opted_in, .. } = request {
            // Say so rather than save a choice this build cannot act on.
            // Opting out always works.
            if opted_in && !available {
                return Err(JobError::PlatformUnavailable);
            }
            state.set_game_telemetry(opted_in)?;
        }
        Ok(GameTelemetryStatus {
            schema_version: 1,
            available,
            opted_in: state.game_telemetry()?.opted_in,
            last_outcome: state.game_telemetry_status()?.map(|status| status.outcome),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_launcher_engine::launch::{Artifact, Resources};
    use sha2::{Digest, Sha256};

    fn artifact(root: &std::path::Path, name: &str) -> Artifact {
        let path = root.join(name);
        std::fs::write(&path, b"fixture").unwrap();
        let digest: String = Sha256::digest(b"fixture")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Artifact::open(path, &digest).unwrap()
    }
    fn host(root: &std::path::Path, with_dll: bool) -> NativeHost {
        let mut host = NativeHost::new(root.join("state"));
        host.launch_resources = Some(Resources {
            helper: artifact(root, "helper.exe"),
            client_patches: None,
            client_telemetry: with_dll.then(|| artifact(root, "telemetry.dll")),
            graphics: None,
        });
        host
    }
    fn inspect(host: &NativeHost) -> GameTelemetryStatus {
        host.game_telemetry_command(GameTelemetryCommand::Inspect { schema_version: 1 })
            .unwrap()
    }
    fn set(host: &NativeHost, opted_in: bool) -> Result<GameTelemetryStatus, JobError> {
        host.game_telemetry_command(GameTelemetryCommand::Set {
            schema_version: 1,
            opted_in,
        })
    }

    #[test]
    fn a_build_without_the_dll_refuses_an_opt_in_instead_of_saving_one() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let host = host(&root, false);
        let status = inspect(&host);
        assert!(!status.available && !status.opted_in && status.last_outcome.is_none());
        assert_eq!(set(&host, true), Err(JobError::PlatformUnavailable));
        assert!(!inspect(&host).opted_in);
        // Opting out needs no DLL.
        assert!(!set(&host, false).unwrap().opted_in);
        assert_eq!(
            host.game_telemetry_command(GameTelemetryCommand::Inspect { schema_version: 2 }),
            Err(JobError::UnsupportedSchema)
        );
    }

    #[test]
    fn the_choice_is_saved_before_it_is_acknowledged_and_survives_a_restart() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        {
            let host = host(&root, true);
            assert!(inspect(&host).available);
            assert!(set(&host, true).unwrap().opted_in);
            // The launcher-summary choice is a different record and is untouched.
            let native = host
                .with_state(|state| Ok(state.preferences().clone()))
                .unwrap();
            assert!(!native.launcher_summary_consent);
        }
        let reopened = host(&root, true);
        assert!(inspect(&reopened).opted_in);
        assert!(!set(&reopened, false).unwrap().opted_in);
        assert!(!inspect(&reopened).opted_in);
    }

    /// The JS harness drives the production host; every write stays in a
    /// temporary fixture and nothing is sent anywhere.
    #[test]
    #[ignore = "JSON-lines production-host bridge for game-telemetry-native-uat.mjs"]
    fn game_telemetry_uat_bridge() {
        use std::io::{BufRead, Write};
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let mut current = host(&root, true);
        for line in std::io::stdin().lock().lines() {
            let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
            let reply = match value["command"].as_str().unwrap() {
                // A restart, with or without the module in the new build.
                command @ ("reopen" | "reopen_without_module") => {
                    drop(current);
                    current = host(&root, command == "reopen");
                    serde_json::json!({"ok": inspect(&current)})
                }
                "preferences" => {
                    let preferences = current
                        .with_state(|state| Ok(state.preferences().clone()))
                        .unwrap();
                    serde_json::json!({"ok": preferences})
                }
                _ => match current.game_telemetry_command(serde_json::from_value(value).unwrap()) {
                    Ok(status) => serde_json::json!({"ok": status}),
                    Err(error) => serde_json::json!({"error": error}),
                },
            };
            println!("GAME_TELEMETRY_UAT {reply}");
            std::io::stdout().flush().unwrap();
        }
    }
}
