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
    /// SGW.exe's display arguments (C-PD7). An empty list is omitted, so a
    /// request without display arguments serializes exactly as before.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}
/// The only game arguments the helper passes on: `-windowed`, `ResX=<n>` and
/// `ResY=<n>` (3-4 ASCII digits each), at most once each.
pub fn valid_args(args: &[String]) -> bool {
    let digits = |value: Option<&str>| {
        value.is_some_and(|v| (3..=4).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_digit()))
    };
    args.len() <= 3
        && args.iter().enumerate().all(|(index, arg)| {
            !args[..index].contains(arg)
                && (arg == "-windowed"
                    || digits(arg.strip_prefix("ResX="))
                    || digits(arg.strip_prefix("ResY=")))
        })
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
/// Where the helper stopped. A closed set; never a path or a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Request,
    CreateProcess,
    Inject,
    Resume,
    Wait,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    ProcessStarted {
        guest_pid: u32,
    },
    ProcessExited {
        guest_pid: u32,
        code: i32,
    },
    NotStarted {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stage: Option<Stage>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        win32: Option<u32>,
    },
    Unknown {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stage: Option<Stage>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        win32: Option<u32>,
    },
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
#[cfg(test)]
#[path = "args_tests.rs"]
mod args_tests;
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

    #[test]
    fn an_old_helper_event_without_stage_still_decodes() {
        let id = Uuid::from_u128(1);
        for (event, expected) in [
            (
                r#"{"event":"not_started"}"#,
                Event::NotStarted {
                    stage: None,
                    win32: None,
                },
            ),
            (
                r#"{"event":"unknown"}"#,
                Event::Unknown {
                    stage: None,
                    win32: None,
                },
            ),
        ] {
            let bytes = serde_json::to_vec(&serde_json::json!({
                "schema_version": 1,
                "operation_id": id,
                "observation": serde_json::from_str::<serde_json::Value>(event).unwrap(),
            }))
            .unwrap();
            assert_eq!(Message::decode(&bytes, id).unwrap(), expected);
        }
    }

    #[test]
    fn stage_and_win32_round_trip() {
        let event = Event::Unknown {
            stage: Some(Stage::Inject),
            win32: Some(5),
        };
        let json = serde_json::to_value(event).unwrap();
        assert_eq!(json["stage"], "inject");
        assert_eq!(json["win32"], 5);
        let decoded: Event = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, event);
    }
}
