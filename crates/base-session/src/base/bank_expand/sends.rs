//! What the player receives after an expansion: the vault's new size
//! (`onBagInfo`), the new balance (`onCashChanged`) and a chat line.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::Inventory;
use cimmeria_mercury::transport::Transport;

use super::super::feedback::{send_feedback_to_entity, send_player_method, FeedbackCtx};
use super::super::ConnectedClientState;
use super::ExpandCaller;
use crate::mercury::method_idx;

/// `onBagInfo(ARRAY<BagInfo>)` args re-declaring every container, with the
/// personal vault (17) at `bank_slots`.
///
/// The whole array, not a one-entry update: that is the shape the legacy
/// server sent whenever a bag changed size (`Inventory.py` `flushUpdates`,
/// on `bagsDirty`), and the shape the post-respawn resync already sends
/// mid-session. BV-E1 Q2 infers that a changed size raises the client's
/// `InventoryUpdateContainerSize`, which an open vault window answers by
/// re-validating its scrollbar; that is confirmed in UAT.
pub fn vault_resize_bag_info_args(bank_slots: i16) -> Vec<u8> {
    Inventory::new(0)
        .with_bank_slots(i32::from(bank_slots))
        .serialize_bag_info()
}

/// The caller's client, for the three sends.
pub(super) struct Client<'a> {
    pub(super) caller: ExpandCaller,
    pub(super) transport: &'a Arc<dyn Transport>,
    pub(super) connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub(super) entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl Client<'_> {
    fn ctx(&self) -> FeedbackCtx<'_> {
        FeedbackCtx {
            transport: self.transport,
            connected: self.connected,
        }
    }

    /// The client's address, or a `bank_feedback_send_failed` WARN when
    /// the entity has none (it logged off after the cell sent the request).
    fn addr(&self, what: &'static str) -> Option<SocketAddr> {
        let addr = self
            .entity_to_addr
            .lock()
            .ok()
            .and_then(|m| m.get(&self.caller.entity_id).copied());
        if addr.is_none() {
            tracing::warn!(
                target: "bank",
                event = "bank_feedback_send_failed",
                account_id = self.caller.account_id,
                player_id = self.caller.player_id,
                entity_id = self.caller.entity_id,
                reason = "no_client_address",
                what,
                "bank_feedback_send_failed: no client address for the entity -- the player \
                 does not see the expansion result"
            );
        }
        addr
    }

    /// Re-declare the vault at `bank_slots`.
    pub(super) async fn send_vault_size(&self, bank_slots: i16) {
        if let Some(addr) = self.addr("bag_info") {
            let args = vault_resize_bag_info_args(bank_slots);
            send_player_method(
                &self.ctx(),
                addr,
                self.caller.entity_id,
                method_idx::ON_BAG_INFO,
                &args,
            )
            .await;
        }
    }

    /// The new naquadah balance.
    pub(super) async fn send_cash(&self, naquadah: i32) {
        if let Some(addr) = self.addr("cash") {
            send_player_method(
                &self.ctx(),
                addr,
                self.caller.entity_id,
                method_idx::ON_CASH_CHANGED,
                &naquadah.to_le_bytes(),
            )
            .await;
        }
    }

    /// One chat line on the feedback channel.
    pub(super) async fn send_line(&self, text: &str) {
        if let Some(addr) = self.addr("feedback_line") {
            send_feedback_to_entity(&self.ctx(), addr, self.caller.entity_id, text).await;
        }
    }
}
