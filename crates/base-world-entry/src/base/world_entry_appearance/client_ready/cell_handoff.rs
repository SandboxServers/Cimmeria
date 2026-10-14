//! Ordered, session-bound base-to-cell world-entry handoff.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cimmeria_wire::cell::messages::{
    BaseToCellMsg, LetterTurnInEntryEpoch, LetterTurnInSession, LetterTurnInToken,
};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

use super::super::super::ConnectedClientState;
use crate::base::PendingClientReadyInfo;

const TOKEN_DOMAIN: &[u8] = b"cimmeria:letter-turn-in:v1";
static NEXT_ENTRY_EPOCH: AtomicU64 = AtomicU64::new(0);

pub(super) fn assign_entry_epoch(
    state: &mut ConnectedClientState,
) -> Option<LetterTurnInEntryEpoch> {
    state.extensions.remove::<LetterTurnInEntryEpoch>();
    let prior = NEXT_ENTRY_EPOCH
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            value.checked_add(1)
        })
        .ok()?;
    let epoch = LetterTurnInEntryEpoch::new(prior + 1);
    state.extensions.insert(epoch);
    Some(epoch)
}

/// Identity captured with the pending finalization under one session lock.
pub(super) struct SessionSnapshot {
    cancelled: Arc<AtomicBool>,
    generation: Instant,
    token: LetterTurnInToken,
    epoch: LetterTurnInEntryEpoch,
}

impl SessionSnapshot {
    pub(super) fn capture(
        state: &ConnectedClientState,
        pending: Option<&PendingClientReadyInfo>,
    ) -> Option<Self> {
        let pending = pending?;
        let epoch = *state.extensions.get::<LetterTurnInEntryEpoch>()?;
        (state.player_entity_id == Some(pending.entity_id)
            && state.active_player_id == Some(pending.player_id)
            && !state.cancelled.load(Ordering::Acquire))
        .then(|| Self {
            cancelled: Arc::clone(&state.cancelled),
            generation: state.connected_at,
            token: token_for_key(&state.key),
            epoch,
        })
    }

    pub(super) fn matches(
        &self,
        state: &ConnectedClientState,
        entity_id: u32,
        player_id: i32,
    ) -> bool {
        Arc::ptr_eq(&self.cancelled, &state.cancelled)
            && !state.cancelled.load(Ordering::Acquire)
            && state.connected_at == self.generation
            && state.player_entity_id == Some(entity_id)
            && state.active_player_id == Some(player_id)
            && state.extensions.get::<LetterTurnInEntryEpoch>() == Some(&self.epoch)
            && token_for_key(&state.key) == self.token
    }
}

pub(super) fn token_for_key(key: &[u8; 32]) -> LetterTurnInToken {
    let mut digest = Sha256::new();
    digest.update(TOKEN_DOMAIN);
    digest.update(key);
    let hash = digest.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&hash[..16]);
    LetterTurnInToken::from_bytes(bytes)
}

/// Context for three ordered sends, with one live-session guard per permit.
pub(super) struct HandoffContext<'a> {
    pub addr: SocketAddr,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub tx: &'a mpsc::Sender<BaseToCellMsg>,
    pub session: SessionSnapshot,
    pub entity_id: u32,
    pub player_id: i32,
    pub account_id: u32,
    pub account_name: Option<&'a str>,
    pub player_name: Option<&'a str>,
    pub world_name: &'a str,
}

enum SendOutcome {
    Sent,
    Replaced,
    Closed,
}

impl HandoffContext<'_> {
    async fn send(&self, msg: BaseToCellMsg) -> SendOutcome {
        let Ok(permit) = self.tx.reserve().await else {
            return SendOutcome::Closed;
        };
        #[cfg(test)]
        if matches!(msg, BaseToCellMsg::ConnectEntity { .. }) {
            super::session_token_tests::pause_after(self.addr, "connect_permit_acquired").await;
        }
        let Ok(clients) = self.connected.lock() else {
            return SendOutcome::Replaced;
        };
        if !clients
            .get(&self.addr)
            .is_some_and(|c| self.session.matches(c, self.entity_id, self.player_id))
        {
            return SendOutcome::Replaced;
        }
        permit.send(msg);
        SendOutcome::Sent
    }
}

pub(super) async fn send_handoff(
    ctx: HandoffContext<'_>,
    init: BaseToCellMsg,
    advance_ring_destination_id: Option<i32>,
) -> bool {
    match ctx
        .send(BaseToCellMsg::ConnectEntity {
            entity_id: ctx.entity_id,
        })
        .await
    {
        SendOutcome::Sent => {}
        SendOutcome::Replaced => return false,
        SendOutcome::Closed => {
            tracing::error!(
                entity_id = ctx.entity_id,
                entity_name = ctx.player_name,
                account_id = ctx.account_id,
                account_name = ctx.account_name,
                player_id = ctx.player_id,
                player_name = ctx.player_name,
                "ConnectEntity: base→cell send failed -- cell will not see this player, all AoI traffic will drop: channel closed"
            );
            return false;
        }
    }
    #[cfg(test)]
    super::session_token_tests::pause_after(ctx.addr, "after_connect").await;

    let token = BaseToCellMsg::LetterTurnInSession {
        entity_id: ctx.entity_id,
        session: LetterTurnInSession {
            player_id: ctx.player_id,
            token: ctx.session.token,
            generation: ctx.session.generation,
            epoch: ctx.session.epoch,
        },
    };
    match ctx.send(token).await {
        SendOutcome::Sent => {}
        SendOutcome::Replaced => return false,
        SendOutcome::Closed => {
            tracing::error!(
                entity_id = ctx.entity_id,
                player_id = ctx.player_id,
                "LetterTurnInSession: base→cell send failed -- withholding InitPlayerState: channel closed"
            );
            return false;
        }
    }
    #[cfg(test)]
    super::session_token_tests::pause_after(ctx.addr, "after_token").await;

    match ctx.send(init).await {
        SendOutcome::Sent => {}
        SendOutcome::Replaced => return false,
        SendOutcome::Closed => {
            tracing::error!(
                entity_id = ctx.entity_id,
                entity_name = ctx.player_name,
                account_id = ctx.account_id,
                account_name = ctx.account_name,
                player_id = ctx.player_id,
                player_name = ctx.player_name,
                world = ctx.world_name,
                "InitPlayerState: base→cell send failed -- player loaded with empty mission/ability state: channel closed"
            );
            return false;
        }
    }
    #[cfg(test)]
    super::session_token_tests::pause_after(ctx.addr, "after_init").await;

    if let Some(region_id) = advance_ring_destination_id {
        match ctx
            .send(BaseToCellMsg::AdvanceRingDestination {
                entity_id: ctx.entity_id,
                region_id,
            })
            .await
        {
            SendOutcome::Sent | SendOutcome::Replaced => {}
            SendOutcome::Closed => {
                tracing::error!(
                    entity_id = ctx.entity_id,
                    entity_name = ctx.player_name,
                    region_id,
                    "AdvanceRingDestination: base→cell send failed -- ring FSM stuck, player invisible to other ring riders: channel closed"
                );
            }
        }
    }
    true
}
