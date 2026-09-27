//! `ContentEvents`: the content-engine events combat raises, as a trait.
//!
//! Combat has to fire content chains at four points: a kill
//! (`entity_dead_tag`), the drain of the queued pre-hit health samples
//! (`entity_health_below`), and the two flank triggers of the NPC cover step
//! (`npc_flanked`, `player_flanked_npc`). The content executor that resolves
//! those chains sits *above* combat in the crate split, because it drives
//! combat itself (the `generate_threat` action, for one). A direct call would
//! be a cycle, so combat calls through this trait instead, and the content
//! crate implements it (docs/architecture/services-crate-split.md §2E).
//!
//! The content side implements it on a newtype over its `ChainEngine`
//! (`EngineEvents`), because `impl ContentEvents for ChainEngine` would break
//! the orphan rule there: both the trait and `ChainEngine` are foreign to the
//! content crate.
//!
//! # Contract
//!
//! An implementation runs the content dispatcher each method names
//! (`content::fire_*`), and combat calls the methods at the same points and in
//! the same order it called those dispatchers, so nothing about when chains
//! fire changes. The methods return a boxed future ([`EventFuture`])
//! rather than being `async fn`, because the trait is used as
//! `&dyn ContentEvents` and an `async fn` in a trait is not object-safe.
//!
//! Test code uses the fakes in `test_fixtures` (`NoContentEvents`,
//! `RecordingContentEvents`) behind the `test-support` feature.

use std::future::Future;
use std::pin::Pin;

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// The future every [`ContentEvents`] method returns.
pub type EventFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

/// The content-engine events the combat layer raises. See the module docs.
pub trait ContentEvents: Sync {
    /// An entity carrying a content tag died; `killer_entity_id` is the
    /// entity credited with the kill and `player_id` its character id. Fires
    /// the `entity_dead_tag` chains (`content::fire_entity_death`).
    fn entity_death<'a>(
        &'a self,
        killer_entity_id: u32,
        player_id: i32,
        entity_tag: &'a str,
        tx: &'a mpsc::Sender<CellToBaseMsg>,
        space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a>;

    /// Drain `SpaceManager`'s queued pre-hit health samples and fire the
    /// `entity_health_below` chains each crossing earns
    /// (`content::fire_pending_health_below`). Must run promptly after the
    /// damage seam that queued them; see `combat::damage_credit`.
    fn pending_health_below<'a>(
        &'a self,
        tx: &'a mpsc::Sender<CellToBaseMsg>,
        space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a>;

    /// The NPC `npc_entity_id` (template `npc_template`) is flanked by
    /// `threat_entity_id` at its cover slot (`content::fire_npc_flanked`).
    fn npc_flanked<'a>(
        &'a self,
        npc_entity_id: u32,
        threat_entity_id: u32,
        npc_template: &'a str,
        tx: &'a mpsc::Sender<CellToBaseMsg>,
        space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a>;

    /// The player `player_entity_id` flanked the NPC `npc_entity_id`: the
    /// mission-scoped form, with the player as the action target
    /// (`content::fire_player_flanked_npc`). A no-op when the flanker is not a
    /// player.
    fn player_flanked_npc<'a>(
        &'a self,
        npc_entity_id: u32,
        player_entity_id: u32,
        npc_template: &'a str,
        tx: &'a mpsc::Sender<CellToBaseMsg>,
        space_mgr: &'a mut SpaceManager,
    ) -> EventFuture<'a>;
}
