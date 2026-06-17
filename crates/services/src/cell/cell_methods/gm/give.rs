//! GM give/remove handlers — `gmGiveXp`, `gmGiveItem`, `gmGiveCash`,
//! `gmRemoveItem`. Each reuses a canonical base-side persistence sink
//! (`GrantXP` / `GrantItem` / `GrantCash` / `RemoveInventoryItem`) with
//! defense-in-depth input validation; the GM gate has already authorized the
//! caller upstream.

use cimmeria_entity::inventory::INV_MAIN;
use tokio::sync::mpsc;

use super::read_i32;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::read_wstring;

/// Upper bound on a single `gmGiveItem` grant. The audit required the chat-
/// command path clamp quantity; the native path applies the same rule so a
/// fat-fingered (or replayed) `gmGiveItem("x", 2000000000)` can't blow up the
/// inventory / DB. 1000 is generous for any legitimate GM use.
const GM_GIVE_ITEM_MAX_QTY: i32 = 1000;

/// `gmGiveXp(INT32 amount)` — grant XP to the calling GM.
///
/// Routes through the same `GrantXP` sink mob-kills use, so the base side runs
/// the normal XP→level→`onXpUpdate` path. Amount must be positive: a `<= 0`
/// grant is a no-op at best, and casting a negative `i32` to the message's
/// `u64` would produce an absurd grant.
pub(super) async fn handle_give_xp(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let amount = match read_i32(args, 0) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmGiveXp: truncated args (need INT32)"
            );
            return true;
        }
    };
    if amount <= 0 {
        tracing::warn!(entity_id, amount, "gmGiveXp: non-positive amount rejected");
        return true;
    }
    // Caller must be a player entity (XP is a player concept).
    if space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.player_id)
        .is_none()
    {
        tracing::warn!(entity_id, "gmGiveXp: caller has no player_id");
        return true;
    }
    tracing::info!(entity_id, amount, "gmGiveXp: granting XP to GM");
    let _ = tx
        .send(CellToBaseMsg::GrantXP {
            entity_id,
            xp_amount: amount as u64,
        })
        .await;
    true
}

/// `gmGiveItem(WSTRING DesignId, INT32 Quantity)` — grant the item to the
/// calling GM's own inventory.
///
/// `DesignId` is a WSTRING in the def. In the original GM console it can be a
/// numeric design id or an internal item name; we only resolve the numeric
/// form here (the name→type_id table isn't wired into the cell). A non-numeric
/// id is rejected with a warn rather than guessed. Quantity is clamped to
/// `[1, GM_GIVE_ITEM_MAX_QTY]`.
pub(super) async fn handle_give_item(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let (design_id_str, consumed) = match read_wstring(args, 0) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(entity_id, error = %e, "gmGiveItem: malformed DesignId WSTRING");
            return true;
        }
    };
    let quantity = match read_i32(args, consumed) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmGiveItem: truncated args (missing INT32 Quantity)"
            );
            return true;
        }
    };

    // DesignId resolution: numeric form only. SGW design ids are positive
    // integers; reject anything that isn't a positive i32 rather than grant a
    // bogus item_id.
    let type_id = match design_id_str.trim().parse::<i32>() {
        Ok(id) if id > 0 => id,
        _ => {
            tracing::warn!(
                entity_id,
                design_id = %design_id_str,
                "gmGiveItem: DesignId is not a positive numeric design id — \
                 internal-name resolution is not wired in the cell; rejecting"
            );
            return true;
        }
    };

    // Clamp quantity. Reject < 1 outright (a zero/negative grant is a no-op
    // at best, a stack-underflow footgun at worst).
    if quantity < 1 {
        tracing::warn!(entity_id, quantity, "gmGiveItem: quantity < 1 rejected");
        return true;
    }
    let count = quantity.min(GM_GIVE_ITEM_MAX_QTY);

    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(pid) => pid,
        None => {
            tracing::warn!(
                entity_id,
                "gmGiveItem: caller has no player_id (not a player entity)"
            );
            return true;
        }
    };

    tracing::info!(
        entity_id,
        player_id,
        type_id,
        count,
        requested_qty = quantity,
        "gmGiveItem: granting item to GM"
    );

    // Reuse the canonical grant primitive — base persists to sgw_inventory and
    // emits onUpdateItem. Container defaults to INV_Main; the base side
    // re-homes weapons/ammo to the bandolier via the item_containers cache.
    let _ = tx
        .send(CellToBaseMsg::GrantItem {
            entity_id,
            player_id,
            item_id: type_id,
            container_id: INV_MAIN,
            count,
        })
        .await;
    true
}

/// `gmGiveCash(INT32 Amount)` — grant naquadah to the calling GM.
///
/// "Give" is additive-only here: a negative amount could drive the balance
/// negative on the base side, so a `<= 0` value is rejected (there is no
/// separate remove-cash GM command in the def).
pub(super) async fn handle_give_cash(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let amount = match read_i32(args, 0) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmGiveCash: truncated args (need INT32)"
            );
            return true;
        }
    };
    if amount <= 0 {
        tracing::warn!(
            entity_id,
            amount,
            "gmGiveCash: non-positive amount rejected"
        );
        return true;
    }
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(pid) => pid,
        None => {
            tracing::warn!(entity_id, "gmGiveCash: caller has no player_id");
            return true;
        }
    };
    tracing::info!(
        entity_id,
        player_id,
        amount,
        "gmGiveCash: granting cash to GM"
    );
    let _ = tx
        .send(CellToBaseMsg::GrantCash {
            entity_id,
            player_id,
            amount,
        })
        .await;
    true
}

/// `gmGiveExpertise(INT32 aDisciplineId, INT32 aExpertise)` — grant crafting
/// expertise in one discipline to the calling GM.
///
/// One-way sink mirroring `gmGiveCash` → `GrantCash`: the base owns the
/// `CraftingState` load/clamp/save and the `onUpdateDiscipline` client push.
/// Both fields must be positive — a non-positive `aExpertise` would be a no-op
/// (or, cast through the clamp, a regression to 0), and a non-positive
/// `aDisciplineId` can't name a real discipline.
pub(super) async fn handle_give_expertise(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let discipline_id = match read_i32(args, 0) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmGiveExpertise: truncated args (need INT32 disciplineId)"
            );
            return true;
        }
    };
    let amount = match read_i32(args, 4) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmGiveExpertise: truncated args (missing INT32 expertise)"
            );
            return true;
        }
    };
    if discipline_id <= 0 {
        tracing::warn!(
            entity_id,
            discipline_id,
            "gmGiveExpertise: non-positive discipline id rejected"
        );
        return true;
    }
    if amount <= 0 {
        tracing::warn!(
            entity_id,
            amount,
            "gmGiveExpertise: non-positive amount rejected"
        );
        return true;
    }
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(pid) => pid,
        None => {
            tracing::warn!(entity_id, "gmGiveExpertise: caller has no player_id");
            return true;
        }
    };
    tracing::info!(
        entity_id,
        player_id,
        discipline_id,
        amount,
        "gmGiveExpertise: granting expertise to GM"
    );
    let _ = tx
        .send(CellToBaseMsg::GrantExpertise {
            entity_id,
            player_id,
            discipline_id,
            amount,
        })
        .await;
    true
}

/// `gmGiveAppliedSciencePoints(INT32 aPoints)` — grant applied-science points
/// to the calling GM.
///
/// One-way sink mirroring `gmGiveCash` → `GrantCash`. Additive-only: a
/// non-positive amount could drive the balance negative on the base side, so
/// `<= 0` is rejected.
pub(super) async fn handle_give_applied_science(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let amount = match read_i32(args, 0) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmGiveAppliedSciencePoints: truncated args (need INT32)"
            );
            return true;
        }
    };
    if amount <= 0 {
        tracing::warn!(
            entity_id,
            amount,
            "gmGiveAppliedSciencePoints: non-positive amount rejected"
        );
        return true;
    }
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(pid) => pid,
        None => {
            tracing::warn!(
                entity_id,
                "gmGiveAppliedSciencePoints: caller has no player_id"
            );
            return true;
        }
    };
    tracing::info!(
        entity_id,
        player_id,
        amount,
        "gmGiveAppliedSciencePoints: granting ASP to GM"
    );
    let _ = tx
        .send(CellToBaseMsg::GrantAppliedSciencePoints {
            entity_id,
            player_id,
            amount,
        })
        .await;
    true
}

/// `gmRemoveItem(ItemID itemID, INT16 quantity)` — remove a quantity of an
/// inventory **instance** from the calling GM.
///
/// `ItemID` resolves to INT32 (`entities/defs/alias.xml`); the def's quantity
/// is INT16. Both must be positive — `item_id <= 0` can't name a real instance
/// and a non-positive quantity would be a no-op (or, on the base side, an
/// unintended add). Routes through the canonical `RemoveInventoryItem` sink.
pub(super) async fn handle_remove_item(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let item_id = match read_i32(args, 0) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmRemoveItem: truncated args (need INT32 ItemID)"
            );
            return true;
        }
    };
    let quantity = match args.get(4..6) {
        Some(b) => i16::from_le_bytes([b[0], b[1]]),
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmRemoveItem: truncated args (missing INT16 quantity)"
            );
            return true;
        }
    };
    if item_id <= 0 {
        tracing::warn!(
            entity_id,
            item_id,
            "gmRemoveItem: non-positive item id rejected"
        );
        return true;
    }
    if quantity <= 0 {
        tracing::warn!(
            entity_id,
            quantity,
            "gmRemoveItem: non-positive quantity rejected"
        );
        return true;
    }
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(pid) => pid,
        None => {
            tracing::warn!(entity_id, "gmRemoveItem: caller has no player_id");
            return true;
        }
    };
    tracing::info!(
        entity_id,
        player_id,
        item_id,
        quantity,
        "gmRemoveItem: removing item from GM"
    );
    let _ = tx
        .send(CellToBaseMsg::RemoveInventoryItem {
            entity_id,
            player_id,
            item_id,
            quantity: i32::from(quantity),
        })
        .await;
    true
}
