//! Sending Team and Command vault rows (`sgw_organization_vault_items`,
//! bank-vault BV-07) to a member's client, in the same `onUpdateItem`
//! layout as the player's own rows: the select shares
//! `inventory_item_select_head!`, only the table and the filter differ.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use super::{send_update_item, InventoryRow};
use crate::base::ConnectedClientState;

/// Every row of one organization's vault, in slot order. `$1` is the org.
pub(crate) const ORG_VAULT_ITEM_SELECT: &str = concat!(
    inventory_item_select_head!("sgw_organization_vault_items"),
    "WHERE inv.org_id = $1\n",
    "ORDER BY inv.container_id, inv.slot_id\n",
);

/// Which vault rows to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OrgVaultSend {
    /// The whole vault (an open).
    All,
}

/// Read `org_id`'s vault rows through `executor` (so a caller can read under
/// the organization lock it holds) and send them in one `onUpdateItem` to
/// the member's client. Returns how many rows were sent; nothing is sent for
/// zero rows. The read error is returned for the caller to log with its own
/// event.
pub(crate) async fn send_org_vault_items_via<'c, E>(
    entity_id: u32,
    org_id: i32,
    which: OrgVaultSend,
    executor: E,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<usize, sqlx::Error>
where
    E: sqlx::PgExecutor<'c>,
{
    let rows: Vec<InventoryRow> = match which {
        OrgVaultSend::All => {
            sqlx::query_as(ORG_VAULT_ITEM_SELECT)
                .bind(org_id)
                .fetch_all(executor)
                .await?
        }
    };
    if !rows.is_empty() {
        send_update_item(entity_id, &rows, transport, connected, entity_to_addr).await;
    }
    Ok(rows.len())
}
