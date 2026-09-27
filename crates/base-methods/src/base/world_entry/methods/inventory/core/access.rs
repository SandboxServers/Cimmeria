//! Using or removing an item needs it in a container the player can reach
//! (bank-vault BV-03). `useItem`, `removeItem`, content `RemoveItem` and
//! `gmRemoveItem` all name an item by id (or by type) and used to find it in
//! any container: an item in buyback (16) could be used, and a banked item
//! (17) could be used from anywhere. The rule is
//! [`player_accessible`](super::super::move_::player_accessible), beside the
//! move allowlist: 1-15, and 17 only with a vault session.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::INV_BANK;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::vault::VaultAccess;
use sqlx::PgPool;

use super::super::move_::player_accessible;
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::base::ConnectedClientState;

/// Which path was refused, for the `op` field.
#[derive(Debug, Clone, Copy)]
pub(super) enum AccessOp {
    /// `useItem`.
    Use,
    /// `removeItem`, content `RemoveItem` by instance, or `gmRemoveItem`.
    Remove,
}

impl AccessOp {
    fn label(self) -> &'static str {
        match self {
            AccessOp::Use => "use",
            AccessOp::Remove => "remove",
        }
    }
}

/// The containers a by-type removal may search: 1-15, plus 17 when the
/// vault is open. Content `RemoveItem` binds this as `container_id =
/// ANY($n)`, so an instance in buyback or a closed vault is never consumed.
pub(super) fn accessible_containers(vault: &VaultAccess) -> Vec<i32> {
    // Every container id the client knows (`Constants.py` BAG_SIZES, 1-20),
    // through the one rule.
    (1..=20).filter(|c| player_accessible(*c, vault)).collect()
}

/// Refuse a use or removal of an item in a container the player cannot
/// reach: log `use_rejected` (WARN, target `bank`) and tell the player why.
pub(super) async fn refuse_inaccessible(
    op: AccessOp,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container_id: i32,
    vault: &VaultAccess,
    pool: &PgPool,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let account_id: Option<i32> =
        match sqlx::query_scalar("SELECT account_id FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_optional(pool)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                tracing::warn!(
                    target: "bank",
                    event = "use_rejected",
                    player_id,
                    entity_id,
                    item_id,
                    reason = "account_lookup_failed",
                    "use_rejected: could not read the account for the refusal: {e}"
                );
                None
            }
        };
    tracing::warn!(
        target: "bank",
        event = "use_rejected",
        account_id,
        player_id,
        entity_id,
        item_id,
        container = container_id,
        op = op.label(),
        reason = "container_not_accessible",
        vault_reason = vault.reason(),
        banker_id = vault.banker_id(),
        "use_rejected: the item is not in a container the player can reach"
    );
    let text = if container_id == INV_BANK {
        "That item is in your vault. Visit a Banker to use your vault."
    } else {
        "That item is not in your inventory."
    };
    let addr = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied());
    let Some(addr) = addr else {
        tracing::warn!(
            target: "bank",
            event = "bank_feedback_send_failed",
            player_id,
            entity_id,
            item_id,
            reason = "no_client_address",
            "bank_feedback_send_failed: no client address for the use refusal line"
        );
        return;
    };
    let ctx = FeedbackCtx {
        transport,
        connected,
    };
    send_feedback_line(&ctx, addr, text).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::cell_entity::VaultScope;

    #[test]
    fn by_type_search_covers_the_vault_only_when_open() {
        let closed = accessible_containers(&VaultAccess::NO_SESSION);
        assert_eq!(closed, (1..=15).collect::<Vec<_>>());
        let open = accessible_containers(&VaultAccess::Open {
            org_id: None,
            scope: VaultScope::Personal,
            banker_id: None,
            distance: None,
        });
        assert!(open.contains(&INV_BANK));
        assert!(!open.contains(&16), "buyback is never searched");
    }
}
