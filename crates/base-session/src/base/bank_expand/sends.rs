//! What the player receives after an expansion: the vault's new size
//! (`onBagInfo`), the new balance (`onCashChanged`) and a chat line.
//!
//! Every send is addressed to the **character**, not to the entity id the
//! cell sent: the session playing `player_id` is looked up now, and the
//! method goes to whatever player entity it has at the moment of the send
//! (`send_to_current_player`). A player who gated or logged out between the
//! cell's request and the base's answer gets nothing (the next world entry
//! declares the vault from the database anyway), and a recycled entity id
//! never delivers one player's vault size or balance to another.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::Inventory;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::super::feedback::{
    send_to_current_player, FeedbackCtx, FeedbackOutcome, FEEDBACK_SPEAKER,
};
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
    /// What asked for the purchase; `None` for the quote.
    pub(super) trigger: Option<cimmeria_wire::cell::messages::ExpandTrigger>,
    pub(super) transport: &'a Arc<dyn Transport>,
    pub(super) connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
}

impl Client<'_> {
    /// The session playing the caller's character now.
    fn addr(&self) -> Option<SocketAddr> {
        let clients = self.connected.lock().ok()?;
        clients
            .iter()
            .find(|(_, c)| c.active_player_id == Some(self.caller.player_id))
            .map(|(addr, _)| *addr)
    }

    /// Send one method to the caller's character, or log
    /// `bank_feedback_send_failed` with why it was dropped.
    async fn send(&self, what: &'static str, method_index: u16, payload: &[u8]) {
        let outcome = match self.addr() {
            None => Err("no_client_address"),
            Some(addr) => {
                let ctx = FeedbackCtx {
                    transport: self.transport,
                    connected: self.connected,
                };
                match send_to_current_player(
                    &ctx,
                    addr,
                    self.caller.player_id,
                    method_index,
                    payload,
                )
                .await
                .0
                {
                    FeedbackOutcome::Sent => Ok(()),
                    FeedbackOutcome::NoSession => Err("no_session"),
                    FeedbackOutcome::NotInWorld => Err("not_in_world"),
                    FeedbackOutcome::SendError => Err("send_error"),
                }
            }
        };
        if let Err(reason) = outcome {
            tracing::warn!(
                target: "bank",
                event = "bank_feedback_send_failed",
                account_id = self.caller.account_id,
                player_id = self.caller.player_id,
                entity_id = self.caller.entity_id,
                reason,
                what,
                "bank_feedback_send_failed: the character is not in the world -- the player \
                 does not see the expansion result now (world entry re-declares the vault)"
            );
        }
    }

    /// Re-declare the vault at `bank_slots`.
    pub(super) async fn send_vault_size(&self, bank_slots: i16) {
        let args = vault_resize_bag_info_args(bank_slots);
        self.send("bag_info", method_idx::ON_BAG_INFO, &args).await;
    }

    /// The new naquadah balance.
    pub(super) async fn send_cash(&self, naquadah: i32) {
        self.send("cash", method_idx::ON_CASH_CHANGED, &naquadah.to_le_bytes())
            .await;
    }

    /// One chat line on the feedback channel.
    pub(super) async fn send_line(&self, text: &str) {
        let payload = serialize_on_player_communication(FEEDBACK_SPEAKER, 0, CHAN_FEEDBACK, text);
        self.send(
            "feedback_line",
            method_idx::ON_PLAYER_COMMUNICATION,
            &payload,
        )
        .await;
    }
}
