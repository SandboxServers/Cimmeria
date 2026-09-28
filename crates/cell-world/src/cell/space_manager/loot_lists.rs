//! Which loot list a looter sees: the shared list on a corpse, or the
//! looter's own roll on a live container.
//!
//! A corpse's loot is rolled once, on death, into `CellEntity::loot`, and
//! whoever loots first takes it. A container (a chest or crate opened by the
//! content `open_loot` action) rolls per looter into
//! `CellEntity::container_loot`, keyed by `player_id`, so two players never
//! share or take each other's roll (Decision (@Cadacious, 2026-09-28)).
//! Keying by `player_id`, not entity id, keeps a pending roll across a relog
//! for as long as the container entity lives.

use cimmeria_entity::cell_entity::LootItem;

use super::SpaceManager;

impl SpaceManager {
    /// The loot `looter_player_id` sees on `source`: its own pending roll on
    /// a container, the shared list on a corpse, empty when `source` is gone.
    pub fn loot_view(&self, source: u32, looter_player_id: Option<i32>) -> Vec<LootItem> {
        let Some(e) = self.get_entity(source) else {
            return Vec::new();
        };
        if e.is_loot_container {
            looter_player_id
                .and_then(|pid| e.container_loot.get(&pid))
                .cloned()
                .unwrap_or_default()
        } else {
            e.loot.clone()
        }
    }

    /// Mutable access to the list [`Self::loot_view`] reads. `None` when the
    /// source is gone, or it is a container holding no roll for this looter.
    pub fn loot_list_mut(
        &mut self,
        source: u32,
        looter_player_id: i32,
    ) -> Option<&mut Vec<LootItem>> {
        let e = self.get_entity_mut(source)?;
        if e.is_loot_container {
            e.container_loot.get_mut(&looter_player_id)
        } else {
            Some(&mut e.loot)
        }
    }

    /// Drop the looter's list on a container once it is empty, so an
    /// emptied chest reads as "already looted" rather than "pending".
    /// A corpse is left alone: its caller clears the loot bit instead.
    pub fn prune_container_loot(&mut self, source: u32, looter_player_id: i32) {
        if let Some(e) = self.get_entity_mut(source) {
            if e.is_loot_container
                && e.container_loot
                    .get(&looter_player_id)
                    .is_some_and(Vec::is_empty)
            {
                e.container_loot.remove(&looter_player_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::make_space_manager;

    fn item(index: i32) -> LootItem {
        LootItem {
            design_id: Some(2893),
            quantity: 1,
            index,
        }
    }

    /// Two looters on one container see only their own roll; a corpse shows
    /// everyone the same list.
    #[test]
    fn a_container_shows_each_looter_their_own_roll() {
        let mut mgr = make_space_manager();
        mgr.create_entity(10, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        {
            let c = mgr.get_entity_mut(10).unwrap();
            c.is_loot_container = true;
            c.container_loot.insert(1, vec![item(1)]);
            c.container_loot.insert(2, vec![item(2), item(3)]);
        }
        assert_eq!(mgr.loot_view(10, Some(1)).len(), 1);
        assert_eq!(mgr.loot_view(10, Some(2)).len(), 2);
        assert!(mgr.loot_view(10, Some(3)).is_empty());
        assert!(mgr.loot_view(10, None).is_empty());

        mgr.loot_list_mut(10, 2).unwrap().clear();
        mgr.prune_container_loot(10, 2);
        assert!(!mgr.get_entity(10).unwrap().container_loot.contains_key(&2));
        assert!(mgr.loot_list_mut(10, 2).is_none());

        mgr.create_entity(11, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        mgr.get_entity_mut(11).unwrap().loot = vec![item(1)];
        assert_eq!(mgr.loot_view(11, Some(1)).len(), 1);
        assert_eq!(mgr.loot_view(11, Some(2)).len(), 1);
    }
}
