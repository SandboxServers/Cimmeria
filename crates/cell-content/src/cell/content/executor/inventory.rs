//! Inventory action handlers: `Action::GrantItem` and `Action::RemoveItem`
//! (with by-instance vs by-type fork).

use std::collections::HashMap;

use serde_json::Value;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{vault_access, SpaceManager};
use cimmeria_wire::cell::vault::VaultAccess;

/// Determine the inventory container for an item from the DB-loaded map.
/// Falls back to INV_Main (1) if the item has no explicit container_sets entry.
pub(in crate::cell::content) fn item_container(
    item_id: i32,
    item_containers: &HashMap<i32, i32>,
) -> i32 {
    *item_containers.get(&item_id).unwrap_or(&1)
}

/// `Action::GrantItem` — ask the base to add an item to the player's
/// inventory.
///
/// A weapon granted into the bandolier is NOT written into the cell's
/// bandolier here. The base picks the slot (the first free one), so a guess
/// at the active slot overwrote whatever weapon was already there, with no
/// later correction (CS-01b review). The base's `UpdateBandolierItem`, sent
/// after the row commits, is authoritative: it inserts the real slot,
/// instance id and ammo (0: every gun is acquired empty, OD-CS13) and seeds
/// the AmmoSlot{N} stat the client's counter reads.
pub(super) async fn grant(
    item_id: i32,
    count: i32,
    container_id: Option<i32>,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    tracing::info!(
        entity_id,
        entity_name = space_mgr.entity_names(entity_id).entity_name,
        item_id,
        item_name = cimmeria_names::book().item(item_id),
        count,
        chain_id,
        chain_name = cimmeria_names::book().chain(chain_id),
        "Content: granting item"
    );
    let cid = container_id
        .filter(|&c| c > 0)
        .unwrap_or_else(|| item_container(item_id, &space_mgr.item_containers));

    if let Err(e) = tx
        .send(CellToBaseMsg::GrantItem {
            entity_id,
            player_id,
            item_id,
            container_id: cid,
            count,
            // Content-chain grant is not GM-sourced — no GM feedback line.
            notify_gm: false,
            loot: None,
        })
        .await
    {
        let id = space_mgr.player_identity(entity_id);
        let names = cimmeria_names::book();
        tracing::error!(
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            player_id,
            player_name = id.player_name,
            item_id,
            item_name = names.item(item_id),
            container_id = cid,
            container_name = names.container(cid),
            count,
            chain_id,
            chain_name = names.chain(chain_id),
            error = %e,
            "GrantItem send to base failed -- item not persisted to inventory"
        );
    }
}

/// `Action::RemoveItem` — consume the originating inventory `instance_id`
/// from chain context if present (set by `fire_item_use` — the OnItemUse
/// dispatch path) for "consume the slappack the player clicked", otherwise
/// fall back to by-type resolution.
pub(super) async fn remove(
    item_id: i32,
    count: i32,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    params: &HashMap<String, Value>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    // Prefer the originating inventory `instance_id` if the
    // chain context carries one (set by `fire_item_use` —
    // the OnItemUse dispatch path). This is the difference
    // between "consume the slappack the player clicked" and
    // "consume the leftmost slappack of that type": when a
    // player has two stacks of the same item and clicks the
    // second one, the first stack must NOT be silently
    // consumed instead.
    //
    // For chains fired by other paths (mission events,
    // ambernol vial consumption via `enter_region`, etc.)
    // there's no instance_id in context, so we fall back to
    // the by-type resolution that picks the player's first
    // matching instance. Both paths converge on the same
    // wire-update sequence on the base side.
    let instance_id = params
        .get("instance_id")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .filter(|&v| v != 0);

    let send_result = match instance_id {
        Some(instance) => {
            {
                let names = cimmeria_names::book();
                tracing::info!(
                    entity_id,
                    entity_name = space_mgr.entity_names(entity_id).entity_name,
                    player_id,
                    player_name = space_mgr.player_identity(entity_id).player_name,
                    instance,
                    item_type_id = item_id,
                    item_name = names.item(item_id),
                    count,
                    chain_id,
                    chain_name = names.chain(chain_id),
                    "Content: RemoveItem → RemoveInventoryItem (by instance from context)"
                );
            }
            tx.send(CellToBaseMsg::RemoveInventoryItem {
                entity_id,
                player_id,
                item_id: instance,
                quantity: count,
                // Content-chain remove is not GM-sourced — no GM feedback line.
                notify_gm: false,
                // The instance the player just used: in the vault (17) only
                // if the use passed a vault session, so it is consumed under
                // the same live verdict (BV-03).
                vault: vault_access(entity_id, space_mgr),
            })
            .await
        }
        None => {
            {
                let names = cimmeria_names::book();
                tracing::info!(
                    entity_id,
                    entity_name = space_mgr.entity_names(entity_id).entity_name,
                    player_id,
                    player_name = space_mgr.player_identity(entity_id).player_name,
                    item_type_id = item_id,
                    item_name = names.item(item_id),
                    count,
                    chain_id,
                    chain_name = names.chain(chain_id),
                    "Content: RemoveItem → RemoveInventoryItemByType"
                );
            }
            tx.send(CellToBaseMsg::RemoveInventoryItemByType {
                entity_id,
                player_id,
                type_id: item_id,
                count,
                // A by-type removal (a turn-in) never reaches into the
                // vault, whether or not its window is open: the bank is
                // storage only (D-BV04), so what a chain consumes must not
                // depend on UI state (BV-03 review).
                vault: VaultAccess::NO_SESSION,
            })
            .await
        }
    };

    if let Err(e) = send_result {
        // Saturated/closed channel — the consume silently
        // skips otherwise. Surface it loudly so missions
        // that depend on the removal (e.g., FindAmbernol
        // chain 1034 consumes the vial) don't silently
        // strand the player with the item still in their
        // bag while the chain reports completion.
        let names = cimmeria_names::book();
        tracing::error!(
            entity_id,
            entity_name = space_mgr.entity_names(entity_id).entity_name,
            player_id,
            player_name = space_mgr.player_identity(entity_id).player_name,
            item_type_id = item_id,
            item_name = names.item(item_id),
            count,
            chain_id,
            chain_name = names.chain(chain_id),
            error = %e,
            "Content: RemoveItem cell→base channel send failed — item NOT removed"
        );
    }
}
