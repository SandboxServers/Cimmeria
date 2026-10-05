//! Per-player, per-chain debounce for content actions that cost the server
//! real work on every click (DA-02 review F3).
//!
//! `interact` accepts as many packets as a client sends. Most interact
//! chains are cheap, but some are not: the Debug Area ability granter and
//! reset NPC forward a locked base write for a GM, and a Gate Mail Clerk
//! forwards a row-locked mail transaction even inside its 10-minute
//! window. A scripted client could turn those into a stream of locks. The
//! executor arms of those actions ask [`SpaceManager::chain_debounce`]
//! first: one firing per player per chain per [`CHAIN_DEBOUNCE`], and the
//! rest are dropped with a DEBUG row. A human double-click falls inside
//! the window and costs one firing, which already answered the first press.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::SpaceManager;

/// The shortest gap between two firings of one chain for one player.
pub const CHAIN_DEBOUNCE: Duration = Duration::from_secs(1);

/// When each chain last fired for this entity (`CellEntity::extensions`).
#[derive(Debug, Clone, Default)]
pub struct ChainDebounce {
    last_fired: HashMap<i64, Instant>,
}

impl SpaceManager {
    /// Whether `chain_id` may fire for `entity_id` at `now`; records the
    /// firing when it may. `false` for a firing inside [`CHAIN_DEBOUNCE`] of
    /// the last one, and for a missing entity.
    pub fn chain_debounce(&mut self, entity_id: u32, chain_id: i64, now: Instant) -> bool {
        let Some(e) = self.get_entity_mut(entity_id) else {
            return false;
        };
        if e.extensions.get::<ChainDebounce>().is_none() {
            e.extensions.insert(ChainDebounce::default());
        }
        let Some(d) = e.extensions.get_mut::<ChainDebounce>() else {
            return false;
        };
        match d.last_fired.get(&chain_id) {
            Some(&last) if now.saturating_duration_since(last) < CHAIN_DEBOUNCE => false,
            _ => {
                d.last_fired.insert(chain_id, now);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_firing_per_chain_per_window() {
        let mut mgr = crate::test_fixtures::make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        let t0 = Instant::now();
        assert!(mgr.chain_debounce(1, 13000, t0));
        assert!(!mgr.chain_debounce(1, 13000, t0 + Duration::from_millis(999)));
        assert!(
            mgr.chain_debounce(1, 13001, t0),
            "another chain is its own window"
        );
        assert!(mgr.chain_debounce(1, 13000, t0 + CHAIN_DEBOUNCE));
        assert!(
            !mgr.chain_debounce(2, 13000, t0),
            "a missing entity never fires"
        );
    }
}
