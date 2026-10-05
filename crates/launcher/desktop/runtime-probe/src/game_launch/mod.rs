//! Bounded launch-helper wire contract. Guest IDs are Windows IDs, never host PIDs.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
pub const MAX_MESSAGE: usize = 16384;
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub operation_id: Uuid,
    pub exe: PathBuf,
    pub directory: PathBuf,
    pub dlls: Vec<PathBuf>,
}
impl Request {
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > MAX_MESSAGE {
            return Err("too_large");
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| "invalid")?;
        if value.schema_version != 1 || value.operation_id.is_nil() || value.dlls.len() > 2 {
            return Err("invalid");
        }
        Ok(value)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    ProcessStarted { guest_pid: u32 },
    ProcessExited { guest_pid: u32, code: i32 },
    NotStarted,
    Unknown,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub schema_version: u32,
    pub operation_id: Uuid,
    pub observation: Event,
}
impl Message {
    pub fn decode(bytes: &[u8], id: Uuid) -> Result<Event, &'static str> {
        if bytes.len() > MAX_MESSAGE {
            return Err("too_large");
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| "invalid")?;
        if value.schema_version != 1
            || value.operation_id != id
            || id.is_nil()
            || matches!(
                value.observation,
                Event::ProcessStarted { guest_pid: 0 } | Event::ProcessExited { guest_pid: 0, .. }
            )
        {
            return Err("invalid");
        }
        Ok(value.observation)
    }
}
#[cfg(windows)]
pub mod windows;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_wrong_identity_zero_pid_and_unbounded_messages() {
        let id = Uuid::from_u128(1);
        let bytes = serde_json::to_vec(&Message {
            schema_version: 1,
            operation_id: id,
            observation: Event::ProcessStarted { guest_pid: 42 },
        })
        .unwrap();
        assert_eq!(
            Message::decode(&bytes, id).unwrap(),
            Event::ProcessStarted { guest_pid: 42 }
        );
        assert!(Message::decode(&bytes, Uuid::from_u128(2)).is_err());
        assert!(Message::decode(&vec![b' '; MAX_MESSAGE + 1], id).is_err());
        let bytes = serde_json::to_vec(&Message {
            schema_version: 1,
            operation_id: id,
            observation: Event::ProcessStarted { guest_pid: 0 },
        })
        .unwrap();
        assert!(Message::decode(&bytes, id).is_err());
    }
}
