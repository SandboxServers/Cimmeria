//! Refusing to open a vault: the `bank` `vault_open_rejected` event
//! (WARN, D-BV19) and the chat line the player sees. Every refusal does
//! both, so a refused click is never silent (project rule) and is always
//! answerable from SigNoz.

use tokio::sync::mpsc;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Why a vault did not open. [`VaultOpenReject::reason`] is the stable
/// `reason` of `vault_open_rejected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultOpenReject {
    /// The player clicked a Banker from outside the interact distance, or
    /// from another space.
    OutOfRange,
    /// A Team or Command Banker: the org vaults are not built yet (Wave 4).
    OrgVaultNotAvailable,
    /// A player without GM access typed `.bank`.
    NotGm,
    /// The Banker passed the range gate but was gone by the time the arm
    /// looked it up (a lookup miss).
    BankerMissing,
}

impl VaultOpenReject {
    /// The stable `reason` string.
    pub fn reason(self) -> &'static str {
        match self {
            VaultOpenReject::OutOfRange => "out_of_range",
            VaultOpenReject::OrgVaultNotAvailable => "org_vault_not_available",
            VaultOpenReject::NotGm => "not_gm",
            VaultOpenReject::BankerMissing => "banker_missing",
        }
    }

    /// The line the player sees.
    fn feedback(self, scope_label: &str) -> String {
        match self {
            VaultOpenReject::OutOfRange => "You are too far away to use the vault.".to_string(),
            VaultOpenReject::OrgVaultNotAvailable => {
                format!("The {scope_label} vault is not available yet.")
            }
            VaultOpenReject::NotGm => {
                ".bank needs GM access. Visit a Banker to open your vault.".to_string()
            }
            VaultOpenReject::BankerMissing => {
                "That Banker is no longer here. Your vault did not open.".to_string()
            }
        }
    }
}

/// Log `vault_open_rejected` and send the player the matching chat line.
///
/// `banker_id` and `distance` are `None` where there is no Banker (`.bank`)
/// or no position to measure; `tracing` omits a `None` field rather than
/// writing a sentinel. `scope_label` names the vault in the chat line
/// ("Team", "Command"); it is only read for `OrgVaultNotAvailable`.
pub async fn reject_vault_open(
    entity_id: u32,
    reject: VaultOpenReject,
    banker_id: Option<u32>,
    distance: Option<f32>,
    scope_label: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    tracing::warn!(
        target: "bank",
        event = "vault_open_rejected",
        account_id = id.account_id,
        player_id = id.player_id,
        entity_id,
        reason = reject.reason(),
        banker_id,
        distance,
        "vault_open_rejected: the vault did not open -- the player sees a chat line saying why"
    );
    send_bank_feedback(entity_id, &reject.feedback(scope_label), tx, space_mgr).await;
}

/// A single-recipient `SYSTEM` line on the feedback channel, the shape the
/// console and chat-channel refusals use. `onErrorCode` alone is not
/// enough: no client Lua consumes it (AT-E1 §2).
pub(super) async fn send_bank_feedback(
    entity_id: u32,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_PLAYER_COMMUNICATION,
            args: serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
        })
        .await
    {
        let id = space_mgr.player_identity(entity_id);
        tracing::warn!(
            target: "bank",
            event = "bank_feedback_send_failed",
            account_id = id.account_id,
            player_id = id.player_id,
            entity_id,
            reason = "base_channel_closed",
            error = %e,
            "bank feedback line could not be queued (base channel closed) -- \
             the player gets no explanation for the refusal"
        );
    }
}
