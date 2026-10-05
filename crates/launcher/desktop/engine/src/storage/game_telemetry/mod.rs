//! Opt-in game telemetry: the `cimmeria-client-telemetry` DLL the launch helper
//! injects after the client patches. Its choice is its own record and is off
//! until the player turns it on; launcher-summary consent never enables it.
use super::*;
use sha2::{Digest, Sha256};
use uuid::Uuid;
mod session;
pub use session::{passthrough_environment, start, Session};
#[cfg(test)]
mod tests;

const NAME: &str = "game-telemetry.json";
const STATUS: &str = "game-telemetry-status.json";

/// Minted at the first opt-in and kept after an opt-out, so a player who turns
/// telemetry back on does not become a new install to the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub install_id: Uuid,
    /// `sha256(host name)[..16 hex]`, as the Windows launcher's fallback: the
    /// name itself never leaves the machine.
    pub machine_id: String,
}
impl Identity {
    fn mint() -> Self {
        let digest = Sha256::digest(host_name().as_bytes());
        Self {
            install_id: Uuid::new_v4(),
            machine_id: digest[..8].iter().map(|b| format!("{b:02x}")).collect(),
        }
    }
    fn valid(&self) -> bool {
        !self.install_id.is_nil()
            && self.machine_id.len() == 16
            && self.machine_id.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameTelemetry {
    pub schema_version: u32,
    pub opted_in: bool,
    pub identity: Option<Identity>,
}
impl Default for GameTelemetry {
    fn default() -> Self {
        Self {
            schema_version: 1,
            opted_in: false,
            identity: None,
        }
    }
}

/// What the last launch did about game telemetry. Path-free and safe for IPC.
/// `Attached` means the DLL was on the injection list with a session marker on
/// disk; it does not claim the DLL loaded or that an upload reached the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Attached,
    /// The server gave no session (unreachable, refused, kill switch, bad reply).
    SessionUnavailable,
    /// The mint or upload address is plain http to a host that is not a login server.
    EndpointRefused,
    /// A session was minted but its marker could not be written beside the game.
    SessionNotWritten,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub schema_version: u32,
    pub operation_id: Uuid,
    pub outcome: Outcome,
}

impl DesktopState {
    pub fn game_telemetry(&self) -> Result<GameTelemetry, StorageError> {
        let value: GameTelemetry = read(&self.directory.root.join(NAME))?.unwrap_or_default();
        if value.schema_version != 1 {
            return Err(StorageError::UnsupportedSchema);
        }
        // An opt-in without a usable identity could not have been written here.
        if value.identity.as_ref().is_some_and(|id| !id.valid())
            || (value.opted_in && value.identity.is_none())
        {
            return Err(StorageError::Corrupt);
        }
        Ok(value)
    }

    /// Saved before it is acknowledged. The choice may change during an
    /// operation: it takes effect at the next Play, never on a running game.
    pub fn set_game_telemetry(&mut self, opted_in: bool) -> Result<GameTelemetry, StorageError> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain);
        }
        let mut value = self.game_telemetry()?;
        if value.opted_in == opted_in {
            return Ok(value);
        }
        if opted_in && value.identity.is_none() {
            value.identity = Some(Identity::mint());
        }
        value.opted_in = opted_in;
        self.write_game_telemetry(NAME, &value)?;
        Ok(value)
    }

    pub fn game_telemetry_status(&self) -> Result<Option<Status>, StorageError> {
        let status: Option<Status> = read(&self.directory.root.join(STATUS))?;
        if status.is_some_and(|status| status.schema_version != 1) {
            return Err(StorageError::UnsupportedSchema);
        }
        Ok(status)
    }

    pub(crate) fn record_game_telemetry(
        &mut self,
        operation_id: Uuid,
        outcome: Outcome,
    ) -> Result<(), StorageError> {
        self.write_game_telemetry(
            STATUS,
            &Status {
                schema_version: 1,
                operation_id,
                outcome,
            },
        )
    }

    fn write_game_telemetry<T: Serialize>(
        &mut self,
        name: &str,
        value: &T,
    ) -> Result<(), StorageError> {
        let result = atomic::write(&self.directory.root, name, value);
        self.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
        result
    }
}

fn host_name() -> String {
    #[cfg(target_os = "macos")]
    {
        let mut name = [0u8; 256];
        // SAFETY: the buffer is valid for its whole length and outlives the call.
        if unsafe { libc::gethostname(name.as_mut_ptr().cast(), name.len()) } == 0 {
            let end = name.iter().position(|b| *b == 0).unwrap_or(name.len());
            return String::from_utf8_lossy(&name[..end]).into_owned();
        }
    }
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default()
}
