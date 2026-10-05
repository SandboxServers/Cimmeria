//! Opening a Team or Command vault on the base (bank-vault BV-07): the
//! answer to the cell's `BankCellToBase::OrgVaultOpen`.
//!
//! The player's Team or Command is found by type (a player has at most one
//! of each, D-ORG18), then membership is read under the organization lock
//! ([`super::access::lock_actor`]). While the lock is held the base sends
//! `onBagInfo`, declaring the vault at its real size, and the vault's rows
//! in one `onUpdateItem`, so the client never sees a half-moved vault. It
//! then logs `org_vault_opened` and tells the cell to open the window
//! (`BankBaseToCell::OrgVaultGranted`). Every refusal logs
//! `org_vault_open_rejected` with a stable `reason` and sends a chat line.
//!
//! Opening needs no bank bit: every member may look (D-BV12). The bits are
//! checked per move.

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_entity::inventory::{Inventory, INV_TEAM_BANK};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::core::{send_org_vault_items_via, OrgVaultSend};
use super::access::{lock_actor, team_vault_slots, OrgLockMiss, OrgVaultActor};
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::base::helpers::send_to_witness_reliable;
use crate::base::world_entry::space_registry;
use crate::base::ConnectedClientState;
use crate::cell::messages::{BankBaseToCell, BaseToCellMsg};
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// One org vault open, as the cell forwarded it.
#[derive(Debug, Clone, Copy)]
pub struct OrgVaultOpenRequest {
    pub entity_id: u32,
    pub account_id: Option<u32>,
    pub player_id: Option<i32>,
    pub scope: VaultScope,
    pub banker_id: u32,
    pub distance: Option<f32>,
    pub space_id: u32,
}

/// What the open needs to reach the database, the cell and the client.
pub struct OrgVaultIo<'a> {
    pub db_pool: &'a Option<Arc<PgPool>>,
    pub cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// Why an org vault did not open on the base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenRefusal {
    /// The cell had no `player_id` for the entity.
    PlayerUnknown,
    /// The player is in no Team (or no Command).
    NotInOrg,
    /// The lock-time check failed.
    Lock(OrgLockMiss),
    /// A database error; logged with it.
    QueryFailed,
}

impl OpenRefusal {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            OpenRefusal::PlayerUnknown => "player_unknown",
            OpenRefusal::NotInOrg => "not_in_org",
            OpenRefusal::Lock(miss) => miss.reason(),
            OpenRefusal::QueryFailed => "open_query_failed",
        }
    }

    fn feedback(self, scope: VaultScope) -> String {
        let org = org_label(scope);
        match self {
            OpenRefusal::NotInOrg | OpenRefusal::Lock(OrgLockMiss::NotAMember) => {
                format!("You are not in a {org}, so there is no {org} vault to open.")
            }
            OpenRefusal::Lock(OrgLockMiss::NoSuchOrg | OrgLockMiss::WrongOrgType) => {
                format!("Your {org} no longer exists.")
            }
            OpenRefusal::PlayerUnknown
            | OpenRefusal::Lock(OrgLockMiss::PlayerMissing)
            | OpenRefusal::QueryFailed => {
                format!("The {org} vault could not be opened. Please try again.")
            }
        }
    }
}

/// "Team" or "Command", for the player's lines.
pub(crate) fn org_label(scope: VaultScope) -> &'static str {
    match scope {
        VaultScope::Command => "Command",
        _ => "Team",
    }
}

/// The `onBagInfo` args an org vault open sends: every container at its
/// usual size, the personal vault (17) at `bank_slots`, the Team vault (19)
/// at `team_slots` and the Command vault (20) at its fixed 100. The whole
/// set, the same shape world entry and the resync send, so a client that
/// re-registers its containers from it loses none.
pub fn org_vault_bag_info(bank_slots: i32, team_slots: i32) -> Vec<u8> {
    Inventory::new(0)
        .with_bank_slots(bank_slots)
        .with_org_vault_slots(INV_TEAM_BANK, team_slots)
        .serialize_bag_info()
}

/// Handle `BankCellToBase::OrgVaultOpen`.
#[tracing::instrument(
    name = "bank.org_vault_open",
    level = "info",
    skip_all,
    fields(
        entity_id = req.entity_id,
        player_id = req.player_id,
        scope = req.scope.as_str(),
        banker_id = req.banker_id
    )
)]
pub async fn handle_org_vault_open(req: OrgVaultOpenRequest, io: OrgVaultIo<'_>) {
    let Some(pool) = io.db_pool else {
        tracing::debug!(
            entity_id = req.entity_id,
            entity_name = known_names::player_name(req.player_id),
            "OrgVaultOpen: no DB pool"
        );
        return;
    };
    let Some(player_id) = req.player_id else {
        refuse(&req, OpenRefusal::PlayerUnknown, None, None, &io).await;
        return;
    };
    match open(&req, player_id, pool, &io).await {
        Ok((org_id, actor, item_count)) => {
            granted(&req, player_id, org_id, &actor, item_count, &io).await;
        }
        Err((refusal, org_id, account_id)) => {
            refuse(&req, refusal, org_id, account_id, &io).await;
        }
    }
}

/// The locked part: find the org, authorize, send size and contents.
/// `Err` carries the refusal plus whatever ids were read before it.
async fn open(
    req: &OrgVaultOpenRequest,
    player_id: i32,
    pool: &Arc<PgPool>,
    io: &OrgVaultIo<'_>,
) -> Result<(i32, OrgVaultActor, usize), (OpenRefusal, Option<i32>, Option<i32>)> {
    let failed = |what: &str, e: sqlx::Error, org_id: Option<i32>| {
        let player_label = known_names::player_name(player_id);
        tracing::error!(
            target: "bank",
            player_id,
            player_name = player_label,
            entity_id = req.entity_id,
            entity_name = player_label,
            org_id,
            org_name = known_names::org_name(org_id),
            "OrgVaultOpen: {what} failed: {e}"
        );
        (OpenRefusal::QueryFailed, org_id, None)
    };
    let org_type = req.scope.org_type().map_or(0, |t| i16::from(t.as_u8()));
    // Unlocked: it only picks which organization to lock. Membership is
    // re-read under the lock below.
    let org_id: Option<i32> = sqlx::query_scalar(
        "SELECT org_id FROM sgw_organization_members WHERE player_id = $1 AND org_type = $2",
    )
    .bind(player_id)
    .bind(org_type)
    .fetch_optional(pool.as_ref())
    .await
    .map_err(|e| failed("membership lookup", e, None))?;
    let Some(org_id) = org_id else {
        return Err((OpenRefusal::NotInOrg, None, None));
    };

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| failed("begin", e, Some(org_id)))?;
    let actor = match lock_actor(&mut tx, player_id, org_id, req.scope).await {
        Ok(Ok(actor)) => actor,
        Ok(Err(miss)) => return Err((OpenRefusal::Lock(miss), Some(org_id), None)),
        Err(e) => return Err(failed("lock", e, Some(org_id))),
    };
    let team_slots = if req.scope == VaultScope::Team {
        actor.vault_slots
    } else {
        team_vault_slots(&mut tx, player_id)
            .await
            .map_err(|e| failed("team size read", e, Some(org_id)))?
    };

    // Size first, so the rows land in a container the client has sized.
    let bag_info = org_vault_bag_info(i32::from(actor.bank_slots), team_slots);
    send_to_witness_reliable(
        io.transport,
        io.connected,
        io.entity_to_addr,
        req.entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                req.entity_id,
                method_idx::ON_BAG_INFO,
                &bag_info,
                version,
            )
        },
    )
    .await;
    let item_count = send_org_vault_items_via(
        req.entity_id,
        org_id,
        OrgVaultSend::All,
        &mut *tx,
        io.transport,
        io.connected,
        io.entity_to_addr,
    )
    .await
    .map_err(|e| failed("vault read", e, Some(org_id)))?;
    // Read-only: nothing to commit, the rollback releases the locks.
    let _ = tx.rollback().await; // Defensible silent: a read-only transaction; the locks go either way.
    Ok((org_id, actor, item_count))
}

/// Log `org_vault_opened` and tell the cell to open the window.
async fn granted(
    req: &OrgVaultOpenRequest,
    player_id: i32,
    org_id: i32,
    actor: &OrgVaultActor,
    item_count: usize,
    io: &OrgVaultIo<'_>,
) {
    let perms = actor.access.permissions();
    let player_label = known_names::player_name(player_id);
    tracing::debug!(
        target: "bank",
        event = "org_vault_opened",
        account_id = actor.account_id,
        account_name = known_names::account_name(actor.account_id),
        player_id,
        player_name = player_label,
        entity_id = req.entity_id,
        entity_name = player_label,
        org_id,
        org_name = known_names::org_name(org_id),
        org_type = actor.access.org_type().name(),
        rank = actor.access.rank().as_u8(),
        perm = "none",
        permissions = perms.bits(),
        can_deposit = perms.contains(cimmeria_entity::organization::OrgPermission::DEPOSIT_BANK),
        can_withdraw = perms.contains(cimmeria_entity::organization::OrgPermission::WITHDRAW_BANK),
        scope = req.scope.as_str(),
        vault_slots = actor.vault_slots,
        item_count,
        banker_id = req.banker_id, // nt:id-only banker NPC, unnamed on the base
        space_id = req.space_id,
        world = space_registry::world_for_space(req.space_id),
        distance = req.distance,
        "org_vault_opened: size and contents sent, asking the cell to open the window"
    );
    let Some(cell_tx) = io.cell_tx else {
        return;
    };
    let grant = BaseToCellMsg::Bank(BankBaseToCell::OrgVaultGranted {
        entity_id: req.entity_id,
        player_id,
        scope: req.scope,
        org_id,
        banker_id: req.banker_id,
    });
    if let Err(e) = cell_tx.send(grant).await {
        let player_label = known_names::player_name(player_id);
        tracing::warn!(
            target: "bank",
            event = "org_vault_open_rejected",
            account_id = actor.account_id,
            account_name = known_names::account_name(actor.account_id),
            player_id,
            player_name = player_label,
            entity_id = req.entity_id,
            entity_name = player_label,
            org_id,
            org_name = known_names::org_name(org_id),
            reason = "cell_channel_closed",
            "org_vault_open_rejected: the grant could not reach the cell, so no window opens: {e}"
        );
    }
}

/// Log `org_vault_open_rejected` and send the player the line.
async fn refuse(
    req: &OrgVaultOpenRequest,
    refusal: OpenRefusal,
    org_id: Option<i32>,
    db_account_id: Option<i32>,
    io: &OrgVaultIo<'_>,
) {
    let account_id = db_account_id.or(req.account_id.and_then(|a| i32::try_from(a).ok()));
    let player_label = known_names::player_name(req.player_id);
    tracing::warn!(
        target: "bank",
        event = "org_vault_open_rejected",
        account_id,
        account_name = known_names::account_name(account_id),
        player_id = req.player_id,
        player_name = player_label,
        entity_id = req.entity_id,
        entity_name = player_label,
        org_id,
        org_name = known_names::org_name(org_id),
        org_type = req.scope.org_type().map(|t| t.name()),
        scope = req.scope.as_str(),
        banker_id = req.banker_id, // nt:id-only banker NPC, unnamed on the base
        space_id = req.space_id,
        world = space_registry::world_for_space(req.space_id),
        distance = req.distance,
        reason = refusal.reason(),
        "org_vault_open_rejected: the vault did not open -- the player sees a chat line saying why"
    );
    let addr = io
        .entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&req.entity_id).copied());
    let Some(addr) = addr else {
        let player_label = known_names::player_name(req.player_id);
        tracing::warn!(
            target: "bank",
            event = "bank_feedback_send_failed",
            account_id,
            account_name = known_names::account_name(account_id),
            player_id = req.player_id,
            player_name = player_label,
            entity_id = req.entity_id,
            entity_name = player_label,
            reason = "no_client_address",
            "bank_feedback_send_failed: no client address for the org vault refusal line"
        );
        return;
    };
    let ctx = FeedbackCtx {
        transport: io.transport,
        connected: io.connected,
    };
    send_feedback_line(&ctx, addr, &refusal.feedback(req.scope)).await;
}
