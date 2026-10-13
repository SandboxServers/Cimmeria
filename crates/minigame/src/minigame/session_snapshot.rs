//! Ticket-free Livewire board and bounded SmartFox activity retained for lab inspection.

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use serde::Serialize;
use tokio::time::Instant;

pub(super) const FINISHED_BOARD_TTL: Duration = Duration::from_secs(600);
const EVENT_CAP: usize = 128;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoardSnapshot {
    pub entity_id: u32,
    pub game_name: String,
    pub state: &'static str,
    pub outcome: Option<&'static str>,
    pub difficulty: u32,
    pub game_started: bool,
    pub timer_state: u32,
    pub time_remaining: f64,
    pub goal_total: u32,
    pub goal_cut: u32,
    pub wires: Vec<WireSnapshot>,
    pub events: Vec<MinigameEvent>,
    pub events_dropped: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WireSnapshot {
    pub name: String,
    pub lib: String,
    pub kind: &'static str,
    pub clickable: bool,
    pub depth: u32,
    pub x: f64,
    pub y: f64,
    pub cut: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MinigameEvent {
    pub seq: u64,
    pub t_ms: u64,
    pub dir: &'static str,
    pub cmd: String,
    pub params: BTreeMap<String, serde_json::Value>,
    pub event: String,
    pub wire_name: Option<String>,
    pub elapsed_ms: u64,
}

pub(super) struct PublishedBoard {
    pub(super) ticket: String,
    pub(super) snapshot: Option<BoardSnapshot>,
    pub(super) expires_at: Option<Instant>,
    created_at: Instant,
    events: VecDeque<MinigameEvent>,
    dropped: u64,
    next_seq: u64,
}

impl PublishedBoard {
    pub(super) fn new(ticket: String, created_at: Instant) -> Self {
        Self {
            ticket,
            snapshot: None,
            expires_at: None,
            created_at,
            events: VecDeque::new(),
            dropped: 0,
            next_seq: 1,
        }
    }

    pub(super) fn record(&mut self, event: &str, wire_name: Option<&str>) {
        let mut params = BTreeMap::new();
        if let Some(wire) = wire_name {
            params.insert(
                "wirename".to_owned(),
                serde_json::Value::String(wire.chars().take(64).collect()),
            );
        }
        self.record_traffic("internal", event, params);
    }

    pub(super) fn record_traffic(
        &mut self,
        dir: &'static str,
        cmd: &str,
        params: BTreeMap<String, serde_json::Value>,
    ) {
        if self.events.len() == EVENT_CAP {
            self.events.pop_front();
            self.dropped += 1;
        }
        let cmd: String = cmd.chars().take(64).collect();
        let wire_name = params
            .get("wirename")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
        let t_ms = u64::try_from(self.created_at.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.events.push_back(MinigameEvent {
            seq: self.next_seq,
            t_ms,
            dir,
            event: cmd.clone(),
            cmd,
            params,
            wire_name,
            elapsed_ms: t_ms,
        });
        self.next_seq += 1;
    }

    pub(super) fn read(&self) -> Option<BoardSnapshot> {
        let mut snapshot = self.snapshot.clone()?;
        snapshot.events = self.events.iter().cloned().collect();
        snapshot.events_dropped = self.dropped;
        Some(snapshot)
    }

    pub(super) fn finish(&mut self) {
        self.record("ended", None);
        self.expires_at = Some(Instant::now() + FINISHED_BOARD_TTL);
    }

    pub(super) fn is_expired(&self, now: Instant) -> bool {
        self.expires_at.is_some_and(|expires| now >= expires)
    }
}
