//! `moveItem` into, out of, or within a Team (19) or Command (20) vault
//! (bank-vault BV-07; D-BV08, D-BV12, D-BV13, D-BV15).
//!
//! The client moves org vault items with the plain `moveItem` (audit A-05).
//! [`route`] sends a move here, before any lock, when its target is 19 or
//! 20 or its item sits in `sgw_organization_vault_items`. The checks, in
//! order:
//!
//! 1. the cell's verdict: an open, in-range session of the vault's scope
//!    (`VaultAccess::org_vault_refusal`), naming the organization;
//! 2. the carried end is player-movable (1-15, the personal vault's
//!    allowlist);
//! 3. under ORG-LOCK ([`lock_actor`]): the actor's `sgw_player` row, the
//!    organization, membership and rank read under that lock;
//! 4. the per-player move lock, the source row `FOR UPDATE` (the player's
//!    own, or this organization's vault row), the carried container lock;
//! 5. the bank bit: `DepositBank` into the vault or within it,
//!    `WithdrawBank` out of it, both for a swap across;
//! 6. quantity, the vault's size, and for an item entering the vault the
//!    personal vault's rules (no mission items, `container_sets` must allow
//!    17) plus no bound items (another member could withdraw them);
//! 7. the occupant `FOR UPDATE` and the shape (whole, split, merge, swap).
//!
//! The write, its `sgw_organization_vault_log` row and the commit are one
//! transaction. Every refusal goes through [`refusal::refuse_org_move`].

use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_wire::cell::vault::VaultAccess;
use sqlx::{Postgres, Transaction};
use tracing::Instrument;

use super::super::grant::item_allows_container;
use super::bank_rules::{MoveRefusal, MoveShape};
use super::container_policy::{container_refusal, MoveEnd};
use super::finish::choose_shape;
use super::{MoveCtx, MoveRequest};
use crate::base::resources::{bag_max_slots, bag_min_slot};
use crate::base::world_entry::methods::inventory::org_vault::access::{
    lock_actor, BankBit, OrgVaultActor,
};
use crate::base::world_entry::methods::inventory::org_vault::org_label;
use refusal::{refuse_org_move, OrgMoveRefusal};
use rows::{advisory, read_occupant, read_source};
use rules::{entering_vault, has_bit};

mod apply;
mod record;
mod refusal;
mod rows;
mod rules;

/// Which table a row of the move is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Side {
    /// The player's own `sgw_inventory`.
    Carried,
    /// `sgw_organization_vault_items`.
    Vault,
}

/// Which way the item goes, relative to the vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Direction {
    Deposit,
    Withdraw,
    Within,
}

impl Direction {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Direction::Deposit => "deposit",
            Direction::Withdraw => "withdraw",
            Direction::Within => "within",
        }
    }
}

/// The org fields a refusal after the lock logs.
#[derive(Debug, Clone, Copy)]
pub(super) struct OrgActorView {
    pub org_type: &'static str,
    pub rank: u8,
}

impl From<&OrgVaultActor> for OrgActorView {
    fn from(a: &OrgVaultActor) -> Self {
        OrgActorView {
            org_type: a.access.org_type().name(),
            rank: a.access.rank().as_u8(),
        }
    }
}

/// Where [`super::move_item`] sends a move.
#[derive(Debug, Clone, Copy)]
pub(super) enum Route {
    /// Neither end is an org vault.
    Personal,
    /// An org vault move: `vault` is 19 or 20; `item_org` the organization
    /// whose vault held the item when routed (`None` when the target is
    /// the vault and the item is not in one).
    Org { vault: i32, item_org: Option<i32> },
    /// The routing read failed (logged); the move is dropped.
    Dropped,
}

/// Decide the route, before any lock: the target container, else one
/// primary-key read of the vault table. The org path re-reads everything
/// under its locks.
pub(super) async fn route(req: &MoveRequest, ctx: &MoveCtx<'_>) -> Route {
    if VaultScope::org_vault_for_container(req.target_container_id).is_some() {
        return Route::Org {
            vault: req.target_container_id,
            item_org: None,
        };
    }
    let held: Result<Option<(i32, i32)>, _> = sqlx::query_as(
        "SELECT org_id, container_id FROM sgw_organization_vault_items WHERE item_id = $1",
    )
    .bind(req.item_id)
    .fetch_optional(ctx.pool.as_ref())
    .await;
    match held {
        Ok(Some((org_id, container_id))) => Route::Org {
            vault: container_id,
            item_org: Some(org_id),
        },
        Ok(None) => Route::Personal,
        Err(e) => {
            tracing::error!(
                target: "bank",
                player_id = req.player_id,
                item_id = req.item_id,
                "MoveInventoryItem: org vault routing read failed, dropping the move: {e}"
            );
            Route::Dropped
        }
    }
}

/// An org vault move, from the verdict to the commit.
pub(super) async fn move_org_item(
    req: MoveRequest,
    vault_container: i32,
    item_org: Option<i32>,
    vault: &VaultAccess,
    ctx: &MoveCtx<'_>,
) {
    let span = tracing::info_span!(
        target: "bank",
        "bank.org_move_item",
        entity_id = req.entity_id,
        player_id = req.player_id,
        item_id = req.item_id,
        target_container_id = req.target_container_id,
        target_slot_id = req.target_slot_id,
        vault_container,
        banker_id = vault.banker_id(),
    );
    run(req, vault_container, item_org, vault, ctx)
        .instrument(span)
        .await;
}

async fn run(
    req: MoveRequest,
    vault_container: i32,
    item_org: Option<i32>,
    vault: &VaultAccess,
    ctx: &MoveCtx<'_>,
) {
    let Some(scope) = VaultScope::org_vault_for_container(vault_container) else {
        return;
    };
    let label = org_label(scope);
    let vault_end = if req.target_container_id == vault_container {
        MoveEnd::Target
    } else {
        MoveEnd::Source
    };
    let refuse = |refusal, org_id, actor: Option<OrgActorView>| async move {
        refuse_org_move(&req, refusal, org_id, label, actor, vault, ctx).await;
    };

    // 1. The cell's verdict for this vault.
    if let Some(reason) = vault.org_vault_refusal(vault_container) {
        let r = OrgMoveRefusal::Shared(MoveRefusal::VaultSession {
            end: vault_end,
            reason,
        });
        refuse(r, item_org, None).await;
        return;
    }
    let Some((_, org_id)) = vault.open_org_vault() else {
        return;
    };
    if item_org.is_some_and(|o| o != org_id) {
        // The item is in another organization's vault.
        refuse(OrgMoveRefusal::ItemNotInVault, Some(org_id), None).await;
        return;
    }

    // 2. A carried target must be player-movable and in range.
    if vault_end == MoveEnd::Source {
        if let Some(r) = container_refusal(MoveEnd::Target, req.target_container_id, vault) {
            refuse(OrgMoveRefusal::Shared(r), Some(org_id), None).await;
            return;
        }
        let (min, max) = (
            bag_min_slot(req.target_container_id),
            bag_max_slots(req.target_container_id),
        );
        if req.target_slot_id < min || req.target_slot_id >= max {
            refuse(OrgMoveRefusal::InvalidSlot, Some(org_id), None).await;
            return;
        }
    } else if req.target_slot_id < 0 {
        refuse(OrgMoveRefusal::InvalidSlot, Some(org_id), None).await;
        return;
    }

    let mut tx = match ctx.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(target: "bank", player_id = req.player_id, "org vault move: begin failed: {e}");
            return;
        }
    };
    // 3. ORG-LOCK.
    let actor = match lock_actor(&mut tx, req.player_id, org_id, scope).await {
        Ok(Ok(actor)) => actor,
        Ok(Err(miss)) => {
            let _ = tx.rollback().await;
            refuse(OrgMoveRefusal::Lock(miss), Some(org_id), None).await;
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(target: "bank", player_id = req.player_id, org_id, "org vault move: lock failed: {e}");
            return;
        }
    };
    let view = Some(OrgActorView::from(&actor));
    match locked(&mut tx, &req, org_id, vault_container, &actor, vault, ctx).await {
        Ok(Some(accepted)) => {
            if let Err(e) = record::insert_log(&mut tx, &accepted, &actor).await {
                let _ = tx.rollback().await;
                tracing::error!(target: "bank", player_id = req.player_id, org_id, "org vault move: log insert failed, rolled back: {e}");
                return;
            }
            if let Err(e) = tx.commit().await {
                tracing::error!(target: "bank", player_id = req.player_id, org_id, "org vault move: commit failed: {e}");
                return;
            }
            record::after_org_commit(&accepted, &actor, vault, ctx).await;
        }
        Ok(None) => {
            // A no-op (same slot) or an infrastructure failure, logged.
            let _ = tx.rollback().await;
        }
        Err(refusal) => {
            let _ = tx.rollback().await;
            refuse(refusal, Some(org_id), view).await;
        }
    }
}

pub(super) type MoveTx = Transaction<'static, Postgres>;

/// Everything after ORG-LOCK, up to the write. `Ok(None)` is a no-op or an
/// infrastructure failure (logged); `Err` a refusal.
async fn locked(
    tx: &mut MoveTx,
    req: &MoveRequest,
    org_id: i32,
    vault_container: i32,
    actor: &OrgVaultActor,
    vault: &VaultAccess,
    ctx: &MoveCtx<'_>,
) -> Result<Option<record::Accepted>, OrgMoveRefusal> {
    let infra = |what: &str, e: sqlx::Error| {
        tracing::error!(
            target: "bank",
            player_id = req.player_id,
            org_id,
            item_id = req.item_id,
            "org vault move: {what} failed: {e}"
        );
    };
    // 4. The move lock, then the source row.
    if let Err(e) = advisory(tx, req.player_id, 0).await {
        infra("move lock", e);
        return Ok(None);
    }
    let (source_side, source) = match read_source(tx, req, org_id).await {
        Ok(Some(found)) => found,
        Ok(None) => return Err(OrgMoveRefusal::ItemNotInVault),
        Err(e) => {
            infra("source read", e);
            return Ok(None);
        }
    };
    let target_side = if req.target_container_id == vault_container {
        Side::Vault
    } else {
        Side::Carried
    };
    let direction = match (source_side, target_side) {
        (Side::Carried, Side::Vault) => Direction::Deposit,
        (Side::Vault, Side::Carried) => Direction::Withdraw,
        (Side::Vault, Side::Vault) => Direction::Within,
        // Routed here as a vault item, but it is the player's own now.
        (Side::Carried, Side::Carried) => return Err(OrgMoveRefusal::ItemNotInVault),
    };
    if source_side == Side::Carried {
        if let Some(r) = container_refusal(MoveEnd::Source, source.container_id, vault) {
            return Err(OrgMoveRefusal::Shared(r));
        }
    }
    let carried_container = match direction {
        Direction::Deposit => Some(source.container_id),
        Direction::Withdraw => Some(req.target_container_id),
        Direction::Within => None,
    };
    if let Some(c) = carried_container {
        if let Err(e) = advisory(tx, req.player_id, c).await {
            infra("container lock", e);
            return Ok(None);
        }
    }

    // 5. The bank bit.
    let needed = match direction {
        Direction::Withdraw => BankBit::Withdraw,
        Direction::Deposit | Direction::Within => BankBit::Deposit,
    };
    has_bit(actor, needed)?;

    // 6. Quantity, size, the rules for an item entering.
    let quantity = if req.quantity <= 0 {
        source.stack_size
    } else {
        req.quantity
    };
    if quantity > source.stack_size {
        return Err(OrgMoveRefusal::QuantityExceedsStack);
    }
    if source_side == target_side
        && source.container_id == req.target_container_id
        && source.slot_id == req.target_slot_id
    {
        return Ok(None);
    }
    if target_side == Side::Vault && req.target_slot_id >= actor.vault_slots {
        return Err(OrgMoveRefusal::VaultSlotLocked {
            vault_slots: actor.vault_slots,
        });
    }
    match direction {
        Direction::Deposit => {
            entering_vault(
                tx,
                req,
                ctx,
                source.type_id,
                source.bound,
                source.container_id,
            )
            .await?
        }
        Direction::Withdraw => {
            if !item_allows_container(ctx.pool, source.type_id, req.target_container_id).await {
                return Err(OrgMoveRefusal::Shared(MoveRefusal::ItemNotAllowed {
                    end: MoveEnd::Target,
                }));
            }
        }
        Direction::Within => {}
    }

    // 7. The occupant and the shape.
    let occupant = match read_occupant(tx, req, org_id, target_side).await {
        Ok(o) => o,
        Err(e) => {
            infra("occupant read", e);
            return Ok(None);
        }
    };
    let shape = match choose_shape(tx, req, quantity, &source, occupant.as_ref()).await {
        Ok(shape) => shape,
        Err(Some(r)) => return Err(OrgMoveRefusal::Shared(r)),
        Err(None) => return Ok(None),
    };
    let mut perm = needed.name();
    if let (MoveShape::Swap, Some(occ)) = (shape, occupant.as_ref()) {
        match direction {
            // The vault's occupant comes out into the source bag.
            Direction::Deposit => {
                has_bit(actor, BankBit::Withdraw)?;
                perm = "DepositBank+WithdrawBank";
                if !item_allows_container(ctx.pool, occ.type_id, source.container_id).await {
                    return Err(OrgMoveRefusal::Shared(MoveRefusal::ItemNotAllowed {
                        end: MoveEnd::Source,
                    }));
                }
            }
            // The player's occupant goes into the vault.
            Direction::Withdraw => {
                has_bit(actor, BankBit::Deposit)?;
                perm = "DepositBank+WithdrawBank";
                entering_vault(
                    tx,
                    req,
                    ctx,
                    occ.type_id,
                    occ.bound,
                    req.target_container_id,
                )
                .await?;
            }
            Direction::Within => {}
        }
    }

    let plan = apply::Plan {
        player_id: req.player_id,
        org_id,
        org_type: i16::from(actor.access.org_type().as_u8()),
        item_id: req.item_id,
        source_side,
        target_side,
        source,
        occupant,
        shape,
        quantity,
        target_container_id: req.target_container_id,
        target_slot_id: req.target_slot_id,
    };
    let Some(applied) = apply::apply(tx, &plan).await else {
        return Ok(None);
    };
    Ok(Some(record::Accepted {
        plan,
        applied,
        entity_id: req.entity_id,
        direction,
        perm,
    }))
}

/// A move whose item the player does not hold, made while an org vault
/// session is open: most likely a vault item another member moved since
/// this client last saw the vault. Refuse it visibly and take the stale
/// item out of the window.
pub(super) async fn refuse_stale_vault_item(
    req: MoveRequest,
    vault: &VaultAccess,
    ctx: &MoveCtx<'_>,
) {
    let Some((scope, org_id)) = vault.open_org_vault() else {
        return;
    };
    refuse_org_move(
        &req,
        OrgMoveRefusal::ItemNotInVault,
        Some(org_id),
        org_label(scope),
        None,
        vault,
        ctx,
    )
    .await;
}
