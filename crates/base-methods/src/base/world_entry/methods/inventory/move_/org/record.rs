//! What a committed Team or Command vault move leaves behind (bank-vault
//! BV-07): the `sgw_organization_vault_log` row (written in the move
//! transaction), the `org_move_accepted` event, and the client updates.

use cimmeria_entity::known_names;
use sqlx::{Postgres, Transaction};

use cimmeria_base_session::base::organization::handlers::{broadcast_to_org_except, OrgCtx};

use super::super::super::core::{
    org_vault_update_args, send_on_remove_item, send_org_vault_items_via, OrgVaultSend,
};
use super::super::after_commit::{after_commit, AppliedMove};
use super::super::bank_rules::MoveShape;
use super::super::MoveCtx;
use super::apply::{Applied, Plan};
use super::{Direction, Side};
use crate::base::world_entry::methods::inventory::org_vault::access::OrgVaultActor;
use crate::mercury::method_idx;

/// One committed vault move, as the log and the event record it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Accepted {
    pub plan: Plan,
    pub applied: Applied,
    pub entity_id: u32,
    pub direction: Direction,
    /// The bank bits the move needed, for the `perm` field.
    pub perm: &'static str,
}

impl Accepted {
    /// `deposit`, `withdraw` or `within` for a whole stack into an empty
    /// slot; otherwise the write (`split`, `merge`, `swap`). The same labels
    /// as the personal vault's `move_accepted`.
    pub(super) fn kind(&self) -> &'static str {
        match self.plan.shape {
            MoveShape::Split => "split",
            MoveShape::Merge => "merge",
            MoveShape::Swap => "swap",
            MoveShape::Whole => self.direction.as_str(),
        }
    }

    /// `(source_before, source_after, target_before, target_after)`.
    fn stacks(&self) -> (i32, i32, i32, i32) {
        let src = self.plan.source.stack_size;
        let q = self.plan.quantity;
        let occ = self.plan.occupant.map_or(0, |o| o.stack_size);
        let (src_after, dst_after) = match self.plan.shape {
            MoveShape::Whole => (0, src),
            MoveShape::Split => (src - q, q),
            MoveShape::Merge => (src - q, occ + q),
            MoveShape::Swap => (occ, src),
        };
        (src, src_after, occ, dst_after)
    }

    /// The vault rows the client must be sent after the commit.
    fn vault_rows(&self) -> Vec<i32> {
        let p = &self.plan;
        let occ = p.occupant.map(|o| o.item_id);
        let mut ids = Vec::new();
        if p.target_side == Side::Vault {
            match p.shape {
                MoveShape::Whole | MoveShape::Swap => ids.push(p.item_id),
                MoveShape::Split => ids.extend(self.applied.new_item_id),
                MoveShape::Merge => ids.extend(occ),
            }
        }
        if p.source_side == Side::Vault {
            match p.shape {
                MoveShape::Split => ids.push(p.item_id),
                MoveShape::Merge if !self.applied.source_deleted => ids.push(p.item_id),
                MoveShape::Swap => ids.extend(occ),
                _ => {}
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// The rows that left the vault (now in the mover's bags, or merged
    /// away): the other members' clients must drop them.
    fn left_vault(&self) -> Vec<i32> {
        let p = &self.plan;
        let occ = p.occupant.map(|o| o.item_id);
        match (p.source_side, p.target_side, p.shape) {
            (Side::Vault, Side::Carried, MoveShape::Whole | MoveShape::Swap) => vec![p.item_id],
            (Side::Vault, _, MoveShape::Merge) if self.applied.source_deleted => vec![p.item_id],
            (Side::Carried, Side::Vault, MoveShape::Swap) => occ.into_iter().collect(),
            _ => Vec::new(),
        }
    }
}

/// Write the log row, in the move transaction.
pub(super) async fn insert_log(
    tx: &mut Transaction<'static, Postgres>,
    a: &Accepted,
    actor: &OrgVaultActor,
) -> Result<(), sqlx::Error> {
    let (sb, sa, tb, ta) = a.stacks();
    let p = &a.plan;
    sqlx::query(
        "INSERT INTO sgw_organization_vault_log \
         (org_id, org_type, account_id, player_id, rank, direction, kind, item_id, new_item_id, \
          type_id, quantity, source_container_id, source_slot_id, target_container_id, \
          target_slot_id, source_stack_before, source_stack_after, target_stack_before, \
          target_stack_after) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
                 $18, $19)",
    )
    .bind(p.org_id)
    .bind(p.org_type)
    .bind(actor.account_id)
    .bind(p.player_id)
    .bind(i16::from(actor.access.rank().as_u8()))
    .bind(a.direction.as_str())
    .bind(a.kind())
    .bind(p.item_id)
    .bind(a.applied.new_item_id)
    .bind(p.source.type_id)
    .bind(p.quantity)
    .bind(p.source.container_id)
    .bind(p.source.slot_id)
    .bind(p.target_container_id)
    .bind(p.target_slot_id)
    .bind(sb)
    .bind(sa)
    .bind(tb)
    .bind(ta)
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

/// After the commit: `org_move_accepted`, then the client updates.
pub(super) async fn after_org_commit(
    a: &Accepted,
    actor: &OrgVaultActor,
    vault: &cimmeria_wire::cell::vault::VaultAccess,
    ctx: &MoveCtx<'_>,
) {
    let (sb, sa, tb, ta) = a.stacks();
    let p = &a.plan;
    tracing::debug!(
        target: "bank",
        event = "org_move_accepted",
        account_id = actor.account_id,
        account_name = known_names::account_name(actor.account_id),
        player_id = p.player_id,
        player_name = known_names::player_name(p.player_id),
        entity_id = a.entity_id,
        entity_name = known_names::player_name(p.player_id),
        org_id = p.org_id,
        org_name = known_names::org_name(p.org_id),
        org_type = actor.access.org_type().name(),
        rank = actor.access.rank().as_u8(),
        perm = a.perm,
        item_id = p.item_id,
        item_name = cimmeria_names::book().item(p.source.type_id),
        new_item_id = a.applied.new_item_id,
        new_item_name = cimmeria_names::book().item(p.source.type_id),
        item_type_id = p.source.type_id,
        quantity = p.quantity,
        kind = a.kind(),
        direction = a.direction.as_str(),
        source_container_id = p.source.container_id,
        source_container_name = cimmeria_names::book().container(p.source.container_id),
        source_slot_id = p.source.slot_id, // nt:id-only slot index, unnamed
        target_container_id = p.target_container_id,
        target_container_name = cimmeria_names::book().container(p.target_container_id),
        target_slot_id = p.target_slot_id, // nt:id-only slot index, unnamed
        source_stack_before = sb,
        source_stack_after = sa,
        target_stack_before = tb,
        target_stack_after = ta,
        vault_slots = actor.vault_slots,
        banker_id = vault.banker_id(), // nt:id-only banker NPC, unnamed on the base
        distance = vault.distance(),
        "org_move_accepted: org vault move committed and logged"
    );

    // A deleted row is not in either table's resend, so the client needs
    // its removal first (onUpdateItem only upserts).
    if a.applied.source_deleted {
        send_on_remove_item(
            a.entity_id,
            p.item_id,
            ctx.transport,
            ctx.connected,
            ctx.entity_to_addr,
        )
        .await;
    }
    let rows = a.vault_rows();
    if let Err(e) = send_org_vault_items_via(
        a.entity_id,
        p.org_id,
        OrgVaultSend::Ids(rows.clone()),
        ctx.pool.as_ref(),
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
    )
    .await
    {
        tracing::warn!(
            target: "bank",
            event = "org_move_resync_failed",
            account_id = actor.account_id,
            account_name = known_names::account_name(actor.account_id),
            player_id = p.player_id,
            player_name = known_names::player_name(p.player_id),
            entity_id = a.entity_id,
            entity_name = known_names::player_name(p.player_id),
            org_id = p.org_id,
            org_name = known_names::org_name(p.org_id),
            item_id = p.item_id,
            item_name = cimmeria_names::book().item(p.source.type_id),
            reason = "vault_read_failed",
            "org_move_resync_failed: the move committed but its vault rows could not be read \
             back; the client shows the old vault until it reopens: {e}"
        );
    }
    fan_out(a, actor, rows, ctx).await;
    if a.direction == Direction::Within {
        return;
    }
    // The carried end changed: the player's own resync, the cell's
    // notification, the bandolier and the appearance, as for any move.
    let applied_item_id = match (p.target_side, p.shape) {
        (Side::Carried, MoveShape::Split) => a.applied.new_item_id.unwrap_or(p.item_id),
        _ => p.item_id,
    };
    after_commit(
        AppliedMove {
            entity_id: a.entity_id,
            player_id: p.player_id,
            item_id: p.item_id,
            applied_item_id,
            type_id: p.source.type_id,
            source_container_id: p.source.container_id,
            target_container_id: p.target_container_id,
            swapped_item_id: match p.shape {
                MoveShape::Swap => p.occupant.map(|o| o.item_id),
                _ => None,
            },
            // Sent above, for either side.
            source_deleted: false,
        },
        ctx.pool,
        ctx.db_pool,
        ctx.cell_tx,
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
    )
    .await;
}

/// Tell every other online member of the organization what changed in the
/// vault: `onUpdateItem` of the rows now there, and `onRemoveItem` of the
/// rows that left, through ORG-07's `broadcast_to_org`. The mover is left
/// out: its own client already has both, and a removal of a withdrawn item
/// would take it out of the mover's bag. Every member's client caches the
/// vault's rows, open window or not, so it is current on the next open.
async fn fan_out(a: &Accepted, actor: &OrgVaultActor, rows: Vec<i32>, ctx: &MoveCtx<'_>) {
    let p = &a.plan;
    let org = OrgCtx {
        db_pool: ctx.db_pool,
        transport: ctx.transport,
        connected: ctx.connected,
        entity_to_addr: ctx.entity_to_addr,
        cell_tx: ctx.cell_tx,
    };
    let mut updated = 0;
    match org_vault_update_args(p.org_id, rows, ctx.pool.as_ref()).await {
        Ok(Some(args)) => {
            updated = broadcast_to_org_except(
                &org,
                p.org_id,
                method_idx::ON_UPDATE_ITEM,
                &args,
                None,
                p.player_id,
                "vault_rows",
            )
            .await;
        }
        Ok(None) => {}
        Err(e) => tracing::warn!(
            target: "bank",
            event = "org_move_resync_failed",
            account_id = actor.account_id,
            account_name = known_names::account_name(actor.account_id),
            player_id = p.player_id,
            player_name = known_names::player_name(p.player_id),
            entity_id = a.entity_id,
            entity_name = known_names::player_name(p.player_id),
            org_id = p.org_id,
            org_name = known_names::org_name(p.org_id),
            item_id = p.item_id,
            item_name = cimmeria_names::book().item(p.source.type_id),
            reason = "fanout_read_failed",
            "org_move_resync_failed: the other members were not sent the vault rows: {e}"
        ),
    }
    let left = a.left_vault();
    let mut removed = 0;
    if !left.is_empty() {
        let mut args = (left.len() as u32).to_le_bytes().to_vec();
        for id in &left {
            args.extend_from_slice(&id.to_le_bytes());
        }
        removed = broadcast_to_org_except(
            &org,
            p.org_id,
            method_idx::ON_REMOVE_ITEM,
            &args,
            None,
            p.player_id,
            "vault_removed",
        )
        .await;
    }
    tracing::debug!(
        target: "bank",
        event = "org_vault_fanout",
        account_id = actor.account_id,
        account_name = known_names::account_name(actor.account_id),
        player_id = p.player_id,
        player_name = known_names::player_name(p.player_id),
        entity_id = a.entity_id,
        entity_name = known_names::player_name(p.player_id),
        org_id = p.org_id,
        org_name = known_names::org_name(p.org_id),
        item_id = p.item_id,
        item_name = cimmeria_names::book().item(p.source.type_id),
        updated_recipients = updated,
        removed_ids = left.len(),
        removed_recipients = removed,
        "org_vault_fanout: the other online members were sent the vault change"
    );
}
