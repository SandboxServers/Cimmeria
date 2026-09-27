//! Which carried bag each purchased line lands in.
//!
//! A purchase grants into the first carried bag the item's `container_sets`
//! lists ([`first_player_container`]), so a `{17,15}` crafting component
//! goes to the crafting bag instead of the main bag it does not allow. An
//! item that lists no carried bag (a mission item) keeps the main bag, as
//! every purchase did before.

use std::collections::{BTreeMap, HashMap};

use cimmeria_cell_catalog::item_placement::{first_player_container, skipped_storage, INV_MAIN};
use sqlx::{Postgres, Transaction};

use super::super::serializers::reserve_free_inventory_slots;

/// Where one purchase's lines go.
#[derive(Debug)]
pub(super) struct LinePlacement {
    /// The container per line, in line order.
    pub containers: Vec<i32>,
    /// Each design's `container_sets`, for the telemetry.
    pub container_sets: HashMap<i32, Vec<i32>>,
    pub account_id: Option<i32>,
}

impl LinePlacement {
    /// Whether the line for `design_id` placed at `container_id` passed over
    /// a storage container its `container_sets` lists first.
    pub(super) fn skipped_storage(&self, design_id: i32, container_id: i32) -> bool {
        self.container_sets
            .get(&design_id)
            .is_some_and(|sets| skipped_storage(sets, INV_MAIN, container_id))
    }

    /// `design_id`'s `container_sets` as the telemetry writes it: `{17,15}`.
    pub(super) fn container_sets_text(&self, design_id: i32) -> String {
        let sets = self
            .container_sets
            .get(&design_id)
            .map_or(&[][..], Vec::as_slice);
        let inner: Vec<String> = sets.iter().map(i32::to_string).collect();
        format!("{{{}}}", inner.join(","))
    }
}

/// The carried bag for an item with these `container_sets`.
pub(super) fn purchase_container(container_sets: &[i32]) -> i32 {
    first_player_container(container_sets).unwrap_or(INV_MAIN)
}

/// Read every line's `container_sets` (and the buyer's account id) and
/// choose each line's bag.
pub(super) async fn place_lines(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    design_ids: &[i32],
) -> Result<LinePlacement, sqlx::Error> {
    let rows: Vec<(i32, Vec<i32>, Option<i32>)> = sqlx::query_as(
        "SELECT ri.item_id, ri.container_sets, p.account_id \
           FROM resources.items ri \
           LEFT JOIN sgw_player p ON p.player_id = $2 \
          WHERE ri.item_id = ANY($1)",
    )
    .bind(design_ids)
    .bind(player_id)
    .fetch_all(&mut **tx)
    .await?;
    let account_id = rows.iter().find_map(|(_, _, account)| *account);
    let container_sets: HashMap<i32, Vec<i32>> =
        rows.into_iter().map(|(id, sets, _)| (id, sets)).collect();
    let containers = design_ids
        .iter()
        .map(|id| {
            container_sets
                .get(id)
                .map_or(INV_MAIN, |sets| purchase_container(sets))
        })
        .collect();
    Ok(LinePlacement {
        containers,
        container_sets,
        account_id,
    })
}

/// Reserve one free slot per line in its container. The per-bag locks are
/// taken in ascending container order, so two purchases that span the same
/// bags cannot wait on each other. `Ok(None)` when a bag has too few free
/// slots.
pub(super) async fn reserve_line_slots(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
    containers: &[i32],
) -> Result<Option<Vec<i32>>, sqlx::Error> {
    let mut needed: BTreeMap<i32, usize> = BTreeMap::new();
    for &c in containers {
        *needed.entry(c).or_default() += 1;
    }
    let mut free: HashMap<i32, std::vec::IntoIter<i32>> = HashMap::new();
    for (&container_id, &count) in &needed {
        match reserve_free_inventory_slots(tx, player_id, container_id, count).await? {
            Some(slots) => {
                free.insert(container_id, slots.into_iter());
            }
            None => return Ok(None),
        }
    }
    let mut slots = Vec::with_capacity(containers.len());
    for c in containers {
        match free.get_mut(c).and_then(Iterator::next) {
            Some(slot) => slots.push(slot),
            None => return Ok(None),
        }
    }
    Ok(Some(slots))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bank_first_component_is_bought_into_the_crafting_bag() {
        assert_eq!(purchase_container(&[17, 15]), 15);
    }

    #[test]
    fn main_bag_items_and_mission_items_keep_the_main_bag() {
        assert_eq!(purchase_container(&[3, 1, 17]), INV_MAIN);
        assert_eq!(purchase_container(&[1, 7, 17]), INV_MAIN);
        assert_eq!(purchase_container(&[2]), INV_MAIN);
        assert_eq!(purchase_container(&[]), INV_MAIN);
    }
}
