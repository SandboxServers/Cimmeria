//! One-use result authority retained after normal SmartFox teardown.

use std::time::Duration;

use tokio::time::Instant;

use super::{MinigameSession, SessionRegistry, SessionRegistryInner};

pub(super) const RESULT_RECEIPT_TTL: Duration = Duration::from_secs(180);
pub(super) const RESULT_RECEIPT_CAP: usize = 4096;

/// Server-reported terminal metadata held until Base redeems the ticket once.
#[derive(Debug, Clone)]
pub struct ResultReceipt {
    /// Opaque ticket that names this one-use receipt.
    pub ticket: String,
    /// Original player entity for the launch.
    pub entity_id: u32,
    /// Original character identifier for the launch.
    pub player_id: i32,
    /// Character name captured at launch, for logs only.
    pub player_name: Option<String>,
    /// Original Base connection instance, used to reject relog takeover.
    pub session_instance_id: u64,
    /// Registered minigame name.
    pub game_name: String,
    /// Server terminal code: canceled, victory or defeat.
    pub result_code: u8,
    /// Whether Base should send the client close notification.
    pub notify_client: bool,
    /// Registered victory chains; empty for non-victory results.
    pub on_victory_chains: Vec<i64>,
    /// Registered failure chains (D-MG5); empty unless the result is a defeat.
    pub on_failure_chains: Vec<i64>,
    /// Monotonic issue time used for the receipt TTL.
    pub issued_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DepositOutcome {
    Deposited,
    Refused { player_canceled: bool },
}

impl SessionRegistryInner {
    pub(super) fn invalidate_ticket_receipt(&mut self, ticket: &str) {
        self.receipts.remove(ticket);
        self.receipt_order.retain(|current| current != ticket);
    }

    pub(super) fn invalidate_entity_receipts(&mut self, entity_id: u32) {
        let tickets: Vec<_> = self
            .receipts
            .iter()
            .filter(|(_, receipt)| receipt.entity_id == entity_id)
            .map(|(ticket, _)| ticket.clone())
            .collect();
        for ticket in tickets {
            if let Some(receipt) = self.receipts.get(&ticket) {
                if receipt.result_code == 1 {
                    tracing::warn!(
                        entity_id = receipt.entity_id,
                        entity_name = receipt.player_name.as_deref(),
                        player_id = receipt.player_id,
                        player_name = receipt.player_name.as_deref(),
                        game = %receipt.game_name,
                        chain_count = receipt.on_victory_chains.len(),
                        reason = "victory_receipt_invalidated",
                        "Minigame victory receipt dropped before redemption; its chains will not fire"
                    );
                }
            }
            self.invalidate_ticket_receipt(&ticket);
        }
    }

    pub(super) fn expire_receipts(&mut self, now: Instant) {
        let expired: Vec<_> = self
            .receipt_order
            .iter()
            .filter_map(|ticket| {
                let receipt = self.receipts.get(ticket)?;
                (now.saturating_duration_since(receipt.issued_at) >= RESULT_RECEIPT_TTL)
                    .then(|| ticket.clone())
            })
            .collect();
        for ticket in expired {
            if let Some(receipt) = self.receipts.get(&ticket) {
                tracing::warn!(entity_id = receipt.entity_id,
                    entity_name = receipt.player_name.as_deref(),
                    player_id = receipt.player_id,
                    player_name = receipt.player_name.as_deref(),
                    game = %receipt.game_name, reason = "receipt_expired",
                    age_ms = now.saturating_duration_since(receipt.issued_at).as_millis() as u64,
                    "Minigame result receipt expired");
            }
            self.invalidate_ticket_receipt(&ticket);
        }
    }
}

impl SessionRegistry {
    /// Deposits at most one outcome for the claimed current registration.
    /// The supplied session is an identity claim, never a source for mutable
    /// result metadata; all owner and chain fields come from the registry.
    pub async fn deposit_result(
        &self,
        claimed: &MinigameSession,
        result_code: u8,
        notify_client: bool,
    ) -> bool {
        self.deposit_result_decision(claimed, result_code, notify_client)
            .await
            == DepositOutcome::Deposited
    }

    /// Return cancellation knowledge from the same lock acquisition that
    /// decided whether this terminal result may be deposited.
    pub(crate) async fn deposit_result_decision(
        &self,
        claimed: &MinigameSession,
        result_code: u8,
        notify_client: bool,
    ) -> DepositOutcome {
        let mut inner = self.inner.lock().await;
        let refused = || DepositOutcome::Refused {
            player_canceled: *claimed.cancel.borrow(),
        };
        if result_code > 2 {
            return refused();
        }
        let now = Instant::now();
        inner.expire_receipts(now);
        let Some(session) = inner.sessions.get_mut(&claimed.entity_id) else {
            return refused();
        };
        if session.ticket != claimed.ticket
            || session.player_id != claimed.player_id
            || session.session_instance_id != claimed.session_instance_id
            || !session.connected
            || session.terminal_reported
            || *session.cancel.borrow()
            || session.session_instance_id == 0
        {
            return refused();
        }
        session.terminal_reported = true;
        let receipt = ResultReceipt {
            ticket: session.ticket.clone(),
            entity_id: session.entity_id,
            player_id: session.player_id,
            player_name: session.player_name.clone(),
            session_instance_id: session.session_instance_id,
            game_name: session.game_name.clone(),
            result_code,
            notify_client,
            on_victory_chains: if result_code == RESULT_CODE_VICTORY {
                session.on_victory_chains.clone()
            } else {
                vec![]
            },
            on_failure_chains: if result_code == RESULT_CODE_DEFEAT {
                session.on_failure_chains.clone()
            } else {
                vec![]
            },
            issued_at: now,
        };
        inner.push_receipt(receipt, now);
        DepositOutcome::Deposited
    }

    /// Redeem under the same mutex as deposit, cancel and replacement.
    pub async fn redeem_result(&self, ticket: &str) -> Option<ResultReceipt> {
        let mut inner = self.inner.lock().await;
        inner.expire_receipts(Instant::now());
        let receipt = inner.receipts.get(ticket).cloned();
        inner.invalidate_ticket_receipt(ticket);
        receipt
    }

    /// Failed send may only revoke the ticket it attempted to publish.
    pub async fn revoke_result(&self, ticket: &str) {
        self.inner.lock().await.invalidate_ticket_receipt(ticket);
    }

    #[cfg(test)]
    pub async fn receipt_count(&self) -> usize {
        self.inner.lock().await.receipts.len()
    }

    #[cfg(test)]
    pub(crate) async fn hold_registry_lock_for_test(&self) -> impl Drop + '_ {
        self.inner.lock().await
    }
}

/// Server terminal codes as the receipt stores them.
const RESULT_CODE_VICTORY: u8 = 1;
const RESULT_CODE_DEFEAT: u8 = 2;
/// Interrupted or never started: a pending session the TTL sweep evicted.
pub(super) const RESULT_CODE_NOT_STARTED: u8 = 3;

impl SessionRegistryInner {
    /// Mint the one-use receipt for a pending session the sweep evicted, so the
    /// Base can release the cell's minigame hold (D-MG15) through the same
    /// ticket-only redemption as every other result. No chains ride it.
    pub(super) fn deposit_expiry_receipt(&mut self, session: &MinigameSession) {
        let now = Instant::now();
        let receipt = ResultReceipt {
            ticket: session.ticket.clone(),
            entity_id: session.entity_id,
            player_id: session.player_id,
            player_name: session.player_name.clone(),
            session_instance_id: session.session_instance_id,
            game_name: session.game_name.clone(),
            result_code: RESULT_CODE_NOT_STARTED,
            notify_client: true,
            on_victory_chains: vec![],
            on_failure_chains: vec![],
            issued_at: now,
        };
        self.push_receipt(receipt, now);
    }

    fn push_receipt(&mut self, receipt: ResultReceipt, now: Instant) {
        let inner = self;
        if inner.receipts.len() >= RESULT_RECEIPT_CAP {
            if let Some(oldest) = inner.receipt_order.pop_front() {
                if let Some(evicted) = inner.receipts.remove(&oldest) {
                    tracing::warn!(entity_id = evicted.entity_id,
                        entity_name = evicted.player_name.as_deref(),
                        player_id = evicted.player_id,
                        player_name = evicted.player_name.as_deref(),
                        game = %evicted.game_name, reason = "receipt_capacity",
                        age_ms = now.saturating_duration_since(evicted.issued_at).as_millis() as u64,
                        "Minigame result receipt evicted");
                }
            }
        }
        inner.receipt_order.push_back(receipt.ticket.clone());
        inner.receipts.insert(receipt.ticket.clone(), receipt);
    }
}
