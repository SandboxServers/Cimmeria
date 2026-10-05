//! Keep content grants that `InitPlayerState` does not know about yet
//! (CS-01a review F5).
//!
//! **The race.** The base reads `sgw_player` for `InitPlayerState` on its
//! client-packet task (`onClientReady`), and handles cell messages on
//! another task. A `grant_ability` that commits after that read but whose
//! `ContentAbilitiesGranted` reaches the cell before the (slower)
//! `InitPlayerState` would be overwritten: `tree_progress` is replaced
//! wholesale, and the grant's credit and ability vanish from the cell until
//! the next world entry. The database is right; only the mirror is stale.
//!
//! **The fix: merge, don't overwrite, the credited grants.** A content grant
//! is append-only: no path deletes a non-`gm` provenance row (reset keeps
//! them, respec ignores them, only a character delete cascades). So every
//! id the cell already credits for the same character is still credited in
//! the database, and is kept, along with its ability, even when the
//! snapshot predates it. If the snapshot still lists that id as trained
//! (a grant that converted a purchase after the read), the conversion is
//! re-applied from the catalog: out of `trained_abilities`, its node cost
//! back to `training_points` and off `tree_points_spent`, as the base did.

use cimmeria_entity::cell_entity::TreeProgress;

use crate::ability_tree::AbilityTreeCatalog;
use crate::cell::space_manager::SpaceManager;

/// Apply [`merge_late_grants`] to one `InitPlayerState` before it is
/// stamped, reading the entity's current credit when it still plays
/// `player_id`.
pub(super) fn merge_into_snapshot(
    entity_id: u32,
    player_id: i32,
    archetype_id: i32,
    incoming: &mut TreeProgress,
    abilities: &mut Vec<i32>,
    space_mgr: &SpaceManager,
) {
    let previous = space_mgr
        .get_entity(entity_id)
        .filter(|e| e.player_id == Some(player_id))
        .map(|e| &e.tree_progress);
    let late = merge_late_grants(
        previous,
        incoming,
        abilities,
        &space_mgr.ability_tree_catalog,
        archetype_id,
    );
    if !late.is_empty() {
        let identity = space_mgr.player_identity(entity_id);
        let book = cimmeria_names::book();
        let names = late
            .iter()
            .map(|&id| match book.ability(id) {
                Some(name) => format!("{id}:{name}"),
                None => id.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        tracing::info!(
            target: "abilities",
            event = "init_grant_merge",
            decision_outcome = "kept_late_grants",
            entity_id,
            entity_name = identity.player_name,
            account_id = identity.account_id,
            account_name = identity.account_name,
            player_id,
            player_name = identity.player_name,
            ability_ids = ?late,
            ability_names = %names,
            "InitPlayerState predates a content grant the cell already applied; kept it"
        );
    }
}

/// Merge `previous` (the entity's credit before this `InitPlayerState`,
/// same character only) into the incoming snapshot. Returns the ids the
/// snapshot was missing.
pub(super) fn merge_late_grants(
    previous: Option<&TreeProgress>,
    incoming: &mut TreeProgress,
    abilities: &mut Vec<i32>,
    catalog: &AbilityTreeCatalog,
    archetype_id: i32,
) -> Vec<i32> {
    let Some(previous) = previous else {
        return Vec::new();
    };
    let late: Vec<i32> = previous
        .credited_grants
        .iter()
        .copied()
        .filter(|id| !incoming.credited_grants.contains(id))
        .collect();
    for &id in &late {
        incoming.credited_grants.push(id);
        if !abilities.contains(&id) {
            abilities.push(id);
        }
        if incoming.trained_abilities.contains(&id) {
            incoming.trained_abilities.retain(|&t| t != id);
            let cost = catalog
                .node(archetype_id, id)
                .map_or(0, |n| n.skill_point_cost.max(0));
            incoming.training_points = incoming.training_points.saturating_add(cost);
            incoming.tree_points_spent = (incoming.tree_points_spent - cost).max(0);
        }
    }
    late
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability_tree::TreeNode;

    const ARCH: i32 = 2;
    const SIGNATURE: i32 = 646;
    const OTHER: i32 = 597;

    fn catalog() -> AbilityTreeCatalog {
        let mut sig = TreeNode::with_defaults(ARCH, 0, SIGNATURE, 1, vec![]);
        sig.skill_point_cost = 2;
        AbilityTreeCatalog::from_nodes([sig])
    }

    fn progress(trained: Vec<i32>, spent: i32, points: i32, credited: Vec<i32>) -> TreeProgress {
        TreeProgress {
            trained_abilities: trained,
            tree_points_spent: spent,
            training_points: points,
            credited_grants: credited,
        }
    }

    /// **Guard (F5):** a grant applied before a stale snapshot survives it.
    /// Overwrite (the old code) and the credit and the ability are gone.
    #[test]
    fn a_late_grant_survives_a_stale_snapshot() {
        let previous = progress(vec![], 0, 1, vec![SIGNATURE]);
        let mut incoming = progress(vec![], 0, 1, vec![]);
        let mut abilities = vec![OTHER];
        let late = merge_late_grants(
            Some(&previous),
            &mut incoming,
            &mut abilities,
            &catalog(),
            ARCH,
        );
        assert_eq!(late, vec![SIGNATURE]);
        assert_eq!(incoming.credited_grants, vec![SIGNATURE]);
        assert_eq!(abilities, vec![OTHER, SIGNATURE]);
    }

    /// A late grant that converted a purchase is re-converted on a snapshot
    /// that still lists it as trained, so credit and spend never both count.
    #[test]
    fn a_late_conversion_is_reapplied_to_a_stale_snapshot() {
        let previous = progress(vec![], 0, 3, vec![SIGNATURE]);
        let mut incoming = progress(vec![SIGNATURE], 2, 1, vec![]);
        let mut abilities = vec![SIGNATURE];
        merge_late_grants(
            Some(&previous),
            &mut incoming,
            &mut abilities,
            &catalog(),
            ARCH,
        );
        assert_eq!(incoming, progress(vec![], 0, 3, vec![SIGNATURE]));
        assert_eq!(abilities, vec![SIGNATURE], "no duplicate");
    }

    /// A fresh snapshot that already has the grant changes nothing, and a
    /// different character (no `previous`) merges nothing.
    #[test]
    fn a_current_snapshot_or_another_character_merges_nothing() {
        let previous = progress(vec![], 0, 1, vec![SIGNATURE]);
        let mut incoming = progress(vec![], 0, 1, vec![SIGNATURE]);
        let mut abilities = vec![SIGNATURE];
        let before = incoming.clone();
        assert!(merge_late_grants(
            Some(&previous),
            &mut incoming,
            &mut abilities,
            &catalog(),
            ARCH
        )
        .is_empty());
        assert_eq!(incoming, before);
        let mut fresh = progress(vec![], 0, 1, vec![]);
        assert!(merge_late_grants(None, &mut fresh, &mut abilities, &catalog(), ARCH).is_empty());
        assert!(fresh.credited_grants.is_empty());
    }
}
