//! Fakes for [`ContentEvents`], the seam combat raises content events
//! through.
//!
//! Both fakes drain `SpaceManager::pending_health_below` the way the real
//! drain does (`content::fire_pending_health_below` takes the whole queue
//! before it looks for chains), so a test that runs a damage path through a
//! fake leaves the queue as production would.

use std::sync::Mutex;

use tokio::sync::mpsc;

use crate::cell::combat::HealthBelowSample;
use crate::cell::content_events::{ContentEvents, EventFuture};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// A content engine with no chains: every event is dropped, the way the real
/// dispatchers drop an event no chain matches.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoContentEvents;

impl ContentEvents for NoContentEvents {
    fn entity_death<'a>(
        &'a self,
        _killer_entity_id: u32,
        _player_id: i32,
        _entity_tag: &'a str,
        _tx: &'a mpsc::Sender<CellToBaseMsg>,
        _space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        Box::pin(async {})
    }

    fn pending_health_below<'a>(
        &'a self,
        _tx: &'a mpsc::Sender<CellToBaseMsg>,
        space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        Box::pin(async move {
            space_mgr.pending_health_below.clear();
        })
    }

    fn npc_flanked<'a>(
        &'a self,
        _npc_entity_id: u32,
        _threat_entity_id: u32,
        _npc_template: &'a str,
        _tx: &'a mpsc::Sender<CellToBaseMsg>,
        _space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        Box::pin(async {})
    }

    fn player_flanked_npc<'a>(
        &'a self,
        _npc_entity_id: u32,
        _player_entity_id: u32,
        _npc_template: &'a str,
        _tx: &'a mpsc::Sender<CellToBaseMsg>,
        _space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        Box::pin(async {})
    }
}

/// One event a [`RecordingContentEvents`] saw.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordedContentEvent {
    EntityDeath {
        killer_entity_id: u32,
        player_id: i32,
        entity_tag: String,
    },
    /// The samples the drain took off the queue, in queue order. Empty when
    /// the drain ran on an empty queue.
    PendingHealthBelow { samples: Vec<HealthBelowSample> },
    NpcFlanked {
        npc_entity_id: u32,
        threat_entity_id: u32,
        npc_template: String,
    },
    PlayerFlankedNpc {
        npc_entity_id: u32,
        player_entity_id: u32,
        npc_template: String,
    },
}

/// Records every event in call order and fires nothing, so a test can assert
/// which content events a combat path raised, and in what order.
#[derive(Debug, Default)]
pub struct RecordingContentEvents {
    events: Mutex<Vec<RecordedContentEvent>>,
}

impl RecordingContentEvents {
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything recorded so far, oldest first.
    pub fn events(&self) -> Vec<RecordedContentEvent> {
        self.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn record(&self, event: RecordedContentEvent) {
        self.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(event);
    }
}

impl ContentEvents for RecordingContentEvents {
    fn entity_death<'a>(
        &'a self,
        killer_entity_id: u32,
        player_id: i32,
        entity_tag: &'a str,
        _tx: &'a mpsc::Sender<CellToBaseMsg>,
        _space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        self.record(RecordedContentEvent::EntityDeath {
            killer_entity_id,
            player_id,
            entity_tag: entity_tag.to_string(),
        });
        Box::pin(async {})
    }

    fn pending_health_below<'a>(
        &'a self,
        _tx: &'a mpsc::Sender<CellToBaseMsg>,
        space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        let samples = std::mem::take(&mut space_mgr.pending_health_below);
        self.record(RecordedContentEvent::PendingHealthBelow { samples });
        Box::pin(async {})
    }

    fn npc_flanked<'a>(
        &'a self,
        npc_entity_id: u32,
        threat_entity_id: u32,
        npc_template: &'a str,
        _tx: &'a mpsc::Sender<CellToBaseMsg>,
        _space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        self.record(RecordedContentEvent::NpcFlanked {
            npc_entity_id,
            threat_entity_id,
            npc_template: npc_template.to_string(),
        });
        Box::pin(async {})
    }

    fn player_flanked_npc<'a>(
        &'a self,
        npc_entity_id: u32,
        player_entity_id: u32,
        npc_template: &'a str,
        _tx: &'a mpsc::Sender<CellToBaseMsg>,
        _space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        self.record(RecordedContentEvent::PlayerFlankedNpc {
            npc_entity_id,
            player_entity_id,
            npc_template: npc_template.to_string(),
        });
        Box::pin(async {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::combat::HealthPct;

    fn sample(target: u32, pct: f64) -> HealthBelowSample {
        HealthBelowSample {
            attacker_entity_id: 1,
            target_entity_id: target,
            pct_before: HealthPct(pct),
        }
    }

    /// Both fakes must leave the health-below queue empty, as the real drain
    /// does: a fake that left it full would let a later drain in the same
    /// test see stale samples production never would.
    #[tokio::test]
    async fn both_fakes_drain_the_health_below_queue() {
        let (tx, _rx) = mpsc::channel(4);
        let mut mgr = SpaceManager::new(1);

        mgr.pending_health_below.push(sample(50, 80.0));
        let events: &dyn ContentEvents = &NoContentEvents;
        events.pending_health_below(&tx, &mut mgr).await;
        assert!(mgr.pending_health_below.is_empty());

        mgr.pending_health_below.push(sample(51, 70.0));
        mgr.pending_health_below.push(sample(52, 60.0));
        let recorder = RecordingContentEvents::new();
        let events: &dyn ContentEvents = &recorder;
        events.pending_health_below(&tx, &mut mgr).await;
        assert!(mgr.pending_health_below.is_empty());
        assert_eq!(
            recorder.events(),
            vec![RecordedContentEvent::PendingHealthBelow {
                samples: vec![sample(51, 70.0), sample(52, 60.0)],
            }]
        );
    }

    /// The recorder keeps call order across methods, which is what a combat
    /// test asserts ("death, then health-below" per pulse).
    #[tokio::test]
    async fn the_recorder_keeps_call_order() {
        let (tx, _rx) = mpsc::channel(4);
        let mut mgr = SpaceManager::new(1);
        let recorder = RecordingContentEvents::new();
        let events: &dyn ContentEvents = &recorder;

        events.entity_death(7, 100, "Guard01", &tx, &mut mgr).await;
        events.pending_health_below(&tx, &mut mgr).await;
        events.npc_flanked(8, 7, "HumanGuard", &tx, &mut mgr).await;
        events
            .player_flanked_npc(8, 7, "HumanGuard", &tx, &mut mgr)
            .await;

        assert_eq!(
            recorder.events(),
            vec![
                RecordedContentEvent::EntityDeath {
                    killer_entity_id: 7,
                    player_id: 100,
                    entity_tag: "Guard01".to_string(),
                },
                RecordedContentEvent::PendingHealthBelow { samples: vec![] },
                RecordedContentEvent::NpcFlanked {
                    npc_entity_id: 8,
                    threat_entity_id: 7,
                    npc_template: "HumanGuard".to_string(),
                },
                RecordedContentEvent::PlayerFlankedNpc {
                    npc_entity_id: 8,
                    player_entity_id: 7,
                    npc_template: "HumanGuard".to_string(),
                },
            ]
        );
    }
}
