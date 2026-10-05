//! What the actor of a treasury change receives: the new wallet
//! (`onCashChanged` [75]), the treasury (`onOrganizationCashUpdate` [48],
//! on a refusal's resync; the success goes to every member through
//! `broadcast_to_org`) and a chat line.
//!
//! Addressed to the **character** (D-BV33): the session whose active player
//! is the actor, resolved at the send, then that session's current player
//! entity. A character that gated or logged off in between gets nothing
//! (world entry sends both balances again), and a recycled entity id never
//! receives another player's balance.

use std::net::SocketAddr;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_organization_cash_update, ON_ORGANIZATION_CASH_UPDATE,
};

use super::super::feedback::{
    send_to_current_player, FeedbackCtx, FeedbackOutcome, FEEDBACK_SPEAKER,
};
use super::super::organization::handlers::{OrgCtx, OrgPlayer};
use super::super::session_identity::identity_for_entity;
use crate::mercury::method_idx;

/// The actor's client.
pub struct Actor<'a> {
    pub ctx: &'a OrgCtx<'a>,
    pub player: OrgPlayer,
}

impl Actor<'_> {
    fn addr(&self) -> Option<SocketAddr> {
        let clients = self.ctx.connected.lock().ok()?;
        clients
            .iter()
            .find(|(_, c)| c.active_player_id == Some(self.player.player_id))
            .map(|(addr, _)| *addr)
    }

    /// Send one method to the actor's character, or log
    /// `bank_feedback_send_failed` with why it was dropped. `what` names
    /// the send in that log.
    pub async fn send(&self, what: &'static str, method_index: u16, payload: &[u8]) {
        let outcome = match self.addr() {
            None => Err("no_client_address"),
            Some(addr) => {
                let fctx = FeedbackCtx {
                    transport: self.ctx.transport,
                    connected: self.ctx.connected,
                };
                match send_to_current_player(
                    &fctx,
                    addr,
                    self.player.player_id,
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
            let who = identity_for_entity(
                self.ctx.connected,
                self.ctx.entity_to_addr,
                self.player.entity_id,
            );
            tracing::warn!(
                target: "bank",
                event = "bank_feedback_send_failed",
                account_id = self.player.account_id,
                account_name = who.account_name,
                player_id = self.player.player_id,
                player_name = who.player_name,
                entity_id = self.player.entity_id,
                entity_name = who.player_name,
                reason,
                what,
                "bank_feedback_send_failed: the character is not in the world -- the player \
                 does not see the treasury result now (world entry resends both balances)"
            );
        }
    }

    /// The wallet's balance.
    pub async fn send_cash(&self, naquadah: i32) {
        self.send("cash", method_idx::ON_CASH_CHANGED, &naquadah.to_le_bytes())
            .await;
    }

    /// The treasury's balance, to the actor alone.
    pub async fn send_org_cash(&self, org_id: i32, cash: i64) {
        let args = build_on_organization_cash_update(org_id, u64::try_from(cash).unwrap_or(0));
        self.send("org_cash", ON_ORGANIZATION_CASH_UPDATE, &args)
            .await;
    }

    /// One chat line on the feedback channel.
    pub async fn send_line(&self, text: &str) {
        let payload = serialize_on_player_communication(FEEDBACK_SPEAKER, 0, CHAN_FEEDBACK, text);
        self.send(
            "feedback_line",
            method_idx::ON_PLAYER_COMMUNICATION,
            &payload,
        )
        .await;
    }
}
