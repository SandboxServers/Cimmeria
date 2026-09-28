//! Which spawn tags are still alive in a player's space.
//!
//! The content engine's `entity_tag_state` condition reads this set. It is
//! the *state* counterpart of the `entity_dead_tag` event: an event is missed
//! by a chain that was not live when it fired, a state can be tested later.
//! The Castle_CellBlock hallway backstops (2026-09-28 soft-lock) complete a
//! controller mission on accept when its guard is already dead.

use std::collections::HashSet;

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::stats::HEALTH;
use cimmeria_wire::state_field::BSF_DEAD;

use super::SpaceManager;

/// A corpse carries `BSF_DEAD`; an NPC whose health reached zero on a path
/// that has not set the flag yet is dead too. A tagged prop with no health
/// pool (`max == 0`) is never read as dead because of its stats.
fn is_alive(e: &CellEntity) -> bool {
    if e.state_field & BSF_DEAD != 0 {
        return false;
    }
    e.stats
        .get(HEALTH)
        .is_none_or(|h| h.max <= 0 || h.cur > 0)
}

impl SpaceManager {
    /// The tags carried by at least one living entity in `entity_id`'s
    /// space, or `None` when the entity is in no space.
    ///
    /// One pass over the space's entities. The dispatchers build it once per
    /// fired event, the same cadence as the mission-context snapshot, so a
    /// chain never sees a tag die halfway through its own evaluation.
    pub fn live_tags_in_space_of(&self, entity_id: u32) -> Option<HashSet<String>> {
        let space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(space_id)?;
        Some(
            space
                .entities
                .values()
                .filter(|e| is_alive(e))
                .filter_map(|e| e.tag.clone())
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::make_space_manager;

    #[test]
    fn a_corpse_and_a_zero_health_npc_drop_out_of_the_live_set() {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3])
            .unwrap();
        for (id, tag) in [(2, "Alive"), (3, "Corpse"), (4, "ZeroHp")] {
            mgr.create_entity(id, "Agnos", [1.0, 0.0, 1.0], [0.0; 3])
                .unwrap();
            let e = mgr.get_entity_mut(id).unwrap();
            e.tag = Some(tag.to_string());
        }
        mgr.get_entity_mut(3).unwrap().state_field |= BSF_DEAD;
        mgr.get_entity_mut(4)
            .unwrap()
            .stats
            .get_mut(HEALTH)
            .unwrap()
            .update(0, 0, 100);

        let live = mgr.live_tags_in_space_of(1).expect("player is in a space");
        assert!(live.contains("Alive"));
        assert!(!live.contains("Corpse"), "BSF_DEAD is dead");
        assert!(!live.contains("ZeroHp"), "zero health is dead");
        assert_eq!(mgr.live_tags_in_space_of(999), None);
    }
}
