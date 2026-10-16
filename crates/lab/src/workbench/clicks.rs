//! Correlate CEGUI clicks using only fields captured in the click callback.

use serde::{Deserialize, Serialize};

use crate::supervisor::events::store::StoredEvent;

const MAX_CLICKS: usize = 512;
const CLICK_KIND: &str = "ui.click";

/// One click with its identity and target observation fixed at callback time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientClick {
    pub event_id: String,
    /// Host time when the supervisor ingested the ring row, not click time.
    pub host_ms: i64,
    /// Unknown until a verified client tick getter is available in the callback.
    pub client_tick: Option<u64>,
    pub widget_path: String,
    /// LR-10's clicked widget name.
    pub button: String,
    pub selected_entity_id: Option<u32>,
    pub client_pid: u32,
    pub source: String,
    pub window: String,
    /// LR-10's left, right, or middle mouse button.
    pub mouse: String,
    /// Only an at-click observation; a name is never treated as an entity ID.
    pub selected_target_name: Option<String>,
    pub target_present: Option<bool>,
    pub target_id_gap: Option<String>,
}

impl ClientClick {
    /// Convert a stored click without consulting live client or server state.
    pub fn from_stored(event: &StoredEvent, client_pid: u32) -> Option<Self> {
        if event.kind != CLICK_KIND {
            return None;
        }
        let fields = &event.fields;
        let epoch = fields.get("click_epoch")?.as_str()?;
        let seq = fields.get("click_seq")?.as_u64()?;
        let expected_id = format!("{epoch}:{seq}");
        if fields.get("event_id")?.as_str()? != expected_id {
            return None;
        }
        let window = fields.get("window")?.as_str()?.to_owned();
        let button = fields.get("button")?.as_str()?.to_owned();
        let widget_path = if window == button {
            window.clone()
        } else {
            format!("{window}/{button}")
        };
        let selected_entity_id = fields
            .get("selected_entity_id")
            .and_then(|v| v.as_u64())
            .and_then(|v| u32::try_from(v).ok());
        Some(Self {
            event_id: expected_id,
            host_ms: event.ts_ms,
            client_tick: None,
            widget_path,
            button,
            selected_entity_id,
            client_pid,
            source: "client.lua.cegui_click".into(),
            window,
            mouse: fields
                .get("mouse")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .into(),
            selected_target_name: fields
                .get("selected_target_name")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            target_present: fields.get("target_present").and_then(|v| v.as_bool()),
            target_id_gap: selected_entity_id
                .is_none()
                .then(|| "target_entity_id_unobservable_at_click".into()),
        })
    }
}

/// A bounded ordered click sequence with explicit missing-ring-row count.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClickSequence {
    pub events: Vec<ClientClick>,
    pub missing_ring_rows: u64,
    pub evicted: u64,
}

impl ClickSequence {
    /// Read a supervisor slice; an epoch change resets only the ring-seq gap count.
    pub fn from_stored<'a>(
        events: impl IntoIterator<Item = &'a StoredEvent>,
        client_pid: u32,
    ) -> Self {
        let mut result = Self::default();
        let mut previous: Option<(String, u64)> = None;
        for stored in events {
            let Some(click) = ClientClick::from_stored(stored, client_pid) else {
                continue;
            };
            let epoch = stored.fields["click_epoch"].as_str().unwrap_or("");
            let seq = stored.fields["click_seq"].as_u64().unwrap_or(0);
            if let Some((last_epoch, last_seq)) = &previous {
                if last_epoch == epoch && seq > *last_seq {
                    result.missing_ring_rows += seq - *last_seq - 1;
                }
            }
            previous = Some((epoch.to_owned(), seq));
            if result.events.len() == MAX_CLICKS {
                result.events.remove(0);
                result.evicted += 1;
            }
            result.events.push(click);
        }
        result
    }
}

#[cfg(test)]
#[path = "clicks_tests.rs"]
mod tests;
