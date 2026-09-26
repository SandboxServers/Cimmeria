//! `EngineEvents`: the content executor behind combat's `ContentEvents` seam.
//!
//! Combat, the effect pulses and the NPC AI raise content events through
//! `cell::content_events::ContentEvents` (in `cimmeria-cell-world`) instead of
//! calling the `fire_*` dispatchers here, because the content executor sits
//! above them in the crate split and drives them itself
//! (docs/architecture/services-crate-split.md §2E). This is the production
//! implementation: each method is the dispatcher it names, called with the
//! same arguments, so moving combat behind the trait changes nothing about
//! when or how a chain fires.
//!
//! A newtype rather than `impl ContentEvents for ChainEngine`: once content
//! is its own crate, both the trait and `ChainEngine` are foreign to it, and
//! the orphan rule forbids that impl. Callers wrap the engine they already
//! hold: `&EngineEvents(&engine)`.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::content_events::{ContentEvents, EventFuture};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// The live chain engine as the combat layer's [`ContentEvents`]. See the
/// module docs.
#[derive(Clone, Copy)]
pub struct EngineEvents<'e>(pub &'e ChainEngine);

impl ContentEvents for EngineEvents<'_> {
    fn entity_death<'a>(
        &'a self,
        killer_entity_id: u32,
        player_id: i32,
        entity_tag: &'a str,
        tx: &'a mpsc::Sender<CellToBaseMsg>,
        space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        Box::pin(super::fire_entity_death(
            killer_entity_id,
            player_id,
            entity_tag,
            self.0,
            tx,
            space_mgr,
        ))
    }

    fn pending_health_below<'a>(
        &'a self,
        tx: &'a mpsc::Sender<CellToBaseMsg>,
        space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        Box::pin(super::fire_pending_health_below(self.0, tx, space_mgr))
    }

    fn npc_flanked<'a>(
        &'a self,
        npc_entity_id: u32,
        threat_entity_id: u32,
        npc_template: &'a str,
        tx: &'a mpsc::Sender<CellToBaseMsg>,
        space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        Box::pin(super::fire_npc_flanked(
            npc_entity_id,
            threat_entity_id,
            npc_template,
            self.0,
            tx,
            space_mgr,
        ))
    }

    fn player_flanked_npc<'a>(
        &'a self,
        npc_entity_id: u32,
        player_entity_id: u32,
        npc_template: &'a str,
        tx: &'a mpsc::Sender<CellToBaseMsg>,
        space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a> {
        Box::pin(super::fire_player_flanked_npc(
            npc_entity_id,
            player_entity_id,
            npc_template,
            self.0,
            tx,
            space_mgr,
        ))
    }
}
