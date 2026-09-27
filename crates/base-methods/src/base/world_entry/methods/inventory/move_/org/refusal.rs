//! Refusing a Team or Command vault move (bank-vault BV-07): the `bank`
//! `org_move_rejected` event, the chat line, and the snap-back.
//!
//! The client has no reply method for a vault move: `onOrgMoveItemResult`
//! is a server-internal *cell* method (`SGWInventoryManager.def`, a
//! `PYTHON` argument, no Lua subscriber; audit A-13), so the refusal is the
//! same line plus snap-back the personal vault uses (BV-03).
//!
//! The snap-back resends the dragged item from wherever the server has it,
//! under the same locks a move takes (the actor's `sgw_player` row, the
//! organization, the `(player, 0)` move lock, then the row), so no committed
//! move can overtake the packet:
//!
//! - in the player's own inventory: `onUpdateItem` of that row;
//! - in this organization's vault: `onUpdateItem` of the vault row;
//! - neither (another member moved it since this client last saw the vault,
//!   and the fan-out did not reach this client): `onRemoveItem`, so
//!   the stale item leaves the window.

use sqlx::{Postgres, Transaction};

use cimmeria_base_session::base::organization::api::lock_org;
use cimmeria_wire::cell::vault::VaultAccess;

use super::super::super::core::{
    send_inventory_item_update_via, send_on_remove_item, send_org_vault_items_via, OrgVaultSend,
};
use super::super::bank_rules::MoveRefusal;
use super::super::{MoveCtx, MoveRequest};
use super::OrgActorView;
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::base::world_entry::methods::inventory::org_vault::access::{BankBit, OrgLockMiss};

/// Why an org vault move is refused. [`OrgMoveRefusal::reason`] is the
/// stable `org_move_rejected reason=`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum OrgMoveRefusal {
    /// A refusal the personal vault shares: an unmovable container, a
    /// closed or out-of-range session, a mission item, an item the vault
    /// does not take, a split onto an occupied slot.
    Shared(MoveRefusal),
    /// The lock-time authorization failed.
    Lock(OrgLockMiss),
    /// The rank lacks the bank bit this move needs.
    MissingPermission(BankBit),
    /// The dragged item is not where the client thinks: not the player's
    /// and not in this organization's vault.
    ItemNotInVault,
    /// A vault slot at or beyond the vault's size.
    VaultSlotLocked { vault_slots: i32 },
    /// A carried slot outside its container.
    InvalidSlot,
    /// More than the stack holds.
    QuantityExceedsStack,
    /// A bound item bound for a shared vault.
    BoundItem,
    /// A database failure or a write that matched the wrong rows; logged
    /// where it happened. The move rolled back.
    MoveFailed,
}

impl OrgMoveRefusal {
    pub(super) fn reason(self) -> &'static str {
        match self {
            OrgMoveRefusal::Shared(r) => r.reason(),
            OrgMoveRefusal::Lock(miss) => miss.reason(),
            OrgMoveRefusal::MissingPermission(_) => "missing_permission",
            OrgMoveRefusal::ItemNotInVault => "item_not_in_vault",
            OrgMoveRefusal::VaultSlotLocked { .. } => "target_slot_beyond_vault_slots",
            OrgMoveRefusal::InvalidSlot => "invalid_target_slot",
            OrgMoveRefusal::QuantityExceedsStack => "quantity_exceeds_stack",
            OrgMoveRefusal::BoundItem => "bound_item_not_org_storable",
            OrgMoveRefusal::MoveFailed => "move_failed",
        }
    }

    /// The bank bit this refusal is about, for the `perm` field.
    pub(super) fn perm(self) -> Option<&'static str> {
        match self {
            OrgMoveRefusal::MissingPermission(bit) => Some(bit.name()),
            _ => None,
        }
    }

    pub(super) fn vault_end(self) -> Option<&'static str> {
        match self {
            OrgMoveRefusal::Shared(r) => r.vault_end(),
            OrgMoveRefusal::VaultSlotLocked { .. } | OrgMoveRefusal::BoundItem => Some("target"),
            _ => None,
        }
    }

    /// The line the player sees before the snap-back.
    pub(super) fn feedback(self, org: &str) -> String {
        match self {
            OrgMoveRefusal::Shared(MoveRefusal::NotPlayerMovable(_)) => {
                format!("Items can only move between your bags and the {org} vault.")
            }
            OrgMoveRefusal::Shared(MoveRefusal::VaultSession { reason, .. }) => match reason {
                "banker_out_of_range" | "banker_other_space" | "vault_session_other_space" => {
                    format!("You are too far from the Banker. Return to the Banker to use the {org} vault.")
                }
                "vault_scope_mismatch" => {
                    format!("The {org} vault is not open. Visit a {org} Banker to use it.")
                }
                _ => format!("The {org} vault is closed. Visit a {org} Banker to use it."),
            },
            OrgMoveRefusal::Shared(MoveRefusal::MissionItem) => {
                format!("Mission items cannot be stored in the {org} vault.")
            }
            OrgMoveRefusal::Shared(other) => other
                .feedback()
                .unwrap_or_else(|| "That item cannot be placed there.".to_owned()),
            OrgMoveRefusal::Lock(OrgLockMiss::NotAMember | OrgLockMiss::WrongOrgType) => {
                format!("You are no longer in this {org}, so you cannot use its vault.")
            }
            OrgMoveRefusal::Lock(OrgLockMiss::NoSuchOrg) => format!("That {org} no longer exists."),
            OrgMoveRefusal::Lock(OrgLockMiss::PlayerMissing) => {
                "The item could not be moved. Please try again.".to_owned()
            }
            OrgMoveRefusal::MissingPermission(BankBit::Deposit) => {
                format!("Your {org} rank cannot deposit items in the {org} vault.")
            }
            OrgMoveRefusal::MissingPermission(BankBit::Withdraw) => {
                format!("Your {org} rank cannot withdraw items from the {org} vault.")
            }
            OrgMoveRefusal::ItemNotInVault => {
                format!("That item is no longer in the {org} vault.")
            }
            OrgMoveRefusal::VaultSlotLocked { vault_slots } => {
                format!("That vault slot is locked. The {org} vault has {vault_slots} slots.")
            }
            OrgMoveRefusal::InvalidSlot => "That item cannot be placed there.".to_owned(),
            OrgMoveRefusal::QuantityExceedsStack => "That stack is not that large.".to_owned(),
            OrgMoveRefusal::BoundItem => {
                format!("Bound items cannot be stored in the {org} vault.")
            }
            OrgMoveRefusal::MoveFailed => {
                "The item could not be moved. Please try again.".to_owned()
            }
        }
    }
}

/// The dragged item as found at refusal time, for the log.
#[derive(Debug, Default, sqlx::FromRow)]
struct Found {
    type_id: Option<i32>,
    stack_size: Option<i32>,
    container_id: Option<i32>,
    slot_id: Option<i32>,
}

/// Where the snap-back found the item.
enum Where {
    Carried(Found),
    Vault(Found),
    Nowhere,
}

/// Refuse `req`: log, tell, snap back. `org_id` is the vault's organization
/// when known; `actor` the rank read under the lock, when the refusal came
/// after it. The caller has already rolled back its own transaction.
pub(super) async fn refuse_org_move(
    req: &MoveRequest,
    refusal: OrgMoveRefusal,
    org_id: Option<i32>,
    org_label: &str,
    actor: Option<OrgActorView>,
    vault: &VaultAccess,
    ctx: &MoveCtx<'_>,
) {
    let mut tx = match ctx.pool.begin().await {
        Ok(tx) => Some(tx),
        Err(e) => {
            tracing::warn!(
                target: "bank",
                event = "org_move_rejected",
                player_id = req.player_id,
                entity_id = req.entity_id,
                item_id = req.item_id,
                reason = "move_lock_begin_failed",
                "org_move_rejected: begin failed, not resyncing without the locks: {e}"
            );
            None
        }
    };
    let (account_id, found) = match tx.as_mut() {
        Some(tx) => match lock_and_find(tx, req, org_id).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    target: "bank",
                    event = "org_move_rejected",
                    player_id = req.player_id,
                    entity_id = req.entity_id,
                    item_id = req.item_id,
                    reason = "move_lock_failed",
                    "org_move_rejected: the refusal's locks or read failed, not resyncing: {e}"
                );
                (None, None)
            }
        },
        None => (None, None),
    };
    let item = match &found {
        Some(Where::Carried(f) | Where::Vault(f)) => Some(f),
        _ => None,
    };
    tracing::warn!(
        target: "bank",
        event = "org_move_rejected",
        account_id,
        player_id = req.player_id,
        entity_id = req.entity_id,
        org_id,
        org_type = actor.map(|a| a.org_type),
        rank = actor.map(|a| a.rank),
        perm = refusal.perm(),
        item_id = req.item_id,
        type_id = item.and_then(|f| f.type_id),
        quantity = req.quantity,
        stack_size = item.and_then(|f| f.stack_size),
        source_container_id = item.and_then(|f| f.container_id),
        source_slot_id = item.and_then(|f| f.slot_id),
        target_container_id = req.target_container_id,
        target_slot_id = req.target_slot_id,
        reason = refusal.reason(),
        vault_end = refusal.vault_end(),
        vault_slots = match refusal {
            OrgMoveRefusal::VaultSlotLocked { vault_slots } => Some(vault_slots),
            _ => None,
        },
        banker_id = vault.banker_id(),
        distance = vault.distance(),
        "org_move_rejected: item stays put (snap-back follows unless move_resync_skipped)"
    );
    send_line(req, &refusal.feedback(org_label), ctx).await;
    let Some(mut tx) = tx else {
        return;
    };
    match found {
        Some(Where::Carried(_)) => {
            send_inventory_item_update_via(
                req.entity_id,
                req.player_id,
                req.item_id,
                &mut *tx,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await;
        }
        Some(Where::Vault(_)) => {
            if let Some(org_id) = org_id {
                if let Err(e) = send_org_vault_items_via(
                    req.entity_id,
                    org_id,
                    OrgVaultSend::Ids(vec![req.item_id]),
                    &mut *tx,
                    ctx.transport,
                    ctx.connected,
                    ctx.entity_to_addr,
                )
                .await
                {
                    tracing::warn!(
                        target: "bank",
                        event = "move_resync_skipped",
                        player_id = req.player_id,
                        entity_id = req.entity_id,
                        item_id = req.item_id,
                        reason = "resync_read_failed",
                        "move_resync_skipped: could not read the vault row back: {e}"
                    );
                }
            }
        }
        Some(Where::Nowhere) => {
            send_on_remove_item(
                req.entity_id,
                req.item_id,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await;
        }
        None => {}
    }
    let _ = tx.rollback().await; // Defensible silent: a read-only transaction; the locks go either way.
}

/// Take the move's locks in its order (the move lock, the actor's
/// `sgw_player` row, the organization) and find the dragged item. A vault
/// row is resent only while the player is a member of that organization.
async fn lock_and_find(
    tx: &mut Transaction<'static, Postgres>,
    req: &MoveRequest,
    org_id: Option<i32>,
) -> Result<(Option<i32>, Option<Where>), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1, 0)")
        .bind(req.player_id)
        .execute(&mut **tx)
        .await?;
    let account_id: Option<i32> =
        sqlx::query_scalar("SELECT account_id FROM sgw_player WHERE player_id = $1 FOR KEY SHARE")
            .bind(req.player_id)
            .fetch_optional(&mut **tx)
            .await?;
    let member_org = match org_id {
        Some(org_id) if lock_org(tx, org_id).await?.is_some() => {
            let member: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM sgw_organization_members \
                 WHERE org_id = $1 AND player_id = $2)",
            )
            .bind(org_id)
            .bind(req.player_id)
            .fetch_one(&mut **tx)
            .await?;
            member.then_some(org_id)
        }
        _ => None,
    };
    let carried: Option<Found> = sqlx::query_as(
        "SELECT type_id, stack_size, container_id, slot_id FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2 FOR UPDATE",
    )
    .bind(req.player_id)
    .bind(req.item_id)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(f) = carried {
        return Ok((account_id, Some(Where::Carried(f))));
    }
    if let Some(org_id) = member_org {
        let vaulted: Option<Found> = sqlx::query_as(
            "SELECT type_id, stack_size, container_id, slot_id FROM sgw_organization_vault_items \
             WHERE org_id = $1 AND item_id = $2 FOR UPDATE",
        )
        .bind(org_id)
        .bind(req.item_id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(f) = vaulted {
            return Ok((account_id, Some(Where::Vault(f))));
        }
    }
    Ok((account_id, Some(Where::Nowhere)))
}

async fn send_line(req: &MoveRequest, text: &str, ctx: &MoveCtx<'_>) {
    let addr = ctx
        .entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&req.entity_id).copied());
    let Some(addr) = addr else {
        tracing::warn!(
            target: "bank",
            event = "bank_feedback_send_failed",
            player_id = req.player_id,
            entity_id = req.entity_id,
            item_id = req.item_id,
            reason = "no_client_address",
            "bank_feedback_send_failed: no client address for the org vault refusal line"
        );
        return;
    };
    let fctx = FeedbackCtx {
        transport: ctx.transport,
        connected: ctx.connected,
    };
    send_feedback_line(&fctx, addr, text).await;
}
