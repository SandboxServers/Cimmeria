//! Ticket-free Livewire board and bounded SmartFox activity retained for lab inspection.

use std::collections::VecDeque;
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
    pub depth: u32,
    pub x: f64,
    pub y: f64,
    pub cut: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MinigameEvent {
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
        }
    }

    pub(super) fn record(&mut self, event: &str, wire_name: Option<&str>) {
        if self.events.len() == EVENT_CAP {
            self.events.pop_front();
            self.dropped += 1;
        }
        self.events.push_back(MinigameEvent {
            event: event.to_owned(),
            wire_name: wire_name.map(|name| name.chars().take(64).collect()),
            elapsed_ms: u64::try_from(self.created_at.elapsed().as_millis()).unwrap_or(u64::MAX),
        });
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
