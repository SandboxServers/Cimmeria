//! Which bags a trade takes items from, and where each item lands.
//!
//! A trade may take items from the backpack (1) and the crafting bag (15)
//! ([`TRADEABLE_CONTAINERS`]). Each item lands in its recipient's bag by
//! the item's own `resources.items.container_sets`, with the grant rule
//! ([`grant_container`]) and the bag it came from as the request: a
//! crafting component (`{17,15}`) goes from bag 15 to bag 15, a backpack
//! item stays in the backpack, and a component someone still holds in
//! the backpack moves to the crafting bag. The wire carries only instance
//! ids, so nothing here is client-supplied.
//!
//! Slots are reserved per (recipient, bag), lowest free first. A bag the
//! recipient is trading items out of counts those slots as free, since
//! the same transaction vacates them. A full destination bag refuses the
//! whole trade; it never spills into the other bag.

use cimmeria_cell_catalog::item_placement::grant_container;
use cimmeria_entity::inventory::{INV_CRAFTING, INV_MAIN};
use sqlx::{Postgres, Transaction};

use super::super::super::vendor::serializers::free_inventory_slots;
use super::swap::TradeItemRow;
use super::{TradeAbort, TradeSide};
use crate::base::resources::{bag_max_slots, bag_min_slot};

/// Containers an item may sit in to be eligible for trade: the backpack
/// and the crafting bag, where crafting components live.
///
/// Players who want to trade equipped gear, vault items or bandolier
/// weapons must unequip / withdraw / unload first. Anything outside this
/// list is refused with [`TradeAbort::IneligibleContainer`]; the server
/// decides this independently of the client, which sends only instance
/// ids.
///
/// **Do not add `INV_BUYBACK` (16) here** — buyback bag items must
/// remain reclaimable only by their original seller. Nor the vaults
/// (17-20): they are reachable only through a banker.
pub(super) const TRADEABLE_CONTAINERS: &[i32] = &[INV_MAIN, INV_CRAFTING];

/// One committed item move, for the post-commit telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ItemMove {
    pub(super) item_id: i32,
    pub(super) type_id: i32,
    pub(super) from_entity: u32,
    pub(super) from_player: i32,
    pub(super) from_container: i32,
    pub(super) from_slot: i32,
    pub(super) to_entity: u32,
    pub(super) to_player: i32,
    pub(super) to_container: i32,
    pub(super) to_slot: i32,
}

impl ItemMove {
    pub(super) fn new(
        row: &TradeItemRow,
        from: &TradeSide,
        to: &TradeSide,
        to_container: i32,
        to_slot: i32,
    ) -> Self {
        Self {
            item_id: row.item_id,
            type_id: row.type_id,
            from_entity: from.entity_id,
            from_player: from.player_id,
            from_container: row.container_id,
            from_slot: row.slot_id,
            to_entity: to.entity_id,
            to_player: to.player_id,
            to_container,
            to_slot,
        }
    }
}

/// The bag an item from `source_container` lands in, given its type's
/// `container_sets`: always one of [`TRADEABLE_CONTAINERS`], or `None`.
///
/// `({17,15}, 15)` and `({17,15}, 1)` give 15; `({3,1,17}, 1)` gives 1;
/// `({2}, 1)` gives 1 (an item that lists no carried bag keeps the bag it
/// was traded from); `({}, 15)` gives 1.
pub(super) fn trade_destination(container_sets: &[i32], source_container: i32) -> Option<i32> {
    grant_container(container_sets, source_container)
        .filter(|dest| TRADEABLE_CONTAINERS.contains(dest))
}

/// The destination bag of each of `rows`, in order. An item whose type is
/// missing from `resources.items` is refused rather than guessed at.
pub(super) fn plan_destinations(
    rows: &[TradeItemRow],
    player_id: i32,
    which: &'static str,
) -> Result<Vec<i32>, TradeAbort> {
    rows.iter()
        .map(|row| {
            let dest = row
                .known_type
                .then(|| {
                    trade_destination(
                        row.container_sets.as_deref().unwrap_or_default(),
                        row.container_id,
                    )
                })
                .flatten();
            dest.ok_or(TradeAbort::NoDestination {
                which,
                player_id,
                item_id: row.item_id,
                type_id: row.type_id,
            })
        })
        .collect()
}

/// Reserve a `(container_id, slot_id)` for each incoming item, whose
/// destination bags are `incoming_dest`, in the recipient's bags. The
/// recipient's own outgoing rows free their slots.
///
/// The caller already holds the recipient's advisory keys for both
/// tradeable bags; this locks the bag rows `FOR UPDATE`.
pub(super) async fn reserve_slots(
    tx: &mut Transaction<'_, Postgres>,
    recipient_player_id: i32,
    incoming_dest: &[i32],
    recipient_outgoing: &[TradeItemRow],
) -> Result<Vec<(i32, i32)>, TradeAbort> {
    let mut picked = vec![(0, 0); incoming_dest.len()];
    for &container_id in TRADEABLE_CONTAINERS {
        let wanted: Vec<usize> = (0..incoming_dest.len())
            .filter(|&i| incoming_dest[i] == container_id)
            .collect();
        if wanted.is_empty() {
            continue;
        }
        let raw_occupied: Vec<i32> = sqlx::query_scalar(
            "SELECT slot_id FROM sgw_inventory \
             WHERE character_id = $1 AND container_id = $2 \
             FOR UPDATE",
        )
        .bind(recipient_player_id)
        .bind(container_id)
        .fetch_all(&mut **tx)
        .await?;
        let vacating = vacating_slots(recipient_outgoing, container_id);
        let slots = pick_free_slots_excluding(container_id, &raw_occupied, &vacating, wanted.len())
            .ok_or_else(|| TradeAbort::NotEnoughSlots {
                recipient_player_id,
                container_id,
                needed: wanted.len(),
                free: free_slot_count(container_id, &raw_occupied, &vacating),
            })?;
        for (i, slot_id) in wanted.into_iter().zip(slots) {
            picked[i] = (container_id, slot_id);
        }
    }
    Ok(picked)
}

/// Slots in `container_id` that `rows` (the recipient's own outgoing
/// items) vacate in the same transaction.
pub(super) fn vacating_slots(rows: &[TradeItemRow], container_id: i32) -> Vec<i32> {
    rows.iter()
        .filter(|r| r.container_id == container_id)
        .map(|r| r.slot_id)
        .collect()
}

/// Pure slot pick: given the recipient's current occupancy of
/// `container_id` and the slots they vacate in the same transaction,
/// the lowest `needed` slots that will be free post-swap, or `None` if
/// the bag can't fit them.
pub(super) fn pick_free_slots_excluding(
    container_id: i32,
    raw_occupied: &[i32],
    vacating: &[i32],
    needed: usize,
) -> Option<Vec<i32>> {
    free_inventory_slots(
        bag_min_slot(container_id),
        bag_max_slots(container_id),
        &occupied_after_exclusion(raw_occupied, vacating),
        needed,
    )
}

fn occupied_after_exclusion(raw_occupied: &[i32], vacating: &[i32]) -> Vec<i32> {
    raw_occupied
        .iter()
        .copied()
        .filter(|slot| !vacating.contains(slot))
        .collect()
}

/// Free slots the bag will have post-swap, for the refusal's telemetry.
fn free_slot_count(container_id: i32, raw_occupied: &[i32], vacating: &[i32]) -> usize {
    let (min, max) = (bag_min_slot(container_id), bag_max_slots(container_id));
    let taken = occupied_after_exclusion(raw_occupied, vacating)
        .into_iter()
        .filter(|s| (min..max).contains(s))
        .count();
    usize::try_from(max - min)
        .unwrap_or(0)
        .saturating_sub(taken)
}
